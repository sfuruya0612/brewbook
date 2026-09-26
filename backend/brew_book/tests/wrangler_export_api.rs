//! 全記録のエクスポートの結合テスト (HTTP)。
//!
//! FR-14 の受け入れ基準 (6 テーブルのアーカイブ済みを含む全行と全列、パスキーとセッションと
//! チャレンジと登録用トークンの除外、`photo_key` と写真取得 API のパス、写真の実体の除外) と、
//! 成功指標の測定方法 (テスト専用の復元処理で別の利用者に取り込み、`user_id` と `photo_key` の
//! 利用者 ID を付け替えた上で 6 テーブルの全行と全列を比較する) を検査する。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。
//! テストは並行に走るため、記録を書き換えるテストは利用者ごとに分ける。

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use reqwest::blocking::Response;
use serde_json::{json, Map, Value};
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, Seed};
use support::{DevServer, ServerLease};

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const CREATED: &str = "2026-09-01T00:00:00.000Z";
const FUTURE: &str = "2099-01-01T00:00:00.000Z";
const T21: &str = "2026-09-21T00:00:00.000Z";
const T20: &str = "2026-09-20T00:00:00.000Z";
const T19: &str = "2026-09-19T00:00:00.000Z";

/// 下ごしらえに使う日付。
const D21: &str = "2026-09-21";
const D20: &str = "2026-09-20";
const D19: &str = "2026-09-19";

/// 下ごしらえに使う抽出日時。
const B21: &str = "2026-09-21T10:00:00.000Z";
const B20: &str = "2026-09-20T10:00:00.000Z";
const B19: &str = "2026-09-19T10:00:00.000Z";

/// エクスポートに含まれないことの検査に使うチャレンジの値。
const CHALLENGE: &str = "challenge-marker-4f6a";

/// エクスポートの対象の 1 テーブル。
struct Table {
    name: &'static str,
    /// 行を一意にする列。比較の並びに使う。
    key: &'static [&'static str],
    /// 全ての列。スキーマの列の並びと同じ。
    columns: &'static [&'static str],
}

/// 写真のバケット名 (wrangler.toml の R2 バインディングと同じ)。
const PHOTO_BUCKET: &str = "brewbook-photos";

/// エクスポートの対象の 6 テーブル (ADR-0006)。親から子の順に並べる (復元の挿入の順)。
const TABLES: &[Table] = &[
    Table {
        name: "shops",
        key: &["id"],
        columns: &[
            "id",
            "user_id",
            "name",
            "address",
            "created_at",
            "updated_at",
            "archived_at",
        ],
    },
    Table {
        name: "products",
        key: &["id"],
        columns: &[
            "id",
            "user_id",
            "name",
            "producer",
            "origin",
            "region",
            "process",
            "variety",
            "created_at",
            "updated_at",
            "archived_at",
        ],
    },
    Table {
        name: "flavor_tags",
        key: &["id"],
        columns: &["id", "user_id", "name"],
    },
    Table {
        name: "product_flavor_tags",
        key: &["product_id", "tag_id"],
        columns: &["user_id", "product_id", "tag_id"],
    },
    Table {
        name: "purchases",
        key: &["id"],
        columns: &[
            "id",
            "user_id",
            "product_id",
            "shop_id",
            "purchased_on",
            "roast",
            "roast_date",
            "price_amount",
            "price_currency",
            "weight_grams",
            "photo_key",
            "created_at",
            "updated_at",
            "archived_at",
        ],
    },
    Table {
        name: "brews",
        key: &["id"],
        columns: &[
            "id",
            "user_id",
            "purchase_id",
            "brewed_at",
            "dose_grams",
            "water_grams",
            "water_temp_c",
            "brew_time_seconds",
            "method",
            "grind_setting",
            "rating",
            "notes",
            "created_at",
            "updated_at",
            "archived_at",
        ],
    },
];

/// 記録の利用者が持つテーブルごとの行数。下ごしらえの [`records`] と一致させる。
const RECORD_COUNTS: &[(&str, usize)] = &[
    ("shops", 2),
    ("products", 2),
    ("flavor_tags", 3),
    ("product_flavor_tags", 3),
    ("purchases", 3),
    ("brews", 3),
];

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// エクスポートの内容のテストの利用者 (6 テーブルの全列を埋めた記録を持つ)。
    export_user: String,
    export_session: String,
    /// エクスポートの行の ID に使う接頭辞。
    export_prefix: String,
    /// 復元のテストの利用者 (取り込み元) とセッション。
    round_trip_user: String,
    round_trip_session: String,
    /// 復元のテストの取り込み先の利用者。
    restore_user: String,
    /// 他の利用者 (その記録がエクスポートに含まれないことの検査に使う)。行の ID も持つ。
    other_user: String,
    other_session: String,
    other_ids: Vec<String>,
    /// 記録が無い利用者のセッション。
    empty_session: String,
    /// エクスポートに含まれないことの検査に使う、利用者に属する秘密の値。
    secrets: Vec<String>,
}

/// 下ごしらえを 1 回だけ組み立てる。
fn data() -> &'static TestData {
    static DATA: OnceLock<TestData> = OnceLock::new();
    DATA.get_or_init(build_data)
}

/// 共有のサーバーを借りる。
fn server() -> ServerLease {
    support::shared_server("main", || {
        support::DevServer::start_with(|_| Vec::new(), &data().seed_sql)
    })
    .expect("wrangler dev must start")
}

/// 下ごしらえの SQL と、テストが使う値を作る。
fn build_data() -> TestData {
    let mut seed = Seed::new();

    // エクスポートの内容のテストの利用者。パスキー、チャレンジ、登録用トークンも持ち、
    // エクスポートに含まれないことを検査する。
    let export_user = user_id(1);
    seed.user(&export_user, "export user", CREATED);
    let export_session = seed.session(&export_user, FUTURE, CREATED);
    let passkey = seed.passkey(&export_user, "エクスポートのパスキー", CREATED, None);
    let registration_token = seed.registration_token(&export_user, FUTURE);
    seed.challenge(Some(&export_user), "registration", CHALLENGE, FUTURE);
    let export_prefix = "export".to_owned();
    records(&mut seed, &export_user, &export_prefix);

    // 復元のテストの利用者 (取り込み元) と、取り込み先の利用者。
    let round_trip_user = user_id(2);
    seed.user(&round_trip_user, "round trip user", CREATED);
    let round_trip_session = seed.session(&round_trip_user, FUTURE, CREATED);
    let round_trip_prefix = "round-trip".to_owned();
    records(&mut seed, &round_trip_user, &round_trip_prefix);
    let restore_user = user_id(3);
    seed.user(&restore_user, "restore user", CREATED);

    // 他の利用者。6 テーブルに記録を持つ。
    let other_user = user_id(4);
    seed.user(&other_user, "other user", CREATED);
    let other_session = seed.session(&other_user, FUTURE, CREATED);
    let other_prefix = "other".to_owned();
    records(&mut seed, &other_user, &other_prefix);
    let other_ids: Vec<String> = (0..2)
        .map(|index| row_id(&other_prefix, "shop", index))
        .chain((0..2).map(|index| row_id(&other_prefix, "product", index)))
        .chain((0..3).map(|index| row_id(&other_prefix, "tag", index)))
        .chain((0..3).map(|index| row_id(&other_prefix, "purchase", index)))
        .chain((0..3).map(|index| row_id(&other_prefix, "brew", index)))
        .collect();

    // 記録が無い利用者。
    let empty_user = user_id(5);
    seed.user(&empty_user, "empty user", CREATED);
    let empty_session = seed.session(&empty_user, FUTURE, CREATED);

    // エクスポートに含まれない秘密の値。生の値と、データベースに保存されるハッシュを含める。
    let secrets = vec![
        export_session.clone(),
        brew_book_core::auth::hash_secret(&export_session),
        passkey.credential_id.clone(),
        registration_token.clone(),
        brew_book_core::auth::hash_secret(&registration_token),
        CHALLENGE.to_owned(),
    ];

    TestData {
        seed_sql: seed.sql(),
        export_user,
        export_session,
        export_prefix,
        round_trip_user,
        round_trip_session,
        restore_user,
        other_user,
        other_session,
        other_ids,
        empty_session,
        secrets,
    }
}

/// 6 テーブルに全列を埋めた記録を 1 利用者分入れる。行の ID は `<接頭辞>-<テーブル>-<添字>`。
///
/// 任意の列は NULL の行と値のある行を混ぜ、エクスポートが全列を運ぶことを比較で検出できるようにする。
/// 写真の `photo_key` は ADR-0003 のキーの形 (`users/<利用者 ID>/purchases/<購入 ID>/<UUID>.jpg`) にする。
fn records(seed: &mut Seed, user: &str, prefix: &str) {
    // 店。添字 0 は住所あり、添字 1 は住所なしでアーカイブ済み。
    seed.raw(&format!(
        "INSERT INTO shops (id, user_id, name, address, created_at, updated_at, archived_at) \
         VALUES ('{}', '{user}', '{prefix} の店 0 O''Brien', '{prefix} の住所 0', '{T21}', '{T21}', NULL)",
        row_id(prefix, "shop", 0)
    ));
    seed.raw(&format!(
        "INSERT INTO shops (id, user_id, name, address, created_at, updated_at, archived_at) \
         VALUES ('{}', '{user}', '{prefix} の店 1', NULL, '{T20}', '{T20}', '{T20}')",
        row_id(prefix, "shop", 1)
    ));

    // 商品。添字 0 は任意の列を埋め、添字 1 は名前以外を NULL にしてアーカイブ済み。
    seed.raw(&format!(
        "INSERT INTO products (id, user_id, name, producer, origin, region, process, variety, \
         created_at, updated_at, archived_at) VALUES \
         ('{}', '{user}', '{prefix} の商品 0', '{prefix} の生産者 0', 'エチオピア', \
          'イルガチェフェ', 'ウォッシュト', '在来種', '{T21}', '{T21}', NULL)",
        row_id(prefix, "product", 0)
    ));
    seed.raw(&format!(
        "INSERT INTO products (id, user_id, name, producer, origin, region, process, variety, \
         created_at, updated_at, archived_at) VALUES \
         ('{}', '{user}', '{prefix} の商品 1', NULL, NULL, NULL, NULL, NULL, '{T20}', '{T20}', '{T20}')",
        row_id(prefix, "product", 1)
    ));

    // Flavor Notes のタグ。添字 2 はどの商品からも参照されない。
    for (index, name) in ["chocolate", "berry", "unused"].iter().enumerate() {
        seed.raw(&format!(
            "INSERT INTO flavor_tags (id, user_id, name) VALUES ('{}', '{user}', '{prefix}-{name}')",
            row_id(prefix, "tag", index)
        ));
    }
    // 商品とタグの対応。
    seed.raw(&format!(
        "INSERT INTO product_flavor_tags (user_id, product_id, tag_id) VALUES ('{user}', '{}', '{}')",
        row_id(prefix, "product", 0),
        row_id(prefix, "tag", 0)
    ));
    seed.raw(&format!(
        "INSERT INTO product_flavor_tags (user_id, product_id, tag_id) VALUES ('{user}', '{}', '{}')",
        row_id(prefix, "product", 0),
        row_id(prefix, "tag", 1)
    ));
    seed.raw(&format!(
        "INSERT INTO product_flavor_tags (user_id, product_id, tag_id) VALUES ('{user}', '{}', '{}')",
        row_id(prefix, "product", 1),
        row_id(prefix, "tag", 0)
    ));

    // 購入。添字 0 は全列を埋め、添字 1 は店と任意の列を NULL にし、添字 2 はアーカイブ済み。
    seed.raw(&format!(
        "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, roast, roast_date, \
         price_amount, price_currency, weight_grams, photo_key, created_at, updated_at, archived_at) \
         VALUES ('{}', '{user}', '{}', '{}', '{D21}', '中煎り', '{D19}', 1200, 'JPY', 200, '{}', \
          '{T21}', '{T21}', NULL)",
        row_id(prefix, "purchase", 0),
        row_id(prefix, "product", 0),
        row_id(prefix, "shop", 0),
        photo_key(user, prefix, 0)
    ));
    seed.raw(&format!(
        "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, roast, roast_date, \
         price_amount, price_currency, weight_grams, photo_key, created_at, updated_at, archived_at) \
         VALUES ('{}', '{user}', '{}', NULL, '{D20}', NULL, NULL, NULL, NULL, NULL, NULL, \
          '{T20}', '{T20}', NULL)",
        row_id(prefix, "purchase", 1),
        row_id(prefix, "product", 1)
    ));
    seed.raw(&format!(
        "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, roast, roast_date, \
         price_amount, price_currency, weight_grams, photo_key, created_at, updated_at, archived_at) \
         VALUES ('{}', '{user}', '{}', '{}', '{D19}', NULL, NULL, NULL, NULL, NULL, '{}', \
          '{T19}', '{T19}', '{T19}')",
        row_id(prefix, "purchase", 2),
        row_id(prefix, "product", 0),
        row_id(prefix, "shop", 1),
        photo_key(user, prefix, 2)
    ));

    // 抽出。添字 0 は全列を埋め、添字 1 は任意の列を NULL にし、添字 2 はアーカイブ済み。
    seed.raw(&format!(
        "INSERT INTO brews (id, user_id, purchase_id, brewed_at, dose_grams, water_grams, \
         water_temp_c, brew_time_seconds, method, grind_setting, rating, notes, created_at, \
         updated_at, archived_at) VALUES \
         ('{}', '{user}', '{}', '{B21}', 15.5, 250.5, 92.5, 150, 'ペーパードリップ', '中細', 4, \
          '良い出来', '{T21}', '{T21}', NULL)",
        row_id(prefix, "brew", 0),
        row_id(prefix, "purchase", 0)
    ));
    seed.raw(&format!(
        "INSERT INTO brews (id, user_id, purchase_id, brewed_at, dose_grams, water_grams, \
         water_temp_c, brew_time_seconds, method, grind_setting, rating, notes, created_at, \
         updated_at, archived_at) VALUES \
         ('{}', '{user}', '{}', '{B20}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, \
          '{T20}', '{T20}', NULL)",
        row_id(prefix, "brew", 1),
        row_id(prefix, "purchase", 1)
    ));
    seed.raw(&format!(
        "INSERT INTO brews (id, user_id, purchase_id, brewed_at, dose_grams, water_grams, \
         water_temp_c, brew_time_seconds, method, grind_setting, rating, notes, created_at, \
         updated_at, archived_at) VALUES \
         ('{}', '{user}', '{}', '{B19}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, \
          '{T19}', '{T19}', '{T19}')",
        row_id(prefix, "brew", 2),
        row_id(prefix, "purchase", 2)
    ));
}

/// 記録の行の ID。
fn row_id(prefix: &str, kind: &str, index: usize) -> String {
    format!("{prefix}-{kind}-{index}")
}

/// 写真のオブジェクトキー (ADR-0003 のキーの形)。
fn photo_key(user: &str, prefix: &str, index: usize) -> String {
    format!(
        "users/{user}/purchases/{}/a1b2c3d4-0000-4000-8000-{index:012}.jpg",
        row_id(prefix, "purchase", index)
    )
}

/// セッションの Cookie を持たないクライアント。
fn anonymous(base_url: &str) -> ApiClient {
    ApiClient::new(base_url, None)
}

/// 応答の状態コードと本体を確かめる。
fn assert_status(response: Response, expected: u16) -> Value {
    let (status, body) = read(response);
    assert_eq!(status, expected, "the response body was {body}");
    body
}

/// 応答のヘッダーを 1 つ読む。
fn header(response: &Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// 未認証の呼び出しが 401 になることを確かめる。
fn assert_unauthorized(response: Response) {
    let body = assert_status(response, 401);
    assert_eq!(error_code(&body), Some("unauthorized"));
}

/// 名前でテーブルの定義を引く。
fn table(name: &str) -> &'static Table {
    TABLES
        .iter()
        .find(|table| table.name == name)
        .unwrap_or_else(|| panic!("the table {name} must be in the export"))
}

/// エクスポートの応答のテーブルの行を返す。
fn table_rows<'a>(body: &'a Value, table: &Table) -> &'a Vec<Value> {
    body[table.name]
        .as_array()
        .unwrap_or_else(|| panic!("the {} must be an array: {body}", table.name))
}

/// テーブルの行のうち、ID が一致する 1 行を返す。
fn find_row(table: &Table, rows: &[Value], id: &str) -> Value {
    rows.iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("the {} row {id} must be present", table.name))
        .clone()
}

/// エクスポートの行がテーブルの全列 (と `extra` の列) だけを持つことを確かめる。
fn assert_row_columns(table: &Table, row: &Value, extra: &[&str]) {
    let actual: BTreeSet<&str> = row
        .as_object()
        .unwrap_or_else(|| panic!("the {} row must be an object: {row}", table.name))
        .keys()
        .map(String::as_str)
        .collect();
    let mut expected: BTreeSet<&str> = table.columns.iter().copied().collect();
    expected.extend(extra.iter().copied());
    assert_eq!(
        actual, expected,
        "the {} row must carry every column and nothing else: {row}",
        table.name
    );
}

/// エクスポートの行からテーブルの列だけを取り出す (`photo_path` などの列以外を除く)。
fn table_columns(table: &Table, row: &Value) -> Value {
    let row = row
        .as_object()
        .unwrap_or_else(|| panic!("the {} row must be an object: {row}", table.name));
    let columns: Map<String, Value> = table
        .columns
        .iter()
        .map(|column| {
            let value = row
                .get(*column)
                .unwrap_or_else(|| panic!("the {} row must have the column {column}", table.name));
            ((*column).to_owned(), value.clone())
        })
        .collect();
    Value::Object(columns)
}

/// 行の主キーを比較用の文字列にする。
fn key_string(table: &Table, row: &Value) -> String {
    table
        .key
        .iter()
        .map(|column| row.get(*column).map(Value::to_string).unwrap_or_default())
        .collect::<Vec<String>>()
        .join("\u{1f}")
}

/// テーブルの行を主キーで並べる (比較の順を安定させる)。
fn sorted(table: &Table, rows: &[Value]) -> Vec<Value> {
    let mut rows = rows.to_vec();
    rows.sort_by_key(|row| key_string(table, row));
    rows
}

/// データベースの利用者の 6 テーブルの行を読む。
fn snapshot(server: &DevServer, user: &str) -> BTreeMap<&'static str, Vec<Value>> {
    let mut tables = BTreeMap::new();
    for table in TABLES {
        let sql = format!(
            "SELECT {} FROM {} WHERE user_id = '{user}'",
            table.columns.join(", "),
            table.name
        );
        let rows = server
            .query_rows(&sql)
            .unwrap_or_else(|error| panic!("the {} rows must be readable: {error}", table.name));
        tables.insert(table.name, rows);
    }
    tables
}

/// `photo_key` に含まれる利用者 ID を取り込み先に付け替える (ADR-0003 のキーの形)。
fn reassign_photo_key(key: &str, from_user: &str, to_user: &str) -> String {
    let prefix = format!("users/{from_user}/");
    match key.strip_prefix(&prefix) {
        Some(rest) => format!("users/{to_user}/{rest}"),
        None => key.to_owned(),
    }
}

/// 行の `user_id` と `photo_key` の利用者 ID を取り込み先に付け替える。
fn reassign_rows(table: &Table, rows: &[Value], from_user: &str, to_user: &str) -> Vec<Value> {
    rows.iter()
        .map(|row| {
            let mut row = row
                .as_object()
                .unwrap_or_else(|| panic!("the {} row must be an object: {row}", table.name))
                .clone();
            row.insert("user_id".to_owned(), json!(to_user));
            if table.columns.contains(&"photo_key") {
                if let Some(Value::String(key)) = row.get("photo_key").cloned() {
                    row.insert(
                        "photo_key".to_owned(),
                        json!(reassign_photo_key(&key, from_user, to_user)),
                    );
                }
            }
            Value::Object(row)
        })
        .collect()
}

/// テスト専用の復元処理 (成功指標の測定方法)。
///
/// エクスポートの JSON を、取り込み先の利用者の行の SQL に組み立てる。`user_id` と `photo_key` に
/// 含まれる利用者 ID を取り込み先に付け替え、行の ID はそのまま保つ (全列の比較のため)。
/// 主キーが衝突しないよう、取り込み元の行を子から順に消してから、親から順に入れる。
fn restore_statements(export: &Value, from_user: &str, to_user: &str) -> Vec<String> {
    let mut statements = Vec::new();
    for table in TABLES.iter().rev() {
        statements.push(format!(
            "DELETE FROM {} WHERE user_id = '{from_user}'",
            table.name
        ));
    }
    for table in TABLES {
        for row in table_rows(export, table) {
            let row = row
                .as_object()
                .unwrap_or_else(|| panic!("the {} row must be an object: {row}", table.name));
            let values: Vec<String> = table
                .columns
                .iter()
                .map(|column| match *column {
                    // 利用者の付け替え。写真のキーは利用者 ID を含む (ADR-0003)。
                    "user_id" => literal_text(to_user),
                    "photo_key" => match row.get("photo_key") {
                        Some(Value::String(key)) => {
                            literal_text(&reassign_photo_key(key, from_user, to_user))
                        }
                        _ => "NULL".to_owned(),
                    },
                    other => row.get(other).map_or_else(|| "NULL".to_owned(), literal),
                })
                .collect();
            statements.push(format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table.name,
                table.columns.join(", "),
                values.join(", ")
            ));
        }
    }
    statements
}

/// JSON の値を SQL のリテラルにする。列の無い値は NULL にする。
fn literal(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_owned(),
        Value::String(text) => literal_text(text),
        Value::Number(number) => number.to_string(),
        other => panic!("the export must not have the value {other}"),
    }
}

/// 文字列を SQL のリテラルにする。
fn literal_text(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// データベースの行を基準に、エクスポートの全行と全列が一致することを確かめる。
fn assert_export_matches_source(export: &Value, source: &BTreeMap<&'static str, Vec<Value>>) {
    for table in TABLES {
        let rows: Vec<Value> = table_rows(export, table)
            .iter()
            .map(|row| table_columns(table, row))
            .collect();
        assert_eq!(
            sorted(table, &rows),
            sorted(table, &source[table.name]),
            "the exported {} rows must match the database rows: {export}",
            table.name
        );
    }
}

#[test]
fn wrangler_export_ok() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, Some(&data.export_session));

    // 写真の実体が応答に含まれないことを確かめるため、写真の位置に印を付けたオブジェクトを置く。
    let photo_marker = "EXPORT-PHOTO-BODY-9a2c";
    let key = photo_key(&data.export_user, &data.export_prefix, 0);
    lease.use_server(|server| {
        server
            .put_r2_object(PHOTO_BUCKET, &key, photo_marker.as_bytes(), "image/jpeg")
            .expect("the photo object must be placed");
    });

    let response = client.get("/api/export");
    // 応答は JSON のダウンロードにする (設計判断)。
    assert_eq!(
        header(&response, "content-disposition").as_deref(),
        Some("attachment; filename=\"brewbook-export.json\"")
    );
    let content_type = header(&response, "content-type").unwrap_or_default();
    assert!(
        content_type.starts_with("application/json"),
        "the export must be JSON but was {content_type}"
    );
    let body = assert_status(response, 200);

    // トップレベルは 6 テーブルの名前を持つ配列だけにする (追加のメタデータを入れない)。
    let names: BTreeSet<&str> = body
        .as_object()
        .expect("the export must be an object")
        .keys()
        .map(String::as_str)
        .collect();
    let expected: BTreeSet<&str> = TABLES.iter().map(|table| table.name).collect();
    assert_eq!(
        names, expected,
        "the export must have the six tables: {body}"
    );

    // 6 テーブルのアーカイブ済みを含む全行と全列が含まれる (FR-14)。
    for table in TABLES {
        let rows = table_rows(&body, table);
        let count = RECORD_COUNTS
            .iter()
            .find(|(name, _)| *name == table.name)
            .map(|(_, count)| *count)
            .expect("every table must have an expected count");
        assert_eq!(rows.len(), count, "the {} rows: {body}", table.name);
        // 応答は主キーの昇順で並ぶ (再現性のある出力)。隣接する行のキーを比べる。
        for pair in rows.windows(2) {
            let left = key_string(table, &pair[0]);
            let right = key_string(table, &pair[1]);
            assert!(
                left < right,
                "the {} rows must be ordered by the key: {left} >= {right}",
                table.name
            );
        }
        for row in rows {
            let extra: &[&str] = if table.name == "purchases" {
                &["photo_path"]
            } else {
                &[]
            };
            assert_row_columns(table, row, extra);
            // エクスポートは呼び出した利用者の記録だけを含む (FR-5)。
            assert_eq!(
                row["user_id"], data.export_user,
                "the {} row must be the caller's: {body}",
                table.name
            );
        }
        // アーカイブ済みの行も含まれる (FR-14)。
        if table.columns.contains(&"archived_at") {
            assert!(
                rows.iter().any(|row| row["archived_at"].is_string()),
                "the archived {} rows must be exported: {body}",
                table.name
            );
        }
    }

    // 購入の行は photo_key と写真取得 API のパスを持つ (ADR-0003)。
    let purchases = table("purchases");
    let rows = table_rows(&body, purchases);
    let with_photo = find_row(purchases, rows, &row_id(&data.export_prefix, "purchase", 0));
    assert_eq!(
        with_photo["photo_key"],
        json!(photo_key(&data.export_user, &data.export_prefix, 0))
    );
    assert_eq!(
        with_photo["photo_path"],
        json!(format!(
            "/api/purchases/{}/photo",
            row_id(&data.export_prefix, "purchase", 0)
        ))
    );
    // 写真が無い購入でも写真取得 API のパスは含める。
    let without_photo = find_row(purchases, rows, &row_id(&data.export_prefix, "purchase", 1));
    assert_eq!(without_photo["photo_key"], Value::Null, "{body}");
    assert_eq!(
        without_photo["photo_path"],
        json!(format!(
            "/api/purchases/{}/photo",
            row_id(&data.export_prefix, "purchase", 1)
        ))
    );

    // 写真の実体と署名付き GET URL は含めない (ADR-0003)。
    // 実体の除外は、photo_key の位置に印を付けたオブジェクトを R2 に置いて確かめる。
    let text = body.to_string();
    for marker in [
        "X-Amz",
        "Signature",
        "photo_url",
        "photo_data",
        "base64",
        photo_marker,
    ] {
        assert!(
            !text.contains(marker),
            "the export must not carry the photo body or a signed URL: {marker}"
        );
    }
    assert!(
        !with_photo["photo_path"]
            .as_str()
            .expect("photo_path must be a string")
            .contains('?'),
        "the photo path must not carry a query string: {with_photo}"
    );
}

#[test]
fn wrangler_export_covers_every_column_of_the_schema() {
    // エクスポートの列の一覧が migration のスキーマと一致することを確かめる (列の追加漏れの検出)。
    let lease = server();
    lease.use_server(|server| {
        for table in TABLES {
            let rows = server
                .query_rows(&format!(
                    "SELECT name FROM pragma_table_info('{}') ORDER BY cid",
                    table.name
                ))
                .expect("the schema must be readable");
            let mut names: Vec<&str> = rows.iter().filter_map(|row| row["name"].as_str()).collect();
            let mut expected = table.columns.to_vec();
            // 列の並びは物理順と論理順で違いうる (migration 0002 は price_currency を末尾に移す) ため、
            // 名前の集合で照合し、列の追加と削除を検出する。
            names.sort_unstable();
            expected.sort_unstable();
            assert_eq!(
                names, expected,
                "the {} columns must match the schema",
                table.name
            );
        }
    });
}

#[test]
fn wrangler_export_returns_only_own_records() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());

    // 他の利用者の記録は含まれない (FR-5)。
    let body = assert_status(
        ApiClient::new(&base_url, Some(&data.export_session)).get("/api/export"),
        200,
    );
    let text = body.to_string();
    for id in &data.other_ids {
        assert!(
            !text.contains(id.as_str()),
            "the other user's row {id} must not be exported"
        );
    }
    // 他の利用者のエクスポートにも、その利用者の記録だけが含まれる。
    let other = assert_status(
        ApiClient::new(&base_url, Some(&data.other_session)).get("/api/export"),
        200,
    );
    for table in TABLES {
        let rows = table_rows(&other, table);
        assert_eq!(
            rows.len(),
            RECORD_COUNTS
                .iter()
                .find(|(name, _)| *name == table.name)
                .map(|(_, count)| *count)
                .expect("every table must have an expected count"),
            "the other user's {} rows: {other}",
            table.name
        );
        assert!(
            rows.iter().all(|row| row["user_id"] == data.other_user),
            "the export must carry only the caller's records: {other}"
        );
    }

    // 記録が無い利用者には 6 テーブルの空の配列を返す。
    let empty = assert_status(
        ApiClient::new(&base_url, Some(&data.empty_session)).get("/api/export"),
        200,
    );
    for table in TABLES {
        assert_eq!(
            empty[table.name],
            json!([]),
            "the {} of a user without records must be empty: {empty}",
            table.name
        );
    }

    // パスキー、セッション、チャレンジ、登録用トークンは含まれない (FR-14)。
    // 検査に使う秘密の値がデータベースにあることを先に確かめる。
    let lease = server();
    for (table, user) in [
        ("passkey_credentials", &data.export_user),
        ("sessions", &data.export_user),
        ("webauthn_challenges", &data.export_user),
        ("registration_tokens", &data.export_user),
    ] {
        let count = lease.use_server(|server| {
            server.query_int(&format!(
                "SELECT COUNT(*) FROM {table} WHERE user_id = '{user}'"
            ))
        });
        assert_eq!(count, Ok(1), "the test data must have the {table} row");
    }
    for secret in &data.secrets {
        assert!(
            !text.contains(secret.as_str()),
            "the export must not contain the secret {secret}"
        );
    }
}

#[test]
fn wrangler_export_unauthenticated_401() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    assert_unauthorized(anonymous(&base_url).get("/api/export"));
}

#[test]
fn wrangler_export_round_trip_restores_every_row_and_column() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, Some(&data.round_trip_session));

    // 元の記録をデータベースから読み、比較の基準にする。
    let source = lease.use_server(|server| snapshot(server, &data.round_trip_user));
    let export = assert_status(client.get("/api/export"), 200);

    // エクスポートの JSON が、元の記録の 6 テーブルの全行と全列を持つ (FR-14)。
    assert_export_matches_source(&export, &source);

    // テスト専用の復元処理で別の利用者に取り込む (成功指標の測定方法)。
    let statements = restore_statements(&export, &data.round_trip_user, &data.restore_user);
    lease.use_server(|server| {
        server
            .execute_sql_file(&statements)
            .expect("the restore must succeed")
    });
    let restored = lease.use_server(|server| snapshot(server, &data.restore_user));

    // user_id と photo_key の利用者 ID を付け替えた上で、6 テーブルの全行と全列が一致する。
    for table in TABLES {
        let expected = reassign_rows(
            table,
            &source[table.name],
            &data.round_trip_user,
            &data.restore_user,
        );
        assert_eq!(
            sorted(table, &restored[table.name]),
            sorted(table, &expected),
            "the restored {} rows must match the source rows",
            table.name
        );
    }

    // 写真は付け替え後の photo_key と件数の一致で検証する (成功指標の測定方法)。
    let purchases = table("purchases");
    let photos = |rows: &[Value]| {
        rows.iter()
            .filter(|row| row["photo_key"].is_string())
            .count()
    };
    let source_photos = photos(&source[purchases.name]);
    assert_eq!(
        source_photos, 2,
        "the test data must have two purchases with a photo"
    );
    assert_eq!(photos(&restored[purchases.name]), source_photos);
    // 付け替えた photo_key がデータベースに入っている。
    let restored_text = Value::Array(restored[purchases.name].clone()).to_string();
    assert!(
        restored_text.contains(&format!("users/{}/", data.restore_user)),
        "the restored photo keys must carry the destination user: {restored_text}"
    );
    assert!(
        !restored_text.contains(&data.round_trip_user),
        "the restored photo keys must not carry the source user: {restored_text}"
    );
}
