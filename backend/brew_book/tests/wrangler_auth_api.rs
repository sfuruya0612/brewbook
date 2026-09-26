//! 認証とパスキー管理の API の結合テスト (HTTP)。
//!
//! パスキーを伴う一連の流れ (登録、ログイン、追加、名前の変更、削除) は
//! `wrangler_passkey_flow.rs` が CDP の仮想認証器で検査する。ここでは入力の検証、
//! トークンとチャレンジの状態、セッションの要否、他の利用者のパスキーの扱いを検査する。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。

mod support;

use std::sync::OnceLock;

use brew_book_core::auth::{self, hash_secret, KIND_AUTHENTICATION, KIND_REGISTRATION};
use brew_book_core::base64url;
use brew_book_core::ids::uuid_bytes;
use reqwest::blocking::Response;
use serde_json::{json, Value};
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, Seed, SeededPasskey};
use support::ServerLease;

/// テスト側の現在時刻 (epoch ミリ秒)。
fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock must be after the epoch")
        .as_millis() as i64
}

/// 現在時刻から `ttl_seconds` 後の有効期限。負の値は過去になる。
fn expiry(ttl_seconds: i64) -> String {
    auth::expiry_from(now_millis(), ttl_seconds).expect("the expiry must format")
}

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// 一覧のテストの利用者 (パスキー 2 つ。1 つ目は最終使用日時あり)。
    list_user: String,
    list_session: String,
    list_passkeys: Vec<SeededPasskey>,
    /// 名前の変更のテストの利用者のセッション (パスキー 1 つ)。
    rename_session: String,
    rename_passkey: SeededPasskey,
    /// 削除のテストの利用者のセッション (パスキー 2 つ)。
    delete_session: String,
    delete_passkeys: Vec<SeededPasskey>,
    /// 最後の 1 つの削除のテストの利用者 (パスキー 1 つ)。
    last_user: String,
    last_session: String,
    last_passkey: SeededPasskey,
    /// 他の利用者のパスキーのテストの利用者 (パスキー 1 つ)。
    other_user: String,
    other_session: String,
    other_passkey: SeededPasskey,
    /// 登録用トークンのテストの利用者。
    token_user: String,
    valid_token: String,
    used_token: String,
    expired_token: String,
    /// ログアウトのテストのセッション。
    logout_session: String,
    /// ログアウトでセッションの行が消えることのテストの利用者 (セッション 1 つ)。
    logout_row_user: String,
    logout_row_session: String,
    /// 期限切れのセッションのテストのセッション。
    expired_session: String,
    /// ログインの検証のテストに使うチャレンジとパスキー。
    crafted_challenge: String,
    crafted_passkey: SeededPasskey,
    /// 他の利用者に発行された登録の種別のチャレンジと、ログインの種別のチャレンジ。
    other_registration_challenge: String,
    other_login_challenge: String,
}

/// 下ごしらえを 1 回だけ組み立てる。
fn data() -> &'static TestData {
    static DATA: OnceLock<TestData> = OnceLock::new();
    DATA.get_or_init(build_data)
}

/// 共有のサーバーを借りる。
fn server() -> ServerLease {
    support::shared_server("main", || {
        support::DevServer::start_with(
            |port| {
                vec![
                    ("RP_ID".to_owned(), "localhost".to_owned()),
                    ("ORIGIN".to_owned(), format!("http://localhost:{port}")),
                ]
            },
            &data().seed_sql,
        )
    })
    .expect("wrangler dev must start")
}

/// 下ごしらえの SQL と、テストが使う値を作る。
fn build_data() -> TestData {
    let future = expiry(auth::REGISTRATION_TOKEN_TTL_SECONDS);
    let past = expiry(-60);
    let created = expiry(-3_600);
    let used_at = expiry(-30);
    let mut seed = Seed::new();

    let list_user = user_id(1);
    seed.user(&list_user, "list user", &created);
    let list_session = seed.session(&list_user, &future, &created);
    let list_passkeys = vec![
        seed.passkey(&list_user, "最初のパスキー", &created, Some(&used_at)),
        seed.passkey(&list_user, "2 つ目のパスキー", &created, None),
    ];

    let rename_user = user_id(2);
    seed.user(&rename_user, "rename user", &created);
    let rename_session = seed.session(&rename_user, &future, &created);
    let rename_passkey = seed.passkey(&rename_user, "変更前の名前", &created, None);

    let delete_user = user_id(3);
    seed.user(&delete_user, "delete user", &created);
    let delete_session = seed.session(&delete_user, &future, &created);
    let delete_passkeys = vec![
        seed.passkey(&delete_user, "残すパスキー", &created, None),
        seed.passkey(&delete_user, "消すパスキー", &created, None),
    ];

    let last_user = user_id(4);
    seed.user(&last_user, "last user", &created);
    let last_session = seed.session(&last_user, &future, &created);
    let last_passkey = seed.passkey(&last_user, "最後のパスキー", &created, None);

    let other_user = user_id(5);
    seed.user(&other_user, "other user", &created);
    let other_session = seed.session(&other_user, &future, &created);
    let other_passkey = seed.passkey(&other_user, "他の利用者のパスキー", &created, None);

    let token_user = user_id(6);
    seed.user(&token_user, "token user", &created);
    let valid_token = seed.registration_token(&token_user, &future);
    let used_token = seed.used_registration_token(&token_user, &future, &used_at);
    let expired_token = seed.registration_token(&token_user, &past);

    let logout_user = user_id(7);
    seed.user(&logout_user, "logout user", &created);
    let logout_session = seed.session(&logout_user, &future, &created);

    let logout_row_user = user_id(9);
    seed.user(&logout_row_user, "logout row user", &created);
    let logout_row_session = seed.session(&logout_row_user, &future, &created);

    let expired_user = user_id(8);
    seed.user(&expired_user, "expired user", &created);
    let expired_session = seed.session(&expired_user, &past, &created);

    let crafted_challenge = "crafted-challenge-valid".to_owned();
    seed.challenge(None, KIND_AUTHENTICATION, &crafted_challenge, &future);
    // 検証に使うパスキーは、公開鍵が CBOR ではないため署名の検証で必ず失敗する。
    let crafted_passkey = seed.passkey(&token_user, "検証用のパスキー", &created, None);

    // チャレンジの利用者と種別の束縛を検査するための行 (他の利用者に発行されたもの)。
    let other_registration_challenge = "crafted-other-registration-challenge".to_owned();
    seed.challenge(
        Some(&other_user),
        KIND_REGISTRATION,
        &other_registration_challenge,
        &future,
    );
    let other_login_challenge = "crafted-other-login-challenge".to_owned();
    seed.challenge(
        Some(&other_user),
        KIND_AUTHENTICATION,
        &other_login_challenge,
        &future,
    );

    TestData {
        seed_sql: seed.sql(),
        list_user,
        list_session,
        list_passkeys,
        rename_session,
        rename_passkey,
        delete_session,
        delete_passkeys,
        last_user,
        last_session,
        last_passkey,
        other_user,
        other_session,
        other_passkey,
        token_user,
        valid_token,
        used_token,
        expired_token,
        logout_session,
        logout_row_user,
        logout_row_session,
        expired_session,
        crafted_challenge,
        crafted_passkey,
        other_registration_challenge,
        other_login_challenge,
    }
}

/// セッションの Cookie を持たないクライアント。API の基底 URL は引数のものを使う。
fn anonymous(base_url: &str) -> ApiClient {
    ApiClient::new(base_url, None)
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

// 登録のチャレンジ発行 (認証不要、入力あり)。

#[test]
fn wrangler_auth_register_begin_ok() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let body = assert_status(
        anonymous(&base_url).post_json(
            "/api/auth/register/begin",
            &json!({ "token": data.valid_token }),
        ),
        200,
    );
    assert!(
        body["challenge"]
            .as_str()
            .is_some_and(|challenge| challenge.len() == 43),
        "the challenge must be the base64url of 32 bytes: {body}"
    );
    assert_eq!(body["rp"]["id"], "localhost");
    assert_eq!(body["rp"]["name"], "brewbook");
    assert_eq!(body["user"]["name"], data.token_user);
    assert!(
        body["user"].get("displayName").is_none(),
        "displayName must not be set: {body}"
    );
    let expected_id = uuid_bytes(&data.token_user).expect("the user id must be a UUID");
    assert_eq!(body["user"]["id"], base64url::encode(&expected_id));
    assert_eq!(
        body["pubKeyCredParams"],
        json!([{ "type": "public-key", "alg": -7 }])
    );
    assert_eq!(body["attestation"], "none");
    assert_eq!(
        body["authenticatorSelection"],
        json!({ "residentKey": "preferred", "userVerification": "required" })
    );
    assert!(body["timeout"].as_u64().unwrap_or(0) > 0, "timeout: {body}");

    // チャレンジは利用者と種別 (登録) を持つ (ADR-0006)。
    let count = lease
        .use_server(|server| {
            server.query_int(&format!(
                "SELECT COUNT(*) AS count FROM webauthn_challenges WHERE user_id = '{}' AND kind = '{KIND_REGISTRATION}'",
                data.token_user
            ))
        })
        .expect("the challenge rows must be readable");
    assert!(count >= 1, "the registration challenge must be stored");
}

#[test]
fn wrangler_auth_register_begin_invalid_input_400() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let body = assert_status(
        anonymous(&base_url).post_json("/api/auth/register/begin", &json!({})),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
    // 空のトークンも入力不正にする。
    let body = assert_status(
        anonymous(&base_url).post_json("/api/auth/register/begin", &json!({ "token": "  " })),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
}

#[test]
fn wrangler_auth_register_begin_missing_token_404() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let body = assert_status(
        anonymous(&base_url).post_json(
            "/api/auth/register/begin",
            &json!({ "token": "no-such-token" }),
        ),
        404,
    );
    assert_eq!(error_code(&body), Some("not_found"));
}

#[test]
fn wrangler_auth_register_begin_used_token_409() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let body = assert_status(
        anonymous(&base_url).post_json(
            "/api/auth/register/begin",
            &json!({ "token": data.used_token }),
        ),
        409,
    );
    assert_eq!(error_code(&body), Some("conflict"));
}

#[test]
fn wrangler_auth_register_begin_expired_token_410() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let body = assert_status(
        anonymous(&base_url).post_json(
            "/api/auth/register/begin",
            &json!({ "token": data.expired_token }),
        ),
        410,
    );
    assert_eq!(error_code(&body), Some("gone"));
}

// 登録の検証 (認証不要、入力あり)。正常系は `wrangler_passkey_flow.rs` が持つ。

#[test]
fn wrangler_auth_register_complete_invalid_input_400() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = anonymous(&base_url);
    let credential = json!({
        "response": { "clientDataJSON": "AAAA", "attestationObject": "AAAA" }
    });
    // 名前が範囲外 (51 文字)。
    let body = assert_status(
        client.post_json(
            "/api/auth/register/complete",
            &json!({
                "token": data.valid_token,
                "name": "a".repeat(auth::PASSKEY_NAME_MAX_CHARS + 1),
                "credential": credential,
            }),
        ),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
    // 名前が空白だけ。
    let body = assert_status(
        client.post_json(
            "/api/auth/register/complete",
            &json!({ "token": data.valid_token, "name": "   ", "credential": credential }),
        ),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
    // 名前が無い。
    let body = assert_status(
        client.post_json(
            "/api/auth/register/complete",
            &json!({ "token": data.valid_token, "credential": credential }),
        ),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
    // クレデンシャルが無い。
    let body = assert_status(
        client.post_json(
            "/api/auth/register/complete",
            &json!({ "token": data.valid_token, "name": "名前" }),
        ),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
}

#[test]
fn wrangler_auth_register_complete_missing_token_404() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let credential = json!({
        "response": { "clientDataJSON": "AAAA", "attestationObject": "AAAA" }
    });
    let body = assert_status(
        anonymous(&base_url).post_json(
            "/api/auth/register/complete",
            &json!({ "token": "no-such-token", "name": "名前", "credential": credential }),
        ),
        404,
    );
    assert_eq!(error_code(&body), Some("not_found"));
}

#[test]
fn wrangler_auth_register_complete_expired_token_410() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let credential = json!({
        "response": { "clientDataJSON": "AAAA", "attestationObject": "AAAA" }
    });
    let body = assert_status(
        anonymous(&base_url).post_json(
            "/api/auth/register/complete",
            &json!({ "token": data.expired_token, "name": "名前", "credential": credential }),
        ),
        410,
    );
    assert_eq!(error_code(&body), Some("gone"));
}

#[test]
fn wrangler_auth_register_complete_used_token_409() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let credential = json!({
        "response": { "clientDataJSON": "AAAA", "attestationObject": "AAAA" }
    });
    let body = assert_status(
        anonymous(&base_url).post_json(
            "/api/auth/register/complete",
            &json!({ "token": data.used_token, "name": "名前", "credential": credential }),
        ),
        409,
    );
    assert_eq!(error_code(&body), Some("conflict"));
}

#[test]
fn wrangler_auth_register_complete_challenge_of_another_user_409() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let origin = lease.use_server(|server| server.origin());
    // 他の利用者に発行されたチャレンジは、トークンの利用者と一致しないため 409 になる (ADR-0006)。
    let body = assert_status(
        anonymous(&base_url).post_json(
            "/api/auth/register/complete",
            &json!({
                "token": data.valid_token,
                "name": "名前",
                "credential": { "response": {
                    "clientDataJSON": crafted_client_data(
                        "webauthn.create",
                        &data.other_registration_challenge,
                        &origin,
                    ),
                    "attestationObject": "AAAA"
                } }
            }),
        ),
        409,
    );
    assert_eq!(error_code(&body), Some("conflict"));
    // トークンの使用済みなどの別の 409 ではなく、チャレンジの照合で拒否されたことを確かめる。
    assert!(
        body["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("challenge")),
        "the rejection must come from the challenge: {body}"
    );
}

// ログインのチャレンジ発行 (認証不要、入力なし)。

#[test]
fn wrangler_auth_login_begin_ok() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    // 入力を持たないため、本体なしで呼ぶ (FR-2)。
    let body = assert_status(anonymous(&base_url).post("/api/auth/login/begin"), 200);
    assert!(
        body["challenge"]
            .as_str()
            .is_some_and(|challenge| challenge.len() == 43),
        "the challenge must be the base64url of 32 bytes: {body}"
    );
    assert_eq!(body["rpId"], "localhost");
    assert_eq!(body["userVerification"], "required");
    assert_eq!(body["allowCredentials"], json!([]));
    assert!(body["timeout"].as_u64().unwrap_or(0) > 0, "timeout: {body}");

    // ログインのチャレンジは利用者を持たない (ADR-0006)。
    let count = lease
        .use_server(|server| {
            server.query_int(&format!(
                "SELECT COUNT(*) AS count FROM webauthn_challenges WHERE user_id IS NULL AND kind = '{KIND_AUTHENTICATION}'"
            ))
        })
        .expect("the challenge rows must be readable");
    assert!(count >= 1, "the login challenge must be stored");
}

// ログインの検証 (認証不要、入力あり)。正常系は `wrangler_passkey_flow.rs` が持つ。

#[test]
fn wrangler_auth_login_complete_invalid_input_400() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = anonymous(&base_url);
    // クレデンシャルが無い。
    let body = assert_status(
        client.post_json("/api/auth/login/complete", &json!({})),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
    // `clientDataJSON` が base64url ではない。
    let body = assert_status(
        client.post_json(
            "/api/auth/login/complete",
            &json!({ "credential": { "id": "a", "response": {
                "clientDataJSON": "not base64url!",
                "authenticatorData": "AAAA",
                "signature": "AAAA"
            } } }),
        ),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
}

#[test]
fn wrangler_auth_login_complete_failed_verification_400() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let origin = lease.use_server(|server| server.origin());
    let body = assert_status(
        anonymous(&base_url).post_json(
            "/api/auth/login/complete",
            &json!({ "credential": {
                "id": data.crafted_passkey.credential_id,
                "response": {
                    "clientDataJSON": crafted_client_data("webauthn.get", &data.crafted_challenge, &origin),
                    "authenticatorData": crafted_authenticator_data("localhost"),
                    "signature": "AAAA"
                }
            } }),
        ),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
}

#[test]
fn wrangler_auth_login_complete_unknown_challenge_409() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let origin = lease.use_server(|server| server.origin());
    // 一致する行が無いチャレンジは、使用済みの再利用と同じ 409 にする (ADR-0004)。
    let body = assert_status(
        anonymous(&base_url).post_json(
            "/api/auth/login/complete",
            &json!({ "credential": {
                "id": data.crafted_passkey.credential_id,
                "response": {
                    "clientDataJSON": crafted_client_data("webauthn.get", "no-such-challenge", &origin),
                    "authenticatorData": crafted_authenticator_data("localhost"),
                    "signature": "AAAA"
                }
            } }),
        ),
        409,
    );
    assert_eq!(error_code(&body), Some("conflict"));
}

#[test]
fn wrangler_auth_login_complete_challenge_bound_to_another_user_or_kind_409() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let origin = lease.use_server(|server| server.origin());
    let client = anonymous(&base_url);
    // 他の利用者に発行されたログインのチャレンジは、利用者を持たない照合と一致しない (ADR-0006)。
    let body = assert_status(
        client.post_json(
            "/api/auth/login/complete",
            &json!({ "credential": {
                "id": data.crafted_passkey.credential_id,
                "response": {
                    "clientDataJSON": crafted_client_data("webauthn.get", &data.other_login_challenge, &origin),
                    "authenticatorData": crafted_authenticator_data("localhost"),
                    "signature": "AAAA"
                }
            } }),
        ),
        409,
    );
    assert_eq!(error_code(&body), Some("conflict"));
    // 登録の種別のチャレンジは、ログインの種別と一致しない。
    let body = assert_status(
        client.post_json(
            "/api/auth/login/complete",
            &json!({ "credential": {
                "id": data.crafted_passkey.credential_id,
                "response": {
                    "clientDataJSON": crafted_client_data("webauthn.get", &data.other_registration_challenge, &origin),
                    "authenticatorData": crafted_authenticator_data("localhost"),
                    "signature": "AAAA"
                }
            } }),
        ),
        409,
    );
    assert_eq!(error_code(&body), Some("conflict"));
}

// ログアウト (認証が必要、入力なし)。

#[test]
fn wrangler_auth_logout_ok() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let response = ApiClient::new(&base_url, Some(&data.logout_session)).post("/api/auth/logout");
    let cleared = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("session="))
        .map(str::to_owned);
    let cookie = support::http::session_token(&response);
    let body = assert_status(response, 200);
    assert_eq!(body["logged_out"], true);
    let cleared = cleared.expect("the response must clear the session cookie");
    assert!(
        cleared.starts_with("session=;"),
        "the cookie must carry no token: {cleared}"
    );
    assert!(
        cleared.contains("Max-Age=0"),
        "the cookie must expire: {cleared}"
    );
    assert_eq!(cookie, None, "the cookie must not carry a token");
}

#[test]
fn wrangler_auth_logout_unauthenticated_401() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    assert_unauthorized(anonymous(&base_url).post("/api/auth/logout"));
}

#[test]
fn wrangler_auth_logout_deletes_the_session_row_401() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    // ログアウトの前にセッションが通ることを確かめる。
    assert_status(
        ApiClient::new(&base_url, Some(&data.logout_row_session)).get("/api/passkeys"),
        200,
    );
    assert_status(
        ApiClient::new(&base_url, Some(&data.logout_row_session)).post("/api/auth/logout"),
        200,
    );
    // 同じセッションでは認証が必要な API を呼べない (FR-4)。
    assert_unauthorized(
        ApiClient::new(&base_url, Some(&data.logout_row_session)).get("/api/passkeys"),
    );
    // セッションの行は物理削除されている。
    let count = lease
        .use_server(|server| {
            server.query_int(&format!(
                "SELECT COUNT(*) AS count FROM sessions WHERE user_id = '{}'",
                data.logout_row_user
            ))
        })
        .expect("the sessions must be readable");
    assert_eq!(count, 0, "the session row must be deleted");
}

// パスキーの一覧 (認証が必要、入力なし)。

#[test]
fn wrangler_passkeys_list_ok() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let body = assert_status(
        ApiClient::new(&base_url, Some(&data.list_session)).get("/api/passkeys"),
        200,
    );
    let passkeys = body["passkeys"]
        .as_array()
        .expect("the response must have passkeys");
    assert_eq!(passkeys.len(), 2);
    // 登録日時の昇順で返す。
    assert_eq!(passkeys[0]["id"], data.list_passkeys[0].id);
    assert_eq!(passkeys[0]["name"], data.list_passkeys[0].name);
    assert_eq!(passkeys[1]["id"], data.list_passkeys[1].id);
    assert_eq!(passkeys[1]["name"], data.list_passkeys[1].name);
    assert!(
        passkeys[0]["created_at"]
            .as_str()
            .is_some_and(|value| value.len() == 24),
        "created_at must be the fixed length ISO 8601 UTC: {body}"
    );
    assert!(
        passkeys[0]["last_used_at"]
            .as_str()
            .is_some_and(|value| value.len() == 24),
        "the used passkey must carry last_used_at: {body}"
    );
    assert_eq!(passkeys[1]["last_used_at"], Value::Null);
}

#[test]
fn wrangler_passkeys_list_unauthenticated_401() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    assert_unauthorized(anonymous(&base_url).get("/api/passkeys"));
}

// パスキーの追加のチャレンジ発行 (認証が必要、入力なし)。

#[test]
fn wrangler_passkeys_begin_ok() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let body = assert_status(
        ApiClient::new(&base_url, Some(&data.list_session)).post("/api/passkeys/begin"),
        200,
    );
    assert!(
        body["challenge"]
            .as_str()
            .is_some_and(|challenge| challenge.len() == 43),
        "the challenge must be the base64url of 32 bytes: {body}"
    );
    assert_eq!(body["rp"]["id"], "localhost");
    assert_eq!(body["user"]["name"], data.list_user);
    assert_eq!(
        body["authenticatorSelection"],
        json!({ "residentKey": "preferred", "userVerification": "required" })
    );
    // 追加のチャレンジはセッションの利用者と種別 (登録) を持つ。
    let count = lease
        .use_server(|server| {
            server.query_int(&format!(
                "SELECT COUNT(*) AS count FROM webauthn_challenges WHERE user_id = '{}' AND kind = '{KIND_REGISTRATION}'",
                data.list_user
            ))
        })
        .expect("the challenge rows must be readable");
    assert!(count >= 1, "the add challenge must be stored");
}

#[test]
fn wrangler_passkeys_begin_unauthenticated_401() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    assert_unauthorized(anonymous(&base_url).post("/api/passkeys/begin"));
}

// パスキーの追加の検証 (認証が必要、入力あり)。正常系は `wrangler_passkey_flow.rs` が持つ。

#[test]
fn wrangler_passkeys_complete_invalid_input_400() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let credential = json!({
        "response": { "clientDataJSON": "AAAA", "attestationObject": "AAAA" }
    });
    let body = assert_status(
        ApiClient::new(&base_url, Some(&data.list_session)).post_json(
            "/api/passkeys/complete",
            &json!({
                "name": "a".repeat(auth::PASSKEY_NAME_MAX_CHARS + 1),
                "credential": credential,
            }),
        ),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
}

#[test]
fn wrangler_passkeys_complete_challenge_of_another_user_409() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let origin = lease.use_server(|server| server.origin());
    // 他の利用者に発行されたチャレンジは、セッションの利用者と一致しないため 409 になる (ADR-0006)。
    let body = assert_status(
        ApiClient::new(&base_url, Some(&data.list_session)).post_json(
            "/api/passkeys/complete",
            &json!({
                "name": "名前",
                "credential": { "response": {
                    "clientDataJSON": crafted_client_data(
                        "webauthn.create",
                        &data.other_registration_challenge,
                        &origin,
                    ),
                    "attestationObject": "AAAA"
                } }
            }),
        ),
        409,
    );
    assert_eq!(error_code(&body), Some("conflict"));
}

#[test]
fn wrangler_passkeys_complete_unauthenticated_401() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let credential = json!({
        "response": { "clientDataJSON": "AAAA", "attestationObject": "AAAA" }
    });
    assert_unauthorized(anonymous(&base_url).post_json(
        "/api/passkeys/complete",
        &json!({ "name": "名前", "credential": credential }),
    ));
}

// パスキーの名前の変更 (認証が必要、入力あり)。

#[test]
fn wrangler_passkeys_rename_ok() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, Some(&data.rename_session));
    let body = assert_status(
        client.patch_json(
            &format!("/api/passkeys/{}", data.rename_passkey.id),
            &json!({ "name": "  変えた名前  " }),
        ),
        200,
    );
    // 前後の空白は除く。
    assert_eq!(body["name"], "変えた名前");
    assert_eq!(body["id"], data.rename_passkey.id);

    let list = assert_status(client.get("/api/passkeys"), 200);
    assert_eq!(list["passkeys"][0]["name"], "変えた名前");
}

#[test]
fn wrangler_passkeys_rename_invalid_input_400() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, Some(&data.rename_session));
    let body = assert_status(
        client.patch_json(
            &format!("/api/passkeys/{}", data.rename_passkey.id),
            &json!({ "name": "" }),
        ),
        400,
    );
    assert_eq!(error_code(&body), Some("bad_request"));
}

#[test]
fn wrangler_passkeys_rename_unauthenticated_401() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    assert_unauthorized(anonymous(&base_url).patch_json(
        &format!("/api/passkeys/{}", data.rename_passkey.id),
        &json!({ "name": "名前" }),
    ));
}

#[test]
fn wrangler_passkeys_rename_other_user_404() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    // 他の利用者のパスキーの変更は 404 を返す (FR-5)。
    let body = assert_status(
        ApiClient::new(&base_url, Some(&data.list_session)).patch_json(
            &format!("/api/passkeys/{}", data.other_passkey.id),
            &json!({ "name": "のっとり" }),
        ),
        404,
    );
    assert_eq!(error_code(&body), Some("not_found"));
    // 他の利用者のパスキーは変わっていない。
    let list = assert_status(
        ApiClient::new(&base_url, Some(&data.other_session)).get("/api/passkeys"),
        200,
    );
    assert_eq!(list["passkeys"][0]["name"], data.other_passkey.name);
}

// パスキーの削除 (認証が必要、入力なし)。

#[test]
fn wrangler_passkeys_delete_ok() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, Some(&data.delete_session));
    let body = assert_status(
        client.delete(&format!("/api/passkeys/{}", data.delete_passkeys[1].id)),
        200,
    );
    assert_eq!(body["id"], data.delete_passkeys[1].id);

    let list = assert_status(client.get("/api/passkeys"), 200);
    let passkeys = list["passkeys"].as_array().expect("passkeys");
    assert_eq!(passkeys.len(), 1);
    assert_eq!(passkeys[0]["id"], data.delete_passkeys[0].id);
}

#[test]
fn wrangler_passkeys_delete_last_conflict_409() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let body = assert_status(
        ApiClient::new(&base_url, Some(&data.last_session))
            .delete(&format!("/api/passkeys/{}", data.last_passkey.id)),
        409,
    );
    assert_eq!(error_code(&body), Some("conflict"));
    // 最後の 1 つは残る。
    let count = lease
        .use_server(|server| {
            server.query_int(&format!(
                "SELECT COUNT(*) AS count FROM passkey_credentials WHERE user_id = '{}'",
                data.last_user
            ))
        })
        .expect("the passkeys must be readable");
    assert_eq!(count, 1);
}

#[test]
fn wrangler_passkeys_delete_other_user_404() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let body = assert_status(
        ApiClient::new(&base_url, Some(&data.list_session))
            .delete(&format!("/api/passkeys/{}", data.other_passkey.id)),
        404,
    );
    assert_eq!(error_code(&body), Some("not_found"));
    let count = lease
        .use_server(|server| {
            server.query_int(&format!(
                "SELECT COUNT(*) AS count FROM passkey_credentials WHERE user_id = '{}'",
                data.other_user
            ))
        })
        .expect("the passkeys must be readable");
    assert_eq!(count, 1);
}

#[test]
fn wrangler_passkeys_delete_unauthenticated_401() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    assert_unauthorized(
        anonymous(&base_url).delete(&format!("/api/passkeys/{}", data.other_passkey.id)),
    );
}

// テスト専用の経路 (台帳に無い経路)。

#[test]
fn wrangler_test_page_is_disabled_without_its_var() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    // 本番の vars には TEST_PAGE が無いため、経路は台帳に無い経路と同じ 404 になる。
    // テスト専用のページを本番に残さないことを確認する。
    let body = assert_status(anonymous(&base_url).get(brew_book::test_page::PATH), 404);
    assert_eq!(error_code(&body), Some("not_found"));
}

// セッションの有効期限 (認証が必要な経路の 401)。

#[test]
fn wrangler_auth_session_expired_401() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    assert_unauthorized(
        ApiClient::new(&base_url, Some(&data.expired_session)).get("/api/passkeys"),
    );
}

// 検証の材料。

/// 署名を付けない `clientDataJSON` を作る (base64url)。
fn crafted_client_data(type_: &str, challenge: &str, origin: &str) -> String {
    let json = json!({ "type": type_, "challenge": challenge, "origin": origin });
    base64url::encode(json.to_string().as_bytes())
}

/// rpIdHash と UP と UV のフラグだけを持つ authenticatorData を作る (base64url)。
/// 署名カウンタは 0、attested credential data は無し。
fn crafted_authenticator_data(rp_id: &str) -> String {
    let hash = hash_secret(rp_id);
    let mut bytes = hex_bytes(&hash);
    bytes.push(0x01 | 0x04);
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    base64url::encode(&bytes)
}

/// 16 進の文字列をバイト列にする。
fn hex_bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("hex must be ASCII");
            u8::from_str_radix(text, 16).expect("hex must be valid")
        })
        .collect()
}
