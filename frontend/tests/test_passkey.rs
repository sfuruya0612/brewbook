//! `auth::passkey` の単体テスト。サーバーのオプションの JSON の読み取り (FR-1、FR-2) を検査する。
//!
//! オプションの辞書の組み立て (web-sys の型) はブラウザで動かす `test_passkey_web.rs` が担う。

use brew_book_frontend::auth::{CreationOptions, PasskeyError, PasskeyErrorKind, RequestOptions};
use serde_json::{json, Map, Value};

/// JSON のオブジェクトを作る。
fn object(value: Value) -> Map<String, Value> {
    value
        .as_object()
        .expect("the value must be an object")
        .clone()
}

#[test]
fn the_creation_options_read_the_server_response() {
    let options = object(json!({
        "challenge": "AQID",
        "rp": {"id": "localhost", "name": "brewbook"},
        "user": {"id": "AQ", "name": "u1"},
        "pubKeyCredParams": [{"type": "public-key", "alg": -7}],
        "attestation": "none",
        "authenticatorSelection": {"residentKey": "preferred", "userVerification": "required"},
        "timeout": 300000
    }));

    let options = CreationOptions::from_json(&options).expect("the options must be read");

    assert_eq!(options.challenge, vec![1, 2, 3]);
    assert_eq!(options.rp_id, "localhost");
    assert_eq!(options.rp_name, "brewbook");
    assert_eq!(options.user_id, vec![1]);
    assert_eq!(options.user_name, "u1");
    // サーバーのオプションに displayName は無いため、名前で埋める (Flutter と同じ)。
    assert_eq!(options.user_display_name, "u1");
    assert_eq!(options.algorithms, vec![-7]);
    assert_eq!(options.timeout, 300000);
    assert_eq!(options.attestation, "none");
    assert_eq!(options.resident_key, "preferred");
    assert_eq!(options.user_verification, "required");
}

#[test]
fn the_creation_options_fill_the_defaults() {
    // ADR-0004 の要求 (attestation は none、residentKey は preferred、userVerification は required)。
    let options = object(json!({"challenge": "AQ", "user": {"id": "AQ"}}));

    let options = CreationOptions::from_json(&options).expect("the options must be read");

    assert_eq!(options.rp_id, "");
    assert_eq!(options.rp_name, "");
    assert_eq!(options.user_name, "");
    assert_eq!(options.user_display_name, "");
    assert!(options.algorithms.is_empty());
    assert_eq!(options.timeout, 0);
    assert_eq!(options.attestation, "none");
    assert_eq!(options.resident_key, "preferred");
    assert_eq!(options.user_verification, "required");
}

#[test]
fn the_request_options_read_the_server_response() {
    let options = object(json!({
        "challenge": "AQID",
        "rpId": "localhost",
        "userVerification": "required",
        "allowCredentials": [{"type": "public-key", "id": "AQ"}],
        "timeout": 300000
    }));

    let options = RequestOptions::from_json(&options).expect("the options must be read");

    assert_eq!(options.challenge, vec![1, 2, 3]);
    assert_eq!(options.rp_id, "localhost");
    assert_eq!(options.user_verification, "required");
    assert_eq!(options.allow_credentials, vec![vec![1]]);
    assert_eq!(options.timeout, 300000);
}

#[test]
fn an_empty_allow_credentials_targets_the_discoverable_passkeys() {
    let options = object(json!({"challenge": "AQ", "allowCredentials": []}));

    let options = RequestOptions::from_json(&options).expect("the options must be read");

    assert!(options.allow_credentials.is_empty());
    assert_eq!(options.rp_id, "");
    assert_eq!(options.user_verification, "required");
    assert_eq!(options.timeout, 0);
}

#[test]
fn a_missing_or_invalid_challenge_fails() {
    for options in [
        json!({}),
        json!({"challenge": "A"}),
        json!({"challenge": "AQ=="}),
        json!({"challenge": 1}),
    ] {
        let options = object(options);
        let error = CreationOptions::from_json(&options).expect_err("the options must fail");
        assert_eq!(error.kind, PasskeyErrorKind::Failed, "{error:?}");
        let error = RequestOptions::from_json(&options).expect_err("the options must fail");
        assert_eq!(error.kind, PasskeyErrorKind::Failed, "{error:?}");
    }
}

#[test]
fn a_missing_user_fails() {
    let options = object(json!({"challenge": "AQ"}));

    let error = CreationOptions::from_json(&options).expect_err("the options must fail");

    assert_eq!(error.kind, PasskeyErrorKind::Failed);
}

#[test]
fn an_invalid_allow_credentials_entry_fails() {
    for entry in [
        json!({"type": "public-key"}),
        json!({"id": "AQ=="}),
        json!("AQ"),
    ] {
        let options = object(json!({"challenge": "AQ", "allowCredentials": [entry]}));
        let error = RequestOptions::from_json(&options).expect_err("the options must fail");
        assert_eq!(error.kind, PasskeyErrorKind::Failed, "{error:?}");
    }
}

#[test]
fn the_error_name_decides_the_kind() {
    assert_eq!(
        PasskeyError::kind_of_error_name(Some("NotAllowedError")),
        PasskeyErrorKind::Cancelled
    );
    assert_eq!(
        PasskeyError::kind_of_error_name(Some("AbortError")),
        PasskeyErrorKind::Cancelled
    );
    assert_eq!(
        PasskeyError::kind_of_error_name(Some("NotSupportedError")),
        PasskeyErrorKind::Unsupported
    );
    assert_eq!(
        PasskeyError::kind_of_error_name(Some("SecurityError")),
        PasskeyErrorKind::Failed
    );
    assert_eq!(
        PasskeyError::kind_of_error_name(None),
        PasskeyErrorKind::Failed
    );
}
