//! `api` の PBT。要求の JSON と応答の JSON の往復 (ADR-0013)。

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::cell::Cell;
use std::rc::Rc;

use brew_book_frontend::api::{ApiCallError, ApiClient, ApiError, Method};
use proptest::prelude::*;
use serde_json::{json, Map, Value};

use support::{block_on, EchoTransport, FixedTransport};

/// 任意の JSON の値。
fn json_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        "[\\x20-\\x7e]{0,16}".prop_map(Value::String),
        any::<i64>().prop_map(|value| json!(value)),
        any::<bool>().prop_map(|value| json!(value)),
        Just(Value::Null),
    ]
}

/// 任意の JSON のオブジェクト。
fn json_object() -> impl Strategy<Value = Map<String, Value>> {
    prop::collection::btree_map("[a-z][a-z0-9_]{0,8}", json_value(), 0..8)
        .prop_map(|entries| entries.into_iter().collect())
}

proptest! {
    /// 要求の JSON と応答の JSON の往復 (ADR-0013)。
    #[test]
    fn a_json_body_round_trips_through_the_request_and_the_response(value in json_object()) {
        let transport = Rc::new(EchoTransport::new());
        let client = ApiClient::new(transport.clone());

        let body = block_on(client.post_json("/echo", &Value::Object(value.clone())))
            .expect("the echoed body must be read");

        prop_assert_eq!(&body, &value);
        let request = transport.last_request();
        prop_assert_eq!(request.method, Method::Post);
        prop_assert_eq!(request.path, "/api/echo");
        prop_assert_eq!(request.content_type.as_deref(), Some("application/json"));
        let sent: Value = serde_json::from_slice(&request.body).expect("the body must be JSON");
        prop_assert_eq!(sent, Value::Object(value));
    }

    /// 規約の形のエラーの応答は、そのまま共通の型になる。
    #[test]
    fn a_conforming_error_body_becomes_the_common_error(
        status in 400..=599u16,
        code in "[a-z][a-z0-9_]{0,20}",
        message in ".*",
    ) {
        let body = serde_json::to_string(
            &json!({"error": {"code": code.clone(), "message": message.clone()}}),
        )
        .expect("the error body must be JSON");
        let client = ApiClient::new(Rc::new(FixedTransport::new(status, body)));

        let error = block_on(client.get_json("/brews")).expect_err("the call must fail");

        prop_assert_eq!(
            error,
            ApiCallError::Api(ApiError {
                status,
                code: code.clone(),
                message: message.clone(),
            })
        );
    }

    /// 規約の形でないエラーの応答は、ステータスと本文だけを運ぶ。
    #[test]
    fn a_non_conforming_error_body_keeps_the_status_and_the_body(
        status in 400..=599u16,
        body in ".*",
    ) {
        let body = format!("not a JSON object: {body}");
        let client = ApiClient::new(Rc::new(FixedTransport::new(status, body.clone())));

        let error = block_on(client.get_json("/brews")).expect_err("the call must fail");

        prop_assert_eq!(
            error,
            ApiCallError::Api(ApiError {
                status,
                code: "unknown".to_string(),
                message: body.clone(),
            })
        );
    }

    /// 401 の応答はログイン画面への遷移を 1 回だけ呼ぶ。
    #[test]
    fn a_401_calls_the_unauthorized_callback_once(message in ".*") {
        let body = serde_json::to_string(
            &json!({"error": {"code": "unauthorized", "message": message}}),
        )
        .expect("the error body must be JSON");
        let client = ApiClient::new(Rc::new(FixedTransport::new(401, body)));
        let calls = Rc::new(Cell::new(0));
        let counter = calls.clone();
        client.set_on_unauthorized(Rc::new(move || counter.set(counter.get() + 1)));

        let error = block_on(client.get_json("/passkeys")).expect_err("the call must fail");

        prop_assert!(error.is_unauthorized());
        prop_assert_eq!(calls.get(), 1);
    }
}
