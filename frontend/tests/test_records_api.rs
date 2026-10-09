//! `records::api` と `records::upload` の単体テスト (0041)。
//!
//! 経路、クエリ、本文、応答の型への変換を、偽の送信の実装で確かめる。記録の登録、閲覧、編集
//! (完了条件 1) と、写真のアップロードの 3 回の呼び出し (FR-10)、サジェスト (FR-13)、
//! 推測 (FR-19) を覆う。

mod support;

use std::rc::Rc;

use brew_book_frontend::api::Method;
use brew_book_frontend::records::{
    inputs::{BrewInput, ProductInput, PurchaseInput, ShopInput},
    ConvertedImage, ListOptions, PhotoUpload, PhotoUploader, RecordError, RecordsApi, SortOrder,
    SuggestionTarget,
};
use serde_json::{json, Value};

use support::{block_on, client, FakeUploadTransport};

/// 店の応答の JSON。
fn shop_json(id: &str, name: &str) -> Value {
    json!({
        "id": id,
        "user_id": "user",
        "name": name,
        "address": null,
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
    })
}

/// 商品の応答の JSON。
fn product_json(id: &str, name: &str) -> Value {
    json!({
        "id": id,
        "user_id": "user",
        "name": name,
        "producer": null,
        "origin": null,
        "region": null,
        "process": null,
        "variety": null,
        "flavor_notes": [],
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
    })
}

/// 購入の応答の JSON (商品と店をネストする。FR-9)。
fn purchase_json(id: &str, product_id: &str, shop_id: Option<&str>) -> Value {
    json!({
        "id": id,
        "user_id": "user",
        "product_id": product_id,
        "shop_id": shop_id,
        "purchased_on": "2026-10-01",
        "roast": null,
        "roast_date": null,
        "price_amount": null,
        "price_currency": null,
        "weight_grams": null,
        "photo_key": null,
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
        "product": product_json(product_id, "豆"),
        "shop": shop_id.map(|id| shop_json(id, "店")),
    })
}

/// 抽出の応答の JSON (購入をネストする。FR-11)。
fn brew_json(id: &str, purchase_id: &str) -> Value {
    json!({
        "id": id,
        "user_id": "user",
        "purchase_id": purchase_id,
        "brewed_at": "2026-10-01T09:00:00.000Z",
        "dose_grams": 15.0,
        "water_grams": null,
        "water_temp_c": null,
        "brew_time_seconds": null,
        "method": null,
        "grind_setting": null,
        "rating": 4,
        "notes": null,
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
        "purchase": purchase_json(purchase_id, "product", None),
    })
}

/// 応答を 1 つだけ返す API クライアントを作る。
fn with_response(body: Value) -> (RecordsApi, Rc<support::FakeTransport>) {
    let (client, transport) = client(vec![support::FakeTransport::response(
        200,
        &body.to_string(),
    )]);
    (RecordsApi::new(client), transport)
}

#[test]
fn the_shops_list_sends_the_paging_query_and_reads_the_page() {
    let (api, transport) =
        with_response(json!({"shops": [shop_json("s1", "店")], "next_cursor": "cur"}));
    let page =
        block_on(api.shops(&ListOptions::new("created_at"), None)).expect("the page must be read");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].name, "店");
    assert_eq!(page.next_cursor.as_deref(), Some("cur"));
    let request = transport.last_request();
    assert_eq!(request.method, Method::Get);
    assert_eq!(
        request.path,
        "/api/shops?limit=50&sort=created_at&order=desc"
    );

    let (api, transport) = with_response(json!({"shops": [], "next_cursor": null}));
    let page = block_on(api.shops(&ListOptions::new("created_at"), Some("cur")))
        .expect("the page must be read");
    assert!(page.items.is_empty());
    assert_eq!(page.next_cursor, None);
    assert_eq!(
        transport.last_request().path,
        "/api/shops?limit=50&sort=created_at&order=desc&cursor=cur"
    );
}

#[test]
fn a_shop_is_read_created_and_updated() {
    let (api, transport) = with_response(shop_json("s1", "店"));
    let shop = block_on(api.shop("s1")).expect("the shop must be read");
    assert_eq!(shop.id, "s1");
    assert_eq!(transport.last_request().path, "/api/shops/s1");

    let (api, transport) = with_response(shop_json("s1", "店"));
    let input = ShopInput {
        name: "店".to_string(),
        address: Some("住所".to_string()),
    };
    let _ = block_on(api.create_shop(&input)).expect("the shop must be created");
    let request = transport.last_request();
    assert_eq!(request.method, Method::Post);
    assert_eq!(request.path, "/api/shops");
    let body: Value = serde_json::from_slice(&request.body).expect("the body must be JSON");
    assert_eq!(body, json!({"name": "店", "address": "住所"}));

    let (api, transport) = with_response(shop_json("s1", "店"));
    let _ = block_on(api.update_shop("s1", &input)).expect("the shop must be updated");
    let request = transport.last_request();
    assert_eq!(request.method, Method::Patch);
    assert_eq!(request.path, "/api/shops/s1");
}

#[test]
fn the_products_list_sends_the_name_filter_and_reads_the_page() {
    let (api, transport) =
        with_response(json!({"products": [product_json("p1", "豆")], "next_cursor": null}));
    let page = block_on(api.products(&ListOptions::new("name"), None, Some("豆 山")))
        .expect("the page must be read");
    assert_eq!(page.items[0].name, "豆");
    assert_eq!(
        transport.last_request().path,
        "/api/products?limit=50&sort=name&order=desc&name=%E8%B1%86%20%E5%B1%B1"
    );
}

/// 並び順と方向とお気に入りの絞り込みが、そのままクエリパラメータになる (FR-20、FR-21)。
#[test]
fn the_list_options_are_sent_as_the_query_parameters() {
    let (api, transport) = with_response(json!({"products": [], "next_cursor": null}));
    let options = ListOptions {
        sort: "updated_at",
        order: SortOrder::Asc,
        favorite_only: true,
    };
    let _ = block_on(api.products(&options, None, None)).expect("the page must be read");
    assert_eq!(
        transport.last_request().path,
        "/api/products?limit=50&sort=updated_at&order=asc&favorite=true"
    );

    // 並び順を指定しない (選択のシート) ときは `sort` を送らず、API の既定に任せる。
    let (api, transport) = with_response(json!({"purchases": [], "next_cursor": null}));
    let _ = block_on(api.purchases(&ListOptions::default(), None)).expect("the page must be read");
    assert_eq!(
        transport.last_request().path,
        "/api/purchases?limit=50&order=desc"
    );
}

#[test]
fn a_product_is_read_created_and_updated() {
    let (api, transport) = with_response(product_json("p1", "豆"));
    let product = block_on(api.product("p1")).expect("the product must be read");
    assert_eq!(product.id, "p1");
    assert_eq!(transport.last_request().path, "/api/products/p1");

    let input = ProductInput {
        name: "豆".to_string(),
        producer: Some("生産者".to_string()),
        origin: None,
        region: None,
        process: None,
        variety: None,
        flavor_notes: vec!["甘い".to_string()],
    };
    let (api, transport) = with_response(product_json("p1", "豆"));
    let _ = block_on(api.create_product(&input)).expect("the product must be created");
    let body: Value =
        serde_json::from_slice(&transport.last_request().body).expect("the body must be JSON");
    assert_eq!(
        body,
        json!({
            "name": "豆",
            "producer": "生産者",
            "origin": null,
            "region": null,
            "process": null,
            "variety": null,
            "flavor_notes": ["甘い"],
        })
    );

    let (api, transport) = with_response(product_json("p1", "豆"));
    let _ = block_on(api.update_product("p1", &input)).expect("the product must be updated");
    let request = transport.last_request();
    assert_eq!(request.method, Method::Patch);
    assert_eq!(request.path, "/api/products/p1");
}

#[test]
fn a_purchase_is_read_created_and_updated() {
    let (api, transport) = with_response(purchase_json("b1", "p1", Some("s1")));
    let purchase = block_on(api.purchase("b1")).expect("the purchase must be read");
    assert_eq!(purchase.product.name, "豆");
    assert_eq!(
        purchase.shop.as_ref().map(|shop| shop.name.as_str()),
        Some("店")
    );
    assert_eq!(transport.last_request().path, "/api/purchases/b1");

    let input = PurchaseInput {
        product_id: "p1".to_string(),
        shop_id: None,
        purchased_on: "2026-10-01".to_string(),
        roast: None,
        roast_date: None,
        price_amount: Some(1200),
        price_currency: Some("JPY".to_string()),
        weight_grams: Some(200),
    };
    let (api, transport) = with_response(purchase_json("b1", "p1", None));
    let _ = block_on(api.create_purchase(&input)).expect("the purchase must be created");
    let request = transport.last_request();
    assert_eq!(request.path, "/api/purchases");
    let body: Value = serde_json::from_slice(&request.body).expect("the body must be JSON");
    assert_eq!(body["price_amount"], json!(1200));
    assert_eq!(body["price_currency"], json!("JPY"));
    assert_eq!(body["weight_grams"], json!(200));

    let (api, transport) = with_response(purchase_json("b1", "p1", None));
    let _ = block_on(api.update_purchase("b1", &input)).expect("the purchase must be updated");
    assert_eq!(transport.last_request().path, "/api/purchases/b1");
}

#[test]
fn the_purchases_and_brews_lists_send_the_paging_query() {
    let (api, transport) =
        with_response(json!({"purchases": [purchase_json("b1", "p1", None)], "next_cursor": null}));
    let page = block_on(api.purchases(&ListOptions::new("purchased_on"), None))
        .expect("the page must be read");
    assert_eq!(page.items.len(), 1);
    assert_eq!(
        transport.last_request().path,
        "/api/purchases?limit=50&sort=purchased_on&order=desc"
    );

    let (api, transport) =
        with_response(json!({"brews": [brew_json("w1", "b1")], "next_cursor": "c"}));
    let page =
        block_on(api.brews(&ListOptions::new("brewed_at"), None)).expect("the page must be read");
    assert_eq!(page.items[0].purchase.product.name, "豆");
    assert_eq!(page.next_cursor.as_deref(), Some("c"));
    assert_eq!(
        transport.last_request().path,
        "/api/brews?limit=50&sort=brewed_at&order=desc"
    );
}

/// お気に入りの付け外しが `PUT` と `DELETE` の経路になり、応答を読む (FR-21)。
#[test]
fn the_favorites_are_put_and_deleted() {
    let (api, transport) = with_response(shop_json("s1", "店"));
    let shop = block_on(api.set_shop_favorite("s1", true)).expect("the favorite must be set");
    assert_eq!(shop.id, "s1");
    let request = transport.last_request();
    assert_eq!(request.method, Method::Put);
    assert_eq!(request.path, "/api/shops/s1/favorite");

    let (api, transport) = with_response(shop_json("s1", "店"));
    let _ = block_on(api.set_shop_favorite("s1", false)).expect("the favorite must be removed");
    let request = transport.last_request();
    assert_eq!(request.method, Method::Delete);
    assert_eq!(request.path, "/api/shops/s1/favorite");

    let (api, transport) = with_response(product_json("p1", "豆"));
    let _ = block_on(api.set_product_favorite("p1", true)).expect("the favorite must be set");
    assert_eq!(transport.last_request().path, "/api/products/p1/favorite");

    let (api, transport) = with_response(purchase_json("b1", "p1", None));
    let _ = block_on(api.set_purchase_favorite("b1", true)).expect("the favorite must be set");
    assert_eq!(transport.last_request().path, "/api/purchases/b1/favorite");

    let (api, transport) = with_response(brew_json("w1", "b1"));
    let _ = block_on(api.set_brew_favorite("w1", false)).expect("the favorite must be removed");
    let request = transport.last_request();
    assert_eq!(request.method, Method::Delete);
    assert_eq!(request.path, "/api/brews/w1/favorite");
}

/// 応答の `favorited_at` を読み、無い場合と null は未設定にする (FR-21)。
#[test]
fn the_favorited_at_is_read_from_the_records_and_the_nested_records() {
    let mut shop = shop_json("s1", "店");
    shop["favorited_at"] = json!("2026-10-05T00:00:00.000Z");
    let (api, _) = with_response(shop);
    let shop = block_on(api.shop("s1")).expect("the shop must be read");
    assert_eq!(
        shop.favorited_at.as_deref(),
        Some("2026-10-05T00:00:00.000Z")
    );

    // 入れ子の商品と店にも `favorited_at` が入る (FR-9、FR-11)。
    let mut purchase = purchase_json("b1", "p1", Some("s1"));
    purchase["favorited_at"] = json!(null);
    purchase["product"]["favorited_at"] = json!("2026-10-05T00:00:00.000Z");
    purchase["shop"]["favorited_at"] = json!("2026-10-05T00:00:00.000Z");
    let (api, _) = with_response(purchase);
    let purchase = block_on(api.purchase("b1")).expect("the purchase must be read");
    assert_eq!(purchase.favorited_at, None);
    assert_eq!(
        purchase.product.favorited_at.as_deref(),
        Some("2026-10-05T00:00:00.000Z")
    );
    assert_eq!(
        purchase
            .shop
            .as_ref()
            .and_then(|shop| shop.favorited_at.as_deref()),
        Some("2026-10-05T00:00:00.000Z")
    );
}

#[test]
fn a_brew_is_read_created_and_updated() {
    let (api, transport) = with_response(brew_json("w1", "b1"));
    let brew = block_on(api.brew("w1")).expect("the brew must be read");
    assert_eq!(brew.id, "w1");
    assert_eq!(brew.rating, Some(4));
    assert_eq!(transport.last_request().path, "/api/brews/w1");

    let input = BrewInput {
        purchase_id: "b1".to_string(),
        brewed_at: "2026-10-01T09:00:00.000Z".to_string(),
        dose_grams: Some(15.0),
        water_grams: None,
        water_temp_c: None,
        brew_time_seconds: None,
        method: None,
        grind_setting: None,
        rating: Some(4),
        notes: None,
    };
    let (api, transport) = with_response(brew_json("w1", "b1"));
    let _ = block_on(api.create_brew(&input)).expect("the brew must be created");
    let request = transport.last_request();
    assert_eq!(request.path, "/api/brews");
    let body: Value = serde_json::from_slice(&request.body).expect("the body must be JSON");
    assert_eq!(body["brewed_at"], json!("2026-10-01T09:00:00.000Z"));
    assert_eq!(body["rating"], json!(4));

    let (api, transport) = with_response(brew_json("w1", "b1"));
    let _ = block_on(api.update_brew("w1", &input)).expect("the brew must be updated");
    assert_eq!(transport.last_request().path, "/api/brews/w1");
}

#[test]
fn the_photo_urls_are_requested_completed_and_deleted() {
    let (api, transport) =
        with_response(json!({"url": "https://r2.example/put", "key": "pending/u/1.jpg"}));
    let target =
        block_on(api.request_photo_upload_url("b1", 1234)).expect("the upload url must be issued");
    assert_eq!(target.url, "https://r2.example/put");
    assert_eq!(target.key, "pending/u/1.jpg");
    let request = transport.last_request();
    assert_eq!(request.path, "/api/purchases/b1/photo/upload-url");
    let body: Value = serde_json::from_slice(&request.body).expect("the body must be JSON");
    assert_eq!(body, json!({"size": 1234}));

    let (api, transport) = with_response(purchase_json("b1", "p1", None));
    let _ = block_on(api.complete_photo("b1", "pending/u/1.jpg", 1234))
        .expect("the completion must be accepted");
    let request = transport.last_request();
    assert_eq!(request.path, "/api/purchases/b1/photo");
    let body: Value = serde_json::from_slice(&request.body).expect("the body must be JSON");
    assert_eq!(body, json!({"key": "pending/u/1.jpg", "size": 1234}));

    let (api, transport) = with_response(purchase_json("b1", "p1", None));
    let _ = block_on(api.delete_photo("b1")).expect("the photo must be deleted");
    let request = transport.last_request();
    assert_eq!(request.method, Method::Delete);
    assert_eq!(request.path, "/api/purchases/b1/photo");
}

#[test]
fn the_photo_get_url_is_built_from_the_base_path() {
    let (api, _) = with_response(json!({}));
    assert_eq!(api.photo_url("b1"), "/api/purchases/b1/photo");
}

#[test]
fn the_purchase_suggestion_sends_the_jpeg_and_reads_the_response() {
    let (api, transport) = with_response(json!({
        "product": {"name": "豆", "flavor_notes": ["甘い"]},
        "roast": "中煎り",
        "price_amount": 1200,
    }));
    let suggestion =
        block_on(api.suggest_purchase(&[1, 2, 3])).expect("the suggestion must be read");
    assert_eq!(
        suggestion
            .product
            .as_ref()
            .and_then(|product| product.name.as_deref()),
        Some("豆")
    );
    assert_eq!(suggestion.roast.as_deref(), Some("中煎り"));
    assert_eq!(suggestion.price_amount, Some(1200));
    let request = transport.last_request();
    assert_eq!(request.method, Method::Post);
    assert_eq!(request.path, "/api/purchase-suggestions");
    assert_eq!(request.content_type.as_deref(), Some("image/jpeg"));
    assert_eq!(request.body, vec![1, 2, 3]);
}

#[test]
fn the_suggestions_query_is_encoded_and_the_values_are_read() {
    let (api, transport) = with_response(json!({"values": ["中煎り", "深煎り"]}));
    let values = block_on(api.suggestions(SuggestionTarget::Roast, "中 煎"))
        .expect("the suggestions must be read");
    assert_eq!(values, vec!["中煎り".to_string(), "深煎り".to_string()]);
    assert_eq!(
        transport.last_request().path,
        "/api/suggestions/roast?q=%E4%B8%AD%20%E7%85%8E"
    );

    // 対象の 8 つは API の経路の名前と同じ。
    for (target, name) in [
        (SuggestionTarget::Producer, "producer"),
        (SuggestionTarget::Origin, "origin"),
        (SuggestionTarget::Region, "region"),
        (SuggestionTarget::Process, "process"),
        (SuggestionTarget::Variety, "variety"),
        (SuggestionTarget::Roast, "roast"),
        (SuggestionTarget::Method, "method"),
        (SuggestionTarget::GrindSetting, "grind_setting"),
    ] {
        assert_eq!(target.as_str(), name);
    }
    assert_eq!(SuggestionTarget::ALL.len(), 8);
}

#[test]
fn the_place_search_query_is_encoded_and_the_candidates_are_read() {
    let (api, transport) = with_response(json!({
        "candidates": [
            {"name": "丸山珈琲", "address": "長野県北佐久郡軽井沢町"},
            {"name": "丸山珈琲 中目黒", "address": "東京都目黒区"},
        ]
    }));
    let candidates =
        block_on(api.place_search("丸山 珈琲", "ja")).expect("the candidates must be read");
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].name, "丸山珈琲");
    assert_eq!(candidates[0].address, "長野県北佐久郡軽井沢町");
    assert_eq!(
        transport.last_request().path,
        "/api/place-search?q=%E4%B8%B8%E5%B1%B1%20%E7%8F%88%E7%90%B2&lang=ja"
    );

    // 候補の無い応答は空の配列として読む (FR-22)。
    let (api, _) = with_response(json!({"candidates": []}));
    assert!(block_on(api.place_search("店", "en"))
        .expect("the candidates must be read")
        .is_empty());
}

#[test]
fn the_maps_config_reads_the_key_or_null() {
    let (api, transport) = with_response(json!({"embed_api_key": "test-key"}));
    let key = block_on(api.maps_config()).expect("the config must be read");
    assert_eq!(key.as_deref(), Some("test-key"));
    assert_eq!(transport.last_request().path, "/api/maps/config");

    // キーの未設定は null になり、画面は地図を出さない (FR-22)。
    let (api, _) = with_response(json!({"embed_api_key": null}));
    assert_eq!(
        block_on(api.maps_config()).expect("the config must be read"),
        None
    );
}

#[test]
fn a_broken_response_is_a_format_error() {
    let (api, _) = with_response(json!({"shops": "not an array"}));
    let error = block_on(api.shops(&ListOptions::default(), None))
        .expect_err("the response must be rejected");
    assert!(matches!(error, RecordError::Format(_)), "{error:?}");

    // 住所の検索と地図の設定の応答の形の違反も形式の誤りにする (FR-22)。
    let (api, _) = with_response(json!({"candidates": "not an array"}));
    let error = block_on(api.place_search("店", "ja")).expect_err("the response must be rejected");
    assert!(matches!(error, RecordError::Format(_)), "{error:?}");

    let (api, _) = with_response(json!({"candidates": [{"name": "店"}]}));
    let error = block_on(api.place_search("店", "ja")).expect_err("the response must be rejected");
    assert!(matches!(error, RecordError::Format(_)), "{error:?}");

    let (api, _) = with_response(json!({"embed_api_key": 1}));
    let error = block_on(api.maps_config()).expect_err("the response must be rejected");
    assert!(matches!(error, RecordError::Format(_)), "{error:?}");
}

#[test]
fn the_uploader_requests_the_url_puts_the_jpeg_and_completes() {
    let transport = Rc::new(FakeUploadTransport::new());
    let (client, api_transport) = client(vec![
        support::FakeTransport::response(
            200,
            &json!({"url": "https://r2.example/put", "key": "pending/u/1.jpg"}).to_string(),
        ),
        support::FakeTransport::response(200, &purchase_json("b1", "p1", None).to_string()),
    ]);
    let uploader = PhotoUploader::new(RecordsApi::new(client), transport.clone());

    let purchase = block_on(uploader.upload(
        "b1".to_string(),
        ConvertedImage {
            bytes: vec![9, 8, 7],
        },
    ))
    .expect("the upload must complete");
    assert_eq!(purchase.id, "b1");

    // 3 回の呼び出し: URL の発行、R2 への PUT、完了の通知 (ADR-0003)。
    let requests = api_transport.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, "/api/purchases/b1/photo/upload-url");
    assert_eq!(requests[1].path, "/api/purchases/b1/photo");
    let upload = transport.last_request();
    assert_eq!(upload.url, "https://r2.example/put");
    assert_eq!(upload.content_type, "image/jpeg");
    assert_eq!(upload.body, vec![9, 8, 7]);
}

#[test]
fn a_failed_upload_is_a_retryable_upload_error() {
    let transport = Rc::new(FakeUploadTransport::with_status(403));
    let (client, _) = client(vec![support::FakeTransport::response(
        200,
        &json!({"url": "https://r2.example/put", "key": "pending/u/1.jpg"}).to_string(),
    )]);
    let uploader = PhotoUploader::new(RecordsApi::new(client), transport);

    let error = block_on(uploader.upload("b1".to_string(), ConvertedImage { bytes: vec![1] }))
        .expect_err("the upload must fail");
    assert!(matches!(error, RecordError::Upload(_)), "{error:?}");
    assert!(brew_book_frontend::records::record_error_retry(&error));
}

/// 失敗を画面に出す文言のキーに変換する (FR-16。0041 のレビューの指摘)。
#[test]
fn the_error_keys_follow_the_shared_mapping() {
    use brew_book_frontend::api::{ApiCallError, ApiError, NetworkError};
    use brew_book_frontend::i18n::Key;
    use brew_book_frontend::records::record_error_key;

    let api_error = |status: u16| {
        RecordError::Api(ApiCallError::Api(ApiError {
            status,
            code: "code".to_string(),
            message: "message".to_string(),
        }))
    };
    for (error, expected) in [
        (api_error(401), Key::ErrorUnauthorized),
        (api_error(400), Key::ErrorValidation),
        (api_error(404), Key::ErrorNotFound),
        (api_error(409), Key::ErrorConflict),
        (api_error(410), Key::ErrorGone),
        (api_error(500), Key::ErrorUnexpected),
        (
            RecordError::Api(ApiCallError::Network(NetworkError::new("Failed to fetch"))),
            Key::ErrorNetwork,
        ),
        (
            RecordError::Upload("the PUT failed".to_string()),
            Key::ErrorNetwork,
        ),
        (
            RecordError::Format("broken".to_string()),
            Key::ErrorUnexpected,
        ),
        (
            RecordError::Validation(Key::ValidationRequired),
            Key::ValidationRequired,
        ),
    ] {
        assert_eq!(record_error_key(&error), expected, "{error:?}");
    }
}

/// 再試行の案内を出す失敗の判定 (0041 のレビューの指摘)。
///
/// 通信の失敗と、401 以外の API エラー (500 など) は再試行を出す。401 (ログイン画面へ戻す)、
/// 入力の誤り (400 など)、形式の違反と写真の失敗は出さない。
#[test]
fn the_retry_notice_follows_the_shared_rule() {
    use brew_book_frontend::api::{ApiCallError, ApiError, NetworkError};
    use brew_book_frontend::i18n::Key;
    use brew_book_frontend::records::record_error_retry;

    let api_error = |status: u16| {
        RecordError::Api(ApiCallError::Api(ApiError {
            status,
            code: "code".to_string(),
            message: "message".to_string(),
        }))
    };
    for (error, expected) in [
        (api_error(500), true),
        (api_error(503), true),
        // 入力の誤りでも、401 以外の API エラーは再試行を出す (デザインの Feedback の指示)。
        (api_error(400), true),
        (api_error(401), false),
        (
            RecordError::Api(ApiCallError::Network(NetworkError::new("Failed to fetch"))),
            true,
        ),
        (RecordError::Upload("the PUT failed".to_string()), true),
        (RecordError::Format("broken".to_string()), false),
        (RecordError::Validation(Key::ValidationRequired), false),
    ] {
        assert_eq!(record_error_retry(&error), expected, "{error:?}");
    }
}

/// モデルの項目の型の違反が形式の失敗になる (0041 のレビューの指摘)。
#[test]
fn a_broken_model_field_is_a_format_error() {
    use brew_book_frontend::records::models::{Product, Purchase, Shop};
    use serde_json::Map;

    let cases: Vec<(Map<String, Value>, &str)> = vec![
        (
            json!({"id": 1, "name": "x"}).as_object().unwrap().clone(),
            "the id must be a string",
        ),
        (
            json!({"id": "s1", "name": 2}).as_object().unwrap().clone(),
            "the name must be a string",
        ),
        (
            json!({"id": "p1", "name": "x", "flavor_notes": "not an array"})
                .as_object()
                .unwrap()
                .clone(),
            "the flavor_notes must be an array",
        ),
    ];
    for (json, what) in &cases {
        assert!(
            matches!(Shop::from_json(json), Err(RecordError::Format(_))),
            "the shop must reject {what}"
        );
    }

    // 必須の項目が無い応答も形式の失敗にする。
    let missing = json!({"id": "p1"}).as_object().unwrap().clone();
    assert!(matches!(
        Product::from_json(&missing),
        Err(RecordError::Format(_))
    ));
    assert!(matches!(
        Purchase::from_json(&missing),
        Err(RecordError::Format(_))
    ));
}

/// 完了通知が失敗したときは、その失敗を返す (写真が紐づかないまま残るため)。
#[test]
fn a_failed_completion_is_reported() {
    let transport = Rc::new(FakeUploadTransport::new());
    let (client, _) = client(vec![
        support::FakeTransport::response(
            200,
            &json!({"url": "https://r2.example/put", "key": "pending/u/1.jpg"}).to_string(),
        ),
        support::FakeTransport::response(
            500,
            &json!({"error": {"code": "internal", "message": "boom"}}).to_string(),
        ),
    ]);
    let uploader = PhotoUploader::new(RecordsApi::new(client), transport);

    let error = block_on(uploader.upload("b1".to_string(), ConvertedImage { bytes: vec![1] }))
        .expect_err("the upload must fail");
    assert!(matches!(error, RecordError::Api(_)), "{error:?}");
    // 応答の形式が違う応答も失敗にする (完了通知の 200 が JSON でない場合)。
}

/// 記録の削除と、削除の影響の取得 (0056)。
#[test]
fn a_record_is_deleted_and_the_impact_is_read() {
    // 店の削除の影響は購入の件数を読み、無い項目 (抽出) は 0 にする。
    let (api, transport) = with_response(json!({"purchases": 2}));
    let impact = block_on(api.shop_delete_impact("s1")).expect("the impact must be read");
    assert_eq!((impact.purchases, impact.brews), (2, 0));
    assert_eq!(transport.last_request().method, Method::Get);
    assert_eq!(transport.last_request().path, "/api/shops/s1/delete-impact");

    // 商品の削除の影響は購入と抽出の件数を読む。
    let (api, transport) = with_response(json!({"purchases": 2, "brews": 5}));
    let impact = block_on(api.product_delete_impact("p1")).expect("the impact must be read");
    assert_eq!((impact.purchases, impact.brews), (2, 5));
    assert_eq!(
        transport.last_request().path,
        "/api/products/p1/delete-impact"
    );

    // 購入の削除の影響は抽出の件数だけを読む。
    let (api, _) = with_response(json!({"brews": 3}));
    let impact = block_on(api.purchase_delete_impact("b1")).expect("the impact must be read");
    assert_eq!((impact.purchases, impact.brews), (0, 3));

    // 削除の 4 種は `DELETE` を送り、204 の空の本文を成功として扱う。
    let (api_client, transport) = client(vec![support::FakeTransport::response(204, "")]);
    let api = RecordsApi::new(api_client);
    block_on(api.delete_shop("s1")).expect("the shop must be deleted");
    assert_eq!(transport.last_request().method, Method::Delete);
    assert_eq!(transport.last_request().path, "/api/shops/s1");

    let (api_client, transport) = client(vec![support::FakeTransport::response(204, "")]);
    let api = RecordsApi::new(api_client);
    block_on(api.delete_product("p1")).expect("the product must be deleted");
    assert_eq!(transport.last_request().method, Method::Delete);
    assert_eq!(transport.last_request().path, "/api/products/p1");

    let (api_client, transport) = client(vec![support::FakeTransport::response(204, "")]);
    let api = RecordsApi::new(api_client);
    block_on(api.delete_purchase("b1")).expect("the purchase must be deleted");
    assert_eq!(transport.last_request().method, Method::Delete);
    assert_eq!(transport.last_request().path, "/api/purchases/b1");

    let (api_client, transport) = client(vec![support::FakeTransport::response(204, "")]);
    let api = RecordsApi::new(api_client);
    block_on(api.delete_brew("w1")).expect("the brew must be deleted");
    assert_eq!(transport.last_request().method, Method::Delete);
    assert_eq!(transport.last_request().path, "/api/brews/w1");
}
