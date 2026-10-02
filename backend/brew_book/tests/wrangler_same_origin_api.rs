//! 状態を変更する API の `Origin` の検証の結合テスト (HTTP、ADR-0005)。
//!
//! 検証する受け入れ基準は次のとおり。
//!
//! - 別オリジンの `Origin` を付けた状態を変更するリクエスト (POST、PATCH、DELETE) が 403 になる。
//! - `Origin` の無い状態を変更するリクエストも 403 になる (検証できないため)。
//! - 同一オリジンの `Origin` を付けた状態を変更するリクエストは成功する。
//! - GET は `Origin` の検証の対象外である (ブラウザは同一オリジンの GET に `Origin` を付けない)。
//! - API の応答に CORS のヘッダ (`access-control-*`) が含まれない (Backend は CORS を許可しない)。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。

mod support;

use reqwest::blocking::Response;
use serde_json::{json, Value};
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, Seed};
use support::ServerLease;

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const CREATED: &str = "2026-09-01T00:00:00.000Z";
const FUTURE: &str = "2099-01-01T00:00:00.000Z";

/// 別オリジンの `Origin` (検証の対象外であることの検査に使う)。
const OTHER_ORIGIN: &str = "https://evil.example";

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    /// 共有のサーバーの起動時に先に流す下ごしらえの SQL。
    seed_sql: String,
    /// 正常系のテストの利用者と、そのセッション。
    user: String,
    session: String,
}

/// 下ごしらえを 1 回だけ組み立てる。
fn data() -> &'static TestData {
    static DATA: std::sync::OnceLock<TestData> = std::sync::OnceLock::new();
    DATA.get_or_init(build_data)
}

/// 共有のサーバーを借りる。
fn server() -> ServerLease {
    support::shared_server("main", || {
        support::DevServer::start_with(|_| Vec::new(), &data().seed_sql)
    })
    .expect("wrangler dev must start")
}

/// 下ごしらえの SQL と、テストが使う値を作る。
fn build_data() -> TestData {
    let mut seed = Seed::new();
    let user = user_id(1);
    seed.user(&user, "Origin の検査の利用者", CREATED);
    let session = seed.session(&user, FUTURE, CREATED);
    TestData {
        seed_sql: seed.sql(),
        user,
        session,
    }
}

/// 応答の状態コードと本体を確かめる。
fn assert_status(response: Response, expected: u16) -> Value {
    let (status, body) = read(response);
    assert_eq!(status, expected, "the response body was {body}");
    body
}

/// 応答の状態コードを確かめ、CORS のヘッダが無いことも確かめる。
fn assert_status_without_cors(response: Response, expected: u16) -> Value {
    let status = response.status().as_u16();
    for name in response.headers().keys() {
        assert!(
            !name.as_str().starts_with("access-control-"),
            "the {status} response must not have the CORS header {name}"
        );
    }
    assert_eq!(status, expected, "the response must be {expected}");
    read(response).1
}

/// 403 の `forbidden` の応答を確かめ、CORS のヘッダが無いことも確かめる。
fn assert_forbidden(response: Response) -> Value {
    let body = assert_status_without_cors(response, 403);
    assert_eq!(error_code(&body), Some("forbidden"), "{body}");
    body
}

/// セッションを持たないクライアント (別オリジンの `Origin` を付ける)。
fn client_from_another_origin(base_url: &str) -> ApiClient {
    ApiClient::new(base_url, None).with_origin(OTHER_ORIGIN)
}

/// セッションを持たないクライアント (`Origin` を付けない)。
fn client_without_origin(base_url: &str) -> ApiClient {
    ApiClient::new(base_url, None).without_origin()
}

/// 同じホストで別のポートのオリジンを作る (port を無視する実装への退行の検査)。
///
/// scheme の違いは `wrangler dev` が `Origin` の scheme をリクエストの scheme に合わせてから
/// Worker に渡すため、ここでは再現できない。scheme の境界は `test_origin.rs` が検査する。
fn origin_with_another_port(base_url: &str) -> String {
    let host = base_url
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split(':').next())
        .expect("the base URL must have a host");
    format!("http://{host}:1")
}

/// 応答のヘッダーを 1 つ読む。
fn header(response: &Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// 画面と API が同じオリジンから配信されることを確かめる。
///
/// 画面のルーティングのパス (`/register` など) を直接開くと `index.html` が 200 で返り、
/// 一致する静的ファイルは Static Assets から返る。`/api/*` は Worker が処理して JSON を返す
/// (run_worker_first と not_found_handling の設定。ADR-0005)。
#[test]
fn wrangler_spa_routes_return_index_html_200() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("the HTTP client must build");

    for path in ["/", "/register", "/brews/new", "/stats"] {
        let response = client
            .get(format!("{base_url}{path}"))
            .send()
            .expect("the request must reach the dev server");
        let status = response.status().as_u16();
        let content_type = header(&response, "content-type").unwrap_or_default();
        let body = response.text().expect("the response body must be readable");
        assert_eq!(status, 200, "the path {path} must return index.html");
        assert!(
            content_type.starts_with("text/html"),
            "the path {path} must return HTML but was {content_type}"
        );
        assert!(
            body.contains("assets/brew_book_frontend"),
            "the path {path} must return the Dioxus app HTML with the JavaScript loader: {body}"
        );
    }

    // 一致する静的ファイルは Static Assets からそのまま返る。JavaScript のローダーの
    // ファイル名には dx がハッシュを付けるため、index.html から実際のパスを取り出して取得する。
    let index = client
        .get(format!("{base_url}/"))
        .send()
        .expect("the request must reach the dev server")
        .text()
        .expect("the response body must be readable");
    let loader = loader_path(&index);
    let response = client
        .get(format!("{base_url}{loader}"))
        .send()
        .expect("the request must reach the dev server");
    assert_eq!(response.status().as_u16(), 200);
    let content_type = header(&response, "content-type").unwrap_or_default();
    assert!(
        content_type.starts_with("text/javascript"),
        "the script must be served as JavaScript but was {content_type}"
    );

    // `/api/*` は Static Assets ではなく Worker が処理する (JSON を返す)。
    let response = client
        .get(format!("{base_url}/api/passkeys"))
        .send()
        .expect("the request must reach the dev server");
    assert_eq!(response.status().as_u16(), 401);
    let content_type = header(&response, "content-type").unwrap_or_default();
    assert!(
        content_type.starts_with("application/json"),
        "the API must answer JSON but was {content_type}"
    );
}

/// `index.html` から JavaScript のローダーのパスを取り出す。
///
/// dx の成果物のファイル名にはハッシュが付いてビルドのたびに変わるため、固定の名前を
/// 書かずに index.html の参照から解決する。
fn loader_path(html: &str) -> String {
    let start = html
        .find("assets/")
        .expect("index.html must reference the JavaScript loader under assets/");
    let rest = &html[start..];
    let end = rest
        .find(".js")
        .expect("the JavaScript loader reference must end with .js")
        + ".js".len();
    format!("/{}", &rest[..end])
}

#[test]
fn wrangler_same_origin_post_ok() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    // 同一オリジンの POST はチャレンジを発行できる (200)。
    let client = ApiClient::new(&base_url, None);
    assert_status(client.post("/api/auth/login/begin"), 200);
}

#[test]
fn wrangler_same_origin_mutation_with_a_session_ok() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, Some(&data.session));
    // 状態を変更する API が、同一オリジンの Origin 付きで成功する (完了条件)。
    let body = assert_status(
        client.post_json("/api/shops", &json!({ "name": "Origin の検査の店" })),
        200,
    );
    assert_eq!(body["name"], "Origin の検査の店", "{body}");
    assert_eq!(body["user_id"], data.user, "{body}");
}

#[test]
fn wrangler_cross_origin_post_forbidden_403() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    assert_forbidden(client_from_another_origin(&base_url).post("/api/auth/login/begin"));
}

#[test]
fn wrangler_another_port_post_forbidden_403() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    // オリジンは scheme、host、port の組で比較する。ホストが同じでもポートが違えば別のオリジンである。
    assert_forbidden(
        ApiClient::new(&base_url, None)
            .with_origin(&origin_with_another_port(&base_url))
            .post("/api/auth/login/begin"),
    );
}

#[test]
fn wrangler_missing_origin_post_forbidden_403() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    assert_forbidden(client_without_origin(&base_url).post("/api/auth/login/begin"));
}

#[test]
fn wrangler_cross_origin_patch_and_delete_forbidden_403() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, Some(&data.session)).with_origin(OTHER_ORIGIN);
    // PATCH と DELETE の両方を検査する (状態を変更するメソッド)。
    assert_forbidden(client.patch_json(
        "/api/passkeys/00000000-0000-4000-8000-000000000001",
        &json!({ "name": "別オリジンの名前" }),
    ));
    assert_forbidden(client.delete("/api/account"));
    // 認証が失敗する経路でも、`Origin` の検証を先に行う (403 を 401 より先に返す)。
    assert_forbidden(client_without_origin(&base_url).delete("/api/account"));
}

#[test]
fn wrangler_get_is_not_checked_for_the_origin() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    // GET は `Origin` の検証の対象外である。別オリジンの Origin を付けても 401 (認証の失敗) になる。
    let body = assert_status(
        client_from_another_origin(&base_url).get("/api/passkeys"),
        401,
    );
    assert_eq!(error_code(&body), Some("unauthorized"), "{body}");
    // `Origin` を付けない GET も同じである。
    let body = assert_status(client_without_origin(&base_url).get("/api/passkeys"), 401);
    assert_eq!(error_code(&body), Some("unauthorized"), "{body}");
}

#[test]
fn wrangler_api_responses_have_no_cors_headers() {
    let data = data();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    // 認証の失敗 (401)、成功 (200)、`Origin` の検証の失敗 (403) のどれにも CORS のヘッダを付けない。
    assert_status_without_cors(client_without_origin(&base_url).get("/api/passkeys"), 401);
    assert_status_without_cors(
        ApiClient::new(&base_url, None).post("/api/auth/login/begin"),
        200,
    );
    assert_status_without_cors(
        client_from_another_origin(&base_url).post("/api/auth/login/begin"),
        403,
    );
    assert_status_without_cors(
        ApiClient::new(&base_url, Some(&data.session))
            .post_json("/api/shops", &json!({ "name": "CORS の検査の店" })),
        200,
    );
    // プリフライト (OPTIONS) にも CORS のヘッダを付けない。台帳に無いため 404 になる。
    assert_status_without_cors(
        client_from_another_origin(&base_url).options("/api/shops"),
        404,
    );
}
