//! 管理者 Worker の結合テスト (HTTP、FR-17、ADR-0008)。
//!
//! 検証する受け入れ基準は次のとおり。
//!
//! - `GET /` が利用者の一覧 (表示名、作成日時、パスキーの数) を返し、表示名を HTML エスケープする。
//! - `POST /users` が表示名 (前後の空白を除いて 1 文字以上 50 文字以下) で利用者を作成し、
//!   範囲外と項目の無い本文は 400 にする。成功したら一覧へ 303 で戻す。
//! - `POST /users/:id/tokens` が登録用リンクを発行し、リンクは発行直後の応答に 1 回だけ含める。
//!   生のトークンは D1 に保存せず、SHA-256 のハッシュだけを保存し、有効期限は 24 時間にする。
//! - 再発行すると、その利用者の未使用の古いトークンは無効になる (古いトークンでの登録が 404)。
//! - 存在しない利用者の ID は 404 にする。
//! - 状態を変更するフォームの送信は、別オリジンと `Origin` の無いリクエストを 403 にする。
//! - 一覧を含む他の応答に、発行したリンクとトークンが含まれない。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外する
//! ための規約。管理者 Worker と利用者向けの Worker のサーバーは 1 つのテストファイルで 1 回だけ
//! 起動し、テストは 1 つずつ実行する (借用の仕組みは `support` が持つ)。

mod support;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use brew_book_core::auth;
use reqwest::blocking::{Client, Response};
use support::{ServerLease, Servers};

/// HTTP の待ち時間。
const TIMEOUT: Duration = Duration::from_secs(30);

/// 別オリジンの `Origin` (403 の検査に使う)。
const OTHER_ORIGIN: &str = "https://evil.example";

/// 存在しない利用者の ID。
const UNKNOWN_USER: &str = "00000000-0000-4000-8000-000000000000";

/// 下ごしらえに使う日時 (ISO 8601 UTC の固定長)。
const CREATED: &str = "2026-09-01T00:00:00.000Z";

/// 共有のサーバーを借りる。借用の間はこのテストだけがサーバーを使う。
fn servers() -> ServerLease {
    support::shared_servers("admin", Servers::start).expect("wrangler dev must start")
}

/// 共有のサーバーを使う。借用を取ってから本体を実行する。
fn with_servers<T>(action: impl FnOnce(&Servers) -> T) -> T {
    let lease = servers();
    lease.use_servers(action)
}

/// HTTP のクライアントを作る。
///
/// 管理者画面は 303 でリダイレクトするため、追従せずに状態コードと `Location` を検査する。
fn client() -> Client {
    Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("the HTTP client must build")
}

/// フォームの本文にする値をパーセント符号化する。
fn form_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(byte));
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// 表示名のフォームの本文を作る。
fn display_name_body(display_name: &str) -> String {
    format!("display_name={}", form_encode(display_name))
}

/// フォームを送る。`origin` が `None` のときは `Origin` を付けない。
fn post_form(base_url: &str, path: &str, body: &str, origin: Option<&str>) -> Response {
    let mut request = client()
        .post(format!("{base_url}{path}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body.to_owned());
    if let Some(origin) = origin {
        request = request.header("Origin", origin);
    }
    request.send().expect("the request must be sent")
}

/// フォームを同一オリジンから送る。
fn post_form_same_origin(base_url: &str, path: &str, body: &str) -> Response {
    post_form(base_url, path, body, Some(base_url))
}

/// GET する。必要なら `Origin` を付ける。
fn get(base_url: &str, path: &str, origin: Option<&str>) -> Response {
    let mut request = client().get(format!("{base_url}{path}"));
    if let Some(origin) = origin {
        request = request.header("Origin", origin);
    }
    request.send().expect("the request must be sent")
}

/// 応答の本体を読む。
fn body(response: Response) -> String {
    response.text().expect("the response body must be readable")
}

/// 一覧の HTML から、その表示名の利用者の ID を読む (発行のフォームの送信先)。
fn user_id(html: &str, display_name: &str) -> String {
    let escaped = brew_book_admin::html::escape(display_name);
    let row_start = html
        .find(&escaped)
        .unwrap_or_else(|| panic!("the list must show {display_name}: {html}"));
    let row = &html[row_start..];
    let action = row
        .find("action=\"/users/")
        .unwrap_or_else(|| panic!("the row of {display_name} must have the token form: {row}"));
    let rest = &row[action + "action=\"/users/".len()..];
    let end = rest
        .find("/tokens\"")
        .unwrap_or_else(|| panic!("the token form must point to the issue route: {rest}"));
    rest[..end].to_owned()
}

/// 発行の応答からリンクを取り出す。リンクは応答に 1 回だけ含まれることを確かめる。
fn link(response: Response, app_origin: &str) -> String {
    assert_eq!(response.status().as_u16(), 200, "the issue must succeed");
    let html = body(response);
    let prefix = format!("{app_origin}/register?token=");
    let start = html
        .find(&prefix)
        .unwrap_or_else(|| panic!("the page must contain the link: {html}"));
    let rest = &html[start..];
    let end = rest.find(['<', '"']).unwrap_or(rest.len());
    let link = rest[..end].to_owned();
    assert_eq!(html.matches(&link).count(), 1, "the link must appear once");
    link
}

/// リンクのトークンを取り出す。
fn token_of(link: &str) -> String {
    link.rsplit("token=")
        .next()
        .expect("the link must have a token")
        .to_owned()
}

/// D1 の現在時刻 (ミリ秒)。
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock must be after the epoch")
        .as_millis() as i64
}

/// 利用者を作り、その ID を返す (一覧の HTML から読む)。
fn create_user(servers: &Servers, display_name: &str) -> String {
    let base_url = servers.admin.base_url();
    let response = post_form_same_origin(&base_url, "/users", &display_name_body(display_name));
    let status = response.status().as_u16();
    let location = response
        .headers()
        .get("Location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let text = body(response);
    assert_eq!(
        status, 303,
        "the creation must redirect to the list (Location: {location:?}): {text}"
    );
    assert_eq!(
        location.as_deref(),
        Some("/"),
        "the creation must redirect to the list: {text}"
    );
    let html = body(get(&base_url, "/", None));
    user_id(&html, display_name)
}

/// 登録のチャレンジの発行 (利用者向けの Worker) を呼ぶ。トークンが有効なら 200 になる。
fn register_begin(servers: &Servers, token: &str) -> Response {
    let base_url = servers.app.base_url();
    client()
        .post(format!("{base_url}/api/auth/register/begin"))
        .header("Content-Type", "application/json")
        .header("Origin", &base_url)
        .body(format!("{{\"token\":\"{token}\"}}"))
        .send()
        .expect("the request must be sent")
}

#[test]
fn wrangler_admin_users_list_ok() {
    with_servers(|servers| {
        let display_name = "一覧の利用者 <script>";
        let id = create_user(servers, display_name);

        // パスキーを 1 つ下ごしらえし、一覧の数に反映されることを確かめる。
        let passkey = "11111111-1111-4111-8111-111111111111";
        servers
            .admin
            .execute_sql(&format!(
                "INSERT INTO passkey_credentials \
                 (id, user_id, credential_id, public_key, sign_count, name, created_at, last_used_at) \
                 VALUES ('{passkey}', '{id}', 'credential-{passkey}', 'public-key', 0, 'テストのパスキー', \
                 '{CREATED}', NULL)"
            ))
            .expect("the passkey must be seeded");

        let rows = servers
            .admin
            .query_rows(&format!("SELECT created_at FROM users WHERE id = '{id}'"))
            .expect("the user must be readable");
        let created_at = rows[0]["created_at"]
            .as_str()
            .expect("created_at must be a string")
            .to_owned();

        let html = body(get(&servers.admin.base_url(), "/", None));
        // 表示名は HTML エスケープして出る。
        assert!(html.contains("一覧の利用者 &lt;script&gt;"), "{html}");
        assert!(!html.contains("<script>"), "{html}");
        // 作成日時とパスキーの数が出る。
        assert!(html.contains(&created_at), "{html}");
        assert!(html.contains("class=\"count\">1<"), "{html}");
        // 発行のフォームがその利用者を指す。
        assert!(
            html.contains(&format!("action=\"/users/{id}/tokens\"")),
            "{html}"
        );
        // 作成のフォームが出る (FR-17)。
        assert!(html.contains("action=\"/users\""), "{html}");
        assert!(html.contains("name=\"display_name\""), "{html}");
    });
}

#[test]
fn wrangler_admin_users_create_invalid_input_400() {
    with_servers(|servers| {
        let base_url = servers.admin.base_url();
        let before = servers
            .admin
            .query_int("SELECT COUNT(*) FROM users")
            .expect("the users must be countable");

        for display_name in ["", "   ", &"あ".repeat(51)] {
            let response =
                post_form_same_origin(&base_url, "/users", &display_name_body(display_name));
            assert_eq!(
                response.status().as_u16(),
                400,
                "{display_name:?} must be rejected"
            );
        }
        // 前後の空白を除いて 50 文字は受け付ける (境界値)。
        let name = format!("  {}  ", "い".repeat(50));
        let response = post_form_same_origin(&base_url, "/users", &display_name_body(&name));
        assert_eq!(
            response.status().as_u16(),
            303,
            "50 characters must be accepted"
        );

        // 項目の無い本文と、本文が無い送信も 400 にする。
        for body_text in ["other=1", ""] {
            let response = post_form_same_origin(&base_url, "/users", body_text);
            assert_eq!(
                response.status().as_u16(),
                400,
                "{body_text:?} must be rejected"
            );
        }

        // 拒否した入力では利用者を作らない (受け付けた 1 件だけ増える)。
        let after = servers
            .admin
            .query_int("SELECT COUNT(*) FROM users")
            .expect("the users must be countable");
        assert_eq!(after, before + 1, "only the valid name must create a user");
    });
}

#[test]
fn wrangler_admin_tokens_create_ok() {
    with_servers(|servers| {
        let id = create_user(servers, "トークンの利用者");
        let response = post_form_same_origin(
            &servers.admin.base_url(),
            &format!("/users/{id}/tokens"),
            "",
        );
        let link = link(response, &servers.app.base_url());
        let token = token_of(&link);

        // リンクは利用者向けの Worker の登録の経路を指し、トークンは 32 バイトの base64url である。
        assert_eq!(
            link,
            format!("{}/register?token={token}", servers.app.base_url()),
            "the link must point to the app worker"
        );
        assert_eq!(
            token.len(),
            43,
            "the token must be a base64url encoded 32 bytes"
        );

        // D1 には SHA-256 のハッシュだけを保存する (ADR-0004)。
        let rows = servers
            .admin
            .query_rows(&format!(
                "SELECT token_hash, expires_at, used_at FROM registration_tokens WHERE user_id = '{id}'"
            ))
            .expect("the token must be readable");
        assert_eq!(rows.len(), 1, "the user must have one token: {rows:?}");
        let row = &rows[0];
        assert_eq!(
            row["token_hash"].as_str(),
            Some(auth::hash_secret(&token).as_str()),
            "the raw token must not be stored"
        );
        assert_ne!(row["token_hash"].as_str(), Some(token.as_str()));
        assert!(row["used_at"].is_null(), "the new token must be unused");

        // 有効期限は 24 時間である (多少の実行のずれを許す)。
        let slack_seconds = 300;
        let earliest = auth::expiry_from(
            now_millis() - slack_seconds * 1000,
            auth::REGISTRATION_TOKEN_TTL_SECONDS,
        )
        .expect("the expiry must be formattable");
        let latest = auth::expiry_from(
            now_millis() + slack_seconds * 1000,
            auth::REGISTRATION_TOKEN_TTL_SECONDS,
        )
        .expect("the expiry must be formattable");
        let expires_at = row["expires_at"]
            .as_str()
            .expect("the expiry must be a string");
        assert!(
            earliest.as_str() <= expires_at && expires_at <= latest.as_str(),
            "the expiry must be 24 hours from now: {expires_at} not in {earliest}..={latest}"
        );

        // 生のトークンは D1 のどこにも保存しない。
        let stored = servers
            .admin
            .query_int(&format!(
                "SELECT COUNT(*) FROM registration_tokens WHERE token_hash = '{token}'"
            ))
            .expect("the tokens must be countable");
        assert_eq!(stored, 0, "the raw token must not be stored");

        // 一覧を含む他の応答にはリンクとトークンを載せない (再表示できない。ADR-0008)。
        let list = body(get(&servers.admin.base_url(), "/", None));
        assert!(
            !list.contains(&link),
            "the list must not show the link: {list}"
        );
        assert!(!list.contains(&token), "the list must not show the token");
    });
}

#[test]
fn wrangler_admin_tokens_create_reissue_ok() {
    with_servers(|servers| {
        let id = create_user(servers, "再発行の利用者");

        // 1 つ目のトークンで登録のチャレンジを発行できる。
        let first = link(
            post_form_same_origin(
                &servers.admin.base_url(),
                &format!("/users/{id}/tokens"),
                "",
            ),
            &servers.app.base_url(),
        );
        let first_token = token_of(&first);
        assert_eq!(
            register_begin(servers, &first_token).status().as_u16(),
            200,
            "the first token must be usable"
        );

        // 再発行すると、古いトークンは削除される。
        let second = link(
            post_form_same_origin(
                &servers.admin.base_url(),
                &format!("/users/{id}/tokens"),
                "",
            ),
            &servers.app.base_url(),
        );
        let second_token = token_of(&second);
        assert_ne!(
            first_token, second_token,
            "the reissue must make a new token"
        );

        let rows = servers
            .admin
            .query_rows(&format!(
                "SELECT token_hash FROM registration_tokens WHERE user_id = '{id}' AND used_at IS NULL"
            ))
            .expect("the tokens must be readable");
        assert_eq!(
            rows.len(),
            1,
            "the old unused token must be deleted: {rows:?}"
        );
        assert_eq!(
            rows[0]["token_hash"].as_str(),
            Some(auth::hash_secret(&second_token).as_str()),
            "the remaining token must be the new one"
        );

        // 古いトークンでの登録は 404、新しいトークンでは通る。
        let old = register_begin(servers, &first_token);
        assert_eq!(old.status().as_u16(), 404, "the old token must be gone");
        assert_eq!(
            register_begin(servers, &second_token).status().as_u16(),
            200,
            "the new token must be usable"
        );
    });
}

#[test]
fn wrangler_admin_tokens_create_unknown_user_404() {
    with_servers(|servers| {
        let response = post_form_same_origin(
            &servers.admin.base_url(),
            &format!("/users/{UNKNOWN_USER}/tokens"),
            "",
        );
        assert_eq!(response.status().as_u16(), 404, "the user does not exist");
        let html = body(response);
        assert!(
            !html.contains("register?token="),
            "no link must be shown: {html}"
        );
    });
}

#[test]
fn wrangler_admin_origin_403() {
    with_servers(|servers| {
        let base_url = servers.admin.base_url();
        let id = create_user(servers, "Origin の利用者");

        let before = servers
            .admin
            .query_int("SELECT COUNT(*) FROM users")
            .expect("the users must be countable");

        // 状態を変更するフォームの送信は、別オリジンと Origin の無いリクエストを 403 にする。
        let requests = vec![
            ("/users".to_owned(), display_name_body("拒否される利用者")),
            (format!("/users/{id}/tokens"), String::new()),
        ];
        for (path, body_text) in &requests {
            for origin in [Some(OTHER_ORIGIN), None] {
                let response = post_form(&base_url, path, body_text, origin);
                assert_eq!(
                    response.status().as_u16(),
                    403,
                    "POST {path} with the origin {origin:?} must be rejected"
                );
            }
        }

        // 同じホストで別のポートのオリジンも拒否する (port を無視する実装への退行の検査)。
        let other_port = "http://127.0.0.1:1".to_owned();
        let response = post_form(
            &base_url,
            "/users",
            &display_name_body("拒否される利用者"),
            Some(&other_port),
        );
        assert_eq!(
            response.status().as_u16(),
            403,
            "another port is another origin"
        );

        // GET は Origin の検証の対象外である (ブラウザは同一オリジンの GET に Origin を付けない)。
        let response = get(&base_url, "/", Some(OTHER_ORIGIN));
        assert_eq!(
            response.status().as_u16(),
            200,
            "GET must not check the origin"
        );

        // 拒否した送信では何も作らない。
        let after = servers
            .admin
            .query_int("SELECT COUNT(*) FROM users")
            .expect("the users must be countable");
        assert_eq!(after, before, "the rejected requests must not create users");
        let tokens = servers
            .admin
            .query_int(&format!(
                "SELECT COUNT(*) FROM registration_tokens WHERE user_id = '{id}'"
            ))
            .expect("the tokens must be countable");
        assert_eq!(tokens, 0, "the rejected requests must not issue tokens");
    });
}

#[test]
fn wrangler_admin_users_create_ok() {
    with_servers(|servers| {
        let base_url = servers.admin.base_url();
        let response =
            post_form_same_origin(&base_url, "/users", &display_name_body("  空白付き  "));
        assert_eq!(response.status().as_u16(), 303, "the creation must succeed");

        let html = body(get(&base_url, "/", None));
        assert!(
            html.contains("空白付き"),
            "the trimmed name must be shown: {html}"
        );
        assert!(
            !html.contains("  空白付き  "),
            "the name must be trimmed: {html}"
        );
        assert_eq!(
            servers
                .admin
                .query_int("SELECT COUNT(*) FROM users WHERE display_name = '空白付き'")
                .expect("the users must be countable"),
            1,
            "the trimmed name must be stored"
        );
    });
}
