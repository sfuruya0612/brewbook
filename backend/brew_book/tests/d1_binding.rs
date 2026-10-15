//! D1 のバインディングの結合テスト。
//!
//! `wrangler dev` を起動し、D1 のバインディングを通したプレースホルダ付きの INSERT と
//! SELECT が成功することを確認する (0003 の完了条件)。0003 のマイグレーションがアーカイブ済み
//! だった行を残すことと、0005 のマイグレーションが商品の項目と Flavor Notes を購入に移して
//! 購入と抽出を残すこと (ADR-0022) も確認する。テスト名の `wrangler_` は、
//! `wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。

mod support;

use std::fs;
use std::path::Path;
use std::time::Duration;

use support::http::ApiClient;
use support::seed::{user_id, Seed};

/// D1 の検証用の経路を有効にする vars。本番の vars には無い。
const D1_CHECK_VAR: (&str, &str) = ("D1_CHECK", "true");
/// D1 の検証用の経路。
const D1_CHECK_PATH: &str = "/api/__d1_check";

#[test]
fn wrangler_d1_binding_inserts_and_selects_with_placeholders() {
    let server =
        support::DevServer::start_with_vars(&[D1_CHECK_VAR]).expect("wrangler dev must start");
    let client = client();

    // 値に SQL の断片を含めて送り、プレースホルダとして扱われることを確認する。
    let name = "d1's check; DROP TABLE shops; --";
    let address = "東京都; DELETE FROM users; --";
    let response = post(
        &client,
        &server,
        serde_json::json!({ "name": name, "address": address }),
    );
    let status = response.status().as_u16();
    if status != 200 {
        panic!(
            "the D1 check must succeed with 200 but was {status}: {}",
            response.text().unwrap_or_default()
        );
    }

    let body = json_body(response);
    let user_id = body["user_id"]
        .as_str()
        .expect("the response must carry the user id");
    assert_eq!(user_id.len(), 36, "the user id must be a UUID: {user_id}");
    assert_eq!(&user_id[14..15], "4", "the user id must be UUID v4");
    assert!(
        matches!(&user_id[19..20], "8" | "9" | "a" | "b"),
        "the user id must be UUID v4: {user_id}"
    );

    // 2 回目も呼び、乱数が毎回変わること (UUID が同じにならないこと) を確認する。
    let second = post(
        &client,
        &server,
        serde_json::json!({ "name": "second", "address": "second" }),
    );
    assert_eq!(second.status().as_u16(), 200);
    let second_body = json_body(second);
    assert_ne!(
        second_body["user_id"].as_str(),
        Some(user_id),
        "the user id must be random on every request"
    );

    let shop = body["shop"]
        .as_object()
        .expect("the response must carry the selected shop");
    assert_eq!(
        shop["name"].as_str(),
        Some(name),
        "the name must round trip as a bound value"
    );
    assert_eq!(shop["address"].as_str(), Some(address));
    assert_eq!(shop["user_id"].as_str(), Some(user_id));
    assert!(shop.get("archived_at").is_none(), "{shop:?}");
    let created_at = shop["created_at"]
        .as_str()
        .expect("the selected row must carry created_at");
    assert_eq!(
        created_at.len(),
        24,
        "created_at must be the fixed length ISO 8601 UTC: {created_at}"
    );
    assert!(created_at.ends_with('Z'), "created_at is {created_at}");
    assert_eq!(shop["created_at"], shop["updated_at"]);
}

#[test]
fn wrangler_d1_check_is_disabled_without_its_var() {
    // 本番の vars には D1_CHECK が無いため、経路は台帳に無い経路と同じ 404 になる。
    // 認証なしで D1 に書ける経路を本番に残さないことを確認する。
    let server = support::DevServer::start().expect("wrangler dev must start");
    let client = client();
    let response = post(
        &client,
        &server,
        serde_json::json!({ "name": "check", "address": null }),
    );
    assert_eq!(response.status().as_u16(), 404);
    assert_eq!(
        json_body(response),
        serde_json::json!({"error": {"code": "not_found", "message": "route not found"}})
    );
}

/// 0002 の状態で `archived_at` を設定した行を入れ、0003 を適用した後に行が残り、
/// 一覧の API が返すことを確認する (ADR-0018)。
#[test]
fn wrangler_the_migration_removing_the_archive_keeps_the_rows() {
    let server = support::DevServer::start().expect("wrangler dev must start");
    drop_all_tables(&server);
    // migrations/ のファイルを名前順に読み、0002 までを適用してから行を入れ、0003 以降を
    // 順に適用する (将来のマイグレーションの追加にも追随する。レビューの指摘)。
    let names = migration_names();
    let archive_index = apply_migrations_before(&server, &names, "0003_");

    // 0002 の状態では archived_at があり、アーカイブ済みの行を入れられる。
    let at = "2026-09-21T00:00:00.000Z";
    let future = "2099-01-01T00:00:00.000Z";
    let user = user_id(1);
    let mut seed = Seed::new();
    seed.user(&user, "migration user", at);
    let session = seed.session(&user, future, at);
    let shop = "00000000-0000-4000-8000-000000000001";
    let product = "00000000-0000-4000-8000-000000000002";
    let purchase = "00000000-0000-4000-8000-000000000003";
    let brew = "00000000-0000-4000-8000-000000000004";
    seed.raw(&format!(
        "INSERT INTO shops (id, user_id, name, address, created_at, updated_at, archived_at) \
         VALUES ('{shop}', '{user}', 'archived shop', NULL, '{at}', '{at}', '{at}')"
    ));
    seed.raw(&format!(
        "INSERT INTO products (id, user_id, name, created_at, updated_at, archived_at) \
         VALUES ('{product}', '{user}', 'archived product', '{at}', '{at}', '{at}')"
    ));
    seed.raw(&format!(
        "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, created_at, \
         updated_at, archived_at) \
         VALUES ('{purchase}', '{user}', '{product}', '{shop}', '2026-09-21', '{at}', '{at}', '{at}')"
    ));
    seed.raw(&format!(
        "INSERT INTO brews (id, user_id, purchase_id, brewed_at, created_at, updated_at, archived_at) \
         VALUES ('{brew}', '{user}', '{purchase}', '{at}', '{at}', '{at}', '{at}')"
    ));
    server
        .execute_sql_file(&[seed.sql()])
        .expect("the archived rows must be inserted");

    for name in &names[archive_index..] {
        server
            .execute_sql_file(&[migration(name)])
            .expect("the migration must apply");
    }

    // 3 つのテーブルの行は残る (商品の行は 0005 で購入に移る。ADR-0022)。
    for table in ["shops", "purchases", "brews"] {
        let count = server
            .query_int(&format!("SELECT COUNT(*) FROM {table}"))
            .expect("the count must be read");
        assert_eq!(count, 1, "the row of {table} must be kept");
    }
    let rows = server
        .query_rows("SELECT name FROM purchases")
        .expect("the purchase must be read");
    assert_eq!(rows[0]["name"], "archived product", "{rows:?}");
    // アーカイブ済みだった店は、通常の記録として一覧の API が返す。
    let client = ApiClient::for_server(&server, Some(&session));
    let (status, body) = support::http::read(client.get("/api/shops"));
    assert_eq!(status, 200, "the response body was {body}");
    let shops = body["shops"]
        .as_array()
        .expect("the response must have shops");
    assert_eq!(shops.len(), 1, "{body}");
    assert_eq!(shops[0]["id"], shop, "{body}");
    assert!(shops[0].get("archived_at").is_none(), "{body}");
}

/// 0004 の状態で店、購入のある商品と無い商品、購入が 2 件ある商品、Flavor Notes 付きの商品、
/// 写真のキーとお気に入りと店と価格を持つ購入、価格の無い購入、抽出を入れ、0005 以降を適用した
/// 後に、購入が商品の 6 項目と Flavor Notes を持ち、既存の列の値が変わらず、抽出が購入を
/// 参照したまま残り、`products` と `product_flavor_tags` が無いことを確認する (ADR-0022)。
#[test]
fn wrangler_the_migration_merging_products_keeps_the_purchases_and_brews() {
    let server = support::DevServer::start().expect("wrangler dev must start");
    drop_all_tables(&server);
    let names = migration_names();
    let merge_index = apply_migrations_before(&server, &names, "0005_");

    let at = "2026-09-21T00:00:00.000Z";
    let later = "2026-09-22T00:00:00.000Z";
    let future = "2099-01-01T00:00:00.000Z";
    let user = user_id(1);
    let mut seed = Seed::new();
    seed.user(&user, "migration user", at);
    let session = seed.session(&user, future, at);
    let shop = "00000000-0000-4000-8000-000000000001";
    // 購入が 2 件ある商品 (Flavor Notes 付き)、購入の無い商品 (Flavor Notes 付き)、
    // 購入が 1 件ある商品 (Flavor Notes 無し)。
    let twice = "00000000-0000-4000-8000-000000000002";
    let orphan = "00000000-0000-4000-8000-000000000003";
    let plain = "00000000-0000-4000-8000-000000000004";
    let citrus = "00000000-0000-4000-8000-000000000005";
    let floral = "00000000-0000-4000-8000-000000000006";
    let nutty = "00000000-0000-4000-8000-000000000007";
    let priced = "00000000-0000-4000-8000-000000000008";
    let unpriced = "00000000-0000-4000-8000-000000000009";
    let third = "00000000-0000-4000-8000-000000000010";
    let brew_one = "00000000-0000-4000-8000-000000000011";
    let brew_two = "00000000-0000-4000-8000-000000000012";
    seed.raw(&format!(
        "INSERT INTO shops (id, user_id, name, address, created_at, updated_at) \
         VALUES ('{shop}', '{user}', 'merge shop', NULL, '{at}', '{at}')"
    ));
    seed.raw(&format!(
        "INSERT INTO products (id, user_id, name, producer, origin, region, process, variety, \
         created_at, updated_at, favorited_at) VALUES \
         ('{twice}', '{user}', 'twice product', 'Producer A', 'Ethiopia', 'Yirgacheffe', \
         'Washed', 'Heirloom', '{at}', '{at}', '{at}'), \
         ('{orphan}', '{user}', 'orphan product', NULL, NULL, NULL, NULL, NULL, '{at}', '{at}', NULL), \
         ('{plain}', '{user}', 'plain product', NULL, 'Brazil', NULL, NULL, NULL, '{at}', '{at}', NULL)"
    ));
    seed.raw(&format!(
        "INSERT INTO flavor_tags (id, user_id, name) VALUES \
         ('{citrus}', '{user}', 'citrus'), ('{floral}', '{user}', 'floral'), ('{nutty}', '{user}', 'nutty')"
    ));
    seed.raw(&format!(
        "INSERT INTO product_flavor_tags (user_id, product_id, tag_id) VALUES \
         ('{user}', '{twice}', '{citrus}'), ('{user}', '{twice}', '{floral}'), ('{user}', '{orphan}', '{nutty}')"
    ));
    seed.raw(&format!(
        "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, roast, roast_date, \
         price_amount, price_currency, weight_grams, photo_key, created_at, updated_at, favorited_at) \
         VALUES \
         ('{priced}', '{user}', '{twice}', '{shop}', '2026-09-21', 'medium', '2026-09-20', 1200, 'JPY', \
         200, 'purchases/{user}/{priced}.jpg', '{at}', '{later}', '{later}'), \
         ('{unpriced}', '{user}', '{twice}', NULL, '2026-09-22', NULL, NULL, NULL, NULL, NULL, NULL, \
         '{at}', '{at}', NULL), \
         ('{third}', '{user}', '{plain}', NULL, '2026-09-23', NULL, NULL, NULL, NULL, NULL, NULL, \
         '{at}', '{at}', NULL)"
    ));
    seed.raw(&format!(
        "INSERT INTO brews (id, user_id, purchase_id, brewed_at, created_at, updated_at) VALUES \
         ('{brew_one}', '{user}', '{priced}', '{at}', '{at}', '{at}'), \
         ('{brew_two}', '{user}', '{unpriced}', '{at}', '{at}', '{at}')"
    ));
    server
        .execute_sql_file(&[seed.sql()])
        .expect("the rows of the 0004 schema must be inserted");

    for name in &names[merge_index..] {
        server
            .execute_sql_file(&[migration(name)])
            .expect("the migration must apply");
    }

    // 購入は 3 件のまま、商品の 6 項目を持ち、既存の列の値が変わらない。
    let rows = server
        .query_rows(
            "SELECT id, shop_id, purchased_on, roast, roast_date, price_amount, price_currency, \
             weight_grams, photo_key, name, producer, origin, region, process, variety, \
             created_at, updated_at, favorited_at FROM purchases ORDER BY id ASC",
        )
        .expect("the purchases must be read");
    assert_eq!(rows.len(), 3, "{rows:?}");
    let photo_key = format!("purchases/{user}/{priced}.jpg");
    assert_eq!(
        rows[0],
        serde_json::json!({
            "id": priced, "shop_id": shop, "purchased_on": "2026-09-21", "roast": "medium",
            "roast_date": "2026-09-20", "price_amount": 1200, "price_currency": "JPY",
            "weight_grams": 200, "photo_key": photo_key, "name": "twice product",
            "producer": "Producer A", "origin": "Ethiopia", "region": "Yirgacheffe",
            "process": "Washed", "variety": "Heirloom", "created_at": at, "updated_at": later,
            "favorited_at": later,
        })
    );
    assert_eq!(
        rows[1],
        serde_json::json!({
            "id": unpriced, "shop_id": null, "purchased_on": "2026-09-22", "roast": null,
            "roast_date": null, "price_amount": null, "price_currency": null,
            "weight_grams": null, "photo_key": null, "name": "twice product",
            "producer": "Producer A", "origin": "Ethiopia", "region": "Yirgacheffe",
            "process": "Washed", "variety": "Heirloom", "created_at": at, "updated_at": at,
            "favorited_at": null,
        })
    );
    assert_eq!(
        rows[2],
        serde_json::json!({
            "id": third, "shop_id": null, "purchased_on": "2026-09-23", "roast": null,
            "roast_date": null, "price_amount": null, "price_currency": null,
            "weight_grams": null, "photo_key": null, "name": "plain product",
            "producer": null, "origin": "Brazil", "region": null, "process": null,
            "variety": null, "created_at": at, "updated_at": at, "favorited_at": null,
        })
    );
    // Flavor Notes は購入ごとに移り (2 件の購入が同じ 2 つを持つ)、購入の無い商品の対応は消える。
    // タグの行は残る。
    let tags = server
        .query_rows(
            "SELECT pft.purchase_id, t.name FROM purchase_flavor_tags AS pft \
             INNER JOIN flavor_tags AS t ON t.id = pft.tag_id \
             ORDER BY pft.purchase_id ASC, t.name ASC",
        )
        .expect("the purchase flavor tags must be read");
    assert_eq!(
        tags,
        vec![
            serde_json::json!({"purchase_id": priced, "name": "citrus"}),
            serde_json::json!({"purchase_id": priced, "name": "floral"}),
            serde_json::json!({"purchase_id": unpriced, "name": "citrus"}),
            serde_json::json!({"purchase_id": unpriced, "name": "floral"}),
        ]
    );
    let tag_count = server
        .query_int("SELECT COUNT(*) FROM flavor_tags")
        .expect("the tag count must be read");
    assert_eq!(tag_count, 3, "the tag rows must be kept");
    // 抽出は購入を参照したまま残る。
    let brews = server
        .query_rows("SELECT id, purchase_id FROM brews ORDER BY id ASC")
        .expect("the brews must be read");
    assert_eq!(
        brews,
        vec![
            serde_json::json!({"id": brew_one, "purchase_id": priced}),
            serde_json::json!({"id": brew_two, "purchase_id": unpriced}),
        ]
    );
    // 商品のテーブルと対応のテーブルは無い。
    let leftover = server
        .query_int(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
             AND name IN ('products', 'product_flavor_tags', 'purchases_merged')",
        )
        .expect("the table count must be read");
    assert_eq!(leftover, 0, "the product tables must be dropped");
    // 一覧の API が Flavor Notes 付きの購入を返す。
    let client = ApiClient::for_server(&server, Some(&session));
    let (status, body) = support::http::read(client.get("/api/purchases"));
    assert_eq!(status, 200, "the response body was {body}");
    let purchases = body["purchases"]
        .as_array()
        .expect("the response must have purchases");
    assert_eq!(purchases.len(), 3, "{body}");
    assert_eq!(purchases[1]["id"], unpriced, "{body}");
    assert_eq!(purchases[1]["name"], "twice product", "{body}");
    assert_eq!(
        purchases[1]["flavor_notes"],
        serde_json::json!(["citrus", "floral"]),
        "{body}"
    );
    assert!(purchases[1].get("product").is_none(), "{body}");
}

/// 適用済みのテーブルを、外部キーの参照元 (子) から順に消して空のデータベースにする。
fn drop_all_tables(server: &support::DevServer) {
    let drops = [
        "DROP TABLE brews",
        "DROP TABLE purchase_flavor_tags",
        "DROP TABLE purchases",
        "DROP TABLE flavor_tags",
        "DROP TABLE shops",
        "DROP TABLE sessions",
        "DROP TABLE passkey_credentials",
        "DROP TABLE registration_tokens",
        "DROP TABLE webauthn_challenges",
        "DROP TABLE users",
    ];
    let drops: Vec<String> = drops.iter().map(|sql| (*sql).to_owned()).collect();
    server
        .execute_sql_file(&drops)
        .expect("the tables must be dropped");
}

/// `prefix` で始まるマイグレーションの手前までを順に適用し、そのマイグレーションの位置を返す。
fn apply_migrations_before(server: &support::DevServer, names: &[String], prefix: &str) -> usize {
    let index = names
        .iter()
        .position(|name| name.starts_with(prefix))
        .unwrap_or_else(|| panic!("the migration {prefix} must exist"));
    for name in &names[..index] {
        server
            .execute_sql_file(&[migration(name)])
            .expect("the migration must apply");
    }
    index
}

/// マイグレーションのファイルを読む。
fn migration(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("migrations")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

/// `migrations/` の `.sql` のファイル名を名前順に返す (レビューの指摘)。
fn migration_names() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .expect("the migrations directory must be readable")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".sql"))
        .collect();
    names.sort();
    names
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("the HTTP client must build")
}

fn post(
    client: &reqwest::blocking::Client,
    server: &support::DevServer,
    body: serde_json::Value,
) -> reqwest::blocking::Response {
    client
        .post(format!("{}{D1_CHECK_PATH}", server.base_url()))
        .header("content-type", "application/json")
        .body(serde_json::to_string(&body).expect("the request body must be serializable"))
        .send()
        .expect("the request must reach the dev server")
}

fn json_body(response: reqwest::blocking::Response) -> serde_json::Value {
    let text = response.text().expect("the response body must be readable");
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("the response must be JSON but was {text}: {error}"))
}
