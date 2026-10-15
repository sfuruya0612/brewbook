//! 店と Flavor Notes のタグの API の結合テスト (HTTP)。
//!
//! 一覧と単件の取得、登録、更新、入力の検証、他の利用者の記録の 404、
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
use support::seed::{user_id, Seed, SeededPurchase, SeededShop};
use support::ServerLease;

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const T21: &str = "2026-09-21T00:00:00.000Z";
const T20: &str = "2026-09-20T00:00:00.000Z";

/// 下ごしらえに使う日付。
const D21: &str = "2026-09-21";
const D20: &str = "2026-09-20";

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// 一覧のテストの利用者 (有効な店 2 件が同じ作成日時、古い店 1 件)。
    list_user: String,
    list_session: String,
    /// 同じ作成日時の 2 件。ID の昇順で返ることを確かめる (作った順に ID が増える)。
    list_shops: Vec<SeededShop>,
    list_old_shop: SeededShop,
    /// 店の書き換えのテストの利用者。
    shop_write_user: String,
    shop_write_session: String,
    /// タグの一覧のテストの利用者 (購入 2 件がタグを共有する)。
    tag_list_user: String,
    tag_list_session: String,
    /// 他の利用者 (404 の検査に使う)。
    other_user: String,
    other_session: String,
    other_shop: SeededShop,
    /// 記録が無い利用者のセッション。
    empty_session: String,
    /// 記録の内容のログの検査に使う利用者のセッション。
    log_session: String,
    /// お気に入りの絞り込みのテスト専用の利用者のセッション。
    favorite_filter_session: String,
    /// 店の削除のテストの利用者 (0056)。店 1 件と、その店を指定する購入 2 件を持つ。
    delete_shop_session: String,
    delete_shop_shop: SeededShop,
    delete_shop_purchases: Vec<SeededPurchase>,
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
        seed.shop(&list_user, "新しい店", Some("東京都"), T21, T21),
        seed.shop(&list_user, "同じ時刻の店", None, T21, T21),
        seed.shop(&list_user, "古い店", None, T20, T20),
    ];
    let list_old_shop = list_shops[2].clone();

    // 店の書き換えのテストの利用者。
    let shop_write_user = user_id(2);
    seed.user(&shop_write_user, "shop write user", created);
    let shop_write_session = seed.session(&shop_write_user, future, created);

    // タグの一覧のテストの利用者。タグは 2 つの購入で共有し、参照されないタグも 1 つ置く
    // (ADR-0022)。
    let tag_list_user = user_id(3);
    seed.user(&tag_list_user, "tag list user", created);
    let tag_list_session = seed.session(&tag_list_user, future, created);
    let tag_list_purchases = [
        seed.purchase(&tag_list_user, "新しい豆", None, D21, T21, T21),
        seed.purchase(&tag_list_user, "古い豆", None, D20, T20, T20),
    ];
    let chocolate = seed.flavor_tag(&tag_list_user, "chocolate");
    let berry = seed.flavor_tag(&tag_list_user, "berry");
    seed.flavor_tag(&tag_list_user, "vanilla");
    seed.purchase_flavor_tag(&tag_list_user, &tag_list_purchases[0].id, &chocolate);
    seed.purchase_flavor_tag(&tag_list_user, &tag_list_purchases[0].id, &berry);
    seed.purchase_flavor_tag(&tag_list_user, &tag_list_purchases[1].id, &chocolate);

    // 他の利用者。店と購入とタグを 1 つずつ持つ。
    let other_user = user_id(5);
    seed.user(&other_user, "other user", created);
    let other_session = seed.session(&other_user, future, created);
    let other_shop = seed.shop(&other_user, "他人の店", None, T21, T21);
    let other_purchase = seed.purchase(&other_user, "他人の豆", None, D21, T21, T21);
    let other_tag = seed.flavor_tag(&other_user, "other tag");
    seed.purchase_flavor_tag(&other_user, &other_purchase.id, &other_tag);

    // 記録が無い利用者。
    let empty_user = user_id(6);
    seed.user(&empty_user, "empty user", created);
    let empty_session = seed.session(&empty_user, future, created);

    // ログの検査に使う利用者。
    let log_user = user_id(7);
    seed.user(&log_user, "log user", created);
    let log_session = seed.session(&log_user, future, created);

    // お気に入りの絞り込みのテスト専用の利用者 (並列実行の競合を避けるため、他のテストと
    // 共有しない)。
    let favorite_filter_user = user_id(20);
    seed.user(&favorite_filter_user, "favorite filter user", created);
    let favorite_filter_session = seed.session(&favorite_filter_user, future, created);

    // 店の削除のテストの利用者 (0056)。店 1 件と、その店を指定する購入 2 件を持つ。
    let delete_shop_user = user_id(21);
    seed.user(&delete_shop_user, "delete shop user", created);
    let delete_shop_session = seed.session(&delete_shop_user, future, created);
    let delete_shop_shop = seed.shop(&delete_shop_user, "削除する店", Some("東京都"), T21, T21);
    let delete_shop_purchases = vec![
        seed.purchase(
            &delete_shop_user,
            "削除の店の豆",
            Some(&delete_shop_shop.id),
            D21,
            T21,
            T21,
        ),
        seed.purchase(
            &delete_shop_user,
            "削除の店の豆",
            Some(&delete_shop_shop.id),
            D20,
            T20,
            T20,
        ),
    ];

    TestData {
        seed_sql: seed.sql(),
        list_user,
        list_session,
        list_shops,
        list_old_shop,
        shop_write_user,
        shop_write_session,
        tag_list_user,
        tag_list_session,
        other_user,
        other_session,
        other_shop,
        empty_session,
        log_session,
        favorite_filter_session,
        delete_shop_session,
        delete_shop_shop,
        delete_shop_purchases,
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
    //! 店の経路のテスト (FR-6、FR-5)。

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
        assert_eq!(body["next_cursor"], Value::Null);
        // 応答に archived_at は無い。
        for shop in shops {
            assert!(
                shop.get("archived_at").is_none(),
                "the response must not carry archived_at: {shop}"
            );
        }
        // include_archived は未知のパラメータとして無視され、挙動が変わらない。
        let with_parameter = assert_status(client.get("/api/shops?include_archived=true"), 200);
        assert_eq!(with_parameter, body, "{with_parameter}");
    }

    #[test]
    fn wrangler_shops_list_ok_with_a_cursor() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.list_session));

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
            "cursor=not-base64url!",
            "cursor=",
        ] {
            assert_bad_request(client.get(&format!("/api/shops?{query}")));
        }
        // 200 ぴったりは受け付ける。
        assert_status(client.get("/api/shops?limit=200"), 200);
        // sort、order、favorite は受け付けない値を 400 にする (FR-20、FR-21)。
        for query in ["sort=unknown", "order=up", "favorite=yes", "favorite=1"] {
            assert_bad_request(client.get(&format!("/api/shops?{query}")));
        }
        // 受け取らないパラメータは未知のパラメータとして無視する (400 にしない)。
        for query in ["include_archived=yes", "include_archived=", "unknown=1"] {
            assert_status(client.get(&format!("/api/shops?{query}")), 200);
        }
    }

    #[test]
    fn wrangler_shops_list_sorts_and_filters() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.list_session));

        // 名前の昇順と降順で並ぶ (FR-20)。既定は作成日時の降順。
        let asc = assert_status(client.get("/api/shops?sort=name&order=asc"), 200);
        let names: Vec<String> = asc["shops"]
            .as_array()
            .expect("shops")
            .iter()
            .filter_map(|shop| shop["name"].as_str().map(str::to_owned))
            .collect();
        let mut expected = names.clone();
        expected.sort();
        assert_eq!(names, expected, "{asc}");
        let desc = assert_status(client.get("/api/shops?sort=name&order=desc"), 200);
        let desc_names: Vec<String> = desc["shops"]
            .as_array()
            .expect("shops")
            .iter()
            .filter_map(|shop| shop["name"].as_str().map(str::to_owned))
            .collect();
        expected.reverse();
        assert_eq!(desc_names, expected, "{desc}");
        // 応答に favorited_at が含まれる (FR-21)。
        for shop in asc["shops"].as_array().expect("shops") {
            assert!(shop.get("favorited_at").is_some(), "{shop}");
        }
    }

    #[test]
    fn wrangler_shops_list_pages_with_a_sort_key() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.list_session));

        // 名前の昇順で 2 件ずつ引く。重複も漏れも無い。
        let mut ids: Vec<String> = Vec::new();
        let first = assert_status(client.get("/api/shops?sort=name&order=asc&limit=2"), 200);
        for shop in first["shops"].as_array().expect("shops") {
            ids.push(
                shop["id"]
                    .as_str()
                    .expect("the id must be present")
                    .to_owned(),
            );
        }
        let cursor = first["next_cursor"]
            .as_str()
            .expect("a full page must carry the next cursor")
            .to_owned();
        let second = assert_status(
            client.get(&format!(
                "/api/shops?sort=name&order=asc&limit=2&cursor={cursor}"
            )),
            200,
        );
        for shop in second["shops"].as_array().expect("shops") {
            ids.push(
                shop["id"]
                    .as_str()
                    .expect("the id must be present")
                    .to_owned(),
            );
        }
        assert_eq!(second["next_cursor"], Value::Null, "{second}");
        assert_eq!(ids.len(), 3, "{first} {second}");
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 3, "the pages must not repeat a row");
        // カーソルと違う並び順の組は 400 (重複や漏れを返さないため)。
        assert_bad_request(client.get(&format!(
            "/api/shops?sort=name&order=desc&limit=2&cursor={cursor}"
        )));
    }

    #[test]
    fn wrangler_shops_favorite_put_and_delete() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.shop_write_session));
        let created = assert_status(
            client.post_json(
                "/api/shops",
                &json!({ "name": "お気に入りの店", "address": null }),
            ),
            200,
        );
        let id = created["id"]
            .as_str()
            .expect("the id must be present")
            .to_owned();
        assert_eq!(created["favorited_at"], Value::Null, "{created}");

        let first = assert_status(client.put(&format!("/api/shops/{id}/favorite")), 200);
        assert!(first["favorited_at"].is_string(), "{first}");
        let favorited_at = first["favorited_at"].clone();
        let updated_at = first["updated_at"].clone();
        // 同じ状態の繰り返しは 200 で、favorited_at と updated_at を変えない (FR-21)。
        let again = assert_status(client.put(&format!("/api/shops/{id}/favorite")), 200);
        assert_eq!(again["favorited_at"], favorited_at, "{again}");
        assert_eq!(again["updated_at"], updated_at, "{again}");

        let removed = assert_status(client.delete(&format!("/api/shops/{id}/favorite")), 200);
        assert_eq!(removed["favorited_at"], Value::Null, "{removed}");
        let removed_updated_at = removed["updated_at"].clone();
        let again = assert_status(client.delete(&format!("/api/shops/{id}/favorite")), 200);
        assert_eq!(again["favorited_at"], Value::Null, "{again}");
        assert_eq!(again["updated_at"], removed_updated_at, "{again}");

        // 未認証は 401、他の利用者の ID は 404。
        assert_unauthorized(anonymous(&base_url).put(&format!("/api/shops/{id}/favorite")));
        assert_unauthorized(anonymous(&base_url).delete(&format!("/api/shops/{id}/favorite")));
        let other = ApiClient::new(&base_url, Some(&data.other_session));
        assert_not_found(other.put(&format!("/api/shops/{id}/favorite")));
        assert_not_found(other.delete(&format!("/api/shops/{id}/favorite")));
    }

    #[test]
    fn wrangler_shops_list_filters_by_favorite() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        // このテスト専用の利用者を使う (並列実行の競合を避ける)。
        let client = ApiClient::new(&base_url, Some(&data.favorite_filter_session));
        let created = assert_status(
            client.post_json(
                "/api/shops",
                &json!({ "name": "絞り込みの店", "address": null }),
            ),
            200,
        );
        let id = created["id"]
            .as_str()
            .expect("the id must be present")
            .to_owned();
        // この利用者にはお気に入りがまだ無い。
        let body = assert_status(client.get("/api/shops?favorite=true"), 200);
        assert_eq!(body["shops"], json!([]), "{body}");
        assert_status(client.put(&format!("/api/shops/{id}/favorite")), 200);
        let body = assert_status(client.get("/api/shops?favorite=true"), 200);
        let shops = body["shops"].as_array().expect("shops");
        assert_eq!(shops.len(), 1, "{body}");
        assert_eq!(shops[0]["id"], id, "{body}");
        assert!(shops[0]["favorited_at"].is_string(), "{body}");
        // 単件取得と更新の応答にも favorited_at が含まれる (FR-21)。
        let detail = assert_status(client.get(&format!("/api/shops/{id}")), 200);
        assert!(detail["favorited_at"].is_string(), "{detail}");
        let updated = assert_status(
            client.patch_json(
                &format!("/api/shops/{id}"),
                &json!({ "name": "絞り込みの店 2" }),
            ),
            200,
        );
        assert!(updated["favorited_at"].is_string(), "{updated}");
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
        assert!(body.get("archived_at").is_none(), "{body}");
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
        assert!(body.get("archived_at").is_none(), "{body}");
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
        assert!(body.get("archived_at").is_none(), "{body}");
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

    // 店の削除 (0056)。

    #[test]
    fn wrangler_shops_delete_ok_and_keeps_the_purchases() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.delete_shop_session));

        // 削除の影響: 店の指定が外れる購入の件数 (抽出は消えないため brews は無い)。
        let impact = assert_status(
            client.get(&format!(
                "/api/shops/{}/delete-impact",
                data.delete_shop_shop.id
            )),
            200,
        );
        assert_eq!(impact["purchases"], 2, "{impact}");
        assert!(impact.get("brews").is_none(), "{impact}");

        // 削除は 204。店は消え、購入は残って店の指定だけが外れる。
        assert_status(
            client.delete(&format!("/api/shops/{}", data.delete_shop_shop.id)),
            204,
        );
        assert_not_found(client.get(&format!("/api/shops/{}", data.delete_shop_shop.id)));
        for purchase in &data.delete_shop_purchases {
            let body = assert_status(client.get(&format!("/api/purchases/{}", purchase.id)), 200);
            assert_eq!(body["shop_id"], Value::Null, "{body}");
            assert_eq!(body["shop"], Value::Null, "{body}");
        }
        // 削除の後の影響の取得は 404。
        assert_not_found(client.get(&format!(
            "/api/shops/{}/delete-impact",
            data.delete_shop_shop.id
        )));
    }

    #[test]
    fn wrangler_shops_delete_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(
            anonymous(&base_url).delete(&format!("/api/shops/{}", data.delete_shop_shop.id)),
        );
    }

    #[test]
    fn wrangler_shops_delete_impact_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get(&format!(
            "/api/shops/{}/delete-impact",
            data.delete_shop_shop.id
        )));
    }

    #[test]
    fn wrangler_shops_delete_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.delete_shop_session));
        // 他の利用者の店は消えず、影響の取得も 404 (FR-5)。
        assert_not_found(client.delete(&format!("/api/shops/{}", data.other_shop.id)));
        assert_not_found(client.get(&format!("/api/shops/{}/delete-impact", data.other_shop.id)));
        let other = assert_status(
            ApiClient::new(&base_url, Some(&data.other_session))
                .get(&format!("/api/shops/{}", data.other_shop.id)),
            200,
        );
        assert_eq!(other["name"], data.other_shop.name);
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
            ApiClient::new(&base_url, Some(&data.tag_list_session)).get("/api/flavor-tags"),
            200,
        );
        let tags = body["flavor_tags"]
            .as_array()
            .expect("the response must have flavor_tags");
        // 名前の昇順で返り、どの購入からも参照されていないタグも残る (FR-8、ADR-0022)。
        let names: Vec<&str> = tags.iter().filter_map(|tag| tag["name"].as_str()).collect();
        assert_eq!(names, vec!["berry", "chocolate", "vanilla"], "{body}");
        assert_eq!(tags[0]["user_id"], data.tag_list_user);
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
        let route_line = "\"route\":\"purchases_create\"";
        let before = lease.use_server(|server| {
            server
                .output()
                .iter()
                .filter(|line| line.contains(route_line))
                .count()
        });
        // 購入の商品名に印を付けて登録し、その値がログの行に現れないことを確認する (ADR-0022)。
        let marker = "MARKER-PURCHASE-NAME-7f3a";
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.log_session)).post_json(
                "/api/purchases",
                &json!({
                    "name": marker,
                    "purchased_on": D21,
                    "producer": "MARKER-PRODUCER-7f3a",
                }),
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
