//! `auth` の単体テスト。認証の操作 (FR-1、FR-2、FR-4) と起動時のセッション確認を検査する。
//!
//! `navigator.credentials` の呼び出しは偽の実装 ([`FakePasskeyClient`]) に差し替え、API の
//! 呼び出しは偽の送信の実装で確かめる。base64url の検査は `test_base64url.rs`、オプションの
//! 読み取りは `test_passkey.rs` が担う。

mod support;

use std::cell::Cell;
use std::rc::Rc;

use brew_book_frontend::api::{ApiCallError, ApiError, Method, NetworkError};
use brew_book_frontend::app::guard_destination;
use brew_book_frontend::auth::{
    check_session, is_invalid_registration_token, login, login_error_key, logout, message_key,
    passkey_name_for_request, register, register_error_key, AuthError, AuthServices, PasskeyError,
    SessionStatus, PASSKEY_NAME_MAX_CHARS,
};
use brew_book_frontend::i18n::Key;
use brew_book_frontend::router::Route;
use serde_json::Value;

use support::{block_on, client, FakePasskeyClient, FakeTransport};

#[test]
fn a_successful_passkeys_call_means_signed_in() {
    let (client, transport) = client(vec![FakeTransport::response(200, "{}")]);

    assert_eq!(block_on(check_session(&client)), SessionStatus::SignedIn);
    assert_eq!(transport.last_request().path, "/api/passkeys");
}

#[test]
fn a_401_means_signed_out_and_navigates_to_the_login() {
    let (client, _) = client(vec![FakeTransport::response(
        401,
        r#"{"error":{"code":"unauthorized","message":"the session is not valid"}}"#,
    )]);
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    client.set_on_unauthorized(Rc::new(move || counter.set(counter.get() + 1)));

    assert_eq!(block_on(check_session(&client)), SessionStatus::SignedOut);
    // 401 の応答はログイン画面へ遷移させる (ADR-0007)。
    assert_eq!(calls.get(), 1);
}

#[test]
fn a_server_error_means_the_state_is_unknown() {
    let (client, _) = client(vec![FakeTransport::response(500, "")]);

    assert_eq!(block_on(check_session(&client)), SessionStatus::Unknown);
}

#[test]
fn a_failed_connection_means_the_state_is_unknown() {
    let (client, _) = client(vec![FakeTransport::failure("Failed to fetch")]);

    assert_eq!(block_on(check_session(&client)), SessionStatus::Unknown);
}

/// API のエラーの応答から認証の失敗を作る。
fn api_error(status: u16) -> AuthError {
    AuthError::Api(ApiCallError::Api(ApiError {
        status,
        code: "error".to_string(),
        message: "the call failed".to_string(),
    }))
}

#[test]
fn a_passkey_name_is_trimmed_and_limited_to_fifty_characters() {
    assert_eq!(
        passkey_name_for_request("  マイキー  "),
        Ok("マイキー".to_string())
    );
    assert_eq!(passkey_name_for_request("a"), Ok("a".to_string()));

    // 上限ちょうどは受け付け、1 文字超えると拒否する。
    let fifty = "あ".repeat(PASSKEY_NAME_MAX_CHARS);
    assert_eq!(passkey_name_for_request(&fifty), Ok(fifty.clone()));
    assert_eq!(
        passkey_name_for_request(&"あ".repeat(PASSKEY_NAME_MAX_CHARS + 1)),
        Err(Key::PasskeyNameError)
    );

    // サロゲートペアを含む名前も Rust の char (Unicode のスカラー値) の数で数える
    // (Flutter の String.runes と Backend の validate_passkey_name と同じ)。
    let emoji = "\u{1F44D}".repeat(PASSKEY_NAME_MAX_CHARS);
    assert_eq!(passkey_name_for_request(&emoji), Ok(emoji.clone()));
    assert_eq!(
        passkey_name_for_request(&"\u{1F44D}".repeat(PASSKEY_NAME_MAX_CHARS + 1)),
        Err(Key::PasskeyNameError)
    );
}

#[test]
fn an_empty_passkey_name_is_rejected() {
    for value in ["", "   ", "\t\n"] {
        assert_eq!(
            passkey_name_for_request(value),
            Err(Key::PasskeyNameError),
            "{value:?}"
        );
    }
}

#[test]
fn a_successful_login_sends_the_credential_to_the_complete_endpoint() {
    let (client, transport) = client(vec![
        FakeTransport::response(
            200,
            r#"{"challenge":"AQID","rpId":"localhost","userVerification":"required","allowCredentials":[],"timeout":300000}"#,
        ),
        FakeTransport::response(200, r#"{"user_id":"u1"}"#),
    ]);
    let passkeys = Rc::new(FakePasskeyClient::new());
    let services = AuthServices::new(client, passkeys.clone());

    block_on(login(&services)).expect("the login must succeed");

    let requests = transport.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, Method::Post);
    assert_eq!(requests[0].path, "/api/auth/login/begin");
    assert_eq!(requests[1].method, Method::Post);
    assert_eq!(requests[1].path, "/api/auth/login/complete");
    let body: Value = serde_json::from_slice(&requests[1].body).expect("the body must be JSON");
    assert_eq!(body["credential"]["id"], "ZmFrZQ");

    // サーバーのオプションのチャレンジは base64url から復号して渡す。
    let requested = passkeys.requested();
    assert_eq!(requested.len(), 1);
    assert_eq!(requested[0].challenge, vec![1, 2, 3]);
    assert_eq!(requested[0].rp_id, "localhost");
    assert_eq!(requested[0].user_verification, "required");
    assert!(requested[0].allow_credentials.is_empty());
}

#[test]
fn a_successful_registration_sends_the_token_the_name_and_the_credential() {
    let (client, transport) = client(vec![
        FakeTransport::response(
            200,
            r#"{"challenge":"AQID","rp":{"id":"localhost","name":"brewbook"},"user":{"id":"AQ","name":"u1"},"pubKeyCredParams":[{"type":"public-key","alg":-7}],"attestation":"none","authenticatorSelection":{"residentKey":"preferred","userVerification":"required"},"timeout":300000}"#,
        ),
        FakeTransport::response(200, "{}"),
    ]);
    let passkeys = Rc::new(FakePasskeyClient::new());
    let services = AuthServices::new(client, passkeys.clone());

    block_on(register(&services, "token-1", "自宅の Mac")).expect("the registration must succeed");

    let requests = transport.requests();
    assert_eq!(requests.len(), 2);
    let begin: Value = serde_json::from_slice(&requests[0].body).expect("the body must be JSON");
    assert_eq!(begin["token"], "token-1");
    let complete: Value = serde_json::from_slice(&requests[1].body).expect("the body must be JSON");
    assert_eq!(complete["token"], "token-1");
    assert_eq!(complete["name"], "自宅の Mac");
    assert_eq!(complete["credential"]["id"], "ZmFrZQ");

    let created = passkeys.created();
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].rp_id, "localhost");
    assert_eq!(created[0].user_id, vec![1]);
    assert_eq!(created[0].algorithms, vec![-7]);
    assert_eq!(created[0].attestation, "none");
    assert_eq!(created[0].resident_key, "preferred");
    assert_eq!(created[0].user_verification, "required");
}

#[test]
fn a_logout_posts_to_the_logout_endpoint() {
    let (client, transport) = client(vec![FakeTransport::response(200, "")]);
    let services = AuthServices::new(client, Rc::new(FakePasskeyClient::new()));

    block_on(logout(&services)).expect("the logout must succeed");

    let request = transport.last_request();
    assert_eq!(request.method, Method::Post);
    assert_eq!(request.path, "/api/auth/logout");
}

#[test]
fn a_cancelled_passkey_is_not_sent_to_the_server() {
    let (client, transport) = client(vec![FakeTransport::response(
        200,
        r#"{"challenge":"AQID","rpId":"localhost"}"#,
    )]);
    let mut passkeys = FakePasskeyClient::new();
    passkeys.failure = Some(PasskeyError::cancelled("the credential is null"));
    let services = AuthServices::new(client, Rc::new(passkeys));

    let error = block_on(login(&services)).expect_err("the login must fail");

    assert_eq!(message_key(&error), Key::PasskeyCancelled);
    // クレデンシャルを作れなかったため、完了の呼び出しは送らない。
    assert_eq!(transport.requests().len(), 1);
}

#[test]
fn the_error_messages_follow_the_flutter_mapping() {
    assert_eq!(message_key(&api_error(401)), Key::ErrorUnauthorized);
    assert_eq!(message_key(&api_error(400)), Key::ErrorValidation);
    assert_eq!(message_key(&api_error(404)), Key::ErrorNotFound);
    assert_eq!(message_key(&api_error(409)), Key::ErrorConflict);
    assert_eq!(message_key(&api_error(410)), Key::ErrorGone);
    assert_eq!(message_key(&api_error(500)), Key::ErrorUnexpected);
    assert_eq!(
        message_key(&AuthError::Api(ApiCallError::Network(NetworkError::new(
            "Failed to fetch"
        )))),
        Key::ErrorNetwork
    );
    assert_eq!(
        message_key(&AuthError::Passkey(PasskeyError::unsupported("no"))),
        Key::PasskeyUnsupported
    );
    assert_eq!(
        message_key(&AuthError::Passkey(PasskeyError::failed("no"))),
        Key::ErrorUnexpected
    );
}

#[test]
fn the_registration_errors_use_the_token_messages() {
    assert_eq!(register_error_key(&api_error(400)), Key::ErrorValidation);
    assert_eq!(
        register_error_key(&api_error(404)),
        Key::RegisterTokenNotFound
    );
    assert_eq!(register_error_key(&api_error(409)), Key::RegisterTokenUsed);
    assert_eq!(
        register_error_key(&api_error(410)),
        Key::RegisterTokenExpired
    );
    assert_eq!(register_error_key(&api_error(500)), Key::ErrorUnexpected);

    for status in [404, 409, 410] {
        assert!(
            is_invalid_registration_token(&api_error(status)),
            "the {status} must make the token invalid"
        );
    }
    assert!(!is_invalid_registration_token(&api_error(400)));
    assert!(!is_invalid_registration_token(&api_error(500)));
}

#[test]
fn the_login_errors_use_the_retry_message() {
    assert_eq!(login_error_key(&api_error(400)), Key::LoginFailed);
    assert_eq!(login_error_key(&api_error(409)), Key::LoginFailed);
    assert_eq!(login_error_key(&api_error(401)), Key::ErrorUnauthorized);
}

#[test]
fn a_signed_out_session_is_sent_to_the_login() {
    for route in [Route::Home {}, Route::Settings {}, Route::Stats {}] {
        assert_eq!(
            guard_destination(&route, SessionStatus::SignedOut),
            Some(Route::Login {}),
            "{route:?}"
        );
    }
}

#[test]
fn the_login_and_the_register_stay_open_while_signed_out() {
    assert_eq!(
        guard_destination(&Route::Login {}, SessionStatus::SignedOut),
        None
    );
    for token in [Some("x".to_string()), None] {
        assert_eq!(
            guard_destination(&Route::Register { token }, SessionStatus::SignedOut),
            None
        );
    }
}

#[test]
fn a_signed_in_session_returns_to_the_home_from_the_login_and_the_register() {
    assert_eq!(
        guard_destination(&Route::Login {}, SessionStatus::SignedIn),
        Some(Route::Home {})
    );
    assert_eq!(
        guard_destination(&Route::Register { token: None }, SessionStatus::SignedIn),
        Some(Route::Home {})
    );
    assert_eq!(
        guard_destination(&Route::Home {}, SessionStatus::SignedIn),
        None
    );
}

#[test]
fn the_check_and_the_unknown_states_do_not_navigate() {
    for status in [SessionStatus::Checking, SessionStatus::Unknown] {
        for route in [
            Route::Home {},
            Route::Login {},
            Route::Register { token: None },
        ] {
            assert_eq!(guard_destination(&route, status), None, "{route:?}");
        }
    }
}
