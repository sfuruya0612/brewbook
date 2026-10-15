//! 購入と抽出の API の結合テスト (HTTP)。
//!
//! 一覧と単件の取得、登録、更新、入力の検証、参照先の 404、
//! 他の利用者の記録の 404、購入が直下に持つ商品の項目と Flavor Notes、ネストした店
//! (店が無い購入の null を含む)、カーソルの
//! 並び順を検査する。子から親をたどれることも確認する。
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
use support::seed::{user_id, Seed, SeededBrew, SeededPurchase, SeededShop};
use support::ServerLease;

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const T21: &str = "2026-09-21T00:00:00.000Z";
const T20: &str = "2026-09-20T00:00:00.000Z";

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
    /// 購入の一覧のテストの利用者 (同じ購入日の 2 件、店が無い 1 件、古い 1 件)。
    purchase_list_user: String,
    purchase_list_session: String,
    purchase_list_shop: SeededShop,
    purchase_list_purchases: Vec<SeededPurchase>,
    purchase_list_old_purchase: SeededPurchase,
    /// 購入の書き換えのテストの利用者。
    purchase_write_user: String,
    purchase_write_session: String,
    purchase_write_shop: SeededShop,
    /// 抽出の一覧のテストの利用者 (同じ抽出日時の 2 件、古い 1 件)。
    brew_list_user: String,
    brew_list_session: String,
    brew_list_shop: SeededShop,
    brew_list_purchases: Vec<SeededPurchase>,
    brew_list_brews: Vec<SeededBrew>,
    brew_list_old_brew: SeededBrew,
    /// 抽出の書き換えのテストの利用者。
    brew_write_user: String,
    brew_write_session: String,
    brew_write_purchase: SeededPurchase,
    /// 他の利用者 (404 の検査に使う)。
    other_user: String,
    other_session: String,
    other_shop: SeededShop,
    other_purchase: SeededPurchase,
    other_brew: SeededBrew,
    /// 記録が無い利用者のセッション。
    empty_session: String,
    /// 購入の削除のテストの利用者 (0056)。写真つきの購入 1 件と抽出 2 件と、
    /// 同じタグを共有する購入 1 件を持つ (ADR-0022)。
    delete_purchase_session: String,
    delete_purchase_purchase: SeededPurchase,
    delete_purchase_other_purchase: SeededPurchase,
    delete_purchase_brews: Vec<SeededBrew>,
    /// 購入に付けた写真のオブジェクトキー (R2 の検査に使う)。
    delete_purchase_photo_key: String,
    /// 抽出の削除のテストの利用者 (0056)。購入 1 件と抽出 2 件を持つ。
    delete_brew_session: String,
    delete_brew_purchase: SeededPurchase,
    delete_brew_brews: Vec<SeededBrew>,
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
    // 購入の Flavor Notes の検査のためのタグ (乖離 2、ADR-0022)。
    let purchase_list_tag = seed.flavor_tag(&purchase_list_user, "購入のタグ");
    let purchase_list_shop = seed.shop(&purchase_list_user, "一覧の店", Some("東京都"), T21, T21);
    let purchase_list_purchases = vec![
        seed.purchase(
            &purchase_list_user,
            "一覧の豆",
            Some(&purchase_list_shop.id),
            D21,
            T21,
            T21,
        ),
        seed.purchase(&purchase_list_user, "一覧の豆", None, D21, T21, T21),
    ];
    seed.purchase_flavor_tag(
        &purchase_list_user,
        &purchase_list_purchases[0].id,
        &purchase_list_tag,
    );
    let purchase_list_old_purchase = seed.purchase(
        &purchase_list_user,
        "古い豆",
        Some(&purchase_list_shop.id),
        D20,
        T20,
        T20,
    );

    // 購入の書き換えのテストの利用者。
    let purchase_write_user = user_id(2);
    seed.user(&purchase_write_user, "purchase write user", created);
    let purchase_write_session = seed.session(&purchase_write_user, future, created);
    let purchase_write_shop = seed.shop(&purchase_write_user, "書き換えの店", None, T21, T21);

    // 抽出の一覧のテストの利用者。同じ抽出日時の 2 件は、先に作った方の ID が小さい。
    let brew_list_user = user_id(4);
    seed.user(&brew_list_user, "brew list user", created);
    let brew_list_session = seed.session(&brew_list_user, future, created);
    // ネストした購入の Flavor Notes の検査のためのタグ (乖離 2、ADR-0022)。
    let brew_list_tag = seed.flavor_tag(&brew_list_user, "抽出のタグ");
    let brew_list_shop = seed.shop(&brew_list_user, "抽出の店", None, T21, T21);
    let brew_list_purchases = vec![
        seed.purchase(
            &brew_list_user,
            "抽出の豆",
            Some(&brew_list_shop.id),
            D21,
            T21,
            T21,
        ),
        seed.purchase(&brew_list_user, "抽出の豆", None, D21, T21, T21),
    ];
    let brew_list_old_purchase = seed.purchase(
        &brew_list_user,
        "抽出の豆",
        Some(&brew_list_shop.id),
        D20,
        T20,
        T20,
    );
    for purchase in brew_list_purchases
        .iter()
        .chain(std::iter::once(&brew_list_old_purchase))
    {
        seed.purchase_flavor_tag(&brew_list_user, &purchase.id, &brew_list_tag);
    }
    let brew_list_brews = vec![
        seed.brew(&brew_list_user, &brew_list_purchases[0].id, B21, T21, T21),
        seed.brew(&brew_list_user, &brew_list_purchases[1].id, B21, T21, T21),
    ];
    let brew_list_old_brew = seed.brew(&brew_list_user, &brew_list_old_purchase.id, B20, T20, T20);

    // 抽出の書き換えのテストの利用者。
    let brew_write_user = user_id(5);
    seed.user(&brew_write_user, "brew write user", created);
    let brew_write_session = seed.session(&brew_write_user, future, created);
    let brew_write_shop = seed.shop(&brew_write_user, "書き換えの抽出の店", None, T21, T21);
    let brew_write_purchase = seed.purchase(
        &brew_write_user,
        "書き換えの抽出の豆",
        Some(&brew_write_shop.id),
        D21,
        T21,
        T21,
    );

    // 他の利用者。店と購入と抽出を 1 つずつ持つ。
    let other_user = user_id(7);
    seed.user(&other_user, "other user", created);
    let other_session = seed.session(&other_user, future, created);
    let other_shop = seed.shop(&other_user, "他人の店", None, T21, T21);
    let other_purchase =
        seed.purchase(&other_user, "他人の豆", Some(&other_shop.id), D21, T21, T21);
    let other_brew = seed.brew(&other_user, &other_purchase.id, B21, T21, T21);

    // 記録が無い利用者。
    let empty_user = user_id(8);
    seed.user(&empty_user, "empty user", created);
    let empty_session = seed.session(&empty_user, future, created);

    // 購入の削除のテストの利用者 (0056)。写真つきの購入 1 件と抽出 2 件と、同じタグを共有する
    // 購入 1 件を持つ (ADR-0022)。
    let delete_purchase_user = user_id(11);
    seed.user(&delete_purchase_user, "delete purchase user", created);
    let delete_purchase_session = seed.session(&delete_purchase_user, future, created);
    let delete_purchase_purchase =
        seed.purchase(&delete_purchase_user, "削除の購入の豆", None, D21, T21, T21);
    let delete_purchase_other_purchase =
        seed.purchase(&delete_purchase_user, "残る購入の豆", None, D20, T20, T20);
    let shared_tag = seed.flavor_tag(&delete_purchase_user, "shared");
    seed.purchase_flavor_tag(
        &delete_purchase_user,
        &delete_purchase_purchase.id,
        &shared_tag,
    );
    seed.purchase_flavor_tag(
        &delete_purchase_user,
        &delete_purchase_other_purchase.id,
        &shared_tag,
    );
    let delete_purchase_photo_key = format!(
        "users/{delete_purchase_user}/purchases/{}/photo.jpg",
        delete_purchase_purchase.id
    );
    seed.raw(&format!(
        "UPDATE purchases SET photo_key = '{delete_purchase_photo_key}' WHERE id = '{}'",
        delete_purchase_purchase.id
    ));
    let delete_purchase_brews = vec![
        seed.brew(
            &delete_purchase_user,
            &delete_purchase_purchase.id,
            B21,
            T21,
            T21,
        ),
        seed.brew(
            &delete_purchase_user,
            &delete_purchase_purchase.id,
            B20,
            T20,
            T20,
        ),
    ];

    // 抽出の削除のテストの利用者 (0056)。購入 1 件と抽出 2 件を持つ。
    let delete_brew_user = user_id(12);
    seed.user(&delete_brew_user, "delete brew user", created);
    let delete_brew_session = seed.session(&delete_brew_user, future, created);
    let delete_brew_purchase =
        seed.purchase(&delete_brew_user, "削除の抽出の豆", None, D21, T21, T21);
    let delete_brew_brews = vec![
        seed.brew(&delete_brew_user, &delete_brew_purchase.id, B21, T21, T21),
        seed.brew(&delete_brew_user, &delete_brew_purchase.id, B20, T20, T20),
    ];

    TestData {
        seed_sql: seed.sql(),
        purchase_list_user,
        purchase_list_session,
        purchase_list_shop,
        purchase_list_purchases,
        purchase_list_old_purchase,
        purchase_write_user,
        purchase_write_session,
        purchase_write_shop,
        brew_list_user,
        brew_list_session,
        brew_list_shop,
        brew_list_purchases,
        brew_list_brews,
        brew_list_old_brew,
        brew_write_user,
        brew_write_session,
        brew_write_purchase,
        other_user,
        other_session,
        other_shop,
        other_purchase,
        other_brew,
        empty_session,
        delete_purchase_session,
        delete_purchase_purchase,
        delete_purchase_other_purchase,
        delete_purchase_brews,
        delete_purchase_photo_key,
        delete_brew_session,
        delete_brew_purchase,
        delete_brew_brews,
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

/// 固定長の ISO 8601 UTC の文字列であることを確かめる。
fn assert_timestamp(value: &Value) {
    assert!(
        value.as_str().is_some_and(|text| {
            text.len() == 24
                && text.ends_with('Z')
                && brew_book_core::datetime::parse_epoch_millis(text).is_ok()
        }),
        "the value must be an ISO 8601 UTC timestamp: {value}"
    );
}

/// ミリ秒だけ待つ。`updated_at` が確実に進むようにする。
fn sleep_millis(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}

/// 購入が直下に持つ商品の項目を確かめる (FR-9、FR-11、ADR-0022)。
/// `flavor_notes` は期待するタグ名の配列。
fn assert_purchase_fields(body: &Value, name: &str, user_id: &str, flavor_notes: &[&str]) {
    assert_eq!(body["name"], name, "{body}");
    assert_eq!(body["user_id"], user_id, "{body}");
    assert_eq!(body["flavor_notes"], json!(flavor_notes), "{body}");
    // 購入にも favorited_at が含まれる (FR-21)。
    assert!(
        body.get("favorited_at").is_some(),
        "the purchase must carry favorited_at: {body}"
    );
    // 商品はネストせず、商品の ID も無い (ADR-0022)。
    assert!(
        body.get("product").is_none() && body.get("product_id").is_none(),
        "the purchase must not carry a nested product: {body}"
    );
}

/// ネストした店を確かめる (FR-9、FR-11)。
fn assert_nested_shop(body: &Value, shop: &SeededShop, user_id: &str) {
    assert_eq!(body["shop"]["id"], shop.id, "{body}");
    assert_eq!(body["shop"]["name"], shop.name, "{body}");
    assert_eq!(body["shop"]["user_id"], user_id, "{body}");
    // 入れ子の店にも favorited_at が含まれる (FR-21)。
    assert!(
        body["shop"].get("favorited_at").is_some(),
        "the nested shop must carry favorited_at: {body}"
    );
}

mod purchases {
    //! 購入の経路のテスト (FR-9、FR-5)。

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
        // 応答は商品の項目を直下に持ち、店はネストしたオブジェクトとして入る (FR-9、ADR-0022)。
        assert_purchase_fields(
            &purchases[0],
            &data.purchase_list_purchases[0].name,
            &data.purchase_list_user,
            &["購入のタグ"],
        );
        // タグを付けていない購入では flavor_notes は空の配列になる。
        assert_eq!(purchases[1]["flavor_notes"], json!([]), "{body}");
        assert_nested_shop(
            &purchases[0],
            &data.purchase_list_shop,
            &data.purchase_list_user,
        );
        // 店が無い購入では shop は null になる (FR-9)。
        assert_eq!(purchases[1]["shop"], Value::Null, "{body}");
        // 店を参照する購入でも店をたどれる。
        assert_nested_shop(
            &purchases[2],
            &data.purchase_list_shop,
            &data.purchase_list_user,
        );
        // 写真は未設定で、クライアントが有無を知るために photo_key を含める (0009 が操作を扱う)。
        assert_eq!(purchases[0]["photo_key"], Value::Null, "{body}");
        // 一覧は呼び出した利用者の記録だけを返す (FR-5)。
        assert!(
            purchases
                .iter()
                .all(|purchase| purchase["user_id"] == data.purchase_list_user),
            "the list must carry only the caller's records: {body}"
        );
        // 応答に archived_at は無い。
        for purchase in purchases {
            assert!(
                purchase.get("archived_at").is_none(),
                "the response must not carry archived_at: {purchase}"
            );
        }
        assert_eq!(body["next_cursor"], Value::Null);
        // include_archived は未知のパラメータとして無視され、挙動が変わらない。
        let with_parameter = assert_status(client.get("/api/purchases?include_archived=true"), 200);
        assert_eq!(with_parameter, body, "{with_parameter}");
    }

    #[test]
    fn wrangler_purchases_list_ok_with_a_cursor() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_list_session));

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
        for query in ["limit=0", "limit=abc", "limit=201", "cursor=x", "cursor="] {
            assert_bad_request(client.get(&format!("/api/purchases?{query}")));
        }
        // sort、order、favorite は受け付けない値を 400 にする (FR-20、FR-21)。
        // 購入の一覧の sort は purchased_on、price_amount、weight_grams だけで、他の一覧のキーと
        // 商品名は受け付けない (0057)。
        for query in [
            "sort=unknown",
            "sort=name",
            "sort=created_at",
            "sort=rating",
            "order=up",
            "favorite=yes",
            "favorite=1",
        ] {
            assert_bad_request(client.get(&format!("/api/purchases?{query}")));
        }
        // 200 ぴったりは受け付ける。
        assert_status(client.get("/api/purchases?limit=200"), 200);
        // 受け取らないパラメータは未知のパラメータとして無視する (400 にしない)。
        assert_status(client.get("/api/purchases?include_archived=yes"), 200);
    }

    #[test]
    fn wrangler_purchases_list_sorts_and_filters() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_list_session));

        // 購入日の昇順で並ぶ (FR-20)。既定は購入日の降順。
        let asc = assert_status(
            client.get("/api/purchases?sort=purchased_on&order=asc"),
            200,
        );
        let purchases = asc["purchases"].as_array().expect("purchases");
        assert_eq!(
            purchases[0]["id"], data.purchase_list_old_purchase.id,
            "{asc}"
        );
        assert_eq!(
            purchases[1]["id"], data.purchase_list_purchases[0].id,
            "{asc}"
        );
        assert_eq!(
            purchases[2]["id"], data.purchase_list_purchases[1].id,
            "{asc}"
        );
        // 応答に favorited_at が含まれる (FR-21)。
        for purchase in purchases {
            assert!(purchase.get("favorited_at").is_some(), "{purchase}");
        }
        // 価格と重さでも並べられ、3 件とも返る (FR-20、0057 で商品名を足していない)。
        for query in [
            "sort=price_amount&order=asc",
            "sort=weight_grams&order=desc",
        ] {
            let body = assert_status(client.get(&format!("/api/purchases?{query}")), 200);
            let sorted = body["purchases"].as_array().expect("purchases");
            assert_eq!(sorted.len(), purchases.len(), "{body}");
        }
    }

    #[test]
    fn wrangler_purchases_favorite_put_and_delete() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        let created = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "name": "お気に入りの豆",
                    "purchased_on": D21,
                    "price_amount": 1200,
                    "price_currency": "JPY",
                }),
            ),
            200,
        );
        let id = created["id"]
            .as_str()
            .expect("the id must be present")
            .to_owned();
        assert_eq!(created["favorited_at"], Value::Null, "{created}");

        let first = assert_status(client.put(&format!("/api/purchases/{id}/favorite")), 200);
        assert!(first["favorited_at"].is_string(), "{first}");
        let favorited_at = first["favorited_at"].clone();
        let updated_at = first["updated_at"].clone();
        let again = assert_status(client.put(&format!("/api/purchases/{id}/favorite")), 200);
        assert_eq!(again["favorited_at"], favorited_at, "{again}");
        assert_eq!(again["updated_at"], updated_at, "{again}");

        let removed = assert_status(client.delete(&format!("/api/purchases/{id}/favorite")), 200);
        assert_eq!(removed["favorited_at"], Value::Null, "{removed}");
        let removed_updated_at = removed["updated_at"].clone();
        let again = assert_status(client.delete(&format!("/api/purchases/{id}/favorite")), 200);
        assert_eq!(again["favorited_at"], Value::Null, "{again}");
        assert_eq!(again["updated_at"], removed_updated_at, "{again}");

        // お気に入りのみの一覧に反映される (FR-21)。
        assert_status(client.put(&format!("/api/purchases/{id}/favorite")), 200);
        let body = assert_status(client.get("/api/purchases?favorite=true"), 200);
        let purchases = body["purchases"].as_array().expect("purchases");
        assert_eq!(purchases.len(), 1, "{body}");
        assert_eq!(purchases[0]["id"], id, "{body}");

        // 未認証は 401、他の利用者の ID は 404。
        assert_unauthorized(anonymous(&base_url).put(&format!("/api/purchases/{id}/favorite")));
        assert_unauthorized(anonymous(&base_url).delete(&format!("/api/purchases/{id}/favorite")));
        let other = ApiClient::new(&base_url, Some(&data.other_session));
        assert_not_found(other.put(&format!("/api/purchases/{id}/favorite")));
        assert_not_found(other.delete(&format!("/api/purchases/{id}/favorite")));
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

        // 商品名と購入日だけを指定して登録できる。店と価格と重量と商品の他の項目は省略できる
        // (FR-9、ADR-0022)。商品名は前後の空白を除いて保存する。
        let body = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "name": "  名前だけの豆  ",
                    "purchased_on": D21,
                }),
            ),
            200,
        );
        assert_eq!(body["user_id"], data.purchase_write_user, "{body}");
        assert_purchase_fields(&body, "名前だけの豆", &data.purchase_write_user, &[]);
        for field in ["producer", "origin", "region", "process", "variety"] {
            assert_eq!(body[field], Value::Null, "{field}: {body}");
        }
        assert_eq!(body["shop_id"], Value::Null, "{body}");
        assert_eq!(body["purchased_on"], D21, "{body}");
        assert_eq!(body["photo_key"], Value::Null, "{body}");
        assert!(body.get("archived_at").is_none(), "{body}");
        assert_timestamp(&body["created_at"]);
        assert_eq!(body["created_at"], body["updated_at"], "{body}");
        // 価格が無いときは通貨コードも null になる (0007 の設計判断)。
        assert_eq!(body["price_amount"], Value::Null, "{body}");
        assert_eq!(body["price_currency"], Value::Null, "{body}");
        assert_eq!(body["weight_grams"], Value::Null, "{body}");
        // 店は null になる (FR-9)。
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

        // 全項目とタグを指定して登録する。自由記述は前後の空白を除き、タグは重複を除いて共有する
        // (0006 の規則、FR-8、ADR-0022)。
        let body = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "name": "  全部入りの豆  ",
                    "producer": "  生産者  ",
                    "origin": "  エチオピア  ",
                    "region": "  イルガチェフェ  ",
                    "process": "  ウォッシュト  ",
                    "variety": "  在来種  ",
                    "flavor_notes": ["  チョコ  ", "berry", "チョコ", " berry "],
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
        assert_eq!(body["name"], "全部入りの豆", "{body}");
        assert_eq!(body["producer"], "生産者", "{body}");
        assert_eq!(body["origin"], "エチオピア", "{body}");
        assert_eq!(body["region"], "イルガチェフェ", "{body}");
        assert_eq!(body["process"], "ウォッシュト", "{body}");
        assert_eq!(body["variety"], "在来種", "{body}");
        assert_eq!(body["flavor_notes"], json!(["berry", "チョコ"]), "{body}");
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
        // タグは利用者ごとに名前で一意にする (FR-8)。
        let tags = assert_status(client.get("/api/flavor-tags"), 200);
        let names: Vec<&str> = tags["flavor_tags"]
            .as_array()
            .expect("flavor_tags")
            .iter()
            .filter_map(|tag| tag["name"].as_str())
            .collect();
        assert_eq!(
            names.iter().filter(|name| **name == "チョコ").count(),
            1,
            "the tag must be shared: {tags}"
        );

        // 価格だけを指定して通貨コードを省略すると JPY を保存する (FR-9)。
        let body = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "name": "書き換えの豆",
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
    fn wrangler_purchases_create_ok_with_many_flavor_notes() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        // タグの置き換えは 1 つのまとまり (D1 の batch) で行う。多数のタグでも通ることを確認する。
        let names: Vec<String> = (0..60).map(|index| format!("tag-{index:02}")).collect();
        let body = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({ "name": "タグが多い豆", "purchased_on": D21, "flavor_notes": names }),
            ),
            200,
        );
        let flavor_notes = body["flavor_notes"]
            .as_array()
            .expect("flavor_notes")
            .clone();
        assert_eq!(flavor_notes.len(), 60, "{body}");
        assert_eq!(body["flavor_notes"][0], "tag-00");
        assert_eq!(body["flavor_notes"][59], "tag-59");
    }

    #[test]
    fn wrangler_purchases_create_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        let name = "検査の豆";
        for body in [
            json!({}),
            // 商品名と購入日は必須で、商品名は前後の空白を除いて空なら拒否する (FR-9、ADR-0022)。
            json!({ "purchased_on": D21 }),
            json!({ "name": name }),
            json!({ "name": name, "purchased_on": null }),
            json!({ "name": null, "purchased_on": D21 }),
            json!({ "name": "", "purchased_on": D21 }),
            json!({ "name": "   ", "purchased_on": D21 }),
            // 空白だけのタグ名は拒否する (FR-8)。
            json!({ "name": name, "purchased_on": D21, "flavor_notes": ["チョコ", "  "] }),
            json!({ "name": name, "purchased_on": D21, "flavor_notes": [""] }),
            // タグの配列でない値と `null` は拒否する (タグを付けないときは項目を省くか空の配列)。
            json!({ "name": name, "purchased_on": D21, "flavor_notes": "チョコ" }),
            json!({ "name": name, "purchased_on": D21, "flavor_notes": [1] }),
            json!({ "name": name, "purchased_on": D21, "flavor_notes": null }),
            // 商品の ID は受け付けない (商品は購入に統合した。ADR-0022)。
            json!({ "name": name, "purchased_on": D21, "product_id": "no-such-id" }),
            // 購入日は実在する `YYYY-MM-DD` だけを受け付ける (ADR-0002)。
            json!({ "name": name, "purchased_on": "" }),
            json!({ "name": name, "purchased_on": "2026-02-30" }),
            json!({ "name": name, "purchased_on": "2026-2-1" }),
            json!({ "name": name, "purchased_on": "2026-9-1" }),
            json!({ "name": name, "purchased_on": "2026-09-21T00:00:00.000Z" }),
            json!({ "name": name, "purchased_on": " 2026-09-21" }),
            json!({ "name": name, "purchased_on": D21, "roast_date": "2026-02-30" }),
            json!({ "name": name, "purchased_on": D21, "roast_date": "2026-09-21T00:00:00.000Z" }),
            // 価格と重量は 0 以上の整数だけ (FR-9)。
            json!({ "name": name, "purchased_on": D21, "price_amount": -1 }),
            json!({ "name": name, "purchased_on": D21, "price_amount": 1.5 }),
            json!({ "name": name, "purchased_on": D21, "price_amount": "1200" }),
            json!({ "name": name, "purchased_on": D21, "price_amount": 2147483648_i64 }),
            json!({ "name": name, "purchased_on": D21, "weight_grams": -1 }),
            json!({ "name": name, "purchased_on": D21, "weight_grams": 1.5 }),
            json!({ "name": name, "purchased_on": D21, "weight_grams": 2147483648_i64 }),
            // 通貨コードは ISO 4217 の 3 文字の英大文字だけ (FR-9)。
            json!({ "name": name, "purchased_on": D21, "price_amount": 100, "price_currency": "usd" }),
            json!({ "name": name, "purchased_on": D21, "price_amount": 100, "price_currency": "JP" }),
            json!({ "name": name, "purchased_on": D21, "price_amount": 100, "price_currency": "JPYY" }),
            json!({ "name": name, "purchased_on": D21, "price_amount": 100, "price_currency": "12" }),
            json!({ "name": name, "purchased_on": D21, "price_amount": 100, "price_currency": "あいう" }),
            // 価格が無いのに通貨コードだけを指定することはできない。
            json!({ "name": name, "purchased_on": D21, "price_currency": "JPY" }),
            // 受け取らない項目は無視せず 400 にする (0007 の設計判断)。
            json!({ "name": name, "purchased_on": D21, "id": "no-such-id" }),
            json!({ "name": name, "purchased_on": D21, "created_at": T21 }),
            json!({ "name": name, "purchased_on": D21, "updated_at": T21 }),
            json!({ "name": name, "purchased_on": D21, "photo_key": "purchases/photo.jpg" }),
            json!({ "name": name, "purchased_on": D21, "user_id": data.purchase_write_user }),
            json!([1, 2]),
        ] {
            assert_bad_request(client.post_json("/api/purchases", &body));
        }
    }

    #[test]
    fn wrangler_purchases_create_reference_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        // 存在しない店と他の利用者の店は 404 (FR-9、FR-5)。
        assert_not_found(client.post_json(
            "/api/purchases",
            &json!({
                "name": "書き換えの豆",
                "shop_id": "no-such-id",
                "purchased_on": D21,
            }),
        ));
        assert_not_found(client.post_json(
            "/api/purchases",
            &json!({
                "name": "書き換えの豆",
                "shop_id": data.other_shop.id,
                "purchased_on": D21,
            }),
        ));
        // 他の利用者の店は、その利用者からは参照できる (404 が利用者の違いによるものだと裏付ける)。
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/shops/{}", data.other_shop.id)),
            200,
        );
        assert_eq!(other["id"], data.other_shop.id, "{other}");
    }

    #[test]
    fn wrangler_purchases_create_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post_json(
            "/api/purchases",
            &json!({ "name": "豆", "purchased_on": D21 }),
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
        assert_purchase_fields(
            &body,
            &data.purchase_list_purchases[0].name,
            &data.purchase_list_user,
            &["購入のタグ"],
        );
        assert_nested_shop(&body, &data.purchase_list_shop, &data.purchase_list_user);
        assert!(body.get("archived_at").is_none(), "{body}");
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
                    "name": "変更前の豆",
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
        assert!(body.get("archived_at").is_none(), "{body}");
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
        // 商品の項目を指定しない更新では商品の項目は変わらない (ADR-0022)。
        assert_eq!(body["name"], "変更前の豆", "{body}");
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

        // 他の利用者の購入は変わっていない。
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/purchases/{}", data.other_purchase.id)),
            200,
        );
        assert_eq!(other["purchased_on"], D21, "{other}");
    }

    #[test]
    fn wrangler_purchases_update_ok_with_the_product_fields_and_the_flavor_notes() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        let created = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "name": "変更前の商品の豆",
                    "producer": "変更前の生産者",
                    "flavor_notes": ["leftover-check", "shared-check"],
                    "purchased_on": D21,
                }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        sleep_millis(10);

        // 商品の項目を更新する。自由記述は前後の空白を除いて保存する (FR-9、ADR-0022)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({
                    "name": "  変更後の豆  ",
                    "producer": "  変更後の生産者  ",
                    "origin": "  エチオピア  ",
                    "region": "  イルガチェフェ  ",
                    "process": "  ウォッシュト  ",
                    "variety": "  在来種  ",
                }),
            ),
            200,
        );
        assert_eq!(body["name"], "変更後の豆", "{body}");
        assert_eq!(body["producer"], "変更後の生産者", "{body}");
        assert_eq!(body["origin"], "エチオピア", "{body}");
        assert_eq!(body["region"], "イルガチェフェ", "{body}");
        assert_eq!(body["process"], "ウォッシュト", "{body}");
        assert_eq!(body["variety"], "在来種", "{body}");
        assert_eq!(body["created_at"], created["created_at"]);
        assert!(
            body["updated_at"].as_str() > created["updated_at"].as_str(),
            "updated_at must advance: {body}"
        );
        // 購入の項目を指定しない更新では購入の項目は変わらない。
        assert_eq!(body["purchased_on"], D21, "{body}");
        // タグを指定しない更新ではタグは変わらない。
        assert_eq!(
            body["flavor_notes"],
            json!(["leftover-check", "shared-check"]),
            "{body}"
        );

        // タグは配列で置き換える (FR-8)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({ "flavor_notes": ["shared-check"] }),
            ),
            200,
        );
        assert_eq!(body["flavor_notes"], json!(["shared-check"]), "{body}");
        // 置き換えたタグは、単件の取得でも同じ内容になる (応答のエコーだけに頼らない)。
        let fetched = assert_status(client.get(&format!("/api/purchases/{id}")), 200);
        assert_eq!(
            fetched["flavor_notes"],
            json!(["shared-check"]),
            "{fetched}"
        );
        // 参照されなくなったタグは削除せず残す (FR-8)。
        let tags = assert_status(client.get("/api/flavor-tags"), 200);
        assert!(
            tags["flavor_tags"]
                .as_array()
                .expect("flavor_tags")
                .iter()
                .any(|tag| tag["name"] == "leftover-check"),
            "the unused tag must remain: {tags}"
        );

        // 空の配列でタグを外せる。商品の任意の項目は null で NULL にできる (FR-9)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({ "flavor_notes": [], "producer": null, "origin": null }),
            ),
            200,
        );
        assert_eq!(body["flavor_notes"], json!([]), "{body}");
        assert_eq!(body["producer"], Value::Null, "{body}");
        assert_eq!(body["origin"], Value::Null, "{body}");
        assert_eq!(body["region"], "イルガチェフェ", "{body}");
        assert_eq!(body["name"], "変更後の豆", "{body}");

        // 単件の取得でも同じ内容が返る。
        let fetched = assert_status(client.get(&format!("/api/purchases/{id}")), 200);
        assert_eq!(fetched, body);
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
                    "name": "入力の検査の豆",
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
                &json!({ "name": "入力の検査の豆", "purchased_on": D21 }),
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
            // 商品名は必須で、前後の空白を除いて空なら拒否する。タグの `null` と空白だけの名前も
            // 拒否する (FR-8、FR-9、ADR-0022)。
            json!({ "name": null }),
            json!({ "name": "" }),
            json!({ "name": "   " }),
            json!({ "flavor_notes": null }),
            json!({ "flavor_notes": ["  "] }),
            // 商品の ID は受け付けない (ADR-0022)。
            json!({ "product_id": null }),
            json!({ "product_id": "no-such-id" }),
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
            json!({ "photo_key": "purchases/photo.jpg" }),
            json!([1]),
        ] {
            assert_bad_request(client.patch_json(&format!("/api/purchases/{id}"), &body));
        }
        // 拒否した入力は保存されない。
        let body = assert_status(client.get(&format!("/api/purchases/{id}")), 200);
        assert_eq!(body["name"], "入力の検査の豆", "{body}");
        assert_eq!(body["flavor_notes"], json!([]), "{body}");
        assert_eq!(body["purchased_on"], D21, "{body}");
        assert_eq!(body["price_amount"], 1000, "{body}");
        assert_eq!(body["price_currency"], "JPY", "{body}");
    }

    #[test]
    fn wrangler_purchases_update_reference_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_write_session));
        let created = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({
                    "name": "参照の検査の豆",
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
            &json!({ "shop_id": "no-such-id" }),
        ));
        assert_not_found(client.patch_json(
            &format!("/api/purchases/{id}"),
            &json!({ "shop_id": data.other_shop.id }),
        ));
        // 参照先を変えない更新は通る。
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({
                    "shop_id": data.purchase_write_shop.id,
                    "roast": "参照はそのまま",
                }),
            ),
            200,
        );
        assert_eq!(body["roast"], "参照はそのまま", "{body}");
        // 別の有効な店への付け替えは 200 になり、新しい参照先が保存される (設計判断)。
        let new_shop = assert_status(
            client.post_json("/api/shops", &json!({ "name": "付け替え先の店" })),
            200,
        );
        let body = assert_status(
            client.patch_json(
                &format!("/api/purchases/{id}"),
                &json!({
                    "shop_id": new_shop["id"],
                    "roast": "付け替え後",
                }),
            ),
            200,
        );
        assert_eq!(body["shop_id"], new_shop["id"], "{body}");
        assert_eq!(body["shop"]["name"], "付け替え先の店", "{body}");
        // 拒否した入力は保存されず、付け替えた参照先が残る。
        let body = assert_status(client.get(&format!("/api/purchases/{id}")), 200);
        assert_eq!(body["shop_id"], new_shop["id"], "{body}");

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

    // 購入の削除 (0056)。

    #[test]
    fn wrangler_purchases_delete_ok_cascades_to_the_brews_and_the_photo() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.delete_purchase_session));

        // 削除の影響: 消える抽出 2 件 (購入は消えないため purchases は無い)。
        let impact = assert_status(
            client.get(&format!(
                "/api/purchases/{}/delete-impact",
                data.delete_purchase_purchase.id
            )),
            200,
        );
        assert_eq!(impact["brews"], 2, "{impact}");
        assert!(impact.get("purchases").is_none(), "{impact}");

        // 写真のオブジェクトを R2 に置いてから削除する。
        lease
            .use_server(|server| {
                server.put_r2_object(
                    "brewbook-photos",
                    &data.delete_purchase_photo_key,
                    b"photo-bytes",
                    "image/jpeg",
                )
            })
            .expect("the photo object must be put");

        // 削除は 204。購入と抽出が消え、写真のオブジェクトも消える。
        assert_status(
            client.delete(&format!(
                "/api/purchases/{}",
                data.delete_purchase_purchase.id
            )),
            204,
        );
        assert_not_found(client.get(&format!(
            "/api/purchases/{}",
            data.delete_purchase_purchase.id
        )));
        for brew in &data.delete_purchase_brews {
            assert_not_found(client.get(&format!("/api/brews/{}", brew.id)));
        }
        // 他の購入のタグの対応は残る (タグの行は消さない。ADR-0006、ADR-0022)。
        let other = assert_status(
            client.get(&format!(
                "/api/purchases/{}",
                data.delete_purchase_other_purchase.id
            )),
            200,
        );
        assert_eq!(other["flavor_notes"], json!(["shared"]), "{other}");
        let tags = assert_status(client.get("/api/flavor-tags"), 200);
        assert!(
            tags["flavor_tags"]
                .as_array()
                .expect("flavor_tags")
                .iter()
                .any(|tag| tag["name"] == "shared"),
            "the shared tag must remain: {tags}"
        );
        let object = lease
            .use_server(|server| {
                server.get_r2_object("brewbook-photos", &data.delete_purchase_photo_key)
            })
            .expect("the object must be readable");
        assert!(object.is_none(), "the photo object must be deleted");
    }

    #[test]
    fn wrangler_purchases_delete_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).delete(&format!(
            "/api/purchases/{}",
            data.delete_purchase_purchase.id
        )));
    }

    #[test]
    fn wrangler_purchases_delete_impact_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get(&format!(
            "/api/purchases/{}/delete-impact",
            data.delete_purchase_purchase.id
        )));
    }

    #[test]
    fn wrangler_purchases_delete_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.delete_purchase_session));
        // 他の利用者の購入は消えず、影響の取得も 404 (FR-5)。
        assert_not_found(client.delete(&format!("/api/purchases/{}", data.other_purchase.id)));
        assert_not_found(client.get(&format!(
            "/api/purchases/{}/delete-impact",
            data.other_purchase.id
        )));
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/purchases/{}", data.other_purchase.id)),
            200,
        );
        assert_eq!(other["id"], data.other_purchase.id, "{other}");
    }
}

mod brews {
    //! 抽出の経路のテスト (FR-11、FR-5)。

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
        // 応答には購入がネストし、その中に商品の項目と店が入る (FR-11、ADR-0022)。
        assert_eq!(
            brews[0]["purchase"]["id"], data.brew_list_purchases[0].id,
            "{body}"
        );
        assert_purchase_fields(
            &brews[0]["purchase"],
            &data.brew_list_purchases[0].name,
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
        // 古い抽出でも、購入とその商品の項目と店をたどれる。
        assert_eq!(
            brews[2]["purchase"]["id"], data.brew_list_old_brew.purchase_id,
            "{body}"
        );
        assert_purchase_fields(
            &brews[2]["purchase"],
            "抽出の豆",
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
        // 応答に archived_at は無い。
        for brew in brews {
            assert!(
                brew.get("archived_at").is_none(),
                "the response must not carry archived_at: {brew}"
            );
        }
        assert_eq!(body["next_cursor"], Value::Null);
        // include_archived は未知のパラメータとして無視され、挙動が変わらない。
        let with_parameter = assert_status(client.get("/api/brews?include_archived=true"), 200);
        assert_eq!(with_parameter, body, "{with_parameter}");
    }

    #[test]
    fn wrangler_brews_list_ok_with_a_cursor() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_list_session));

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
        for query in ["limit=0", "limit=abc", "limit=201", "cursor=x"] {
            assert_bad_request(client.get(&format!("/api/brews?{query}")));
        }
        // sort、order、favorite は受け付けない値を 400 にする (FR-20、FR-21)。
        for query in ["sort=unknown", "order=up", "favorite=yes", "favorite=1"] {
            assert_bad_request(client.get(&format!("/api/brews?{query}")));
        }
        // 受け取らないパラメータは未知のパラメータとして無視する (400 にしない)。
        assert_status(client.get("/api/brews?include_archived=1"), 200);
    }

    #[test]
    fn wrangler_brews_list_sorts_and_filters() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.brew_list_session));

        // 抽出日時の昇順で並ぶ (FR-20)。既定は抽出日時の降順。
        let asc = assert_status(client.get("/api/brews?sort=brewed_at&order=asc"), 200);
        let brews = asc["brews"].as_array().expect("brews");
        assert_eq!(brews[0]["id"], data.brew_list_old_brew.id, "{asc}");
        assert_eq!(brews[1]["id"], data.brew_list_brews[0].id, "{asc}");
        assert_eq!(brews[2]["id"], data.brew_list_brews[1].id, "{asc}");
        // 評価が未設定 (NULL) の行は末尾になる (FR-20)。3 件とも未設定のため ID の昇順。
        let nulls = assert_status(client.get("/api/brews?sort=rating&order=desc"), 200);
        let brews = nulls["brews"].as_array().expect("brews");
        assert_eq!(brews[0]["id"], data.brew_list_brews[0].id, "{nulls}");
        assert_eq!(brews[1]["id"], data.brew_list_brews[1].id, "{nulls}");
        assert_eq!(brews[2]["id"], data.brew_list_old_brew.id, "{nulls}");
        // 応答に favorited_at が含まれる (FR-21)。
        for brew in brews {
            assert!(brew.get("favorited_at").is_some(), "{brew}");
        }
    }

    #[test]
    fn wrangler_brews_favorite_put_and_delete() {
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
        let id = created["id"]
            .as_str()
            .expect("the id must be present")
            .to_owned();
        assert_eq!(created["favorited_at"], Value::Null, "{created}");

        let first = assert_status(client.put(&format!("/api/brews/{id}/favorite")), 200);
        assert!(first["favorited_at"].is_string(), "{first}");
        let favorited_at = first["favorited_at"].clone();
        let updated_at = first["updated_at"].clone();
        let again = assert_status(client.put(&format!("/api/brews/{id}/favorite")), 200);
        assert_eq!(again["favorited_at"], favorited_at, "{again}");
        assert_eq!(again["updated_at"], updated_at, "{again}");

        let removed = assert_status(client.delete(&format!("/api/brews/{id}/favorite")), 200);
        assert_eq!(removed["favorited_at"], Value::Null, "{removed}");
        let removed_updated_at = removed["updated_at"].clone();
        let again = assert_status(client.delete(&format!("/api/brews/{id}/favorite")), 200);
        assert_eq!(again["favorited_at"], Value::Null, "{again}");
        assert_eq!(again["updated_at"], removed_updated_at, "{again}");

        // お気に入りのみの一覧に反映される (FR-21)。
        assert_status(client.put(&format!("/api/brews/{id}/favorite")), 200);
        let body = assert_status(client.get("/api/brews?favorite=true"), 200);
        let brews = body["brews"].as_array().expect("brews");
        assert_eq!(brews.len(), 1, "{body}");
        assert_eq!(brews[0]["id"], id, "{body}");

        // 未認証は 401、他の利用者の ID は 404。
        assert_unauthorized(anonymous(&base_url).put(&format!("/api/brews/{id}/favorite")));
        assert_unauthorized(anonymous(&base_url).delete(&format!("/api/brews/{id}/favorite")));
        let other = ApiClient::new(&base_url, Some(&data.other_session));
        assert_not_found(other.put(&format!("/api/brews/{id}/favorite")));
        assert_not_found(other.delete(&format!("/api/brews/{id}/favorite")));
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
        assert!(body.get("archived_at").is_none(), "{body}");
        assert_timestamp(&body["created_at"]);
        assert_eq!(body["created_at"], body["updated_at"], "{body}");
        // 購入がネストし、その中に商品の項目と店が入る (FR-11、ADR-0022)。
        assert_eq!(
            body["purchase"]["name"], data.brew_write_purchase.name,
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
            // 購入以外の参照は受け付けない (ADR-0006、ADR-0022)。
            json!({ "purchase_id": purchase, "brewed_at": B21, "product_id": "no-such-id" }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "shop_id": "no-such-id" }),
            // 受け取らない項目は無視せず 400 にする (0007 の設計判断)。
            json!({ "purchase_id": purchase, "brewed_at": B21, "id": "no-such-id" }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "created_at": T21 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "updated_at": T21 }),
            json!({ "purchase_id": purchase, "brewed_at": B21, "user_id": data.brew_write_user }),
            json!([1]),
        ] {
            assert_bad_request(client.post_json("/api/brews", &body));
        }
    }

    #[test]
    fn wrangler_brews_create_reference_404() {
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
        // 他の利用者の購入は、その利用者からは参照できる (404 が利用者の違いによるものだと裏付ける)。
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/purchases/{}", data.other_purchase.id)),
            200,
        );
        assert_eq!(other["id"], data.other_purchase.id, "{other}");
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
        assert_purchase_fields(
            &body["purchase"],
            &data.brew_list_purchases[0].name,
            &data.brew_list_user,
            &["抽出のタグ"],
        );
        assert!(body.get("archived_at").is_none(), "{body}");
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
        assert!(body.get("archived_at").is_none(), "{body}");
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
    fn wrangler_brews_update_reference_404() {
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
        let new_purchase = assert_status(
            client.post_json(
                "/api/purchases",
                &json!({ "name": "付け替え先の購入の豆", "purchased_on": D21 }),
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
        assert_eq!(body["purchase"]["name"], "付け替え先の購入の豆", "{body}");
        // 拒否した入力は保存されない。
        let body = assert_status(client.get(&format!("/api/brews/{id}")), 200);
        assert_eq!(body["purchase_id"], new_purchase["id"], "{body}");

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

    // 抽出の削除 (0056)。

    #[test]
    fn wrangler_brews_delete_ok_removes_only_the_brew() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.delete_brew_session));
        let deleted = &data.delete_brew_brews[0];
        let kept = &data.delete_brew_brews[1];

        // 抽出は他から参照されないため、削除は 204 で行だけが消える。
        assert_status(client.delete(&format!("/api/brews/{}", deleted.id)), 204);
        assert_not_found(client.get(&format!("/api/brews/{}", deleted.id)));
        // 同じ購入の他の抽出と購入は残る。
        assert_status(client.get(&format!("/api/brews/{}", kept.id)), 200);
        assert_status(
            client.get(&format!("/api/purchases/{}", data.delete_brew_purchase.id)),
            200,
        );
    }

    #[test]
    fn wrangler_brews_delete_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(
            anonymous(&base_url).delete(&format!("/api/brews/{}", data.delete_brew_brews[0].id)),
        );
    }

    #[test]
    fn wrangler_brews_delete_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.delete_brew_session));
        // 他の利用者の抽出は消えない (FR-5)。
        assert_not_found(client.delete(&format!("/api/brews/{}", data.other_brew.id)));
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/brews/{}", data.other_brew.id)),
            200,
        );
        assert_eq!(other["id"], data.other_brew.id, "{other}");
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
                     AND name = 'idx_purchases_user_purchased_on'",
                )
            })
            .expect("the schema must be readable");
        assert_eq!(index, 1, "the purchases index must remain");
        // migration 0003 が archived_at を削除し、列を含まないインデックスを作り直す (ADR-0018)。
        let archived = lease
            .use_server(|server| {
                server.query_int(
                    "SELECT COUNT(*) AS count FROM pragma_table_info('purchases') \
                     WHERE name = 'archived_at'",
                )
            })
            .expect("the schema must be readable");
        assert_eq!(archived, 0, "the purchases table must not have archived_at");
    }
}
