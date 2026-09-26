//! パスキーを伴う結合テスト。CDP の仮想認証器 (Chrome DevTools Protocol) を使う (ADR-0004)。
//!
//! テストページ (`/api/__test_page`、`TEST_PAGE` の var で有効) の `window.coffeeLogTest` を呼び、
//! 登録、ログイン、パスキーの追加、名前の変更、削除を一連で検査する。
//!
//! 検査する経路と種別 (0001 の台帳の照合)。
//!
//! - `auth_register_complete`: 正常系と使用済みトークン 409
//! - `auth_login_complete`: 正常系、使用済みチャレンジ 409、検証の失敗 400、期限切れチャレンジ 410、
//!   署名カウンタの後退 409
//! - `passkeys_complete`: 正常系
//! - `passkeys_rename` / `passkeys_delete`: 正常系と最後の 1 つの 409
//! - `auth_logout`: 正常系と、ログアウト後の 401
//! - 認証が必要な経路: 有効期限を短縮した設定での 401
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。

mod support;

use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use coffee_log_core::auth;
use coffee_log_core::base64url;
use serde_json::Value;
use support::cdp::{js_string, TestBrowser};
use support::seed::{user_id, Seed};
use support::ServerLease;

/// 有効期限を短縮するテスト用の秒数。チャレンジとセッションの期限切れを作る。
/// CI の CPU 競合でも、期限までの 2 往復が収まる余裕を残す (短すぎると環境起因で落ちる)。
const SHORT_TTL_SECONDS: i64 = 5;
/// 短縮した有効期限を過ぎるまで待つ時間。
const SHORT_TTL_WAIT: Duration = Duration::from_millis(5_300);

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

/// テストページを有効にした vars。RP ID と Origin をページのオリジンに合わせる。
fn vars(port: u16, extra: Vec<(&str, String)>) -> Vec<(String, String)> {
    let mut vars = vec![
        ("RP_ID".to_owned(), "localhost".to_owned()),
        ("ORIGIN".to_owned(), format!("http://localhost:{port}")),
        (
            coffee_log::test_page::VAR_NAME.to_owned(),
            "true".to_owned(),
        ),
    ];
    vars.extend(
        extra
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value)),
    );
    vars
}

/// 既定の有効期限のサーバーで使う下ごしらえ (利用者 5 人と登録用トークン)。
struct FlowData {
    seed_sql: String,
    users: Vec<String>,
    tokens: Vec<String>,
}

fn flow_data() -> &'static FlowData {
    static DATA: OnceLock<FlowData> = OnceLock::new();
    DATA.get_or_init(|| {
        let future = expiry(auth::REGISTRATION_TOKEN_TTL_SECONDS);
        let created = expiry(-3_600);
        let mut seed = Seed::new();
        let mut users = Vec::new();
        let mut tokens = Vec::new();
        for index in 1..=5 {
            let user = user_id(index);
            seed.user(&user, &format!("flow user {index}"), &created);
            let token = seed.registration_token(&user, &future);
            users.push(user);
            tokens.push(token);
        }
        FlowData {
            seed_sql: seed.sql(),
            users,
            tokens,
        }
    })
}

/// セッションの有効期限を短縮したサーバーで使う下ごしらえ (利用者 1 人と登録用トークン)。
struct SessionData {
    seed_sql: String,
    token: String,
}

fn session_data() -> &'static SessionData {
    static DATA: OnceLock<SessionData> = OnceLock::new();
    DATA.get_or_init(|| {
        let future = expiry(auth::REGISTRATION_TOKEN_TTL_SECONDS);
        let created = expiry(-3_600);
        let mut seed = Seed::new();
        let user = user_id(6);
        seed.user(&user, "session user", &created);
        let token = seed.registration_token(&user, &future);
        SessionData {
            seed_sql: seed.sql(),
            token,
        }
    })
}

/// 既定の有効期限のサーバー。
fn main_server() -> ServerLease {
    support::shared_server("main", || {
        support::DevServer::start_with(|port| vars(port, Vec::new()), &flow_data().seed_sql)
    })
    .expect("wrangler dev must start")
}

/// チャレンジの有効期限を 5 秒にしたサーバー。
fn challenge_server() -> ServerLease {
    support::shared_server("challenge", || {
        support::DevServer::start_with(
            |port| {
                vars(
                    port,
                    vec![(
                        coffee_log::auth::CHALLENGE_TTL_VAR,
                        SHORT_TTL_SECONDS.to_string(),
                    )],
                )
            },
            "",
        )
    })
    .expect("wrangler dev must start")
}

/// セッションの有効期限を 5 秒にしたサーバー。
fn session_server() -> ServerLease {
    support::shared_server("session", || {
        support::DevServer::start_with(
            |port| {
                vars(
                    port,
                    vec![(
                        coffee_log::auth::SESSION_TTL_VAR,
                        SHORT_TTL_SECONDS.to_string(),
                    )],
                )
            },
            &session_data().seed_sql,
        )
    })
    .expect("wrangler dev must start")
}

/// テストページの URL。
fn page_url(lease: &ServerLease) -> String {
    lease.use_server(|server| format!("{}{}", server.localhost_url(), coffee_log::test_page::PATH))
}

/// アサーションの authenticatorData (base64url) から署名カウンタを読む。
/// 並びは rpIdHash (32 バイト)、フラグ (1 バイト)、署名カウンタ (4 バイト、ビッグエンディアン)。
fn sign_count_of(authenticator_data: &str) -> i64 {
    let bytes =
        base64url::decode(authenticator_data).expect("the authenticator data must be base64url");
    let counter: [u8; 4] = bytes
        .get(33..37)
        .expect("the authenticator data must carry the signature counter")
        .try_into()
        .expect("the signature counter must be 4 bytes");
    i64::from(u32::from_be_bytes(counter))
}

/// 式の結果を確かめ、状態コードを取り出す。
fn status_of(result: &Value) -> i64 {
    result["status"]
        .as_i64()
        .unwrap_or_else(|| panic!("the result must carry a status: {result}"))
}

// 登録の検証 (auth_register_complete): 正常系、セッションの発行、使用済みトークンの 409、
// ログアウト (auth_logout) の正常系と、ログアウト後の 401。

#[test]
fn wrangler_auth_register_complete_ok_and_logout_401() {
    let data = flow_data();
    let lease = main_server();
    let browser = TestBrowser::open(&page_url(&lease)).expect("Chrome must open the test page");
    let script = r#"(async () => {
      const registered = await window.coffeeLogTest.registerWithToken(__TOKEN__, "登録したパスキー");
      const listed = await window.coffeeLogTest.listPasskeys();
      const again = await window.coffeeLogTest.registerWithToken(__TOKEN__, "2 回目");
      const loggedOut = await window.coffeeLogTest.logout();
      const afterLogout = await window.coffeeLogTest.listPasskeys();
      return { registered, listed, again, loggedOut, afterLogout };
    })()"#
        .replace("__TOKEN__", &js_string(&data.tokens[0]));
    let result = browser
        .evaluate_json(&script)
        .expect("the registration flow must return JSON");

    assert_eq!(
        status_of(&result["registered"]),
        200,
        "the registration must succeed: {}",
        result["registered"]
    );
    assert_eq!(result["registered"]["body"]["user_id"], data.users[0]);
    assert_eq!(
        result["registered"]["body"]["passkey"]["name"],
        "登録したパスキー"
    );
    assert_eq!(
        status_of(&result["listed"]),
        200,
        "the session must work after the registration: {}",
        result["listed"]
    );
    assert_eq!(
        result["listed"]["body"]["passkeys"][0]["name"],
        "登録したパスキー"
    );
    // 使用済みのトークンでは登録できない (FR-1)。
    assert_eq!(
        status_of(&result["again"]),
        409,
        "the used token must be rejected: {}",
        result["again"]
    );
    assert_eq!(
        status_of(&result["loggedOut"]),
        200,
        "the logout must succeed: {}",
        result["loggedOut"]
    );
    // ログアウト後は同じセッションで認証が必要な API を呼べない (FR-4)。
    assert_eq!(
        status_of(&result["afterLogout"]),
        401,
        "the session must be rejected after the logout: {}",
        result["afterLogout"]
    );
    // セッションの行は物理削除されている。
    let count = lease
        .use_server(|server| {
            server.query_int(&format!(
                "SELECT COUNT(*) AS count FROM sessions WHERE user_id = '{}'",
                data.users[0]
            ))
        })
        .expect("the sessions must be readable");
    assert_eq!(count, 0, "the session row must be deleted");
}

// パスキーの追加の検証 (passkeys_complete)、名前の変更 (passkeys_rename)、削除 (passkeys_delete)。

#[test]
fn wrangler_passkeys_complete_ok_and_the_passkey_is_managed() {
    let data = flow_data();
    let lease = main_server();
    let browser = TestBrowser::open(&page_url(&lease)).expect("Chrome must open the test page");
    let script = r#"(async () => {
      const registered = await window.coffeeLogTest.registerWithToken(__TOKEN__, "最初のパスキー");
      const added = await window.coffeeLogTest.addPasskey("2 つ目のパスキー");
      const listed = await window.coffeeLogTest.listPasskeys();
      const renamed = await window.coffeeLogTest.renamePasskey(added.body.id, "変えた名前");
      const deleted = await window.coffeeLogTest.deletePasskey(registered.body.passkey.id);
      const remaining = await window.coffeeLogTest.listPasskeys();
      const last = await window.coffeeLogTest.deletePasskey(added.body.id);
      const stillRemaining = await window.coffeeLogTest.listPasskeys();
      return { registered, added, listed, renamed, deleted, remaining, last, stillRemaining };
    })()"#
        .replace("__TOKEN__", &js_string(&data.tokens[1]));
    let result = browser
        .evaluate_json(&script)
        .expect("the passkey flow must return JSON");

    assert_eq!(status_of(&result["registered"]), 200);
    assert_eq!(
        status_of(&result["added"]),
        200,
        "the passkey must be added: {}",
        result["added"]
    );
    assert_eq!(
        result["added"]["body"]["name"], "2 つ目のパスキー",
        "the added passkey must carry the name: {}",
        result["added"]
    );
    assert_eq!(
        result["listed"]["body"]["passkeys"]
            .as_array()
            .map(Vec::len),
        Some(2),
        "the list must have two passkeys: {}",
        result["listed"]
    );
    assert_eq!(
        status_of(&result["renamed"]),
        200,
        "the passkey must be renamed: {}",
        result["renamed"]
    );
    assert_eq!(result["renamed"]["body"]["name"], "変えた名前");
    assert_eq!(
        status_of(&result["deleted"]),
        200,
        "the passkey must be deleted: {}",
        result["deleted"]
    );
    let remaining = result["remaining"]["body"]["passkeys"]
        .as_array()
        .expect("the remaining passkeys");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0]["id"], result["added"]["body"]["id"]);
    assert_eq!(remaining[0]["name"], "変えた名前");
    // 最後の 1 つは削除できない (FR-3)。
    assert_eq!(
        status_of(&result["last"]),
        409,
        "the last passkey must not be deleted: {}",
        result["last"]
    );
    assert_eq!(
        result["stillRemaining"]["body"]["passkeys"]
            .as_array()
            .map(Vec::len),
        Some(1),
        "the last passkey must remain: {}",
        result["stillRemaining"]
    );
}

// ログインの検証 (auth_login_complete): 正常系と、最終使用日時の更新 (FR-2)。

#[test]
fn wrangler_auth_login_complete_ok_and_last_used_at_is_updated() {
    let data = flow_data();
    let lease = main_server();
    let browser = TestBrowser::open(&page_url(&lease)).expect("Chrome must open the test page");
    let script = r#"(async () => {
      const registered = await window.coffeeLogTest.registerWithToken(__TOKEN__, "ログイン用のパスキー");
      const before = await window.coffeeLogTest.listPasskeys();
      const loggedOut = await window.coffeeLogTest.logout();
      const loggedIn = await window.coffeeLogTest.login();
      const after = await window.coffeeLogTest.listPasskeys();
      return { registered, before, loggedOut, loggedIn, after };
    })()"#
        .replace("__TOKEN__", &js_string(&data.tokens[2]));
    let result = browser
        .evaluate_json(&script)
        .expect("the login flow must return JSON");

    assert_eq!(status_of(&result["registered"]), 200);
    assert_eq!(
        result["before"]["body"]["passkeys"][0]["last_used_at"],
        Value::Null,
        "the passkey must not be used yet: {}",
        result["before"]
    );
    assert_eq!(status_of(&result["loggedOut"]), 200);
    assert_eq!(
        status_of(&result["loggedIn"]),
        200,
        "the login must succeed: {}",
        result["loggedIn"]
    );
    assert_eq!(result["loggedIn"]["body"]["user_id"], data.users[2]);
    assert_eq!(
        status_of(&result["after"]),
        200,
        "the session must work after the login: {}",
        result["after"]
    );
    let last_used_at = result["after"]["body"]["passkeys"][0]["last_used_at"]
        .as_str()
        .unwrap_or_default();
    assert_eq!(
        last_used_at.len(),
        24,
        "the login must update last_used_at: {}",
        result["after"]
    );
}

// 使用済みチャレンジの再利用 409 と、検証の失敗 400 (auth_login_complete)。

#[test]
fn wrangler_auth_login_complete_reused_challenge_409_and_bad_signature_400() {
    let data = flow_data();
    let lease = main_server();
    let browser = TestBrowser::open(&page_url(&lease)).expect("Chrome must open the test page");
    let script = r#"(async () => {
      const registered = await window.coffeeLogTest.registerWithToken(__TOKEN__, "使い捨てのパスキー");
      const first = await window.coffeeLogTest.login();
      const reuse = await window.coffeeLogTest.completeLogin(first.credential);
      const assertion = await window.coffeeLogTest.getAssertion();
      const tampered = await window.coffeeLogTest.completeLoginWithBadSignature(assertion.credential);
      return { registered, first, reuse, assertion, tampered };
    })()"#
        .replace("__TOKEN__", &js_string(&data.tokens[3]));
    let result = browser
        .evaluate_json(&script)
        .expect("the login flow must return JSON");

    assert_eq!(status_of(&result["registered"]), 200);
    assert_eq!(status_of(&result["first"]), 200);
    // 使用したチャレンジの再利用は 409 (FR-2)。
    assert_eq!(
        status_of(&result["reuse"]),
        409,
        "the used challenge must be rejected: {}",
        result["reuse"]
    );
    assert_eq!(
        status_of(&result["assertion"]),
        200,
        "a new challenge must be issued: {}",
        result["assertion"]
    );
    // 署名の検証に失敗した場合は 400 (FR-2)。
    assert_eq!(
        status_of(&result["tampered"]),
        400,
        "a bad signature must be rejected: {}",
        result["tampered"]
    );
}

// 署名カウンタの後退 409 と、受理したときの保存値の更新 (auth_login_complete、FR-2)。

#[test]
fn wrangler_auth_login_complete_regressed_sign_count_409() {
    let data = flow_data();
    let lease = main_server();
    let browser = TestBrowser::open(&page_url(&lease)).expect("Chrome must open the test page");
    let script = r#"(async () => {
      const registered = await window.coffeeLogTest.registerWithToken(__TOKEN__, "カウンタのパスキー");
      const first = await window.coffeeLogTest.login();
      return { registered, first, credentialId: first.credential.id };
    })()"#
        .replace("__TOKEN__", &js_string(&data.tokens[4]));
    let result = browser
        .evaluate_json(&script)
        .expect("the registration and the login must return JSON");
    assert_eq!(status_of(&result["registered"]), 200);
    assert_eq!(status_of(&result["first"]), 200);
    let credential_id = result["credentialId"]
        .as_str()
        .expect("the credential id must be a string")
        .to_owned();

    // 認証器の署名カウンタを保存値より小さくして、後退を作る。
    browser
        .set_sign_count(&credential_id, 0)
        .expect("the signature counter must be replaced");
    let regressed = browser
        .evaluate_json("await window.coffeeLogTest.login()")
        .expect("the login must return JSON");
    assert_eq!(
        status_of(&regressed),
        409,
        "the regressed signature counter must be rejected: {regressed}"
    );

    // 保存値より大きいカウンタは受理し、保存値を更新する。
    browser
        .set_sign_count(&credential_id, 100)
        .expect("the signature counter must be replaced");
    let accepted = browser
        .evaluate_json("await window.coffeeLogTest.login()")
        .expect("the login must return JSON");
    assert_eq!(
        status_of(&accepted),
        200,
        "a larger signature counter must be accepted: {accepted}"
    );
    // 仮想認証器は署名の前にカウンタを 1 増やすため、受理された値はアサーションが持つ。
    let accepted_counter = sign_count_of(
        accepted["credential"]["response"]["authenticatorData"]
            .as_str()
            .expect("the accepted assertion must carry the authenticator data"),
    );
    let stored = lease
        .use_server(|server| {
            server.query_int(
                "SELECT sign_count FROM passkey_credentials WHERE name = 'カウンタのパスキー'",
            )
        })
        .expect("the sign count must be readable");
    assert_eq!(
        stored, accepted_counter,
        "the accepted signature counter must be stored as the new value"
    );
}

// チャレンジの期限切れ 410 と、チャレンジの発行のたびの削除 (auth_login_complete)。

#[test]
fn wrangler_auth_login_complete_expired_challenge_410_and_deleted_on_issue() {
    let lease = challenge_server();
    let browser = TestBrowser::open(&page_url(&lease)).expect("Chrome must open the test page");

    // 期限切れのチャレンジは 410 になり、その行は削除される (ADR-0004)。
    let expired = browser
        .evaluate_json("await window.coffeeLogTest.beginLogin()")
        .expect("the login begin must return JSON");
    let expired_challenge = expired["body"]["challenge"]
        .as_str()
        .expect("the challenge must be a string")
        .to_owned();
    thread::sleep(SHORT_TTL_WAIT);
    let result = browser
        .evaluate_json(&format!(
            "await window.coffeeLogTest.completeLoginWithChallenge({})",
            js_string(&expired_challenge)
        ))
        .expect("the completion must return JSON");
    assert_eq!(
        status_of(&result),
        410,
        "the expired challenge must be rejected: {result}"
    );
    let count = lease
        .use_server(|server| {
            server.query_int(&format!(
                "SELECT COUNT(*) AS count FROM webauthn_challenges WHERE challenge = '{}'",
                expired_challenge
            ))
        })
        .expect("the challenges must be readable");
    assert_eq!(count, 0, "the expired challenge row must be deleted");

    // 期限切れの行は、次のチャレンジの発行でも削除される。
    let stale = browser
        .evaluate_json("await window.coffeeLogTest.beginLogin()")
        .expect("the login begin must return JSON");
    let stale_challenge = stale["body"]["challenge"]
        .as_str()
        .expect("the challenge must be a string")
        .to_owned();
    thread::sleep(SHORT_TTL_WAIT);
    let fresh = browser
        .evaluate_json("await window.coffeeLogTest.beginLogin()")
        .expect("the login begin must return JSON");
    let fresh_challenge = fresh["body"]["challenge"]
        .as_str()
        .expect("the challenge must be a string")
        .to_owned();
    // 行が残っていれば 410 になる。削除されていれば、使用済みと同じ 409 になる。
    let result = browser
        .evaluate_json(&format!(
            "await window.coffeeLogTest.completeLoginWithChallenge({})",
            js_string(&stale_challenge)
        ))
        .expect("the completion must return JSON");
    assert_eq!(
        status_of(&result),
        409,
        "the stale challenge row must be deleted on the next issue: {result}"
    );
    let count = lease
        .use_server(|server| {
            server.query_int(&format!(
                "SELECT COUNT(*) AS count FROM webauthn_challenges WHERE challenge = '{}'",
                stale_challenge
            ))
        })
        .expect("the challenges must be readable");
    assert_eq!(count, 0, "the stale challenge row must be deleted");

    // 発行し直したチャレンジは、まだ使われていない (使用済みと同じ 409 にはならない)。
    let result = browser
        .evaluate_json(&format!(
            "await window.coffeeLogTest.completeLoginWithChallenge({})",
            js_string(&fresh_challenge)
        ))
        .expect("the completion must return JSON");
    assert_eq!(
        status_of(&result),
        400,
        "a completion without a registered credential must fail: {result}"
    );
}

// セッションの有効期限を短縮した設定で、期限切れのセッションが 401 になる (FR-4)。

#[test]
fn wrangler_auth_session_expired_401() {
    let data = session_data();
    let lease = session_server();
    let browser = TestBrowser::open(&page_url(&lease)).expect("Chrome must open the test page");
    let script = r#"(async () => {
      const registered = await window.coffeeLogTest.registerWithToken(__TOKEN__, "短命のセッション");
      const before = await window.coffeeLogTest.listPasskeys();
      return { registered, before };
    })()"#
        .replace("__TOKEN__", &js_string(&data.token));
    let result = browser
        .evaluate_json(&script)
        .expect("the registration must return JSON");
    assert_eq!(status_of(&result["registered"]), 200);
    assert_eq!(status_of(&result["before"]), 200);

    thread::sleep(SHORT_TTL_WAIT);
    let after = browser
        .evaluate_json("await window.coffeeLogTest.listPasskeys()")
        .expect("the list must return JSON");
    assert_eq!(
        status_of(&after),
        401,
        "the expired session must be rejected: {after}"
    );
}
