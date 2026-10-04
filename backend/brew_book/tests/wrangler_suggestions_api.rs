//! 過去の入力値のサジェストの API の結合テスト (HTTP)。
//!
//! 8 つの項目とテーブルの対応、前後の空白を除いた前方一致 (大文字と小文字を区別しない)、
//! 重複の除去と最大 20 件、`updated_at` の降順と値の Unicode コードポイントの昇順、`q` が空の
//! ときの全値の返却、他の利用者の除外、`%` と `_` と `\` を文字として
//! 扱うこと、項目名と `q` の入力不正の 400、候補に無い値の入力を妨げないことを確認する。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。
//! テストは並行に走るため、記録を書き換えるテストは利用者ごとに分ける。

mod support;

use std::sync::OnceLock;

use brew_book_core::datetime::format_epoch_millis;
use reqwest::blocking::Response;
use serde_json::{json, Value};
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, BrewTexts, ProductTexts, Seed};
use support::ServerLease;

/// 下ごしらえに使う更新日時。値の並び順を決める。
const T22: &str = "2026-09-22T00:00:00.000Z";
const T21: &str = "2026-09-21T00:00:00.000Z";
const T20: &str = "2026-09-20T00:00:00.000Z";
const T19: &str = "2026-09-19T00:00:00.000Z";
const T18: &str = "2026-09-18T00:00:00.000Z";
const T17: &str = "2026-09-17T00:00:00.000Z";
const T16: &str = "2026-09-16T00:00:00.000Z";
const T15: &str = "2026-09-15T00:00:00.000Z";
const T14: &str = "2026-09-14T00:00:00.000Z";
const T13: &str = "2026-09-13T00:00:00.000Z";

/// 下ごしらえに使う購入日。
const D21: &str = "2026-09-21";
const D18: &str = "2026-09-18";
const D17: &str = "2026-09-17";

/// 下ごしらえに使う抽出日時。
const B21: &str = "2026-09-21T10:00:00.000Z";
const B20: &str = "2026-09-20T10:00:00.000Z";
const B19: &str = "2026-09-19T10:00:00.000Z";
const B18: &str = "2026-09-18T10:00:00.000Z";

/// 最大 20 件を超える候補の件数。
const OVER_LIMIT_COUNT: usize = 25;
/// 上限を超える候補の更新日時の基準 (2027-01-15T08:00:00.000Z)。添字の分だけ進める。
const OVER_LIMIT_BASE_MILLIS: i64 = 1_800_000_000_000;
/// 1 分のミリ秒。
const MILLIS_PER_MINUTE: i64 = 60_000;

/// 8 つの項目の名前 (FR-13)。
const FIELDS: [&str; 8] = [
    "producer",
    "origin",
    "region",
    "process",
    "variety",
    "roast",
    "method",
    "grind_setting",
];

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// サジェストの利用者のセッション。
    session: String,
    /// 他の利用者 (候補の除外の検査に使う) のセッション。
    other_session: String,
    /// 記録が無い利用者のセッション。
    empty_session: String,
    /// 候補に無い値を登録する利用者 (書き換えの検査に使う) のセッション。
    free_session: String,
    /// 空白だけの自由記述の利用者 (空文字の候補の検査に使う) のセッション。
    space_session: String,
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

/// 上限を超える候補の更新日時。添字とともに 1 分ずつ進める。
fn over_limit_timestamp(index: usize) -> String {
    format_epoch_millis(OVER_LIMIT_BASE_MILLIS + index as i64 * MILLIS_PER_MINUTE)
        .expect("the timestamp must format")
}

/// 下ごしらえの SQL と、テストが使う値を作る。
fn build_data() -> TestData {
    let future = "2099-01-01T00:00:00.000Z";
    let created = "2026-09-01T00:00:00.000Z";
    let mut seed = Seed::new();

    // サジェストの利用者。商品、購入、抽出に自由記述の値を持たせる。
    let user = user_id(1);
    seed.user(&user, "suggestion user", created);
    let session = seed.session(&user, future, created);

    // 商品。Producer、Origin、Region、Process、Variety の候補の検査に使う。
    seed.product_with_texts(
        &user,
        "モカの豆",
        ProductTexts {
            producer: Some("エチオピア モカ"),
            origin: Some("エチオピア"),
            region: Some("イルガチェフェ"),
            process: Some("ウォッシュド"),
            variety: Some("ティピカ"),
        },
        T21,
        T21,
    );
    seed.product_with_texts(
        &user,
        "シダモの豆",
        ProductTexts {
            producer: Some("エチオピア シダモ"),
            origin: Some("エチオピア"),
            region: Some("シダモ"),
            variety: Some("ブルボン"),
            ..ProductTexts::default()
        },
        T20,
        T20,
    );
    // 大文字と小文字を区別しない前方一致の検査に使う。
    seed.product_with_texts(
        &user,
        "エチオピアの豆",
        ProductTexts {
            producer: Some("Ethiopia"),
            origin: Some("Ethiopia"),
            ..ProductTexts::default()
        },
        T19,
        T19,
    );
    seed.product_with_texts(
        &user,
        "ゲイシャの豆",
        ProductTexts {
            variety: Some("ゲイシャ"),
            ..ProductTexts::default()
        },
        T19,
        T19,
    );
    // 同じ値の 2 件目が新しいとき、まとめた値は新しい方の更新日時を持つ (FR-13)。
    seed.product_with_texts(
        &user,
        "もう一つのブルボンの豆",
        ProductTexts {
            variety: Some("ブルボン"),
            ..ProductTexts::default()
        },
        T22,
        T22,
    );
    seed.product_with_texts(
        &user,
        "別の産地の豆",
        ProductTexts {
            producer: Some("グアテマラ"),
            origin: Some("グアテマラ"),
            ..ProductTexts::default()
        },
        T18,
        T18,
    );
    // 自由記述の値が無い商品は候補に出ない。
    let plain_product = seed.product(&user, "値の無い豆", T17, T17);
    // `%` と `_` と `\` を文字として扱うことの検査に使う。
    // ワイルドカードとして扱われると、先頭が一致するだけの値 (100X アラビカ、abc ロット) や
    // 全ての値が混ざる。
    seed.product_with_texts(
        &user,
        "百分率の豆",
        ProductTexts {
            producer: Some("100% アラビカ"),
            ..ProductTexts::default()
        },
        T16,
        T16,
    );
    seed.product_with_texts(
        &user,
        "紛らわしい豆",
        ProductTexts {
            producer: Some("100X アラビカ"),
            ..ProductTexts::default()
        },
        T16,
        T16,
    );
    seed.product_with_texts(
        &user,
        "百分率から始まる豆",
        ProductTexts {
            producer: Some("%入りの生産者"),
            ..ProductTexts::default()
        },
        T16,
        T16,
    );
    seed.product_with_texts(
        &user,
        "下線の豆",
        ProductTexts {
            producer: Some("a_b ロット"),
            ..ProductTexts::default()
        },
        T15,
        T15,
    );
    seed.product_with_texts(
        &user,
        "下線でない豆",
        ProductTexts {
            producer: Some("abc ロット"),
            ..ProductTexts::default()
        },
        T15,
        T15,
    );
    seed.product_with_texts(
        &user,
        "下線から始まる豆",
        ProductTexts {
            producer: Some("_入りの生産者"),
            ..ProductTexts::default()
        },
        T15,
        T15,
    );
    seed.product_with_texts(
        &user,
        "円記号の豆",
        ProductTexts {
            producer: Some("C:\\豆"),
            ..ProductTexts::default()
        },
        T14,
        T14,
    );
    seed.product_with_texts(
        &user,
        "紛らわしい円記号の豆",
        ProductTexts {
            producer: Some("CX豆"),
            ..ProductTexts::default()
        },
        T14,
        T14,
    );
    seed.product_with_texts(
        &user,
        "円記号から始まる豆",
        ProductTexts {
            producer: Some("\\入りの生産者"),
            ..ProductTexts::default()
        },
        T14,
        T14,
    );
    // 同じ値の重複は 1 つにまとめる (FR-13)。
    seed.product_with_texts(
        &user,
        "重複の豆 1",
        ProductTexts {
            producer: Some("重複候補"),
            ..ProductTexts::default()
        },
        T21,
        T21,
    );
    seed.product_with_texts(
        &user,
        "重複の豆 2",
        ProductTexts {
            producer: Some("重複候補"),
            ..ProductTexts::default()
        },
        T20,
        T20,
    );
    // 更新日時が同じときは値の昇順 (Unicode コードポイント) で並べる (FR-13)。
    seed.product_with_texts(
        &user,
        "並びの豆 B",
        ProductTexts {
            producer: Some("並び B"),
            ..ProductTexts::default()
        },
        T13,
        T13,
    );
    seed.product_with_texts(
        &user,
        "並びの豆 A",
        ProductTexts {
            producer: Some("並び A"),
            ..ProductTexts::default()
        },
        T13,
        T13,
    );
    // 21 件以上あるときは、更新日時の新しい順に先頭の 20 件を返す (FR-13)。
    for index in 0..OVER_LIMIT_COUNT {
        let producer = format!("順序 {index:02}");
        let updated_at = over_limit_timestamp(index);
        seed.product_with_texts(
            &user,
            &format!("並び順の豆 {index:02}"),
            ProductTexts {
                producer: Some(&producer),
                ..ProductTexts::default()
            },
            &updated_at,
            &updated_at,
        );
    }

    // 購入。Roast の候補の検査に使う。
    seed.purchase_with_roast(
        &user,
        &plain_product.id,
        None,
        D21,
        Some("中煎り"),
        T21,
        T21,
    );
    seed.purchase_with_roast(
        &user,
        &plain_product.id,
        None,
        D21,
        Some("中深煎り"),
        T20,
        T20,
    );
    seed.purchase_with_roast(&user, &plain_product.id, None, D21, Some("City"), T19, T19);
    // 自由記述の値が無い購入は候補に出ない。
    let plain_purchase =
        seed.purchase_with_roast(&user, &plain_product.id, None, D18, None, T18, T18);
    seed.purchase_with_roast(
        &user,
        &plain_product.id,
        None,
        D17,
        Some("深煎り"),
        T17,
        T17,
    );

    // 抽出。抽出方法と挽き目の候補の検査に使う。
    seed.brew_with_texts(
        &user,
        &plain_purchase.id,
        B21,
        BrewTexts {
            method: Some("ペーパードリップ"),
            grind_setting: Some("中細"),
        },
        T21,
        T21,
    );
    seed.brew_with_texts(
        &user,
        &plain_purchase.id,
        B20,
        BrewTexts {
            method: Some("フレンチプレス"),
            grind_setting: Some("粗挽き"),
        },
        T20,
        T20,
    );
    // 自由記述の値が無い抽出は候補に出ない。
    seed.brew_with_texts(
        &user,
        &plain_purchase.id,
        B19,
        BrewTexts::default(),
        T19,
        T19,
    );
    seed.brew_with_texts(
        &user,
        &plain_purchase.id,
        B18,
        BrewTexts {
            method: Some("エスプレッソ"),
            grind_setting: Some("極細"),
        },
        T18,
        T18,
    );

    // 他の利用者。自分の候補だけが返ることの検査に使う。
    let other_user = user_id(2);
    seed.user(&other_user, "other user", created);
    let other_session = seed.session(&other_user, future, created);
    let other_product = seed.product_with_texts(
        &other_user,
        "他人の豆",
        ProductTexts {
            producer: Some("他人の生産者"),
            origin: Some("他人の産地"),
            ..ProductTexts::default()
        },
        T21,
        T21,
    );
    let other_purchase = seed.purchase_with_roast(
        &other_user,
        &other_product.id,
        None,
        D21,
        Some("他人のロースト"),
        T21,
        T21,
    );
    seed.brew_with_texts(
        &other_user,
        &other_purchase.id,
        B21,
        BrewTexts {
            method: Some("他人の抽出方法"),
            grind_setting: Some("他人の挽き目"),
        },
        T21,
        T21,
    );

    // 記録が無い利用者。
    let empty_user = user_id(3);
    seed.user(&empty_user, "empty user", created);
    let empty_session = seed.session(&empty_user, future, created);

    // 候補に無い値を登録する利用者。
    let free_user = user_id(4);
    seed.user(&free_user, "free user", created);
    let free_session = seed.session(&free_user, future, created);

    // 空白だけの自由記述を保存する利用者 (空文字の候補の検査)。
    let space_user = user_id(5);
    seed.user(&space_user, "space user", created);
    let space_session = seed.session(&space_user, future, created);

    TestData {
        seed_sql: seed.sql(),
        session,
        other_session,
        empty_session,
        free_session,
        space_session,
    }
}

/// セッションの Cookie を持たないクライアント。
fn anonymous(base_url: &str) -> ApiClient {
    ApiClient::new(base_url, None)
}

/// サジェストの項目の経路。`q` をパーセントエンコードして載せる。
fn suggestion_path(field: &str, q: &str) -> String {
    format!("/api/suggestions/{field}?q={}", encode_query(q))
}

/// `q` の値を URL に載せる形 (パーセントエンコード) にする。
/// テストの値には `%` と `_` と `\` と日本語を含めるため、自前でエンコードする。
fn encode_query(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// 応答の状態コードと本体を確かめる。
fn assert_status(response: Response, expected: u16) -> Value {
    let (status, body) = read(response);
    assert_eq!(status, expected, "the response body was {body}");
    body
}

/// 未認証の呼び出しが 401 になることを確かめる。
fn assert_unauthorized(response: Response) {
    let body = assert_status(response, 401);
    assert_eq!(error_code(&body), Some("unauthorized"));
}

/// 入力不正の呼び出しが 400 になることを確かめる。
fn assert_bad_request(response: Response) {
    let body = assert_status(response, 400);
    assert_eq!(error_code(&body), Some("bad_request"), "{body}");
}

/// 対象が無い呼び出しが 404 になることを確かめる。
fn assert_not_found(response: Response) {
    let body = assert_status(response, 404);
    assert_eq!(error_code(&body), Some("not_found"), "{body}");
}

/// サジェストの応答の候補の値を確かめる。
fn assert_values(body: &Value, expected: &[&str]) {
    assert_eq!(body["values"], json!(expected), "{body}");
}

mod suggestions {
    //! サジェストの経路のテスト (FR-13、FR-5)。

    use super::*;

    // 正常系 (認証が必要、入力あり)。

    #[test]
    fn wrangler_suggestions_list_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 8 つの項目はどれも候補を返す (FR-13)。
        for field in FIELDS {
            let body = assert_status(client.get(&format!("/api/suggestions/{field}")), 200);
            let values = body["values"]
                .as_array()
                .expect("the response must have values");
            assert!(
                !values.is_empty(),
                "the field {field} must have values: {body}"
            );
            assert!(
                values.iter().all(|value| value.is_string()),
                "the values must be strings: {body}"
            );
        }
        // 21 件以上あるときは先頭の 20 件を返し、更新日時が最も新しい値が先頭になる (FR-13)。
        let body = assert_status(client.get("/api/suggestions/producer"), 200);
        let values = body["values"].as_array().expect("values");
        assert_eq!(values.len(), 20, "{body}");
        assert_eq!(values[0], "順序 24", "{body}");
    }

    #[test]
    fn wrangler_suggestions_list_returns_every_value_of_the_field() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // `q` が空のときは、その項目の全ての値を重複を除いて返す (FR-13)。
        // 候補は、その値を持つ記録の updated_at の最大の降順と、同じときの値の昇順で並ぶ。
        // 全ての記録の値も含み、他の利用者の値は含まない (FR-5)。
        for (field, expected) in [
            ("origin", vec!["エチオピア", "Ethiopia", "グアテマラ"]),
            ("region", vec!["イルガチェフェ", "シダモ"]),
            ("process", vec!["ウォッシュド"]),
            // ブルボンは同じ値の 2 件目 (新しい方) の updated_at を使う。
            ("variety", vec!["ブルボン", "ティピカ", "ゲイシャ"]),
            ("roast", vec!["中煎り", "中深煎り", "City", "深煎り"]),
            (
                "method",
                vec!["ペーパードリップ", "フレンチプレス", "エスプレッソ"],
            ),
            ("grind_setting", vec!["中細", "粗挽き", "極細"]),
        ] {
            let body = assert_status(client.get(&format!("/api/suggestions/{field}")), 200);
            assert_values(&body, &expected);
        }
        // 明示的に空の `q` を渡しても同じ結果になる。
        let body = assert_status(client.get("/api/suggestions/origin?q="), 200);
        assert_values(&body, &["エチオピア", "Ethiopia", "グアテマラ"]);
        // 空白だけの `q` は、前後の空白を除くと空になる (FR-13)。
        let body = assert_status(client.get(&suggestion_path("origin", "  ")), 200);
        assert_values(&body, &["エチオピア", "Ethiopia", "グアテマラ"]);
    }

    #[test]
    fn wrangler_suggestions_filters_by_the_trimmed_prefix() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 前方一致で絞る (FR-13)。
        let body = assert_status(client.get(&suggestion_path("producer", "エチオピア")), 200);
        assert_values(&body, &["エチオピア モカ", "エチオピア シダモ"]);
        // 前後の空白は除いてから比べる (FR-13)。
        let body = assert_status(
            client.get(&suggestion_path("producer", "  エチオピア  ")),
            200,
        );
        assert_values(&body, &["エチオピア モカ", "エチオピア シダモ"]);
        // 大文字と小文字を区別しない (FR-13)。SQLite の lower() は ASCII を変換する。
        for query in ["ETH", "eth", "Eth"] {
            let body = assert_status(client.get(&suggestion_path("producer", query)), 200);
            assert_values(&body, &["Ethiopia"]);
        }
        // 候補が無い値でも 200 を返す (候補を返すだけで、入力を制限しない)。
        let body = assert_status(
            client.get(&suggestion_path("producer", "存在しない生産者")),
            200,
        );
        assert_values(&body, &[]);
    }

    #[test]
    fn wrangler_suggestions_dedupes_the_values() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 同じ値は 1 つにまとめる (FR-13)。
        let body = assert_status(client.get(&suggestion_path("producer", "重複")), 200);
        assert_values(&body, &["重複候補"]);
        // まとめた値は 1 件だけである (2 件の記録の値でも候補は 1 つ)。
        let values = body["values"].as_array().expect("values");
        assert_eq!(values.len(), 1, "{body}");
    }

    #[test]
    fn wrangler_suggestions_orders_by_the_latest_update_and_then_by_the_value() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 更新日時が同じときは、値の昇順 (Unicode コードポイント) で並べる (FR-13)。
        let body = assert_status(client.get(&suggestion_path("producer", "並び")), 200);
        assert_values(&body, &["並び A", "並び B"]);
        // 更新日時が新しい値が先になり、21 件以上あるときは先頭の 20 件を返す (FR-13)。
        let body = assert_status(client.get(&suggestion_path("producer", "順序")), 200);
        let expected: Vec<String> = (5..OVER_LIMIT_COUNT)
            .rev()
            .map(|index| format!("順序 {index:02}"))
            .collect();
        assert_eq!(body["values"], json!(expected), "{body}");
    }

    #[test]
    fn wrangler_suggestions_treats_the_wildcards_as_characters() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // `%` と `_` と `\` は LIKE のワイルドカードとエスケープ文字にせず、文字として扱う。
        // ワイルドカードとして扱われると、紛らわしい値 (100X アラビカ、abc ロット、CX豆) や、
        // 先頭が一致するだけの値が混ざる。
        for (query, expected) in [
            ("100%", vec!["100% アラビカ"]),
            ("%", vec!["%入りの生産者"]),
            ("a_b", vec!["a_b ロット"]),
            ("_", vec!["_入りの生産者"]),
            ("C:\\", vec!["C:\\豆"]),
            ("\\", vec!["\\入りの生産者"]),
        ] {
            let body = assert_status(client.get(&suggestion_path("producer", query)), 200);
            assert_values(&body, &expected);
        }
    }

    #[test]
    fn wrangler_suggestions_do_not_restrict_the_input() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.free_session));
        // 記録が無ければ候補も空になる。
        let body = assert_status(client.get("/api/suggestions/producer"), 200);
        assert_values(&body, &[]);
        // 候補に無い値でも登録でき、次のサジェストの候補になる (FR-13)。
        assert_status(
            client.post_json(
                "/api/products",
                &json!({ "name": "自由入力の豆", "producer": "はじめての生産者" }),
            ),
            200,
        );
        let body = assert_status(client.get(&suggestion_path("producer", "はじ")), 200);
        assert_values(&body, &["はじめての生産者"]);
    }

    #[test]
    fn wrangler_suggestions_include_an_empty_value() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.space_session));
        // 空白だけの自由記述は空文字として保存される (0006 の規則)。空文字も候補に含める (乖離 4)。
        assert_status(
            client.post_json(
                "/api/products",
                &json!({ "name": "空文字の豆", "producer": "  " }),
            ),
            200,
        );
        let body = assert_status(client.get(&suggestion_path("producer", "")), 200);
        assert_values(&body, &[""]);
    }

    #[test]
    fn wrangler_suggestions_invalid_field_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 8 つの名前以外は 400 を返す (PRD の 404 の定義は記録とトークンに限る)。
        for field in [
            "Producer",
            "grindSetting",
            "GrindSetting",
            "producers",
            "notes",
            "name",
            "shop",
            "unknown",
        ] {
            assert_bad_request(client.get(&format!("/api/suggestions/{field}")));
        }
        // 項目名が無い経路は台帳に無いため 404 になる。
        assert_not_found(client.get("/api/suggestions"));
        assert_not_found(client.get("/api/suggestions/"));
    }

    #[test]
    fn wrangler_suggestions_invalid_q_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // `q` は 1 つの文字列である。複数回の指定 (配列) は 400 を返す。
        assert_bad_request(client.get("/api/suggestions/producer?q=a&q=b"));
        assert_bad_request(client.get("/api/suggestions/producer?q=a&q="));
        // 1 つの `q` は受け付ける。
        assert_status(client.get("/api/suggestions/producer?q=a"), 200);
    }

    #[test]
    fn wrangler_suggestions_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get("/api/suggestions/producer"));
    }

    #[test]
    fn wrangler_suggestions_returns_only_own_records() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        // 他の利用者の値は候補に含まれない (FR-5)。
        let other = ApiClient::new(&base_url, Some(&data.other_session));
        assert_values(
            &assert_status(other.get("/api/suggestions/producer"), 200),
            &["他人の生産者"],
        );
        assert_values(
            &assert_status(other.get("/api/suggestions/roast"), 200),
            &["他人のロースト"],
        );
        assert_values(
            &assert_status(other.get("/api/suggestions/method"), 200),
            &["他人の抽出方法"],
        );
        assert_values(
            &assert_status(other.get("/api/suggestions/grind_setting"), 200),
            &["他人の挽き目"],
        );
        // サジェストの利用者からは、他の利用者の値が見えない。
        let client = ApiClient::new(&base_url, Some(&data.session));
        for field in FIELDS {
            let body = assert_status(client.get(&suggestion_path(field, "他人")), 200);
            assert_values(&body, &[]);
        }
        // 記録が無い利用者の候補は空になる。
        let empty = ApiClient::new(&base_url, Some(&data.empty_session));
        assert_values(
            &assert_status(empty.get("/api/suggestions/producer"), 200),
            &[],
        );
    }
}
