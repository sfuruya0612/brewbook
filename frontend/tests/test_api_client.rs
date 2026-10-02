//! `api` の単体テスト。要求と応答の往復の性質は PBT (`prop_api.rs`) が担う。

mod support;

use std::cell::Cell;
use std::rc::Rc;

use brew_book_frontend::api::{ApiCallError, ApiError, Method, NetworkError};
use brew_book_frontend::i18n::Key;
use serde_json::{json, Value};

use support::{block_on, client, FakeTransport};

#[test]
fn a_get_calls_under_the_api_base_path() {
    let (client, transport) = client(vec![FakeTransport::response(200, "{}")]);

    let body = block_on(client.get_json("/brews")).expect("the response must be read");

    assert!(body.is_empty());
    let request = transport.last_request();
    assert_eq!(request.method, Method::Get);
    assert_eq!(request.path, "/api/brews");
    assert_eq!(request.content_type, None);
    assert!(request.body.is_empty());
}

#[test]
fn the_base_path_can_be_changed() {
    let transport = Rc::new(FakeTransport::new(vec![FakeTransport::response(200, "{}")]));
    let client = brew_book_frontend::api::ApiClient::with_base_path(transport.clone(), "/other");

    block_on(client.get_json("/brews")).expect("the response must be read");

    assert_eq!(client.base_path(), "/other");
    assert_eq!(transport.last_request().path, "/other/brews");
}

#[test]
fn a_post_sends_a_json_body_and_reads_the_object() {
    let (client, transport) = client(vec![FakeTransport::response(200, r#"{"id":"1"}"#)]);

    let body = block_on(client.post_json("/brews", &json!({"name": "x"})))
        .expect("the response must be read");

    assert_eq!(body["id"], json!("1"));
    let request = transport.last_request();
    assert_eq!(request.method, Method::Post);
    assert_eq!(request.path, "/api/brews");
    assert_eq!(request.content_type.as_deref(), Some("application/json"));
    assert_eq!(
        serde_json::from_slice::<Value>(&request.body).expect("the body must be JSON"),
        json!({"name": "x"})
    );
}

#[test]
fn a_patch_and_a_delete_send_their_methods() {
    let (client, transport) = client(vec![
        FakeTransport::response(200, "{}"),
        FakeTransport::response(200, "{}"),
    ]);

    block_on(client.patch_json("/brews/1", &json!({"name": "y"}))).expect("the response");
    block_on(client.delete_json("/brews/1")).expect("the response");

    let requests = transport.requests();
    assert_eq!(requests[0].method, Method::Patch);
    assert_eq!(requests[1].method, Method::Delete);
    assert!(requests[1].body.is_empty());
}

#[test]
fn a_post_of_bytes_sends_the_content_type() {
    let (client, transport) = client(vec![FakeTransport::response(200, "{}")]);

    block_on(client.post_bytes("/suggestions", vec![1, 2, 3], "image/jpeg")).expect("the response");

    let request = transport.last_request();
    assert_eq!(request.method, Method::Post);
    assert_eq!(request.content_type.as_deref(), Some("image/jpeg"));
    assert_eq!(request.body, vec![1, 2, 3]);
}

#[test]
fn a_get_of_bytes_returns_the_body() {
    let (client, _) = client(vec![FakeTransport::response(200, "the export")]);

    assert_eq!(
        block_on(client.get_bytes("/export")).expect("the response must be read"),
        b"the export".to_vec()
    );
}

#[test]
fn an_empty_body_is_an_empty_object() {
    let (client, _) = client(vec![FakeTransport::response(200, "")]);

    assert!(block_on(client.delete_json("/brews/1"))
        .expect("an empty body must be accepted")
        .is_empty());
}

#[test]
fn an_error_response_becomes_the_common_error() {
    let (client, _) = client(vec![FakeTransport::response(
        404,
        r#"{"error":{"code":"not_found","message":"the brew does not exist"}}"#,
    )]);

    let error = block_on(client.get_json("/brews/1")).expect_err("the call must fail");

    assert_eq!(
        error,
        ApiCallError::Api(ApiError {
            status: 404,
            code: "not_found".to_string(),
            message: "the brew does not exist".to_string(),
        })
    );
    assert!(!error.is_unauthorized());
}

#[test]
fn an_error_body_that_does_not_follow_the_convention_keeps_the_status_and_the_body() {
    let (client, _) = client(vec![
        FakeTransport::response(500, "internal error"),
        FakeTransport::response(500, r#"{"error":{"code":1}}"#),
        FakeTransport::response(500, ""),
    ]);

    for expected_message in ["internal error", r#"{"error":{"code":1}}"#, ""] {
        let error = block_on(client.get_json("/brews")).expect_err("the call must fail");
        assert_eq!(
            error,
            ApiCallError::Api(ApiError {
                status: 500,
                code: "unknown".to_string(),
                message: expected_message.to_string(),
            })
        );
    }
}

#[test]
fn a_401_calls_the_unauthorized_callback() {
    let (client, _) = client(vec![FakeTransport::response(
        401,
        r#"{"error":{"code":"unauthorized","message":"the session is not valid"}}"#,
    )]);
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    client.set_on_unauthorized(Rc::new(move || counter.set(counter.get() + 1)));

    let error = block_on(client.get_json("/passkeys")).expect_err("the call must fail");

    assert!(error.is_unauthorized());
    assert_eq!(calls.get(), 1);
}

#[test]
fn other_errors_do_not_call_the_unauthorized_callback() {
    let (client, _) = client(vec![FakeTransport::response(500, "")]);
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    client.set_on_unauthorized(Rc::new(move || counter.set(counter.get() + 1)));

    block_on(client.get_json("/brews")).expect_err("the call must fail");

    assert_eq!(calls.get(), 0);
}

#[test]
fn a_failed_connection_becomes_a_network_error() {
    let (client, _) = client(vec![FakeTransport::failure("Failed to fetch")]);

    let error = block_on(client.get_json("/passkeys")).expect_err("the call must fail");

    assert_eq!(
        error,
        ApiCallError::Network(NetworkError::new("Failed to fetch"))
    );
    assert!(!error.is_unauthorized());
}

#[test]
fn a_success_body_that_is_not_a_json_object_becomes_a_network_error() {
    let (client, _) = client(vec![
        FakeTransport::response(200, "<html>"),
        FakeTransport::response(200, "[1, 2]"),
    ]);

    for _ in 0..2 {
        let error = block_on(client.get_json("/brews")).expect_err("the call must fail");
        assert!(matches!(error, ApiCallError::Network(_)));
    }
}

#[test]
fn a_network_error_offers_the_retry_notice() {
    // 応答を取得できなかった失敗は再試行を促す (ADR-0007)。
    let network = ApiCallError::Network(NetworkError::new("Failed to fetch"));
    assert_eq!(
        network.retry_keys(),
        Some((Key::ErrorNetwork, Key::RetryButton))
    );

    // エラーの応答は入力の修正で直るため、再試行の案内は出さない。
    let api = ApiCallError::Api(ApiError {
        status: 400,
        code: "invalid_input".to_string(),
        message: "the input is invalid".to_string(),
    });
    assert_eq!(api.retry_keys(), None);
}
