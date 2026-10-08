//! 住所の補完と地図の設定の API の結合テスト (HTTP)。FR-22、ADR-0019。
//!
//! 住所の補完 (`GET /api/place-search`) は、未認証 401、入力不正 400、キーの未設定の 500 を
//! 確認する。正常系 (Google の呼び出し) は CI では実行できないため、staging へのデプロイで
//! 実在の店名を使って確認する (PRD の成功指標の測定方法。FR-19 と同じ扱い)。
//! 地図の設定 (`GET /api/maps/config`) は、未認証 401 と、キーを注入したときの正常系
//! (返ること) と、キーの無いときの null を確認する。
//! 住所の補完のテストはキーを注入しないサーバーで行い、Google を呼ばずに 500 の経路を確かめる
//! (キーを注入すると本物の Google を呼んでしまうため)。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 2 つ (キーを注入する側と、注入しない側) を起動し、
//! 下ごしらえの SQL を先に実行する。

mod support;

use std::sync::OnceLock;

use serde_json::{json, Value};
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, Seed};
use support::ServerLease;

/// 住所の補完の経路 (FR-22)。
const SEARCH_PATH: &str = "/api/place-search";
/// 地図の設定の経路 (FR-22)。
const CONFIG_PATH: &str = "/api/maps/config";

/// テストが注入する API キー (地図と住所の補完で同じ 1 つ)。実値はリポジトリに含めない (ADR-0019)。
const API_KEY: &str = "test-maps-api-key";

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// 検索と設定の利用者のセッション。
    session: String,
}

/// 下ごしらえを 1 回だけ組み立てる。
fn data() -> &'static TestData {
    static DATA: OnceLock<TestData> = OnceLock::new();
    DATA.get_or_init(build_data)
}

/// API キーを注入した共有のサーバーを借りる (地図の設定の正常系の検査)。
fn server() -> ServerLease {
    support::shared_server("maps", || {
        support::DevServer::start_with(
            |_| vec![("GOOGLE_MAPS_API_KEY".to_string(), API_KEY.to_string())],
            &data().seed_sql,
        )
    })
    .expect("wrangler dev must start")
}

/// API キーを注入しない共有のサーバーを借りる (キーの無い環境の検査)。
fn server_without_keys() -> ServerLease {
    support::shared_server("maps-without-keys", || {
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
    seed.user(&user, "maps user", created);
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
fn assert_status(response: reqwest::blocking::Response, expected: u16) -> Value {
    let (status, body) = read(response);
    assert_eq!(status, expected, "the response body was {body}");
    body
}

/// 未認証の呼び出しが 401 になることを確かめる。
fn assert_unauthorized(response: reqwest::blocking::Response) {
    let body = assert_status(response, 401);
    assert_eq!(error_code(&body), Some("unauthorized"));
}

/// 入力不正の呼び出しが 400 になることを確かめる。
fn assert_bad_request(response: reqwest::blocking::Response) {
    let body = assert_status(response, 400);
    assert_eq!(error_code(&body), Some("bad_request"), "{body}");
}

/// 内部エラーの呼び出しが 500 になることを確かめる (キーの未設定。FR-22)。
fn assert_internal_error(response: reqwest::blocking::Response) {
    let body = assert_status(response, 500);
    assert_eq!(error_code(&body), Some("internal_error"), "{body}");
}

mod place_search {
    //! 住所の補完の経路のテスト (FR-22)。

    use super::*;

    #[test]
    fn wrangler_place_search_unauthenticated_401() {
        let lease = server_without_keys();
        let base_url = lease.use_server(|server| server.base_url());
        // 認証が必要な経路である (未認証は 401)。
        assert_unauthorized(anonymous(&base_url).get(&format!("{SEARCH_PATH}?q=店&lang=ja")));
    }

    #[test]
    fn wrangler_place_search_invalid_input_400() {
        let data = data();
        let lease = server_without_keys();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 店名 (`q`) と言語 (`lang`) が無い、空、範囲外の入力は 400 を返す (FR-22)。
        for query in [
            SEARCH_PATH.to_owned(),
            format!("{SEARCH_PATH}?lang=ja"),
            format!("{SEARCH_PATH}?q=&lang=ja"),
            format!("{SEARCH_PATH}?q=%20%20&lang=ja"),
            format!("{SEARCH_PATH}?q={}&lang=ja", "あ".repeat(257)),
            format!("{SEARCH_PATH}?q=店"),
            format!("{SEARCH_PATH}?q=店&lang=fr"),
            format!("{SEARCH_PATH}?q=店&lang="),
            // 複数回の指定は受け付けない。
            format!("{SEARCH_PATH}?q=店&q=店2&lang=ja"),
            format!("{SEARCH_PATH}?q=店&lang=ja&lang=en"),
        ] {
            assert_bad_request(client.get(&query));
        }
        // 256 文字ちょうどは入力として受け付ける (キーの未設定で 500 になる)。
        let just_fits = format!("{SEARCH_PATH}?q={}&lang=ja", "あ".repeat(256));
        assert_internal_error(client.get(&just_fits));
    }

    #[test]
    fn wrangler_place_search_fails_500_without_the_api_key() {
        let data = data();
        let lease = server_without_keys();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // キー (GOOGLE_MAPS_API_KEY) が未設定のときは、Google を呼ばずに 500 を返す (FR-22)。
        // ハーネスはこのキーを注入しないため、CI でも確かめられる。
        assert_internal_error(client.get(&format!("{SEARCH_PATH}?q=丸山珈琲&lang=ja")));
        // 認証が無ければキーの確認より先に 401 になる (順序の確認)。
        assert_unauthorized(anonymous(&base_url).get(&format!("{SEARCH_PATH}?q=丸山珈琲&lang=ja")));
    }
}

mod maps_config {
    //! 地図の設定の経路のテスト (FR-22)。

    use super::*;

    #[test]
    fn wrangler_maps_config_unauthenticated_401() {
        let lease = server_without_keys();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get(CONFIG_PATH));
    }

    #[test]
    fn wrangler_maps_config_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 注入したキーがそのまま返る (画面は iframe の URL を組み立てる)。
        let body = assert_status(client.get(CONFIG_PATH), 200);
        assert_eq!(body, json!({ "embed_api_key": API_KEY }));
    }

    #[test]
    fn wrangler_maps_config_ok_without_the_key() {
        let data = data();
        let lease = server_without_keys();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // キーが無いときは null を返し、画面は地図を出さない (FR-22)。
        let body = assert_status(client.get(CONFIG_PATH), 200);
        assert_eq!(body, json!({ "embed_api_key": null }));
    }
}
