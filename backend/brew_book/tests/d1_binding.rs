//! D1 のバインディングの結合テスト。
//!
//! `wrangler dev` を起動し、D1 のバインディングを通したプレースホルダ付きの INSERT と
//! SELECT が成功することを確認する (0003 の完了条件)。0003 のマイグレーションがアーカイブ済み
//! だった行を残すことも確認する (ADR-0018)。テスト名の `wrangler_` は、
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
    // 適用済みのテーブルを、外部キーの参照元 (子) から順に消して空のデータベースにする。
    let drops = [
        "DROP TABLE brews",
        "DROP TABLE purchases",
        "DROP TABLE product_flavor_tags",
        "DROP TABLE flavor_tags",
        "DROP TABLE products",
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
    // migrations/ のファイルを名前順に読み、0002 までを適用してから行を入れ、0003 以降を
    // 順に適用する (将来のマイグレーションの追加にも追随する。レビューの指摘)。
    let names = migration_names();
    let archive_index = names
        .iter()
        .position(|name| name.starts_with("0003_"))
        .expect("the archive removal migration must exist");
    for name in &names[..archive_index] {
        server
            .execute_sql_file(&[migration(name)])
            .expect("the migration must apply");
    }

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

    // 4 つのテーブルの行は残る。
    for table in ["shops", "products", "purchases", "brews"] {
        let count = server
            .query_int(&format!("SELECT COUNT(*) FROM {table}"))
            .expect("the count must be read");
        assert_eq!(count, 1, "the row of {table} must be kept");
    }
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
