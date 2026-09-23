//! 購入と抽出の API の結合テスト (HTTP)。
//!
//! 一覧と単件の取得、登録、更新、アーカイブと解除、入力の検証、参照先の 404 と 409、
//! 他の利用者の記録の 404、ネストした商品と店 (店が無い購入の null を含む)、カーソルの
//! 並び順を検査する。親 (商品、店、購入) をアーカイブしても子から親をたどれることも確認する。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。
//! テストは並行に走るため、記録を書き換えるテストは利用者ごとに分ける。

mod support;

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::blocking::Response;
use serde_json::{json, Value};
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, Seed, SeededBrew, SeededProduct, SeededPurchase, SeededShop};
use support::ServerLease;

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const T21: &str = "2026-09-21T00:00:00.000Z";
const T20: &str = "2026-09-20T00:00:00.000Z";
const T19: &str = "2026-09-19T00:00:00.000Z";

/// 下ごしらえに使う日付。
const D21: &str = "2026-09-21";
const D20: &str = "2026-09-20";
const D19: &str = "2026-09-19";

/// 下ごしらえに使う抽出日時。
const B21: &str = "2026-09-21T10:00:00.000Z";
const B20: &str = "2026-09-20T10:00:00.000Z";
const B19: &str = "2026-09-19T10:00:00.000Z";

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// 購入の一覧のテストの利用者 (同じ購入日の 2 件、店が無い 1 件、アーカイブ済みの店を参照する
    /// 古い 1 件、アーカイブ済みの 1 件)。
    purchase_list_user: String,
    purchase_list_session: String,
    purchase_list_products: Vec<SeededProduct>,
    purchase_list_shop: SeededShop,
    purchase_list_archived_shop: SeededShop,
    purchase_list_purchases: Vec<SeededPurchase>,
    purchase_list_old_purchase: SeededPurchase,
    purchase_list_archived_purchase: SeededPurchase,
    purchase_list_archived_product: SeededProduct,
    /// 購入の書き換えのテストの利用者。
    purchase_write_user: String,
    purchase_write_session: String,
    purchase_write_product: SeededProduct,
    purchase_write_archived_product: SeededProduct,
    purchase_write_shop: SeededShop,
    purchase_write_archived_shop: SeededShop,
    /// 親のアーカイブのテストの利用者。
    purchase_parent_user: String,
    purchase_parent_session: String,
    purchase_parent_product: SeededProduct,
    purchase_parent_shop: SeededShop,
    purchase_parent_purchase: SeededPurchase,
    /// 抽出の一覧のテストの利用者 (同じ抽出日時の 2 件、アーカイブ済みの購入と店を参照する古い
    /// 1 件、アーカイブ済みの 1 件)。
    brew_list_user: String,
    brew_list_session: String,
    brew_list_product: SeededProduct,
    brew_list_shop: SeededShop,
    brew_list_purchases: Vec<SeededPurchase>,
    brew_list_brews: Vec<SeededBrew>,
    brew_list_old_brew: SeededBrew,
    brew_list_archived_brew: SeededBrew,
    /// 抽出の書き換えのテストの利用者。
    brew_write_user: String,
    brew_write_session: String,
    brew_write_purchase: SeededPurchase,
    brew_write_archived_purchase: SeededPurchase,
    /// 親のアーカイブのテストの利用者。
    brew_parent_user: String,
    brew_parent_session: String,
    brew_parent_purchase: SeededPurchase,
    brew_parent_product: SeededProduct,
    brew_parent_shop: SeededShop,
    brew_parent_brew: SeededBrew,
    /// 他の利用者 (404 の検査に使う)。
    other_user: String,
    other_session: String,
    other_product: SeededProduct,
    other_shop: SeededShop,
    other_purchase: SeededPurchase,
    other_brew: SeededBrew,
    /// 記録が無い利用者のセッション。
    empty_session: String,
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

    // 購入の一覧のテストの利用者。同じ購入日の 2 件は、先に作った方の ID が小さい。
    let purchase_list_user = user_id(1);
    seed.user(&purchase_list_user, "purchase list user", created);
    let purchase_list_session = seed.session(&purchase_list_user, future, created);
    let purchase_list_products = vec![
        seed.product(&purchase_list_user, "一覧の豆", T21, T21, None),
        seed.product(&purchase_list_user, "古い豆", T20, T20, None),
    ];
    let purchase_list_archived_product =
        seed.product(&purchase_list_user, "しまった豆", T19, T19, Some(T19));
    // ネストした商品の Flavor Notes の検査のためのタグ (乖離 2)。
    let purchase_list_tag = seed.flavor_tag(&purchase_list_user, "購入のタグ");
    seed.product_flavor_tag(
        &purchase_list_user,
        &purchase_list_products[0].id,
        &purchase_list_tag,
    );
    let purchase_list_shop = seed.shop(
        &purchase_list_user,
        "一覧の店",
        Some("東京都"),
        T21,
        T21,
        None,
    );
    let purchase_list_archived_shop =
        seed.shop(&purchase_list_user, "しまった店", None, T19, T19, Some(T19));
    let purchase_list_purchases = vec![
        seed.purchase(
            &purchase_list_user,
            &purchase_list_products[0].id,
            Some(&purchase_list_shop.id),
            D21,
            T21,
            T21,
            None,
        ),
        seed.purchase(
            &purchase_list_user,
            &purchase_list_products[0].id,
            None,
            D21,
            T21,
            T21,
            None,
        ),
    ];
    let purchase_list_old_purchase = seed.purchase(
        &purchase_list_user,
        &purchase_list_products[1].id,
        Some(&purchase_list_archived_shop.id),
        D20,
        T20,
        T20,
        None,
    );
    let purchase_list_archived_purchase = seed.purchase(
        &purchase_list_user,
        &purchase_list_archived_product.id,
        None,
        D19,
        T19,
        T19,
        Some(T19),
    );

    // 購入の書き換えのテストの利用者。
    let purchase_write_user = user_id(2);
    seed.user(&purchase_write_user, "purchase write user", created);
    let purchase_write_session = seed.session(&purchase_write_user, future, created);
    let purchase_write_product = seed.product(&purchase_write_user, "書き換えの豆", T21, T21, None);
    let purchase_write_archived_product = seed.product(
        &purchase_write_user,
        "書き換えのしまった豆",
        T19,
        T19,
        Some(T19),
    );
    let purchase_write_shop = seed.shop(&purchase_write_user, "書き換えの店", None, T21, T21, None);
    let purchase_write_archived_shop = seed.shop(
        &purchase_write_user,
        "書き換えのしまった店",
        None,
        T19,
        T19,
        Some(T19),
    );

    // 親のアーカイブのテストの利用者。
    let purchase_parent_user = user_id(3);
    seed.user(&purchase_parent_user, "purchase parent user", created);
    let purchase_parent_session = seed.session(&purchase_parent_user, future, created);
    let purchase_parent_product = seed.product(&purchase_parent_user, "親の豆", T21, T21, None);
    let purchase_parent_shop = seed.shop(&purchase_parent_user, "親の店", None, T21, T21, None);
    let purchase_parent_purchase = seed.purchase(
        &purchase_parent_user,
        &purchase_parent_product.id,
        Some(&purchase_parent_shop.id),
        D21,
        T21,
        T21,
        None,
    );

    // 抽出の一覧のテストの利用者。同じ抽出日時の 2 件は、先に作った方の ID が小さい。
    let brew_list_user = user_id(4);
    seed.user(&brew_list_user, "brew list user", created);
    let brew_list_session = seed.session(&brew_list_user, future, created);
    let brew_list_product = seed.product(&brew_list_user, "抽出の豆", T21, T21, None);
    // ネストした商品の Flavor Notes の検査のためのタグ (乖離 2)。
    let brew_list_tag = seed.flavor_tag(&brew_list_user, "抽出のタグ");
    seed.product_flavor_tag(&brew_list_user, &brew_list_product.id, &brew_list_tag);
    let brew_list_shop = seed.shop(&brew_list_user, "抽出の店", None, T21, T21, None);
    let brew_list_archived_shop = seed.shop(
        &brew_list_user,
        "抽出のしまった店",
        None,
        T19,
        T19,
        Some(T19),
    );
    let brew_list_purchases = vec![
        seed.purchase(
            &brew_list_user,
            &brew_list_product.id,
            Some(&brew_list_shop.id),
            D21,
            T21,
            T21,
            None,
        ),
        seed.purchase(
            &brew_list_user,
            &brew_list_product.id,
            None,
            D21,
            T21,
            T21,
            None,
        ),
    ];
    let brew_list_archived_purchase = seed.purchase(
        &brew_list_user,
        &brew_list_product.id,
        Some(&brew_list_archived_shop.id),
        D20,
        T20,
        T20,
        Some(T20),
    );
    let brew_list_brews = vec![
        seed.brew(
            &brew_list_user,
            &brew_list_purchases[0].id,
            B21,
            T21,
            T21,
            None,
        ),
        seed.brew(
            &brew_list_user,
            &brew_list_purchases[1].id,
            B21,
            T21,
            T21,
            None,
        ),
    ];
    let brew_list_old_brew = seed.brew(
        &brew_list_user,
        &brew_list_archived_purchase.id,
        B20,
        T20,
        T20,
        None,
    );
    let brew_list_archived_brew = seed.brew(
        &brew_list_user,
        &brew_list_purchases[0].id,
        B19,
        T19,
        T19,
        Some(T19),
    );

    // 抽出の書き換えのテストの利用者。
    let brew_write_user = user_id(5);
    seed.user(&brew_write_user, "brew write user", created);
    let brew_write_session = seed.session(&brew_write_user, future, created);
    let brew_write_product = seed.product(&brew_write_user, "書き換えの抽出の豆", T21, T21, None);
    let brew_write_shop = seed.shop(&brew_write_user, "書き換えの抽出の店", None, T21, T21, None);
    let brew_write_purchase = seed.purchase(
        &brew_write_user,
        &brew_write_product.id,
        Some(&brew_write_shop.id),
        D21,
        T21,
        T21,
        None,
    );
    let brew_write_archived_purchase = seed.purchase(
        &brew_write_user,
        &brew_write_product.id,
        None,
        D19,
        T19,
        T19,
        Some(T19),
    );

    // 親のアーカイブのテストの利用者。
    let brew_parent_user = user_id(6);
    seed.user(&brew_parent_user, "brew parent user", created);
    let brew_parent_session = seed.session(&brew_parent_user, future, created);
    let brew_parent_product = seed.product(&brew_parent_user, "親の抽出の豆", T21, T21, None);
    let brew_parent_shop = seed.shop(&brew_parent_user, "親の抽出の店", None, T21, T21, None);
    let brew_parent_purchase = seed.purchase(
        &brew_parent_user,
        &brew_parent_product.id,
        Some(&brew_parent_shop.id),
        D21,
        T21,
        T21,
        None,
    );
    let brew_parent_brew = seed.brew(
        &brew_parent_user,
        &brew_parent_purchase.id,
        B21,
        T21,
        T21,
        None,
    );

    // 他の利用者。商品と店と購入と抽出を 1 つずつ持つ。
    let other_user = user_id(7);
    seed.user(&other_user, "other user", created);
    let other_session = seed.session(&other_user, future, created);
    let other_product = seed.product(&other_user, "他人の豆", T21, T21, None);
    let other_shop = seed.shop(&other_user, "他人の店", None, T21, T21, None);
    let other_purchase = seed.purchase(
        &other_user,
        &other_product.id,
        Some(&other_shop.id),
        D21,
        T21,
        T21,
        None,
    );
    let other_brew = seed.brew(&other_user, &other_purchase.id, B21, T21, T21, None);

    // 記録が無い利用者。
    let empty_user = user_id(8);
    seed.user(&empty_user, "empty user", created);
    let empty_session = seed.session(&empty_user, future, created);

    TestData {
        seed_sql: seed.sql(),
        purchase_list_user,
        purchase_list_session,
        purchase_list_products,
        purchase_list_shop,
        purchase_list_archived_shop,
        purchase_list_purchases,
        purchase_list_old_purchase,
        purchase_list_archived_purchase,
        purchase_list_archived_product,
        purchase_write_user,
        purchase_write_session,
        purchase_write_product,
        purchase_write_archived_product,
        purchase_write_shop,
        purchase_write_archived_shop,
        purchase_parent_user,
        purchase_parent_session,
        purchase_parent_product,
        purchase_parent_shop,
        purchase_parent_purchase,
        brew_list_user,
        brew_list_session,
        brew_list_product,
        brew_list_shop,
        brew_list_purchases,
        brew_list_brews,
        brew_list_old_brew,
        brew_list_archived_brew,
        brew_write_user,
        brew_write_session,
        brew_write_purchase,
        brew_write_archived_purchase,
        brew_parent_user,
        brew_parent_session,
        brew_parent_purchase,
        brew_parent_product,
        brew_parent_shop,
        brew_parent_brew,
        other_user,
        other_session,
        other_product,
        other_shop,
        other_purchase,
        other_brew,
        empty_session,
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

/// 対象が無い呼び出しが 404 になることを確かめる。
fn assert_not_found(response: Response) {
    let body = assert_status(response, 404);
    assert_eq!(error_code(&body), Some("not_found"), "{body}");
}

/// 参照先を指定できない呼び出しが 409 になることを確かめる (FR-9、FR-11)。
fn assert_conflict(response: Response) {
    let body = assert_status(response, 409);
    assert_eq!(error_code(&body), Some("conflict"), "{body}");
}

/// 固定長の ISO 8601 UTC の文字列であることを確かめる。
fn assert_timestamp(value: &Value) {
    assert!(
        value.as_str().is_some_and(|text| {
            text.len() == 24
                && text.ends_with('Z')
                && coffee_log_core::datetime::parse_epoch_millis(text).is_ok()
        }),
        "the value must be an ISO 8601 UTC timestamp: {value}"
    );
}

/// ミリ秒だけ待つ。`updated_at` が確実に進むようにする。
fn sleep_millis(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}

/// ネストした商品を確かめる (FR-9、FR-11)。`flavor_notes` は期待するタグ名の配列。
fn assert_nested_product(
    body: &Value,
    product: &SeededProduct,
    user_id: &str,
    flavor_notes: &[&str],
) {
    assert_eq!(body["product"]["id"], product.id, "{body}");
    assert_eq!(body["product"]["name"], product.name, "{body}");
    assert_eq!(body["product"]["user_id"], user_id, "{body}");
    assert_eq!(
        body["product"]["flavor_notes"],
        json!(flavor_notes),
        "{body}"
    );
}

/// ネストした店を確かめる (FR-9、FR-11)。
fn assert_nested_shop(body: &Value, shop: &SeededShop, user_id: &str) {
    assert_eq!(body["shop"]["id"], shop.id, "{body}");
    assert_eq!(body["shop"]["name"], shop.name, "{body}");
    assert_eq!(body["shop"]["user_id"], user_id, "{body}");
}

mod purchases {
    //! 購入の経路のテスト (FR-9、FR-12、FR-5)。

    use super::*;

    // 購入の一覧 (認証が必要、クエリパラメータあり)。

    #[test]
    fn wrangler_purchases_list_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_list_session));

        let body = assert_status(client.get("/api/purchases"), 200);
        let purchases = body["purchases"]
            .as_array()
            .expect("the response must have purchases");
        // 購入日の降順と、同じ日付のときの ID の昇順で返る (FR-9)。
        assert_eq!(purchases.len(), 3, "{body}");
        assert_eq!(purchases[0]["id"], data.purchase_list_purchases[0].id);
        assert_eq!(purchases[1]["id"], data.purchase_list_purchases[1].id);
        assert_eq!(purchases[2]["id"], data.purchase_list_old_purchase.id);
        assert!(
            purchases[0]["id"].as_str() < purchases[1]["id"].as_str(),
            "{body}"
        );
        assert_eq!(purchases[0]["purchased_on"], D21, "{body}");
        assert_eq!(purchases[2]["purchased_on"], D20, "{body}");
        // 応答には商品と店がネストしたオブジェクトとして入る (FR-9)。
        assert_nested_product(
            &purchases[0],
            &data.purchase_list_products[0],
            &data.purchase_list_user,
            &["購入のタグ"],
        );
        assert_nested_shop(
            &purchases[0],
            &data.purchase_list_shop,
            &data.purchase_list_user,
        );
        // 店が無い購入では shop は null になる (FR-9)。
        assert_eq!(purchases[1]["shop"], Value::Null, "{body}");
        // アーカイブ済みの店を参照する購入でも店をたどれる (FR-12)。
        assert_nested_shop(
            &purchases[2],
            &data.purchase_list_archived_shop,
            &data.purchase_list_user,
        );
        assert_timestamp(&purchases[2]["shop"]["archived_at"]);
        // 写真は未設定で、クライアントが有無を知るために photo_key を含める (0009 が操作を扱う)。
        assert_eq!(purchases[0]["photo_key"], Value::Null, "{body}");
        // 一覧は呼び出した利用者の記録だけを返す (FR-5)。
        assert!(
            purchases
                .iter()
                .all(|purchase| purchase["user_id"] == data.purchase_list_user),
            "the list must carry only the caller's records: {body}"
        );
        // 既定では、アーカイブ済みの購入は含まれない (FR-12)。
        assert!(
            !purchases
                .iter()
                .any(|purchase| purchase["id"] == data.purchase_list_archived_purchase.id),
            "the archived purchase must not be listed: {body}"
        );
        assert_eq!(body["next_cursor"], Value::Null);
    }

    #[test]
    fn wrangler_purchases_list_ok_with_include_archived_and_a_cursor() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_list_session));

        // アーカイブ済みを含めると、アーカイブ済みの購入も返る (FR-12)。
        let body = assert_status(client.get("/api/purchases?include_archived=true"), 200);
        let purchases = body["purchases"].as_array().expect("purchases");
        assert_eq!(purchases.len(), 4, "{body}");
        assert_eq!(purchases[3]["id"], data.purchase_list_archived_purchase.id);
        assert_timestamp(&purchases[3]["archived_at"]);
        // アーカイブ済みの商品を参照する購入でも商品をたどれる (FR-12)。
        assert_eq!(
            purchases[3]["product"]["id"], data.purchase_list_archived_product.id,
            "{body}"
        );
        assert_timestamp(&purchases[3]["product"]["archived_at"]);
        // 親のアーカイブのテストの利用者が混ざっていないことを確認する。
        assert!(
            purchases
                .iter()
                .all(|purchase| purchase["user_id"] == data.purchase_list_user),
            "{body}"
        );

        // カーソルで続きを引く。同じ行が重複せず、並び順が保たれる。
        let first = assert_status(client.get("/api/purchases?limit=2"), 200);
        let first_purchases = first["purchases"].as_array().expect("purchases");
        assert_eq!(first_purchases.len(), 2, "{first}");
        let cursor = first["next_cursor"]
            .as_str()
            .expect("a full page must carry the next cursor")
            .to_owned();
        let second = assert_status(
            client.get(&format!("/api/purchases?limit=2&cursor={cursor}")),
            200,
        );
        let second_purchases = second["purchases"].as_array().expect("purchases");
        assert_eq!(second_purchases.len(), 1, "{second}");
        assert_eq!(
            second_purchases[0]["id"],
            data.purchase_list_old_purchase.id
        );
        assert_eq!(second["next_cursor"], Value::Null);
    }

    #[test]
    fn wrangler_purchases_list_returns_only_own_records() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        // 他の利用者の購入は、その利用者の一覧に出るが、こちらの一覧には出ない (FR-5)。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session)).get("/api/purchases"),
            200,
        );
        let ids: Vec<&str> = body["purchases"]
            .as_array()
            .expect("purchases")
            .iter()
            .filter_map(|purchase| purchase["id"].as_str())
            .collect();
        assert_eq!(ids, vec![data.other_purchase.id.as_str()], "{body}");
        assert_eq!(body["purchases"][0]["user_id"], data.other_user, "{body}");

        // 記録が無い利用者の一覧は空になる。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.empty_session)).get("/api/purchases"),
            200,
        );
        assert_eq!(body["purchases"], json!([]), "{body}");
        assert_eq!(body["next_cursor"], Value::Null);
    }

    #[test]
    fn wrangler_purchases_list_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_list_session));
        for query in [
            "limit=0",
            "limit=abc",
            "limit=201",
            "include_archived=yes",
            "cursor=x",
            "cursor=",
        ] {
            assert_bad_request(client.get(&format!("/api/purchases?{query}")));
        }
        // 200 ぴったりは受け付ける。
        assert_status(client.get("/api/purchases?limit=200"), 200);
    }

    #[test]
    fn wrangler_purchases_list_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get("/api/purchases"));
    }

    // 購入の登録 (認証が必要、入力あり)。

    #[test]
    fn wrangler_purchases_create_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));

        // 商品と購入日だけを指定して登録できる。店と価格と重量は省略できる (FR-9)。
        let body = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "product_id": data.purchase_write_product.id,
                    "purchased_on": D21,
                }),
            ),
            200,
        );
        assert_eq!(body["user_id"], data.purchase_write_user, "{body}");
        assert_eq!(body["product_id"], data.purchase_write_product.id, "{body}");
        assert_eq!(body["shop_id"], Value::Null, "{body}");
        assert_eq!(body["purchased_on"], D21, "{body}");
        assert_eq!(body["photo_key"], Value::Null, "{body}");
        assert_eq!(body["archived_at"], Value::Null, "{body}");
        assert_timestamp(&body["created_at"]);
        assert_eq!(body["created_at"], body["updated_at"], "{body}");
        // 価格が無いときは通貨コードも null になる (0007 の設計判断)。
        assert_eq!(body["price_amount"], Value::Null, "{body}");
        assert_eq!(body["price_currency"], Value::Null, "{body}");
        assert_eq!(body["weight_grams"], Value::Null, "{body}");
        // 商品はネストして返り、店は null になる (FR-9)。
        assert_eq!(
            body["product"]["id"], data.purchase_write_product.id,
            "{body}"
        );
        assert_eq!(body["shop"], Value::Null, "{body}");
        let id = body["id"].as_str().expect("the id must be present");
        assert_eq!(id.len(), 36, "the id must be a UUID: {id}");
        assert_eq!(&id[14..15], "4", "the id must be UUID v4: {id}");
        assert!(
            matches!(&id[19..20], "8" | "9" | "a" | "b"),
            "the id must be UUID v4: {id}"
        );

        // 登録した購入は単件で取得できる。
        let fetched = assert_status(client.get(&format!("/api/purchases/{id}")), 200);
        assert_eq!(fetched, body);

        // 全項目を指定して登録する。自由記述は前後の空白を除いて保存する (0006 の規則)。
        let body = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "product_id": data.purchase_write_product.id,
                    "shop_id": data.purchase_write_shop.id,
                    "purchased_on": D20,
                    "roast": "  中煎り  ",
                    "roast_date": D19,
                    "price_amount": 1200,
                    "price_currency": "USD",
                    "weight_grams": 200,
                }),
            ),
            200,
        );
        assert_eq!(body["shop_id"], data.purchase_write_shop.id, "{body}");
        assert_eq!(body["purchased_on"], D20, "{body}");
        assert_eq!(body["roast"], "中煎り", "{body}");
        assert_eq!(body["roast_date"], D19, "{body}");
        assert_eq!(body["price_amount"], 1200, "{body}");
        assert_eq!(body["price_currency"], "USD", "{body}");
        assert_eq!(body["weight_grams"], 200, "{body}");
        assert_nested_shop(&body, &data.purchase_write_shop, &data.purchase_write_user);
        let fetched = assert_status(
            client.get(&format!(
                "/api/purchases/{}",
                body["id"].as_str().expect("id")
            )),
            200,
        );
        assert_eq!(fetched, body);

        // 価格だけを指定して通貨コードを省略すると JPY を保存する (FR-9)。
        let body = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "product_id": data.purchase_write_product.id,
                    "purchased_on": D19,
                    "price_amount": 0,
                }),
            ),
            200,
        );
        assert_eq!(body["price_amount"], 0, "{body}");
        assert_eq!(body["price_currency"], "JPY", "{body}");
    }

    #[test]
    fn wrangler_purchases_create_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        let product = &data.purchase_write_product.id;
        for body in [
            json!({}),
            // 商品と購入日は必須 (FR-9)。
            json!({ "purchased_on": D21 }),
            json!({ "product_id": product }),
            json!({ "product_id": product, "purchased_on": null }),
            json!({ "product_id": null, "purchased_on": D21 }),
            // 購入日は実在する `YYYY-MM-DD` だけを受け付ける (ADR-0002)。
            json!({ "product_id": product, "purchased_on": "" }),
            json!({ "product_id": product, "purchased_on": "2026-02-30" }),
            json!({ "product_id": product, "purchased_on": "2026-2-1" }),
            json!({ "product_id": product, "purchased_on": "2026-9-1" }),
            json!({ "product_id": product, "purchased_on": "2026-09-21T00:00:00.000Z" }),
            json!({ "product_id": product, "purchased_on": " 2026-09-21" }),
            json!({ "product_id": product, "purchased_on": D21, "roast_date": "2026-02-30" }),
            json!({ "product_id": product, "purchased_on": D21, "roast_date": "2026-09-21T00:00:00.000Z" }),
            // 価格と重量は 0 以上の整数だけ (FR-9)。
            json!({ "product_id": product, "purchased_on": D21, "price_amount": -1 }),
            json!({ "product_id": product, "purchased_on": D21, "price_amount": 1.5 }),
            json!({ "product_id": product, "purchased_on": D21, "price_amount": "1200" }),
            json!({ "product_id": product, "purchased_on": D21, "price_amount": 2147483648_i64 }),
            json!({ "product_id": product, "purchased_on": D21, "weight_grams": -1 }),
            json!({ "product_id": product, "purchased_on": D21, "weight_grams": 1.5 }),
            json!({ "product_id": product, "purchased_on": D21, "weight_grams": 2147483648_i64 }),
            // 通貨コードは ISO 4217 の 3 文字の英大文字だけ (FR-9)。
            json!({ "product_id": product, "purchased_on": D21, "price_amount": 100, "price_currency": "usd" }),
            json!({ "product_id": product, "purchased_on": D21, "price_amount": 100, "price_currency": "JP" }),
            json!({ "product_id": product, "purchased_on": D21, "price_amount": 100, "price_currency": "JPYY" }),
            json!({ "product_id": product, "purchased_on": D21, "price_amount": 100, "price_currency": "12" }),
            json!({ "product_id": product, "purchased_on": D21, "price_amount": 100, "price_currency": "あいう" }),
            // 価格が無いのに通貨コードだけを指定することはできない。
            json!({ "product_id": product, "purchased_on": D21, "price_currency": "JPY" }),
            // 受け取らない項目は無視せず 400 にする (0007 の設計判断)。
            json!({ "product_id": product, "purchased_on": D21, "id": "no-such-id" }),
            json!({ "product_id": product, "purchased_on": D21, "created_at": T21 }),
            json!({ "product_id": product, "purchased_on": D21, "updated_at": T21 }),
            json!({ "product_id": product, "purchased_on": D21, "archived_at": T21 }),
            json!({ "product_id": product, "purchased_on": D21, "photo_key": "purchases/photo.jpg" }),
            json!({ "product_id": product, "purchased_on": D21, "user_id": data.purchase_write_user }),
            json!([1, 2]),
        ] {
            assert_bad_request(client.post_json("/api/purchases", &body));
        }
    }

    #[test]
    fn wrangler_purchases_create_reference_404_and_409() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        // 存在しない商品と店、他の利用者の商品と店は 404 (FR-9、FR-5)。
        assert_not_found(client.post_json(
            "/api/purchases",
            &json!({ "product_id": "no-such-id", "purchased_on": D21 }),
        ));
        assert_not_found(client.post_json(
            "/api/purchases",
            &json!({ "product_id": data.other_product.id, "purchased_on": D21 }),
        ));
        assert_not_found(client.post_json(
            "/api/purchases",
            &json!({
                "product_id": data.purchase_write_product.id,
                "shop_id": "no-such-id",
                "purchased_on": D21,
            }),
        ));
        assert_not_found(client.post_json(
            "/api/purchases",
            &json!({
                "product_id": data.purchase_write_product.id,
                "shop_id": data.other_shop.id,
                "purchased_on": D21,
            }),
        ));
        // アーカイブ済みの商品と店は 409 (FR-9)。
        assert_conflict(client.post_json(
            "/api/purchases",
            &json!({
                "product_id": data.purchase_write_archived_product.id,
                "purchased_on": D21,
            }),
        ));
        assert_conflict(client.post_json(
            "/api/purchases",
            &json!({
                "product_id": data.purchase_write_product.id,
                "shop_id": data.purchase_write_archived_shop.id,
                "purchased_on": D21,
            }),
        ));
        // 他の利用者の商品と店は、アーカイブされていないことを確かめる (404 が利用者の違いによるものだと裏付ける)。
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/products/{}", data.other_product.id)),
            200,
        );
        assert_eq!(other["archived_at"], Value::Null, "{other}");
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/shops/{}", data.other_shop.id)),
            200,
        );
        assert_eq!(other["archived_at"], Value::Null, "{other}");
    }

    #[test]
    fn wrangler_purchases_create_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post_json(
            "/api/purchases",
            &json!({ "product_id": "no-such-id", "purchased_on": D21 }),
        ));
    }

    // 購入の単件の取得 (認証が必要、入力なし)。

    #[test]
    fn wrangler_purchases_get_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_list_session));
        let body = assert_status(
            client.get(&format!(
                "/api/purchases/{}",
                data.purchase_list_purchases[0].id
            )),
            200,
        );
        assert_eq!(body["id"], data.purchase_list_purchases[0].id);
        assert_eq!(body["purchased_on"], D21);
        assert_nested_product(
            &body,
            &data.purchase_list_products[0],
            &data.purchase_list_user,
            &["購入のタグ"],
        );
        assert_nested_shop(&body, &data.purchase_list_shop, &data.purchase_list_user);
        // アーカイブ済みでも単件では返す (FR-12)。
        let body = assert_status(
            client.get(&format!(
                "/api/purchases/{}",
                data.purchase_list_archived_purchase.id
            )),
            200,
        );
        assert_timestamp(&body["archived_at"]);
    }

    #[test]
    fn wrangler_purchases_get_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get(&format!(
            "/api/purchases/{}",
            data.purchase_list_purchases[0].id
        )));
    }

    #[test]
    fn wrangler_purchases_get_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_list_session));
        // 他の利用者の ID と存在しない ID は区別せず 404 を返す (FR-5)。
        assert_not_found(client.get(&format!("/api/purchases/{}", data.other_purchase.id)));
        assert_not_found(client.get("/api/purchases/no-such-id"));
    }

    // 購入の更新 (認証が必要、入力あり)。

    #[test]
    fn wrangler_purchases_update_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        let created = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "product_id": data.purchase_write_product.id,
                    "shop_id": data.purchase_write_shop.id,
                    "purchased_on": D21,
                    "roast": "変更前のロースト",
                    "price_amount": 1000,
                }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        let created_at = created["created_at"].clone();
        sleep_millis(10);

        // 全項目を更新する (FR-9)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({
                    "purchased_on": D20,
                    "roast": "  変更後のロースト  ",
                    "roast_date": D19,
                    "price_currency": "USD",
                    "weight_grams": 250,
                }),
            ),
            200,
        );
        assert_eq!(body["purchased_on"], D20, "{body}");
        assert_eq!(body["roast"], "変更後のロースト", "{body}");
        assert_eq!(body["roast_date"], D19, "{body}");
        // 価格は変更せず、通貨コードだけを変更すると USD になる。
        assert_eq!(body["price_amount"], 1000, "{body}");
        assert_eq!(body["price_currency"], "USD", "{body}");
        assert_eq!(body["weight_grams"], 250, "{body}");
        assert_eq!(body["created_at"], created_at, "created_at must not change");
        assert!(
            body["updated_at"].as_str() > created_at.as_str(),
            "updated_at must advance: {body}"
        );

        // 価格だけを指定した更新では、現在の通貨コード (USD) を保つ (項目が無ければ変更しない)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({ "price_amount": 1500 }),
            ),
            200,
        );
        assert_eq!(body["price_amount"], 1500, "{body}");
        assert_eq!(body["price_currency"], "USD", "{body}");

        // 店を null で外せる (FR-9)。
        let body = assert_status(
            client.patch_json(&format!("/api/purchases/{id}"), &json!({ "shop_id": null })),
            200,
        );
        assert_eq!(body["shop_id"], Value::Null, "{body}");
        assert_eq!(body["shop"], Value::Null, "{body}");
        assert_eq!(
            body["product"]["id"], data.purchase_write_product.id,
            "{body}"
        );
        // 単件の取得でも同じ内容になる (応答のエコーだけに頼らない)。
        let fetched = assert_status(client.get(&format!("/api/purchases/{id}")), 200);
        assert_eq!(fetched, body);

        // 価格を null にすると通貨コードも null になる (0007 の設計判断)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({ "price_amount": null, "weight_grams": null, "roast_date": null }),
            ),
            200,
        );
        assert_eq!(body["price_amount"], Value::Null, "{body}");
        assert_eq!(body["price_currency"], Value::Null, "{body}");
        assert_eq!(body["weight_grams"], Value::Null, "{body}");
        assert_eq!(body["roast_date"], Value::Null, "{body}");
        assert_eq!(body["roast"], "変更後のロースト", "{body}");

        // 価格だけを指定したときは、通貨コードが無ければ既定の JPY を保存する (FR-9)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({ "price_amount": 800 }),
            ),
            200,
        );
        assert_eq!(body["price_amount"], 800, "{body}");
        assert_eq!(body["price_currency"], "JPY", "{body}");

        // 無い項目は変更しない。
        let body = assert_status(
            client.patch_json(&format!("/api/purchases/{id}"), &json!({})),
            200,
        );
        assert_eq!(body["purchased_on"], D20, "{body}");
        assert_eq!(body["price_amount"], 800, "{body}");
        assert_eq!(body["price_currency"], "JPY", "{body}");
        let fetched = assert_status(client.get(&format!("/api/purchases/{id}")), 200);
        assert_eq!(fetched, body);

        // アーカイブ済みの購入も更新できる (単件の取得と同じくアーカイブ済みを引くため)。
        assert_status(client.post(&format!("/api/purchases/{id}/archive")), 200);
        let archived = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({ "roast": "アーカイブ後" }),
            ),
            200,
        );
        assert_eq!(archived["roast"], "アーカイブ後", "{archived}");
        assert!(
            archived["archived_at"].as_str().is_some(),
            "the archived_at must be kept: {archived}"
        );

        // 他の利用者の購入は変わっていない。
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/purchases/{}", data.other_purchase.id)),
            200,
        );
        assert_eq!(other["purchased_on"], D21, "{other}");
    }

    #[test]
    fn wrangler_purchases_update_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        let created = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "product_id": data.purchase_write_product.id,
                    "purchased_on": D21,
                    "price_amount": 1000,
                }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        // 価格が無い購入に通貨コードだけを指定することはできない (組は片方だけを持たない)。
        let no_price = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({ "product_id": data.purchase_write_product.id, "purchased_on": D21 }),
            ),
            200,
        );
        assert_bad_request(client.patch_json(
            &format!(
                "/api/purchases/{}",
                no_price["id"].as_str().expect("the id must be present")
            ),
            &json!({ "price_currency": "JPY" }),
        ));
        for body in [
            json!({ "product_id": null }),
            json!({ "purchased_on": null }),
            json!({ "purchased_on": "2026-02-30" }),
            json!({ "roast_date": "2026-13-01" }),
            json!({ "price_amount": -1 }),
            json!({ "price_amount": 1.5 }),
            json!({ "price_amount": 2147483648_i64 }),
            json!({ "weight_grams": -1 }),
            json!({ "weight_grams": 1.5 }),
            json!({ "price_currency": "usd" }),
            json!({ "price_currency": null }),
            json!({ "id": "no-such-id" }),
            json!({ "created_at": T21 }),
            json!({ "updated_at": T21 }),
            json!({ "archived_at": T21 }),
            json!({ "photo_key": "purchases/photo.jpg" }),
            json!([1]),
        ] {
            assert_bad_request(client.patch_json(&format!("/api/purchases/{id}"), &body));
        }
        // 拒否した入力は保存されない。
        let body = assert_status(client.get(&format!("/api/purchases/{id}")), 200);
        assert_eq!(body["purchased_on"], D21, "{body}");
        assert_eq!(body["price_amount"], 1000, "{body}");
        assert_eq!(body["price_currency"], "JPY", "{body}");
    }

    #[test]
    fn wrangler_purchases_update_reference_404_and_409() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        let created = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "product_id": data.purchase_write_product.id,
                    "shop_id": data.purchase_write_shop.id,
                    "purchased_on": D21,
                }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        // 参照先を変更する更新でも、登録と同じ検査をする (FR-9)。
        assert_not_found(client.patch_json(
            &format!("/api/purchases/{id}"),
            &json!({ "product_id": "no-such-id" }),
        ));
        assert_not_found(client.patch_json(
            &format!("/api/purchases/{id}"),
            &json!({ "product_id": data.other_product.id }),
        ));
        assert_not_found(client.patch_json(
            &format!("/api/purchases/{id}"),
            &json!({ "shop_id": data.other_shop.id }),
        ));
        // アーカイブ済みの親への付け替えは 409 (FR-9、FR-12)。
        assert_conflict(client.patch_json(
            &format!("/api/purchases/{id}"),
            &json!({ "product_id": data.purchase_write_archived_product.id }),
        ));
        assert_conflict(client.patch_json(
            &format!("/api/purchases/{id}"),
            &json!({ "shop_id": data.purchase_write_archived_shop.id }),
        ));
        // 参照先を変えない更新は通る。
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({
                    "product_id": data.purchase_write_product.id,
                    "shop_id": data.purchase_write_shop.id,
                    "roast": "参照はそのまま",
                }),
            ),
            200,
        );
        assert_eq!(body["roast"], "参照はそのまま", "{body}");
        // 別の有効な商品と店への付け替えは 200 になり、新しい参照先が保存される (設計判断)。
        let new_product = assert_status(
            client.post_json("/api/products", &json!({ "name": "付け替え先の豆" })),
            200,
        );
        let new_shop = assert_status(
            client.post_json("/api/shops", &json!({ "name": "付け替え先の店" })),
            200,
        );
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({
                    "product_id": new_product["id"],
                    "shop_id": new_shop["id"],
                    "roast": "付け替え後",
                }),
            ),
            200,
        );
        assert_eq!(body["product_id"], new_product["id"], "{body}");
        assert_eq!(body["shop_id"], new_shop["id"], "{body}");
        assert_eq!(body["product"]["name"], "付け替え先の豆", "{body}");
        assert_eq!(body["shop"]["name"], "付け替え先の店", "{body}");
        // 拒否した入力は保存されず、付け替えた参照先が残る。
        let body = assert_status(client.get(&format!("/api/purchases/{id}")), 200);
        assert_eq!(body["product_id"], new_product["id"], "{body}");
        assert_eq!(body["shop_id"], new_shop["id"], "{body}");

        // 親をアーカイブしても、参照先を変えない更新は通る (乖離 3)。
        let product = assert_status(
            client.post_json("/api/products", &json!({ "name": "アーカイブする親の豆" })),
            200,
        );
        let shop = assert_status(
            client.post_json("/api/shops", &json!({ "name": "アーカイブする親の店" })),
            200,
        );
        let purchase = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "product_id": product["id"],
                    "shop_id": shop["id"],
                    "purchased_on": D21,
                }),
            ),
            200,
        );
        let archived_parent_id = purchase["id"].as_str().expect("the id must be present");
        let product_id = product["id"].as_str().expect("the id must be present");
        let shop_id = shop["id"].as_str().expect("the id must be present");
        assert_status(
            client.post(&format!("/api/products/{product_id}/archive")),
            200,
        );
        assert_status(client.post(&format!("/api/shops/{shop_id}/archive")), 200);
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{archived_parent_id}"),
                &json!({
                    "product_id": product_id,
                    "shop_id": shop_id,
                    "roast": "親はアーカイブ済み",
                }),
            ),
            200,
        );
        assert_eq!(body["roast"], "親はアーカイブ済み", "{body}");
        assert_eq!(body["product_id"], product_id, "{body}");
        assert_eq!(body["shop_id"], shop_id, "{body}");

        // 他の利用者の購入の更新は 404 (FR-5)。
        assert_not_found(
            ApiClient::new(&base_url, Some(&data.purchase_list_session)).patch_json(
                &format!("/api/purchases/{}", data.other_purchase.id),
                &json!({ "roast": "のっとり" }),
            ),
        );
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/purchases/{}", data.other_purchase.id)),
            200,
        );
        assert_eq!(other["roast"], Value::Null, "{other}");
    }

    #[test]
    fn wrangler_purchases_update_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).patch_json(
            &format!("/api/purchases/{}", data.purchase_list_purchases[0].id),
            &json!({ "roast": "ロースト" }),
        ));
    }

    // 購入のアーカイブと解除 (認証が必要、入力なし)。

    #[test]
    fn wrangler_purchases_archive_and_unarchive_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        let created = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "product_id": data.purchase_write_product.id,
                    "purchased_on": D21,
                }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        sleep_millis(10);

        let body = assert_status(client.post(&format!("/api/purchases/{id}/archive")), 200);
        assert_timestamp(&body["archived_at"]);
        assert!(
            body["updated_at"].as_str() > created["updated_at"].as_str(),
            "updated_at must advance on archive: {body}"
        );
        // アーカイブした購入は既定の一覧に含まれない (FR-12)。
        let list = assert_status(client.get("/api/purchases"), 200);
        assert!(
            !list["purchases"]
                .as_array()
                .expect("purchases")
                .iter()
                .any(|purchase| purchase["id"] == created["id"]),
            "the archived purchase must not be listed: {list}"
        );
        // アーカイブ済みの購入は include_archived で取得でき、単件でも取得できる (FR-12)。
        let list = assert_status(client.get("/api/purchases?include_archived=true"), 200);
        assert!(
            list["purchases"]
                .as_array()
                .expect("purchases")
                .iter()
                .any(|purchase| purchase["id"] == created["id"]),
            "the archived purchase must be listed: {list}"
        );
        assert_status(client.get(&format!("/api/purchases/{id}")), 200);

        // 繰り返しのアーカイブも 200 を返す (同じ状態への遷移はエラーにしない)。
        assert_status(client.post(&format!("/api/purchases/{id}/archive")), 200);

        // アーカイブ解除で既定の一覧に戻る (FR-12)。
        let body = assert_status(client.post(&format!("/api/purchases/{id}/unarchive")), 200);
        assert_eq!(body["archived_at"], Value::Null, "{body}");
        let list = assert_status(client.get("/api/purchases"), 200);
        assert!(
            list["purchases"]
                .as_array()
                .expect("purchases")
                .iter()
                .any(|purchase| purchase["id"] == created["id"]),
            "the unarchived purchase must be listed: {list}"
        );
        // 繰り返しのアーカイブ解除も 200 を返す。
        assert_status(client.post(&format!("/api/purchases/{id}/unarchive")), 200);
    }

    #[test]
    fn wrangler_purchases_archive_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post(&format!(
            "/api/purchases/{}/archive",
            data.purchase_list_purchases[0].id
        )));
    }

    #[test]
    fn wrangler_purchases_unarchive_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post(&format!(
            "/api/purchases/{}/unarchive",
            data.purchase_list_purchases[0].id
        )));
    }

    #[test]
    fn wrangler_purchases_archive_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_list_session));
        assert_not_found(client.post(&format!(
            "/api/purchases/{}/archive",
            data.other_purchase.id
        )));
        assert_not_found(client.post(&format!(
            "/api/purchases/{}/unarchive",
            data.other_purchase.id
        )));
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/purchases/{}", data.other_purchase.id)),
            200,
        );
        assert_eq!(other["archived_at"], Value::Null);
    }

    #[test]
    fn wrangler_purchases_unarchive_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_not_found(
            ApiClient::new(&base_url, Some(&data.purchase_list_session))
                .post("/api/purchases/no-such-id/unarchive"),
        );
    }

    // 親 (商品、店) のアーカイブ (FR-12)。

    #[test]
    fn wrangler_purchases_keep_tracing_the_archived_parents() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_parent_session));
        // 親をアーカイブしても、購入はアーカイブされず、親をたどれる (FR-12)。
        assert_status(
            client.post(&format!(
                "/api/products/{}/archive",
                data.purchase_parent_product.id
            )),
            200,
        );
        assert_status(
            client.post(&format!(
                "/api/shops/{}/archive",
                data.purchase_parent_shop.id
            )),
            200,
        );
        let body = assert_status(
            client.get(&format!(
                "/api/purchases/{}",
                data.purchase_parent_purchase.id
            )),
            200,
        );
        assert_eq!(body["archived_at"], Value::Null, "{body}");
        assert_nested_product(
            &body,
            &data.purchase_parent_product,
            &data.purchase_parent_user,
            &[],
        );
        assert_nested_shop(
            &body,
            &data.purchase_parent_shop,
            &data.purchase_parent_user,
        );
        assert_timestamp(&body["product"]["archived_at"]);
        assert_timestamp(&body["shop"]["archived_at"]);
        // 既定の一覧にも残る。
        let list = assert_status(client.get("/api/purchases"), 200);
        assert!(
            list["purchases"]
                .as_array()
                .expect("purchases")
                .iter()
                .any(|purchase| purchase["id"] == data.purchase_parent_purchase.id),
            "the purchase of the archived parents must be listed: {list}"
        );
    }
}

mod brews {
    //! 抽出の経路のテスト (FR-11、FR-12、FR-5)。

    use super::*;

    // 抽出の一覧 (認証が必要、クエリパラメータあり)。

    #[test]
    fn wrangler_brews_list_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_list_session));

        let body = assert_status(client.get("/api/brews"), 200);
        let brews = body["brews"]
            .as_array()
            .expect("the response must have brews");
        // 抽出日時の降順と、同じ日時のときの ID の昇順で返る (FR-11)。
        assert_eq!(brews.len(), 3, "{body}");
        assert_eq!(brews[0]["id"], data.brew_list_brews[0].id);
        assert_eq!(brews[1]["id"], data.brew_list_brews[1].id);
        assert_eq!(brews[2]["id"], data.brew_list_old_brew.id);
        assert!(brews[0]["id"].as_str() < brews[1]["id"].as_str(), "{body}");
        assert_eq!(brews[0]["brewed_at"], B21, "{body}");
        // 応答には購入がネストし、その中に商品と店が入る (FR-11)。
        assert_eq!(
            brews[0]["purchase"]["id"], data.brew_list_purchases[0].id,
            "{body}"
        );
        assert_nested_product(
            &brews[0]["purchase"],
            &data.brew_list_product,
            &data.brew_list_user,
            &["抽出のタグ"],
        );
        assert!(
            brews[0]["purchase"]["shop"].is_object(),
            "the shop must be nested: {body}"
        );
        assert_eq!(
            brews[0]["purchase"]["shop"]["id"], data.brew_list_shop.id,
            "{body}"
        );
        // 店が無い購入では shop は null になる (FR-9、FR-11)。
        assert_eq!(brews[1]["purchase"]["shop"], Value::Null, "{body}");
        // アーカイブ済みの購入と店を参照する抽出でも、購入と商品と店をたどれる (FR-12)。
        assert_eq!(
            brews[2]["purchase"]["id"], data.brew_list_old_brew.purchase_id,
            "{body}"
        );
        assert_timestamp(&brews[2]["purchase"]["archived_at"]);
        assert_timestamp(&brews[2]["purchase"]["shop"]["archived_at"]);
        assert_nested_product(
            &brews[2]["purchase"],
            &data.brew_list_product,
            &data.brew_list_user,
            &["抽出のタグ"],
        );
        // 一覧は呼び出した利用者の記録だけを返す (FR-5)。
        assert!(
            brews
                .iter()
                .all(|brew| brew["user_id"] == data.brew_list_user),
            "the list must carry only the caller's records: {body}"
        );
        // 既定では、アーカイブ済みの抽出は含まれない (FR-12)。
        assert!(
            !brews
                .iter()
                .any(|brew| brew["id"] == data.brew_list_archived_brew.id),
            "the archived brew must not be listed: {body}"
        );
        assert_eq!(body["next_cursor"], Value::Null);
    }

    #[test]
    fn wrangler_brews_list_ok_with_include_archived_and_a_cursor() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_list_session));

        // アーカイブ済みを含めると、アーカイブ済みの抽出も返る (FR-12)。
        let body = assert_status(client.get("/api/brews?include_archived=true"), 200);
        let brews = body["brews"].as_array().expect("brews");
        assert_eq!(brews.len(), 4, "{body}");
        assert_eq!(brews[3]["id"], data.brew_list_archived_brew.id);
        assert_timestamp(&brews[3]["archived_at"]);

        // カーソルで続きを引く。同じ行が重複せず、並び順が保たれる。
        let first = assert_status(client.get("/api/brews?limit=2"), 200);
        let first_brews = first["brews"].as_array().expect("brews");
        assert_eq!(first_brews.len(), 2, "{first}");
        let cursor = first["next_cursor"]
            .as_str()
            .expect("a full page must carry the next cursor")
            .to_owned();
        let second = assert_status(
            client.get(&format!("/api/brews?limit=2&cursor={cursor}")),
            200,
        );
        let second_brews = second["brews"].as_array().expect("brews");
        assert_eq!(second_brews.len(), 1, "{second}");
        assert_eq!(second_brews[0]["id"], data.brew_list_old_brew.id);
        assert_eq!(second["next_cursor"], Value::Null);
    }

    #[test]
    fn wrangler_brews_list_returns_only_own_records() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        // 他の利用者の抽出は、その利用者の一覧に出るが、こちらの一覧には出ない (FR-5)。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session)).get("/api/brews"),
            200,
        );
        let ids: Vec<&str> = body["brews"]
            .as_array()
            .expect("brews")
            .iter()
            .filter_map(|brew| brew["id"].as_str())
            .collect();
        assert_eq!(ids, vec![data.other_brew.id.as_str()], "{body}");

        // 記録が無い利用者の一覧は空になる。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.empty_session)).get("/api/brews"),
            200,
        );
        assert_eq!(body["brews"], json!([]), "{body}");
        assert_eq!(body["next_cursor"], Value::Null);
    }

    #[test]
    fn wrangler_brews_list_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_list_session));
        for query in [
            "limit=0",
            "limit=abc",
            "limit=201",
            "include_archived=1",
            "cursor=x",
        ] {
            assert_bad_request(client.get(&format!("/api/brews?{query}")));
        }
    }

    #[test]
    fn wrangler_brews_list_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get("/api/brews"));
    }

    // 抽出の登録 (認証が必要、入力あり)。

    #[test]
    fn wrangler_brews_create_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_write_session));

        // 購入と抽出日時だけを指定して登録できる。他の項目は省略できる (FR-11)。
        let body = assert_status(
            client.post_json(
                "/api/brews",
                &json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "brewed_at": B21,
                }),
            ),
            200,
        );
        assert_eq!(body["user_id"], data.brew_write_user, "{body}");
        assert_eq!(body["purchase_id"], data.brew_write_purchase.id, "{body}");
        assert_eq!(body["brewed_at"], B21, "{body}");
        for field in [
            "dose_grams",
            "water_grams",
            "water_temp_c",
            "brew_time_seconds",
            "method",
            "grind_setting",
            "rating",
            "notes",
        ] {
            assert_eq!(body[field], Value::Null, "{field}: {body}");
        }
        assert_eq!(body["archived_at"], Value::Null, "{body}");
        assert_timestamp(&body["created_at"]);
        assert_eq!(body["created_at"], body["updated_at"], "{body}");
        // 購入がネストし、その中に商品と店が入る (FR-11)。
        assert_eq!(
            body["purchase"]["product"]["id"], data.brew_write_purchase.product_id,
            "{body}"
        );
        assert!(body["purchase"]["shop"].is_object(), "{body}");
        let id = body["id"].as_str().expect("the id must be present");
        assert_eq!(id.len(), 36, "the id must be a UUID: {id}");

        // 登録した抽出は単件で取得できる。
        let fetched = assert_status(client.get(&format!("/api/brews/{id}")), 200);
        assert_eq!(fetched, body);

        // 全項目を指定して登録する。自由記述は前後の空白を除いて保存する (0006 の規則)。
        let body = assert_status(
            client.post_json(
                "/api/brews",
                &json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "brewed_at": B20,
                    "dose_grams": 15.5,
                    "water_grams": 250,
                    "water_temp_c": 92.5,
                    "brew_time_seconds": 150,
                    "method": "  ペーパードリップ  ",
                    "grind_setting": "  中細  ",
                    "rating": 4,
                    "notes": "  良い出来  ",
                }),
            ),
            200,
        );
        assert_eq!(body["dose_grams"], 15.5, "{body}");
        assert_eq!(body["water_grams"], 250.0, "{body}");
        assert_eq!(body["water_temp_c"], 92.5, "{body}");
        assert_eq!(body["brew_time_seconds"], 150, "{body}");
        assert_eq!(body["method"], "ペーパードリップ", "{body}");
        assert_eq!(body["grind_setting"], "中細", "{body}");
        assert_eq!(body["rating"], 4, "{body}");
        assert_eq!(body["notes"], "良い出来", "{body}");
        // 保存した内容は単件の取得でも同じになる。
        let fetched = assert_status(
            client.get(&format!("/api/brews/{}", body["id"].as_str().expect("id"))),
            200,
        );
        assert_eq!(fetched, body);
        // 小数第 1 位までの値は受け付ける (0 と 0.0 を含む)。受け付けた値がそのまま保存されることも確かめる。
        for (field, body, expected) in [
            (
                "dose_grams",
                json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "brewed_at": B19,
                    "dose_grams": 0,
                }),
                0.0,
            ),
            (
                "water_grams",
                json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "brewed_at": B19,
                    "water_grams": 0.0,
                }),
                0.0,
            ),
            (
                "water_temp_c",
                json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "brewed_at": B19,
                    "water_temp_c": 0.1,
                }),
                0.1,
            ),
            // -0.0 は数値として 0 と等しいため、0 として受け付ける (符号は問わない)。
            (
                "dose_grams",
                json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "brewed_at": B19,
                    "dose_grams": -0.0,
                }),
                0.0,
            ),
        ] {
            let response = assert_status(client.post_json("/api/brews", &body), 200);
            let value = response[field]
                .as_f64()
                .unwrap_or_else(|| panic!("{field} must be a number: {response}"));
            assert!(
                (value - expected).abs() < 1e-9,
                "{field} must be stored as {expected}: {response}"
            );
        }
    }

    #[test]
    fn wrangler_brews_create_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_write_session));
        let purchase = &data.brew_write_purchase.id;
        for body in [
            json!({}),
            // 購入と抽出日時は必須 (FR-11)。
            json!({ "brewed_at": B21 }),
            json!({ "purchase_id": purchase }),
            json!({ "purchase_id": purchase, "brewed_at": null }),
            json!({ "purchase_id": null, "brewed_at": B21 }),
            // 抽出日時は ISO 8601 の UTC の固定長だけを受け付ける (ADR-0002)。
            json!({ "purchase_id": purchase, "brewed_at": "" }),
            json!({ "purchase_id": purchase, "brewed_at": D21 }),
            json!({ "purchase_id": purchase, "brewed_at": "2026-09-21T10:00:00Z" }),
            json!({ "purchase_id": purchase, "brewed_at": "2026-09-21T10:00:00.000+09:00" }),
            json!({ "purchase_id": purchase, "brewed_at": "2026-09-21T19:00:00.000+09:00" }),
            json!({ "purchase_id": purchase, "brewed_at": "2026-09-21T10:00:00.000" }),
            json!({ "purchase_id": purchase, "brewed_at": "2026-09-21 10:00:00.000Z" }),
            json!({ "purchase_id": purchase, "brewed_at": "2026-09-21T10:00:00.000z" }),
            json!({ "purchase_id": purchase, "brewed_at": "2026-02-30T10:00:00.000Z" }),
            // 豆の量、湯量、湯の温度は 0 以上で小数第 1 位まで (FR-11)。
            json!({ "purchase_id": purchase, "brewed_at": B21, "dose_grams": -1 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "dose_grams": -0.1 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "dose_grams": 1.25 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "water_grams": -1 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "water_grams": 0.01 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "water_temp_c": -1 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "water_temp_c": 92.55 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "dose_grams": "15.5" }),
            // 指数表記は小数第 1 位までという条件を表記で判定できないため受け付けない。
            json!({ "purchase_id": purchase, "brewed_at": B21, "dose_grams": 1e300 }),
            // 時間は 0 以上の整数、評価は 1 から 5 の整数 (FR-11)。
            json!({ "purchase_id": purchase, "brewed_at": B21, "brew_time_seconds": -1 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "brew_time_seconds": 1.5 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "rating": 0 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "rating": 6 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "rating": 1.5 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "rating": -1 }),
            // 購入以外の参照は受け付けない (ADR-0006)。
            json!({ "purchase_id": purchase, "brewed_at": B21, "product_id": "no-such-id" }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "shop_id": "no-such-id" }),
            // 受け取らない項目は無視せず 400 にする (0007 の設計判断)。
            json!({ "purchase_id": purchase, "brewed_at": B21, "id": "no-such-id" }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "created_at": T21 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "updated_at": T21 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "archived_at": T21 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "user_id": data.brew_write_user }),
            json!([1]),
        ] {
            assert_bad_request(client.post_json("/api/brews", &body));
        }
    }

    #[test]
    fn wrangler_brews_create_reference_404_and_409() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_write_session));
        // 存在しない購入と他の利用者の購入は 404 (FR-11、FR-5)。
        assert_not_found(client.post_json(
            "/api/brews",
            &json!({ "purchase_id": "no-such-id", "brewed_at": B21 }),
        ));
        assert_not_found(client.post_json(
            "/api/brews",
            &json!({ "purchase_id": data.other_purchase.id, "brewed_at": B21 }),
        ));
        // アーカイブ済みの購入は 409 (FR-11、FR-12)。
        assert_conflict(client.post_json(
            "/api/brews",
            &json!({
                "purchase_id": data.brew_write_archived_purchase.id,
                "brewed_at": B21,
            }),
        ));
        // 他の利用者の購入は、アーカイブされていないことを確かめる (404 が利用者の違いによるものだと裏付ける)。
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/purchases/{}", data.other_purchase.id)),
            200,
        );
        assert_eq!(other["archived_at"], Value::Null, "{other}");
    }

    #[test]
    fn wrangler_brews_create_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post_json(
            "/api/brews",
            &json!({ "purchase_id": "no-such-id", "brewed_at": B21 }),
        ));
    }

    // 抽出の単件の取得 (認証が必要、入力なし)。

    #[test]
    fn wrangler_brews_get_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_list_session));
        let body = assert_status(
            client.get(&format!("/api/brews/{}", data.brew_list_brews[0].id)),
            200,
        );
        assert_eq!(body["id"], data.brew_list_brews[0].id);
        assert_eq!(body["brewed_at"], B21);
        assert_eq!(
            body["purchase"]["id"], data.brew_list_purchases[0].id,
            "{body}"
        );
        assert_nested_product(
            &body["purchase"],
            &data.brew_list_product,
            &data.brew_list_user,
            &["抽出のタグ"],
        );
        // アーカイブ済みでも単件では返す (FR-12)。
        let body = assert_status(
            client.get(&format!("/api/brews/{}", data.brew_list_archived_brew.id)),
            200,
        );
        assert_timestamp(&body["archived_at"]);
    }

    #[test]
    fn wrangler_brews_get_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(
            anonymous(&base_url).get(&format!("/api/brews/{}", data.brew_list_brews[0].id)),
        );
    }

    #[test]
    fn wrangler_brews_get_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_list_session));
        // 他の利用者の ID と存在しない ID は区別せず 404 を返す (FR-5)。
        assert_not_found(client.get(&format!("/api/brews/{}", data.other_brew.id)));
        assert_not_found(client.get("/api/brews/no-such-id"));
    }

    // 抽出の更新 (認証が必要、入力あり)。

    #[test]
    fn wrangler_brews_update_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_write_session));
        let created = assert_status(
            client.post_json(
                "/api/brews",
                &json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "brewed_at": B21,
                    "dose_grams": 12.0,
                }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        let created_at = created["created_at"].clone();
        sleep_millis(10);

        // 全項目を更新する (FR-11)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/brews/{id}"),
                &json!({
                    "brewed_at": B20,
                    "dose_grams": 15.5,
                    "water_grams": 250.0,
                    "water_temp_c": 93.0,
                    "brew_time_seconds": 180,
                    "method": "  フレンチプレス  ",
                    "grind_setting": "  粗め  ",
                    "rating": 5,
                    "notes": "  更新後の感想  ",
                }),
            ),
            200,
        );
        assert_eq!(body["brewed_at"], B20, "{body}");
        assert_eq!(body["dose_grams"], 15.5, "{body}");
        assert_eq!(body["water_grams"], 250.0, "{body}");
        assert_eq!(body["water_temp_c"], 93.0, "{body}");
        assert_eq!(body["brew_time_seconds"], 180, "{body}");
        assert_eq!(body["method"], "フレンチプレス", "{body}");
        assert_eq!(body["grind_setting"], "粗め", "{body}");
        assert_eq!(body["rating"], 5, "{body}");
        assert_eq!(body["notes"], "更新後の感想", "{body}");
        assert_eq!(body["created_at"], created_at, "created_at must not change");
        assert!(
            body["updated_at"].as_str() > created_at.as_str(),
            "updated_at must advance: {body}"
        );

        // 任意の項目は null で NULL にできる。
        let body = assert_status(
            client.patch_json(
                &format!("/api/brews/{id}"),
                &json!({
                    "dose_grams": null,
                    "water_grams": null,
                    "water_temp_c": null,
                    "brew_time_seconds": null,
                    "rating": null,
                    "notes": null,
                }),
            ),
            200,
        );
        for field in [
            "dose_grams",
            "water_grams",
            "water_temp_c",
            "brew_time_seconds",
            "rating",
            "notes",
        ] {
            assert_eq!(body[field], Value::Null, "{field}: {body}");
        }
        assert_eq!(body["method"], "フレンチプレス", "{body}");

        // 無い項目は変更しない。
        let body = assert_status(
            client.patch_json(&format!("/api/brews/{id}"), &json!({})),
            200,
        );
        assert_eq!(body["brewed_at"], B20, "{body}");
        assert_eq!(body["method"], "フレンチプレス", "{body}");
        let fetched = assert_status(client.get(&format!("/api/brews/{id}")), 200);
        assert_eq!(fetched, body);

        // アーカイブ済みの抽出も更新できる。
        assert_status(client.post(&format!("/api/brews/{id}/archive")), 200);
        let archived = assert_status(
            client.patch_json(&format!("/api/brews/{id}"), &json!({ "rating": 3 })),
            200,
        );
        assert_eq!(archived["rating"], 3, "{archived}");
        assert!(
            archived["archived_at"].as_str().is_some(),
            "the archived_at must be kept: {archived}"
        );

        // 他の利用者の抽出は変わっていない。
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/brews/{}", data.other_brew.id)),
            200,
        );
        assert_eq!(other["rating"], Value::Null, "{other}");
    }

    #[test]
    fn wrangler_brews_update_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_write_session));
        let created = assert_status(
            client.post_json(
                "/api/brews",
                &json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "brewed_at": B21,
                    "dose_grams": 15.0,
                }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        for body in [
            json!({ "purchase_id": null }),
            json!({ "brewed_at": null }),
            json!({ "brewed_at": "2026-09-21T10:00:00.000+09:00" }),
            json!({ "brewed_at": "2026-09-21T10:00:00Z" }),
            json!({ "dose_grams": -1 }),
            json!({ "dose_grams": 1.25 }),
            json!({ "water_temp_c": 92.55 }),
            json!({ "brew_time_seconds": -1 }),
            json!({ "brew_time_seconds": 1.5 }),
            json!({ "rating": 0 }),
            json!({ "rating": 6 }),
            json!({ "product_id": "no-such-id" }),
            json!({ "shop_id": "no-such-id" }),
            json!({ "id": "no-such-id" }),
            json!({ "created_at": T21 }),
            json!({ "archived_at": T21 }),
            json!({ "user_id": data.brew_write_user }),
            json!([1]),
        ] {
            assert_bad_request(client.patch_json(&format!("/api/brews/{id}"), &body));
        }
        // 拒否した入力は保存されない。
        let body = assert_status(client.get(&format!("/api/brews/{id}")), 200);
        assert_eq!(body["brewed_at"], B21, "{body}");
        assert_eq!(body["dose_grams"], 15.0, "{body}");
    }

    #[test]
    fn wrangler_brews_update_reference_404_and_409() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_write_session));
        let created = assert_status(
            client.post_json(
                "/api/brews",
                &json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "brewed_at": B21,
                }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        // 参照先を変更する更新でも、登録と同じ検査をする (FR-11)。
        assert_not_found(client.patch_json(
            &format!("/api/brews/{id}"),
            &json!({ "purchase_id": "no-such-id" }),
        ));
        assert_not_found(client.patch_json(
            &format!("/api/brews/{id}"),
            &json!({ "purchase_id": data.other_purchase.id }),
        ));
        // アーカイブ済みの購入への付け替えは 409 (FR-11、FR-12)。
        assert_conflict(client.patch_json(
            &format!("/api/brews/{id}"),
            &json!({ "purchase_id": data.brew_write_archived_purchase.id }),
        ));
        // 参照先を変えない更新は通る。
        let body = assert_status(
            client.patch_json(
                &format!("/api/brews/{id}"),
                &json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "rating": 2,
                }),
            ),
            200,
        );
        assert_eq!(body["rating"], 2, "{body}");
        // 別の有効な購入への付け替えは 200 になり、新しい参照先が保存される (設計判断)。
        let new_product = assert_status(
            client.post_json("/api/products", &json!({ "name": "付け替え先の購入の豆" })),
            200,
        );
        let new_purchase = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({ "product_id": new_product["id"], "purchased_on": D21 }),
            ),
            200,
        );
        let body = assert_status(
            client.patch_json(
                &format!("/api/brews/{id}"),
                &json!({ "purchase_id": new_purchase["id"] }),
            ),
            200,
        );
        assert_eq!(body["purchase_id"], new_purchase["id"], "{body}");
        assert_eq!(
            body["purchase"]["product"]["name"], "付け替え先の購入の豆",
            "{body}"
        );
        // 拒否した入力は保存されない。
        let body = assert_status(client.get(&format!("/api/brews/{id}")), 200);
        assert_eq!(body["purchase_id"], new_purchase["id"], "{body}");

        // 購入をアーカイブしても、参照先を変えない更新は通る (乖離 3)。
        let product = assert_status(
            client.post_json(
                "/api/products",
                &json!({ "name": "アーカイブする購入の豆" }),
            ),
            200,
        );
        let purchase = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({ "product_id": product["id"], "purchased_on": D21 }),
            ),
            200,
        );
        let archived_purchase_id = purchase["id"].as_str().expect("the id must be present");
        let brew = assert_status(
            client.post_json(
                "/api/brews",
                &json!({ "purchase_id": archived_purchase_id, "brewed_at": B20 }),
            ),
            200,
        );
        let archived_brew_id = brew["id"].as_str().expect("the id must be present");
        assert_status(
            client.post(&format!("/api/purchases/{archived_purchase_id}/archive")),
            200,
        );
        let body = assert_status(
            client.patch_json(
                &format!("/api/brews/{archived_brew_id}"),
                &json!({ "purchase_id": archived_purchase_id, "rating": 3 }),
            ),
            200,
        );
        assert_eq!(body["rating"], 3, "{body}");
        assert_eq!(body["purchase_id"], archived_purchase_id, "{body}");

        // 他の利用者の抽出の更新は 404 (FR-5)。
        assert_not_found(
            ApiClient::new(&base_url, Some(&data.brew_list_session)).patch_json(
                &format!("/api/brews/{}", data.other_brew.id),
                &json!({ "rating": 1 }),
            ),
        );
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/brews/{}", data.other_brew.id)),
            200,
        );
        assert_eq!(other["rating"], Value::Null, "{other}");
    }

    #[test]
    fn wrangler_brews_update_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).patch_json(
            &format!("/api/brews/{}", data.brew_list_brews[0].id),
            &json!({ "rating": 3 }),
        ));
    }

    // 抽出のアーカイブと解除 (認証が必要、入力なし)。

    #[test]
    fn wrangler_brews_archive_and_unarchive_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_write_session));
        let created = assert_status(
            client.post_json(
                "/api/brews",
                &json!({
                    "purchase_id": data.brew_write_purchase.id,
                    "brewed_at": B21,
                }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        sleep_millis(10);

        let body = assert_status(client.post(&format!("/api/brews/{id}/archive")), 200);
        assert_timestamp(&body["archived_at"]);
        assert!(
            body["updated_at"].as_str() > created["updated_at"].as_str(),
            "updated_at must advance on archive: {body}"
        );
        // アーカイブした抽出は既定の一覧に含まれない (FR-12)。
        let list = assert_status(client.get("/api/brews"), 200);
        assert!(
            !list["brews"]
                .as_array()
                .expect("brews")
                .iter()
                .any(|brew| brew["id"] == created["id"]),
            "the archived brew must not be listed: {list}"
        );
        // アーカイブ済みの抽出は include_archived で取得でき、単件でも取得できる (FR-12)。
        let list = assert_status(client.get("/api/brews?include_archived=true"), 200);
        assert!(
            list["brews"]
                .as_array()
                .expect("brews")
                .iter()
                .any(|brew| brew["id"] == created["id"]),
            "the archived brew must be listed: {list}"
        );
        assert_status(client.get(&format!("/api/brews/{id}")), 200);

        // 繰り返しのアーカイブも 200 を返す。
        assert_status(client.post(&format!("/api/brews/{id}/archive")), 200);

        // アーカイブ解除で既定の一覧に戻る (FR-12)。
        let body = assert_status(client.post(&format!("/api/brews/{id}/unarchive")), 200);
        assert_eq!(body["archived_at"], Value::Null, "{body}");
        let list = assert_status(client.get("/api/brews"), 200);
        assert!(
            list["brews"]
                .as_array()
                .expect("brews")
                .iter()
                .any(|brew| brew["id"] == created["id"]),
            "the unarchived brew must be listed: {list}"
        );
        // 繰り返しのアーカイブ解除も 200 を返す。
        assert_status(client.post(&format!("/api/brews/{id}/unarchive")), 200);
    }

    #[test]
    fn wrangler_brews_archive_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post(&format!(
            "/api/brews/{}/archive",
            data.brew_list_brews[0].id
        )));
    }

    #[test]
    fn wrangler_brews_unarchive_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post(&format!(
            "/api/brews/{}/unarchive",
            data.brew_list_brews[0].id
        )));
    }

    #[test]
    fn wrangler_brews_archive_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_list_session));
        assert_not_found(client.post(&format!("/api/brews/{}/archive", data.other_brew.id)));
        assert_not_found(client.post(&format!("/api/brews/{}/unarchive", data.other_brew.id)));
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/brews/{}", data.other_brew.id)),
            200,
        );
        assert_eq!(other["archived_at"], Value::Null);
    }

    #[test]
    fn wrangler_brews_unarchive_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_not_found(
            ApiClient::new(&base_url, Some(&data.brew_list_session))
                .post("/api/brews/no-such-id/unarchive"),
        );
    }

    // 親 (購入、商品、店) のアーカイブ (FR-12)。

    #[test]
    fn wrangler_brews_keep_tracing_the_archived_parents() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_parent_session));
        // 親 (購入、商品、店) をアーカイブしても、抽出はアーカイブされず、親をたどれる (FR-12)。
        assert_status(
            client.post(&format!(
                "/api/purchases/{}/archive",
                data.brew_parent_purchase.id
            )),
            200,
        );
        assert_status(
            client.post(&format!(
                "/api/products/{}/archive",
                data.brew_parent_product.id
            )),
            200,
        );
        assert_status(
            client.post(&format!("/api/shops/{}/archive", data.brew_parent_shop.id)),
            200,
        );
        let body = assert_status(
            client.get(&format!("/api/brews/{}", data.brew_parent_brew.id)),
            200,
        );
        assert_eq!(body["archived_at"], Value::Null, "{body}");
        assert_eq!(
            body["purchase"]["id"], data.brew_parent_purchase.id,
            "{body}"
        );
        assert_timestamp(&body["purchase"]["archived_at"]);
        assert_nested_product(
            &body["purchase"],
            &data.brew_parent_product,
            &data.brew_parent_user,
            &[],
        );
        assert_nested_shop(
            &body["purchase"],
            &data.brew_parent_shop,
            &data.brew_parent_user,
        );
        assert_timestamp(&body["purchase"]["product"]["archived_at"]);
        assert_timestamp(&body["purchase"]["shop"]["archived_at"]);
        // 既定の一覧にも残る。
        let list = assert_status(client.get("/api/brews"), 200);
        assert!(
            list["brews"]
                .as_array()
                .expect("brews")
                .iter()
                .any(|brew| brew["id"] == data.brew_parent_brew.id),
            "the brew of the archived parents must be listed: {list}"
        );
    }
}

mod schema {
    //! migration の適用後のスキーマの検査。

    use super::*;

    #[test]
    fn wrangler_the_price_currency_column_allows_null() {
        let lease = server();
        // migration 0002 が通貨コードを NULL 許容にしている (乖離 1)。
        let count = lease
            .use_server(|server| {
                server.query_int(
                    "SELECT COUNT(*) AS count FROM pragma_table_info('purchases') \
                     WHERE name = 'price_currency' AND \"notnull\" = 0",
                )
            })
            .expect("the schema must be readable");
        assert_eq!(count, 1, "the price currency column must allow NULL");
        // migration 0002 は列を作り直すため、外部キーとインデックスが残っていることも確かめる。
        let foreign_keys = lease
            .use_server(|server| {
                server.query_int(
                    "SELECT COUNT(*) AS count FROM pragma_foreign_key_list('brews') \
                     WHERE \"table\" = 'purchases'",
                )
            })
            .expect("the schema must be readable");
        assert_eq!(foreign_keys, 1, "the foreign key to purchases must remain");
        let index = lease
            .use_server(|server| {
                server.query_int(
                    "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'index' \
                     AND name = 'idx_purchases_user_archived_purchased_on'",
                )
            })
            .expect("the schema must be readable");
        assert_eq!(index, 1, "the purchases index must remain");
    }
}
