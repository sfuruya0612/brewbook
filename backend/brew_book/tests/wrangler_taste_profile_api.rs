//! 購入の豆の情報から味と特徴を推定する API の結合テスト (HTTP)。FR-24、ADR-0023。
//!
//! 未認証 401、言語の入力不正 400、存在しない購入と他の利用者の購入の 404、入力が正しいときの
//! AI の失敗 (500) を確認する。正常系 (推定が返る) は Workers AI の推論を要するため CI では
//! 実行できず、staging へのデプロイで実在の購入を指定して確認する (PRD の成功指標の測定方法)。
//! ハーネスは `wrangler dev` を `--local` で起動するため、AI バインディングは常に失敗し、
//! 500 の経路は CI でも確かめられる。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。

mod support;

use std::sync::OnceLock;

use reqwest::blocking::Response;
use serde_json::{json, Value};
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, PurchaseTexts, Seed};
use support::ServerLease;

/// 下ごしらえで保存しておく推定 (FR-24)。
const SAVED_PROFILE: &str = r#"{"summary":"保存済みの推定","flavor_notes":["甘い"],"acidity":null,"sweetness":null,"bitterness":null,"body":null,"aroma":null,"brewing_method":"V60","grind_setting":"中細","process_description":"ナチュラル精製。","region_characteristics":"花の香り。"}"#;

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// 推定の利用者のセッション。
    session: String,
    /// 推定する購入の ID。
    purchase_id: String,
    /// 他の利用者の購入の ID。
    other_purchase_id: String,
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
    let other_user = user_id(2);
    seed.user(&user, "taste user", created);
    seed.user(&other_user, "other taste user", created);
    let session = seed.session(&user, future, created);
    let purchase = seed.purchase_with_texts(
        &user,
        "Ethiopia Guji Natural",
        None,
        "2026-09-14",
        PurchaseTexts {
            producer: Some("Guji Highland Farm"),
            origin: Some("Ethiopia"),
            region: Some("Guji"),
            process: Some("Natural"),
            variety: Some("Heirloom"),
            roast: Some("浅煎り"),
        },
        created,
        created,
    );
    let other_purchase = seed.purchase(
        &other_user,
        "Other beans",
        None,
        "2026-09-15",
        created,
        created,
    );
    seed.set_purchase_taste_profile(&user, &purchase.id, SAVED_PROFILE);
    TestData {
        seed_sql: seed.sql(),
        session,
        purchase_id: purchase.id,
        other_purchase_id: other_purchase.id,
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

/// 存在しない購入の呼び出しが 404 になることを確かめる。
fn assert_not_found(response: Response) {
    let body = assert_status(response, 404);
    assert_eq!(error_code(&body), Some("not_found"), "{body}");
}

/// 内部エラーの呼び出しが 500 になることを確かめる (AI の呼び出しの失敗。FR-24)。
fn assert_internal_error(response: Response) {
    let body = assert_status(response, 500);
    assert_eq!(error_code(&body), Some("internal_error"), "{body}");
}

mod taste_profile {
    //! 味と特徴の推定の経路のテスト (FR-24)。

    use super::*;

    /// 購入の推定の経路を組み立てる。
    fn path(purchase_id: &str, lang: &str) -> String {
        format!("/api/purchases/{purchase_id}/taste-profile?lang={lang}")
    }

    #[test]
    fn wrangler_taste_profile_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        // 認証が必要な経路である (未認証は 401)。
        assert_unauthorized(anonymous(&base_url).post(&path(&data.purchase_id, "ja")));
    }

    #[test]
    fn wrangler_taste_profile_language_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 言語は必須で、ja と en だけを受け付ける (FR-16、FR-24)。
        assert_bad_request(client.post(&format!(
            "/api/purchases/{}/taste-profile",
            data.purchase_id
        )));
        for lang in ["fr", "ja-JP", "en-US", ""] {
            assert_bad_request(client.post(&path(&data.purchase_id, lang)));
        }
        // 同じパラメータを 2 回送る入力も 400 にする。
        assert_bad_request(client.post(&format!(
            "/api/purchases/{}/taste-profile?lang=ja&lang=en",
            data.purchase_id
        )));
    }

    #[test]
    fn wrangler_taste_profile_missing_purchase_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 存在しない購入は 404 を返す。
        assert_not_found(client.post(&path("missing-purchase", "ja")));
        // 他の利用者の購入は、存在を区別せず 404 を返す (FR-5)。
        assert_not_found(client.post(&path(&data.other_purchase_id, "ja")));
    }

    #[test]
    fn wrangler_purchase_reads_the_saved_taste_profile() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // 保存した推定は購入の応答に含まれ、画面を開いたときに表示できる (FR-24)。
        let body = assert_status(
            client.get(&format!("/api/purchases/{}", data.purchase_id)),
            200,
        );
        assert_eq!(body["taste_profile"]["summary"], "保存済みの推定", "{body}");
        assert_eq!(
            body["taste_profile"]["flavor_notes"],
            json!(["甘い"]),
            "{body}"
        );
        assert_eq!(body["taste_profile"]["brewing_method"], "V60", "{body}");
        assert_eq!(body["taste_profile"]["grind_setting"], "中細", "{body}");
        assert_eq!(
            body["taste_profile"]["process_description"], "ナチュラル精製。",
            "{body}"
        );
        assert_eq!(
            body["taste_profile"]["region_characteristics"], "花の香り。",
            "{body}"
        );
    }

    #[test]
    fn wrangler_taste_profile_fails_500_without_the_remote_binding() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.session));
        // AI の呼び出しに失敗したときは、理由を区別せず 500 を返す (FR-24)。
        // ハーネスはリモートのバインディングを無効にして起動するため、ここでは常に失敗する。
        assert_internal_error(client.post(&path(&data.purchase_id, "ja")));
        assert_internal_error(client.post(&path(&data.purchase_id, "en")));
        // 失敗しても、前に保存した推定は消さない。
        let body = assert_status(
            client.get(&format!("/api/purchases/{}", data.purchase_id)),
            200,
        );
        assert_eq!(body["taste_profile"]["summary"], "保存済みの推定", "{body}");
        // 認証が無ければ AI を呼ばずに 401 になる (順序の確認)。
        assert_unauthorized(anonymous(&base_url).post(&path(&data.purchase_id, "ja")));
    }
}
