//! `auth` の単体テスト。起動時のセッション確認 (FR-1、FR-2) を検査する。

mod support;

use std::cell::Cell;
use std::rc::Rc;

use brew_book_frontend::app::unauthorized_route;
use brew_book_frontend::auth::{check_session, SessionStatus};
use brew_book_frontend::router::Route;

use support::{block_on, client, FakeTransport};

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

#[test]
fn the_unauthorized_destination_is_the_login() {
    // 401 の応答で遷移させる経路 (FR-2)。AppShell はこの関数で遷移する。
    assert_eq!(unauthorized_route(), Route::Login {});
}
