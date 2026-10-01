//! 写真からの購入と商品の項目の推測の API の結合テスト (HTTP)。FR-19、ADR-0016。
//!
//! 未認証 401、Content-Type の 400、本体が空の 400、5 MB 超の 400、入力が正しいときの
//! AI の失敗 (500) を確認する。正常系 (推測が返る) は Workers AI の推論を要するため CI では
//! 実行できず、実写真を使って手元で確認する (PRD の成功指標の例外。ADR-0016)。
//! ハーネスは `wrangler dev` を `--local` で起動するため、AI バインディングは常に失敗し、
//! 500 の経路は CI でも確かめられる。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。

mod support;

use std::sync::OnceLock;

use brew_book_core::photo::MAX_BYTES;
use reqwest::blocking::Response;
use serde_json::Value;
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, Seed};
use support::ServerLease;

/// 推測の経路 (FR-19)。
const PATH: &str = "/api/purchase-suggestions";

/// テストに送る JPEG。内容は AI が失敗するため推測には使われない (先頭だけのダミー)。
const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// 推測の利用者のセッション。
    session: String,
}

/// 下ごしらえを 1 回だけ組み立てる。
fn data() -> &'static TestData {
    static DATA: OnceLock<TestData> = OnceLock::new();
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
    let future = "2099-01-01T00:00:00.000Z";
    let created = "2026-09-01T00:00:00.000Z";
    let mut seed = Seed::new();
    let user = user_id(1);
    seed.user(&user, "suggestion user", created);
    let session = seed.session(&user, future, created);
    TestData {
        seed_sql: seed.sql(),
        session,
    }
}

/// セッションの Cookie を持たないクライアント。
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

/// 入力不正の呼び出しが 400 になることを確かめる。
fn assert_bad_request(response: Response) {
    let body = assert_status(response, 400);
    assert_eq!(error_code(&body), Some("bad_request"), "{body}");
}

/// 内部エラーの呼び出しが 500 になることを確かめる (AI の呼び出しの失敗。FR-19)。
fn assert_internal_error(response: Response) {
    let body = assert_status(response, 500);
    assert_eq!(error_code(&body), Some("internal_error"), "{body}");
}

mod purchase_suggestions {
    //! 写真からの推測の経路のテスト (FR-19)。

    use super::*;

    #[test]
    fn wrangler_purchase_suggestions_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        // 認証が必要な経路である (未認証は 401)。
        assert_unauthorized(anonymous(&base_url).post_bytes(PATH, "image/jpeg", JPEG));
    }

    #[test]
    fn wrangler_purchase_suggestions_content_type_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // Content-Type が image/jpeg でない入力は 400 を返し、AI を呼ばない (FR-19)。
        for content_type in [
            "application/json",
            "text/plain",
            "image/png",
            "image/jpg",
            "application/octet-stream",
        ] {
            assert_bad_request(client.post_bytes(PATH, content_type, JPEG));
        }
        // Content-Type の無い入力も 400 にする。
        let (status, body) = read(client.post(PATH));
        assert_eq!(status, 400, "the response body was {body}");
        assert_eq!(error_code(&body), Some("bad_request"), "{body}");
        // パラメータ付きの image/jpeg は受け付ける (AI の失敗で 500 になる)。
        assert_internal_error(client.post_bytes(PATH, "image/jpeg; charset=binary", JPEG));
    }

    #[test]
    fn wrangler_purchase_suggestions_oversized_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 5 MB (5,000,000 バイト) を超える入力は 400 を返し、AI を呼ばない (FR-19)。
        let oversized = vec![0xFFu8; MAX_BYTES as usize + 1];
        assert_bad_request(client.post_bytes(PATH, "image/jpeg", &oversized));
        // ちょうどのサイズは入力として受け付ける (AI の失敗で 500 になる)。
        let exact = vec![0xFFu8; MAX_BYTES as usize];
        assert_internal_error(client.post_bytes(PATH, "image/jpeg", &exact));
    }

    #[test]
    fn wrangler_purchase_suggestions_empty_body_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 本体が空の入力は写真として成立しないため 400 を返し、AI を呼ばない (FR-19)。
        assert_bad_request(client.post_bytes(PATH, "image/jpeg", &[]));
    }

    #[test]
    fn wrangler_purchase_suggestions_fails_500_without_the_remote_binding() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // AI の呼び出しに失敗したときは、理由を区別せず 500 を返す (FR-19)。
        // ハーネスはリモートのバインディングを無効にして起動するため、ここでは常に失敗する。
        assert_internal_error(client.post_bytes(PATH, "image/jpeg", JPEG));
        // 認証が無ければ AI を呼ばずに 401 になる (順序の確認)。
        assert_unauthorized(anonymous(&base_url).post_bytes(PATH, "image/jpeg", JPEG));
    }
}
