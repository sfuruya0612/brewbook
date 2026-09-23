//! 店と商品と Flavor Notes のタグの API の結合テスト (HTTP)。
//!
//! 一覧と単件の取得、登録、更新、アーカイブと解除、入力の検証、他の利用者の記録の 404、
//! タグの共有と残存、カーソルの並び順を検査する。記録の内容がログに出ないことも確認する。
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
use support::seed::{user_id, Seed, SeededProduct, SeededShop};
use support::ServerLease;

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const T21: &str = "2026-09-21T00:00:00.000Z";
const T20: &str = "2026-09-20T00:00:00.000Z";
const T19: &str = "2026-09-19T00:00:00.000Z";

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// 一覧のテストの利用者 (有効な店 2 件が同じ作成日時、古い店 1 件、アーカイブ済みの店 1 件)。
    list_user: String,
    list_session: String,
    /// 同じ作成日時の 2 件。ID の昇順で返ることを確かめる (作った順に ID が増える)。
    list_shops: Vec<SeededShop>,
    list_old_shop: SeededShop,
    list_archived_shop: SeededShop,
    /// 店の書き換えのテストの利用者。
    shop_write_user: String,
    shop_write_session: String,
    /// 商品の一覧のテストの利用者 (有効な商品 2 件とアーカイブ済みの商品 1 件)。
    product_list_user: String,
    product_list_session: String,
    product_list_products: Vec<SeededProduct>,
    product_list_archived: SeededProduct,
    /// 商品の書き換えのテストの利用者。
    product_write_user: String,
    product_write_session: String,
    /// 他の利用者 (404 の検査に使う)。
    other_user: String,
    other_session: String,
    other_shop: SeededShop,
    other_product: SeededProduct,
    /// 記録が無い利用者のセッション。
    empty_session: String,
    /// 記録の内容のログの検査に使う利用者のセッション。
    log_session: String,
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

    // 一覧のテストの利用者。同じ作成日時の 2 件は、先に作った方の ID が小さい。
    let list_user = user_id(1);
    seed.user(&list_user, "list user", created);
    let list_session = seed.session(&list_user, future, created);
    let list_shops = vec![
        seed.shop(&list_user, "新しい店", Some("東京都"), T21, T21, None),
        seed.shop(&list_user, "同じ時刻の店", None, T21, T21, None),
        seed.shop(&list_user, "古い店", None, T20, T20, None),
    ];
    let list_old_shop = list_shops[2].clone();
    let list_archived_shop = seed.shop(&list_user, "しまった店", None, T19, T19, Some(T19));

    // 店の書き換えのテストの利用者。
    let shop_write_user = user_id(2);
    seed.user(&shop_write_user, "shop write user", created);
    let shop_write_session = seed.session(&shop_write_user, future, created);

    // 商品の一覧のテストの利用者。タグは 2 つの商品で共有し、参照されないタグも 1 つ置く。
    let product_list_user = user_id(3);
    seed.user(&product_list_user, "product list user", created);
    let product_list_session = seed.session(&product_list_user, future, created);
    let product_list_products = vec![
        seed.product(&product_list_user, "新しい豆", T21, T21, None),
        seed.product(&product_list_user, "古い豆", T20, T20, None),
    ];
    let product_list_archived = seed.product(&product_list_user, "しまった豆", T19, T19, Some(T19));
    let chocolate = seed.flavor_tag(&product_list_user, "chocolate");
    let berry = seed.flavor_tag(&product_list_user, "berry");
    seed.flavor_tag(&product_list_user, "vanilla");
    seed.product_flavor_tag(&product_list_user, &product_list_products[0].id, &chocolate);
    seed.product_flavor_tag(&product_list_user, &product_list_products[0].id, &berry);
    seed.product_flavor_tag(&product_list_user, &product_list_products[1].id, &chocolate);

    // 商品の書き換えのテストの利用者。
    let product_write_user = user_id(4);
    seed.user(&product_write_user, "product write user", created);
    let product_write_session = seed.session(&product_write_user, future, created);

    // 他の利用者。店と商品とタグを 1 つずつ持つ。
    let other_user = user_id(5);
    seed.user(&other_user, "other user", created);
    let other_session = seed.session(&other_user, future, created);
    let other_shop = seed.shop(&other_user, "他人の店", None, T21, T21, None);
    let other_product = seed.product(&other_user, "他人の豆", T21, T21, None);
    let other_tag = seed.flavor_tag(&other_user, "other tag");
    seed.product_flavor_tag(&other_user, &other_product.id, &other_tag);

    // 記録が無い利用者。
    let empty_user = user_id(6);
    seed.user(&empty_user, "empty user", created);
    let empty_session = seed.session(&empty_user, future, created);

    // ログの検査に使う利用者。
    let log_user = user_id(7);
    seed.user(&log_user, "log user", created);
    let log_session = seed.session(&log_user, future, created);

    TestData {
        seed_sql: seed.sql(),
        list_user,
        list_session,
        list_shops,
        list_old_shop,
        list_archived_shop,
        shop_write_user,
        shop_write_session,
        product_list_user,
        product_list_session,
        product_list_products,
        product_list_archived,
        product_write_user,
        product_write_session,
        other_user,
        other_session,
        other_shop,
        other_product,
        empty_session,
        log_session,
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
        value
            .as_str()
            .is_some_and(|text| text.len() == 24 && text.ends_with('Z')),
        "the value must be an ISO 8601 UTC timestamp: {value}"
    );
}

/// 店の応答を確かめる。
fn assert_shop(body: &Value, user_id: &str, name: &str, address: Option<&str>) {
    assert_eq!(body["name"].as_str(), Some(name), "{body}");
    assert_eq!(body["user_id"].as_str(), Some(user_id), "{body}");
    assert_eq!(
        body["address"].as_str(),
        address,
        "the address must round trip: {body}"
    );
    assert_timestamp(&body["created_at"]);
    assert_timestamp(&body["updated_at"]);
    assert_eq!(body["created_at"], body["updated_at"], "{body}");
}

/// ミリ秒だけ待つ。`updated_at` が確実に進むようにする。
fn sleep_millis(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}

mod shops {
    //! 店の経路のテスト (FR-6、FR-12、FR-5)。

    use super::*;

    // 店の一覧 (認証が必要、クエリパラメータあり)。

    #[test]
    fn wrangler_shops_list_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.list_session));

        let body = assert_status(client.get("/api/shops"), 200);
        let shops = body["shops"]
            .as_array()
            .expect("the response must have shops");
        // 作成日時の降順と、同じ日時のときの ID の昇順で返る。
        assert_eq!(shops.len(), 3, "{body}");
        assert_eq!(shops[0]["id"], data.list_shops[0].id);
        assert_eq!(shops[1]["id"], data.list_shops[1].id);
        assert_eq!(shops[2]["id"], data.list_old_shop.id);
        assert!(shops[0]["id"].as_str() < shops[1]["id"].as_str(), "{body}");
        // 一覧は呼び出した利用者の記録だけを返す (FR-5)。
        assert!(
            shops.iter().all(|shop| shop["user_id"] == data.list_user),
            "the list must carry only the caller's records: {body}"
        );
        // 既定では、アーカイブ済みの店は含まれない (FR-12)。
        assert!(
            !shops
                .iter()
                .any(|shop| shop["id"] == data.list_archived_shop.id),
            "the archived shop must not be listed: {body}"
        );
        assert_eq!(body["next_cursor"], Value::Null);
    }

    #[test]
    fn wrangler_shops_list_ok_with_include_archived_and_a_cursor() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.list_session));

        // アーカイブ済みを含めると、アーカイブ済みの店も返る (FR-12)。
        let body = assert_status(client.get("/api/shops?include_archived=true"), 200);
        let shops = body["shops"].as_array().expect("shops");
        assert_eq!(shops.len(), 4, "{body}");
        assert_eq!(shops[3]["id"], data.list_archived_shop.id);
        assert_timestamp(&shops[3]["archived_at"]);

        // カーソルで続きを引く。同じ行が重複せず、並び順が保たれる。
        let first = assert_status(client.get("/api/shops?limit=2"), 200);
        let first_shops = first["shops"].as_array().expect("shops");
        assert_eq!(first_shops.len(), 2, "{first}");
        let cursor = first["next_cursor"]
            .as_str()
            .expect("a full page must carry the next cursor")
            .to_owned();
        let second = assert_status(
            client.get(&format!("/api/shops?limit=2&cursor={cursor}")),
            200,
        );
        let second_shops = second["shops"].as_array().expect("shops");
        assert_eq!(second_shops.len(), 1, "{second}");
        assert_eq!(second_shops[0]["id"], data.list_old_shop.id);
        assert_eq!(second["next_cursor"], Value::Null);
    }

    #[test]
    fn wrangler_shops_list_returns_only_own_records() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        // 他の利用者の店は、その利用者の一覧に出るが、こちらの一覧には出ない (FR-5)。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session)).get("/api/shops"),
            200,
        );
        let ids: Vec<&str> = body["shops"]
            .as_array()
            .expect("shops")
            .iter()
            .filter_map(|shop| shop["id"].as_str())
            .collect();
        assert_eq!(ids, vec![data.other_shop.id.as_str()], "{body}");

        // 記録が無い利用者の一覧は空になる。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.empty_session)).get("/api/shops"),
            200,
        );
        assert_eq!(body["shops"], json!([]), "{body}");
        assert_eq!(body["next_cursor"], Value::Null);
    }

    #[test]
    fn wrangler_shops_list_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.list_session));
        // limit は 0 以下、整数でない値、200 を超える値を受け付けない (PRD の性能)。
        for query in [
            "limit=0",
            "limit=-1",
            "limit=1.5",
            "limit=abc",
            "limit=201",
            "limit=999999999999",
            "limit=",
            "include_archived=yes",
            "include_archived=",
            "cursor=not-base64url!",
            "cursor=",
        ] {
            assert_bad_request(client.get(&format!("/api/shops?{query}")));
        }
        // 200 ぴったりは受け付ける。
        assert_status(client.get("/api/shops?limit=200"), 200);
    }

    #[test]
    fn wrangler_shops_list_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get("/api/shops"));
    }

    // 店の登録 (認証が必要、入力あり)。

    #[test]
    fn wrangler_shops_create_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.shop_write_session));
        let body = assert_status(
            client.post_json(
                "/api/shops",
                &json!({ "name": "  はじめての店  ", "address": "  住所  " }),
            ),
            200,
        );
        // 前後の空白は除いて保存する (FR-6)。
        assert_shop(&body, &data.shop_write_user, "はじめての店", Some("住所"));
        assert_eq!(body["archived_at"], Value::Null);
        let id = body["id"].as_str().expect("the id must be present");
        assert_eq!(id.len(), 36, "the id must be a UUID: {id}");
        assert_eq!(&id[14..15], "4", "the id must be UUID v4: {id}");
        assert!(
            matches!(&id[19..20], "8" | "9" | "a" | "b"),
            "the id must be UUID v4: {id}"
        );

        // 登録した店は単件で取得できる。
        let fetched = assert_status(client.get(&format!("/api/shops/{id}")), 200);
        assert_eq!(fetched, body);

        // 住所は省略でき、NULL で保存できる (FR-6)。
        let body = assert_status(
            client.post_json("/api/shops", &json!({ "name": "住所なしの店" })),
            200,
        );
        assert_eq!(body["address"], Value::Null, "{body}");
        let body = assert_status(
            client.post_json(
                "/api/shops",
                &json!({ "name": "住所を空にした店", "address": null }),
            ),
            200,
        );
        assert_eq!(body["address"], Value::Null, "{body}");
    }

    #[test]
    fn wrangler_shops_create_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.shop_write_session));
        // 店名は必須で、前後の空白を除いて空なら拒否する (FR-6)。
        for body in [
            json!({}),
            json!({ "name": "" }),
            json!({ "name": "   " }),
            json!({ "name": "\u{3000}" }),
            json!({ "name": null }),
            json!({ "address": "住所" }),
            json!({ "name": "店", "unknown": 1 }),
            json!([1, 2]),
        ] {
            assert_bad_request(client.post_json("/api/shops", &body));
        }
    }

    #[test]
    fn wrangler_shops_create_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post_json("/api/shops", &json!({ "name": "店" })));
    }

    // 店の単件の取得 (認証が必要、入力なし)。

    #[test]
    fn wrangler_shops_get_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.list_session));
        let body = assert_status(
            client.get(&format!("/api/shops/{}", data.list_shops[0].id)),
            200,
        );
        assert_eq!(body["id"], data.list_shops[0].id);
        assert_eq!(body["name"], data.list_shops[0].name);
        // アーカイブ済みでも単件では返す (FR-12)。
        let body = assert_status(
            client.get(&format!("/api/shops/{}", data.list_archived_shop.id)),
            200,
        );
        assert_timestamp(&body["archived_at"]);
    }

    #[test]
    fn wrangler_shops_get_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(
            anonymous(&base_url).get(&format!("/api/shops/{}", data.list_shops[0].id)),
        );
    }

    #[test]
    fn wrangler_shops_get_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.list_session));
        // 他の利用者の ID と存在しない ID は区別せず 404 を返す (FR-5)。
        assert_not_found(client.get(&format!("/api/shops/{}", data.other_shop.id)));
        assert_not_found(client.get("/api/shops/no-such-id"));
    }

    // 店の更新 (認証が必要、入力あり)。

    #[test]
    fn wrangler_shops_update_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.shop_write_session));
        let created = assert_status(
            client.post_json(
                "/api/shops",
                &json!({ "name": "変更前の店", "address": "変更前の住所" }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        let created_at = created["created_at"].clone();
        sleep_millis(10);

        // 店名と住所を更新する (FR-6)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/shops/{id}"),
                &json!({ "name": "  変更後の店  ", "address": "  変更後の住所  " }),
            ),
            200,
        );
        assert_eq!(body["id"], created["id"]);
        assert_eq!(body["name"], "変更後の店", "{body}");
        assert_eq!(body["address"], "変更後の住所", "{body}");
        assert_eq!(body["created_at"], created_at, "created_at must not change");
        assert!(
            body["updated_at"].as_str() > created_at.as_str(),
            "updated_at must advance: {body}"
        );

        // 住所だけを NULL にできる (FR-6)。
        let body = assert_status(
            client.patch_json(&format!("/api/shops/{id}"), &json!({ "address": null })),
            200,
        );
        assert_eq!(body["address"], Value::Null, "{body}");
        assert_eq!(body["name"], "変更後の店", "{body}");

        // 無い項目は変更しない。
        let body = assert_status(
            client.patch_json(&format!("/api/shops/{id}"), &json!({})),
            200,
        );
        assert_eq!(body["name"], "変更後の店", "{body}");
        assert_eq!(body["address"], Value::Null, "{body}");

        // 他の利用者の店は変わっていない。
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/shops/{}", data.other_shop.id)),
            200,
        );
        assert_eq!(other["name"], data.other_shop.name);
    }

    #[test]
    fn wrangler_shops_update_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.shop_write_session));
        let created = assert_status(
            client.post_json("/api/shops", &json!({ "name": "入力の検査の店" })),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        for body in [
            json!({ "name": "" }),
            json!({ "name": "   " }),
            json!({ "name": null }),
            json!({ "created_at": T21 }),
            json!({ "archived_at": T21 }),
            json!([1]),
        ] {
            assert_bad_request(client.patch_json(&format!("/api/shops/{id}"), &body));
        }
        // 拒否した入力は保存されない。
        let body = assert_status(client.get(&format!("/api/shops/{id}")), 200);
        assert_eq!(body["name"], "入力の検査の店", "{body}");
    }

    #[test]
    fn wrangler_shops_update_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).patch_json(
            &format!("/api/shops/{}", data.list_shops[0].id),
            &json!({ "name": "店" }),
        ));
    }

    #[test]
    fn wrangler_shops_update_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_not_found(
            ApiClient::new(&base_url, Some(&data.list_session)).patch_json(
                &format!("/api/shops/{}", data.other_shop.id),
                &json!({ "name": "のっとり" }),
            ),
        );
        assert_not_found(
            ApiClient::new(&base_url, Some(&data.list_session))
                .patch_json("/api/shops/no-such-id", &json!({ "name": "のっとり" })),
        );
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/shops/{}", data.other_shop.id)),
            200,
        );
        assert_eq!(other["name"], data.other_shop.name);
    }

    // 店のアーカイブと解除 (認証が必要、入力なし)。

    #[test]
    fn wrangler_shops_archive_and_unarchive_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.shop_write_session));
        let created = assert_status(
            client.post_json("/api/shops", &json!({ "name": "アーカイブの店" })),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        sleep_millis(10);

        let body = assert_status(client.post(&format!("/api/shops/{id}/archive")), 200);
        assert_timestamp(&body["archived_at"]);
        assert!(
            body["updated_at"].as_str() > created["updated_at"].as_str(),
            "updated_at must advance on archive: {body}"
        );
        // アーカイブした店は既定の一覧に含まれない (FR-12)。
        let list = assert_status(client.get("/api/shops"), 200);
        assert!(
            !list["shops"]
                .as_array()
                .expect("shops")
                .iter()
                .any(|shop| shop["id"] == created["id"]),
            "the archived shop must not be listed: {list}"
        );
        // アーカイブ済みの店は include_archived で取得でき、単件でも取得できる (FR-12)。
        let list = assert_status(client.get("/api/shops?include_archived=true"), 200);
        assert!(
            list["shops"]
                .as_array()
                .expect("shops")
                .iter()
                .any(|shop| shop["id"] == created["id"]),
            "the archived shop must be listed: {list}"
        );
        assert_status(client.get(&format!("/api/shops/{id}")), 200);

        // 繰り返しのアーカイブも 200 を返す (同じ状態への遷移はエラーにしない)。
        assert_status(client.post(&format!("/api/shops/{id}/archive")), 200);

        // アーカイブ済みの店も更新できる (単件の取得と同じくアーカイブ済みを引くため)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/shops/{id}"),
                &json!({ "name": "名前だけ変更" }),
            ),
            200,
        );
        assert_eq!(body["name"], "名前だけ変更", "{body}");
        assert!(
            body["archived_at"].as_str().is_some(),
            "the archived_at must be kept: {body}"
        );

        // アーカイブ解除で既定の一覧に戻る (FR-12)。
        let body = assert_status(client.post(&format!("/api/shops/{id}/unarchive")), 200);
        assert_eq!(body["archived_at"], Value::Null, "{body}");
        let list = assert_status(client.get("/api/shops"), 200);
        assert!(
            list["shops"]
                .as_array()
                .expect("shops")
                .iter()
                .any(|shop| shop["id"] == created["id"]),
            "the unarchived shop must be listed: {list}"
        );
        // 繰り返しのアーカイブ解除も 200 を返す。
        assert_status(client.post(&format!("/api/shops/{id}/unarchive")), 200);
    }

    #[test]
    fn wrangler_shops_archive_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(
            anonymous(&base_url).post(&format!("/api/shops/{}/archive", data.list_shops[0].id)),
        );
    }

    #[test]
    fn wrangler_shops_unarchive_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(
            anonymous(&base_url).post(&format!("/api/shops/{}/unarchive", data.list_shops[0].id)),
        );
    }

    #[test]
    fn wrangler_shops_archive_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.list_session));
        assert_not_found(client.post(&format!("/api/shops/{}/archive", data.other_shop.id)));
        assert_not_found(client.post(&format!("/api/shops/{}/unarchive", data.other_shop.id)));
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/shops/{}", data.other_shop.id)),
            200,
        );
        assert_eq!(other["archived_at"], Value::Null);
    }

    #[test]
    fn wrangler_shops_unarchive_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.list_session));
        assert_not_found(client.post("/api/shops/no-such-id/unarchive"));
    }
}

mod products {
    //! 商品の経路のテスト (FR-7、FR-8、FR-12、FR-5)。

    use super::*;

    // 商品の一覧 (認証が必要、クエリパラメータあり)。

    #[test]
    fn wrangler_products_list_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_list_session));
        let body = assert_status(client.get("/api/products"), 200);
        let products = body["products"]
            .as_array()
            .expect("the response must have products");
        assert_eq!(products.len(), 2, "{body}");
        // 作成日時の降順で返る。
        assert_eq!(products[0]["id"], data.product_list_products[0].id);
        assert_eq!(products[1]["id"], data.product_list_products[1].id);
        // 2 つの商品が同じタグを共有し、取得結果にタグ名の配列が含まれる (FR-8)。
        assert_eq!(products[0]["flavor_notes"], json!(["berry", "chocolate"]));
        assert_eq!(products[1]["flavor_notes"], json!(["chocolate"]));
        assert_eq!(products[0]["producer"], Value::Null, "{body}");
        // アーカイブ済みの商品は既定の一覧に含まれない (FR-12)。
        assert!(
            !products
                .iter()
                .any(|product| product["id"] == data.product_list_archived.id),
            "the archived product must not be listed: {body}"
        );
        assert_eq!(body["next_cursor"], Value::Null);

        // include_archived ではアーカイブ済みも返る (FR-12)。
        let body = assert_status(client.get("/api/products?include_archived=true"), 200);
        let products = body["products"].as_array().expect("products");
        assert_eq!(products.len(), 3, "{body}");
        assert_eq!(products[2]["id"], data.product_list_archived.id);
        assert_timestamp(&products[2]["archived_at"]);
        assert_eq!(products[2]["flavor_notes"], json!([]));
    }

    #[test]
    fn wrangler_products_list_ok_with_a_cursor() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_list_session));
        // カーソルで続きを引く。同じ行が重複せず、並び順が保たれる。
        let first = assert_status(client.get("/api/products?limit=1"), 200);
        let products = first["products"].as_array().expect("products");
        assert_eq!(products.len(), 1, "{first}");
        assert_eq!(products[0]["id"], data.product_list_products[0].id);
        let cursor = first["next_cursor"]
            .as_str()
            .expect("a full page must carry the next cursor")
            .to_owned();
        let second = assert_status(
            client.get(&format!("/api/products?limit=1&cursor={cursor}")),
            200,
        );
        let products = second["products"].as_array().expect("products");
        assert_eq!(products.len(), 1, "{second}");
        assert_eq!(products[0]["id"], data.product_list_products[1].id);
        let cursor = second["next_cursor"]
            .as_str()
            .expect("a full page must carry the next cursor")
            .to_owned();
        // 続きの行が無いページは空になり、続きのカーソルも無くなる。
        let third = assert_status(
            client.get(&format!("/api/products?limit=1&cursor={cursor}")),
            200,
        );
        assert_eq!(third["products"], json!([]), "{third}");
        assert_eq!(third["next_cursor"], Value::Null);
    }

    #[test]
    fn wrangler_products_list_returns_only_own_records() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session)).get("/api/products"),
            200,
        );
        let products = body["products"].as_array().expect("products");
        assert_eq!(products.len(), 1, "{body}");
        assert_eq!(products[0]["id"], data.other_product.id);
        assert_eq!(products[0]["flavor_notes"], json!(["other tag"]));

        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.empty_session)).get("/api/products"),
            200,
        );
        assert_eq!(body["products"], json!([]), "{body}");
    }

    #[test]
    fn wrangler_products_list_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_list_session));
        for query in [
            "limit=0",
            "limit=abc",
            "limit=201",
            "include_archived=1",
            "cursor=x",
        ] {
            assert_bad_request(client.get(&format!("/api/products?{query}")));
        }
    }

    #[test]
    fn wrangler_products_list_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get("/api/products"));
    }

    // 商品の登録 (認証が必要、入力あり)。

    #[test]
    fn wrangler_products_create_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_write_session));

        // 商品名以外の項目は NULL で登録できる (FR-7)。
        let body = assert_status(
            client.post_json("/api/products", &json!({ "name": "  名前だけの豆  " })),
            200,
        );
        assert_eq!(body["name"], "名前だけの豆", "{body}");
        assert_eq!(body["user_id"], data.product_write_user);
        for field in ["producer", "origin", "region", "process", "variety"] {
            assert_eq!(body[field], Value::Null, "{field}: {body}");
        }
        assert_eq!(body["flavor_notes"], json!([]), "{body}");
        assert_eq!(body["archived_at"], Value::Null);
        assert_timestamp(&body["created_at"]);
        assert_eq!(body["created_at"], body["updated_at"], "{body}");
        let id = body["id"].as_str().expect("the id must be present");
        assert_eq!(id.len(), 36, "the id must be a UUID: {id}");

        // 登録した商品は単件で取得できる。
        let fetched = assert_status(client.get(&format!("/api/products/{id}")), 200);
        assert_eq!(fetched, body);

        // 全項目とタグを指定して登録する。自由記述は前後の空白を除き、タグは重複を除いて共有する (FR-7、FR-8)。
        let body = assert_status(
            client.post_json(
                "/api/products",
                &json!({
                    "name": "  全部入りの豆  ",
                    "producer": "  生産者  ",
                    "origin": "  エチオピア  ",
                    "region": "  イルガチェフェ  ",
                    "process": "  ウォッシュト  ",
                    "variety": "  在来種  ",
                    "flavor_notes": ["  チョコ  ", "berry", "チョコ", " berry "],
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
    }

    #[test]
    fn wrangler_products_create_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_write_session));
        // 商品名は必須で、前後の空白を除いて空なら拒否する (FR-7)。
        for body in [
            json!({}),
            json!({ "name": "" }),
            json!({ "name": "   " }),
            json!({ "name": null }),
            json!({ "producer": "生産者" }),
            json!({ "name": "豆", "unknown": 1 }),
            // 空白だけのタグ名は拒否する (FR-8)。
            json!({ "name": "豆", "flavor_notes": ["チョコ", "  "] }),
            json!({ "name": "豆", "flavor_notes": [""] }),
            // タグの配列でない値と `null` は拒否する (タグを付けないときは項目を省くか空の配列)。
            json!({ "name": "豆", "flavor_notes": "チョコ" }),
            json!({ "name": "豆", "flavor_notes": [1] }),
            json!({ "name": "豆", "flavor_notes": null }),
        ] {
            assert_bad_request(client.post_json("/api/products", &body));
        }
    }

    #[test]
    fn wrangler_products_create_ok_with_many_flavor_notes() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_write_session));
        // タグの置き換えは 1 つのまとまり (D1 の batch) で行う。多数のタグでも通ることを確認する。
        let names: Vec<String> = (0..60).map(|index| format!("tag-{index:02}")).collect();
        let body = assert_status(
            client.post_json(
                "/api/products",
                &json!({ "name": "タグが多い豆", "flavor_notes": names }),
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
    fn wrangler_products_create_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(
            anonymous(&base_url).post_json("/api/products", &json!({ "name": "豆" })),
        );
    }

    // 商品の単件の取得 (認証が必要、入力なし)。

    #[test]
    fn wrangler_products_get_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_list_session));
        let body = assert_status(
            client.get(&format!(
                "/api/products/{}",
                data.product_list_products[0].id
            )),
            200,
        );
        assert_eq!(body["id"], data.product_list_products[0].id);
        assert_eq!(body["name"], data.product_list_products[0].name);
        assert_eq!(body["flavor_notes"], json!(["berry", "chocolate"]));

        // アーカイブ済みでも単件では返す (FR-12)。
        let body = assert_status(
            client.get(&format!("/api/products/{}", data.product_list_archived.id)),
            200,
        );
        assert_timestamp(&body["archived_at"]);
    }

    #[test]
    fn wrangler_products_get_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get(&format!(
            "/api/products/{}",
            data.product_list_products[0].id
        )));
    }

    #[test]
    fn wrangler_products_get_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_list_session));
        assert_not_found(client.get(&format!("/api/products/{}", data.other_product.id)));
        assert_not_found(client.get("/api/products/no-such-id"));
    }

    // 商品の更新 (認証が必要、入力あり)。

    #[test]
    fn wrangler_products_update_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_write_session));
        let created = assert_status(
            client.post_json(
                "/api/products",
                &json!({
                    "name": "変更前の豆",
                    "producer": "変更前の生産者",
                    "flavor_notes": ["leftover-check", "shared-check"],
                }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        sleep_millis(10);

        // 全項目を更新する (FR-7)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/products/{id}"),
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
        // タグを指定しない更新ではタグは変わらない。
        assert_eq!(
            body["flavor_notes"],
            json!(["leftover-check", "shared-check"]),
            "{body}"
        );

        // タグは配列で置き換える (FR-8)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/products/{id}"),
                &json!({ "flavor_notes": ["shared-check"] }),
            ),
            200,
        );
        assert_eq!(body["flavor_notes"], json!(["shared-check"]), "{body}");
        // 置き換えたタグは、単件の取得でも同じ内容になる (応答のエコーだけに頼らない)。
        let fetched = assert_status(client.get(&format!("/api/products/{id}")), 200);
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

        // 空の配列でタグを外せる。任意の項目は null で NULL にできる (FR-7)。
        let body = assert_status(
            client.patch_json(
                &format!("/api/products/{id}"),
                &json!({ "flavor_notes": [], "producer": null, "origin": null }),
            ),
            200,
        );
        assert_eq!(body["flavor_notes"], json!([]), "{body}");
        assert_eq!(body["producer"], Value::Null, "{body}");
        assert_eq!(body["origin"], Value::Null, "{body}");
        assert_eq!(body["region"], "イルガチェフェ", "{body}");

        // 単件の取得でも同じ内容が返る。
        let fetched = assert_status(client.get(&format!("/api/products/{id}")), 200);
        assert_eq!(fetched, body);
    }

    #[test]
    fn wrangler_products_update_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_write_session));
        let created = assert_status(
            client.post_json("/api/products", &json!({ "name": "入力の検査の豆" })),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        for body in [
            json!({ "name": "" }),
            json!({ "name": "   " }),
            json!({ "name": null }),
            json!({ "flavor_notes": null }),
            json!({ "flavor_notes": ["  "] }),
            json!({ "id": "no-such-id" }),
            json!({ "created_at": T21 }),
            json!({ "archived_at": T21 }),
            json!([1]),
        ] {
            assert_bad_request(client.patch_json(&format!("/api/products/{id}"), &body));
        }
        let body = assert_status(client.get(&format!("/api/products/{id}")), 200);
        assert_eq!(body["name"], "入力の検査の豆", "{body}");
        assert_eq!(body["flavor_notes"], json!([]), "{body}");
    }

    #[test]
    fn wrangler_products_update_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).patch_json(
            &format!("/api/products/{}", data.product_list_products[0].id),
            &json!({ "name": "豆" }),
        ));
    }

    #[test]
    fn wrangler_products_update_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_not_found(
            ApiClient::new(&base_url, Some(&data.product_list_session)).patch_json(
                &format!("/api/products/{}", data.other_product.id),
                &json!({ "name": "のっとり" }),
            ),
        );
        assert_not_found(
            ApiClient::new(&base_url, Some(&data.product_list_session))
                .patch_json("/api/products/no-such-id", &json!({ "name": "のっとり" })),
        );
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/products/{}", data.other_product.id)),
            200,
        );
        assert_eq!(other["name"], data.other_product.name);
    }

    // 商品のアーカイブと解除 (認証が必要、入力なし)。

    #[test]
    fn wrangler_products_archive_and_unarchive_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_write_session));
        let created = assert_status(
            client.post_json(
                "/api/products",
                &json!({ "name": "アーカイブの豆", "flavor_notes": ["berry"] }),
            ),
            200,
        );
        let id = created["id"].as_str().expect("the id must be present");
        sleep_millis(10);

        let body = assert_status(client.post(&format!("/api/products/{id}/archive")), 200);
        assert_timestamp(&body["archived_at"]);
        assert!(
            body["updated_at"].as_str() > created["updated_at"].as_str(),
            "updated_at must advance on archive: {body}"
        );
        // アーカイブしてもタグは残り、商品からたどれる (FR-12)。
        assert_eq!(body["flavor_notes"], json!(["berry"]), "{body}");
        // アーカイブした商品は既定の一覧に含まれない (FR-12)。
        let list = assert_status(client.get("/api/products"), 200);
        assert!(
            !list["products"]
                .as_array()
                .expect("products")
                .iter()
                .any(|product| product["id"] == created["id"]),
            "the archived product must not be listed: {list}"
        );
        assert_status(client.get(&format!("/api/products/{id}")), 200);
        // 繰り返しのアーカイブも 200 を返す。
        assert_status(client.post(&format!("/api/products/{id}/archive")), 200);

        // アーカイブ解除で既定の一覧に戻る (FR-12)。
        let body = assert_status(client.post(&format!("/api/products/{id}/unarchive")), 200);
        assert_eq!(body["archived_at"], Value::Null, "{body}");
        let list = assert_status(client.get("/api/products"), 200);
        assert!(
            list["products"]
                .as_array()
                .expect("products")
                .iter()
                .any(|product| product["id"] == created["id"]),
            "the unarchived product must be listed: {list}"
        );
        assert_status(client.post(&format!("/api/products/{id}/unarchive")), 200);
    }

    #[test]
    fn wrangler_products_archive_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post(&format!(
            "/api/products/{}/archive",
            data.product_list_products[0].id
        )));
    }

    #[test]
    fn wrangler_products_unarchive_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post(&format!(
            "/api/products/{}/unarchive",
            data.product_list_products[0].id
        )));
    }

    #[test]
    fn wrangler_products_archive_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.product_list_session));
        assert_not_found(client.post(&format!("/api/products/{}/archive", data.other_product.id)));
        assert_not_found(client.post(&format!(
            "/api/products/{}/unarchive",
            data.other_product.id
        )));
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/products/{}", data.other_product.id)),
            200,
        );
        assert_eq!(other["archived_at"], Value::Null);
    }

    #[test]
    fn wrangler_products_unarchive_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_not_found(
            ApiClient::new(&base_url, Some(&data.product_list_session))
                .post("/api/products/no-such-id/unarchive"),
        );
    }
}

mod flavor_tags {
    //! Flavor Notes のタグの一覧のテスト (FR-8、FR-5)。

    use super::*;

    // Flavor Notes のタグの一覧 (認証が必要、入力なし)。

    #[test]
    fn wrangler_flavor_tags_list_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.product_list_session)).get("/api/flavor-tags"),
            200,
        );
        let tags = body["flavor_tags"]
            .as_array()
            .expect("the response must have flavor_tags");
        // 名前の昇順で返り、どの商品からも参照されていないタグも残る (FR-8)。
        let names: Vec<&str> = tags.iter().filter_map(|tag| tag["name"].as_str()).collect();
        assert_eq!(names, vec!["berry", "chocolate", "vanilla"], "{body}");
        assert_eq!(tags[0]["user_id"], data.product_list_user);
        let id = tags[0]["id"].as_str().expect("the tag must carry its id");
        assert_eq!(id.len(), 36, "the tag id must be a UUID: {id}");

        // 他の利用者のタグは返らない (FR-5)。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session)).get("/api/flavor-tags"),
            200,
        );
        let names: Vec<&str> = body["flavor_tags"]
            .as_array()
            .expect("flavor_tags")
            .iter()
            .filter_map(|tag| tag["name"].as_str())
            .collect();
        assert_eq!(names, vec!["other tag"], "{body}");
        assert_eq!(
            body["flavor_tags"][0]["user_id"], data.other_user,
            "the list must carry only the caller's tags: {body}"
        );

        // 記録が無い利用者には空の配列を返す。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.empty_session)).get("/api/flavor-tags"),
            200,
        );
        assert_eq!(body["flavor_tags"], json!([]), "{body}");
    }

    #[test]
    fn wrangler_flavor_tags_list_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get("/api/flavor-tags"));
    }
}

mod logging {
    //! 記録の内容がログに出ないことのテスト (PRD の運用)。

    use super::*;

    // 運用 (PRD の運用): 記録の内容をログに出さない。

    /// このテスト専用のサーバー。共有のサーバーは他のテストのログも出すため、
    /// このリクエストの行を数で特定できるよう専用にする。
    fn log_server() -> ServerLease {
        support::shared_server("logging", || {
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

    #[test]
    fn wrangler_records_do_not_appear_in_the_log() {
        let data = data();
        let lease = log_server();
        let base_url = lease.use_server(|server| server.base_url());
        // このリクエストの前の行数を記録し、行数が増えるまで待つ (専用のサーバーなので他のテストの行は混ざらない)。
        let route_line = "\"route\":\"products_create\"";
        let before = lease.use_server(|server| {
            server
                .output()
                .iter()
                .filter(|line| line.contains(route_line))
                .count()
        });
        // 商品名に印を付けて登録し、その値がログの行に現れないことを確認する。
        let marker = "MARKER-PRODUCT-NAME-7f3a";
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.log_session)).post_json(
                "/api/products",
                &json!({ "name": marker, "producer": "MARKER-PRODUCER-7f3a" }),
            ),
            200,
        );
        assert_eq!(body["name"], marker);
        let appeared = lease.use_server(|server| {
            let deadline = std::time::Instant::now() + Duration::from_secs(15);
            loop {
                let count = server
                    .output()
                    .iter()
                    .filter(|line| line.contains(route_line))
                    .count();
                if count > before {
                    return true;
                }
                if std::time::Instant::now() >= deadline {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        assert!(appeared, "the request log line must appear");
        sleep_millis(500);
        let leaked: Vec<String> = lease.use_server(|server| {
            server
                .output()
                .into_iter()
                .filter(|line| line.contains(marker) || line.contains("MARKER-PRODUCER-7f3a"))
                .collect()
        });
        assert!(
            leaked.is_empty(),
            "the record content must not appear in the log: {leaked:?}"
        );
    }
}
