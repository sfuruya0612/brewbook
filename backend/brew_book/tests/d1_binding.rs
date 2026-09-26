//! D1 のバインディングの結合テスト。
//!
//! `wrangler dev` を起動し、D1 のバインディングを通したプレースホルダ付きの INSERT と
//! SELECT が成功することを確認する (0003 の完了条件)。テスト名の `wrangler_` は、
//! `wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。

mod support;

use std::time::Duration;

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
    assert_eq!(shop["archived_at"], serde_json::Value::Null);
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
