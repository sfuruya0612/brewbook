//! 一覧の並び替えとお気に入りのブラウザテスト (0051、FR-20、FR-21)。
//!
//! 4 つの一覧のツールバーと、行の星と、抽出と購入の詳細と、商品と店の編集の星を実際に描き、
//! 操作で送るクエリと経路、行の押下 (詳細を開く) との分離を検査する。ブラウザで動かすため
//! `frontend:test-web` (wasm-bindgen-test) で実行する。

#![cfg(target_arch = "wasm32")]

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use brew_book_frontend::api::{ApiClient, ApiResponse, TransportError};
use brew_book_frontend::auth::{AuthServices, SessionStatus};
use brew_book_frontend::i18n::{set_language, Language};
use brew_book_frontend::records::values::LocalDateTime;
use brew_book_frontend::records::{PhotoUploader, RecordServices, RecordsApi};
use brew_book_frontend::screens::records::brew_detail::BrewDetailScreen;
use brew_book_frontend::screens::records::home::HomeScreen;
use brew_book_frontend::screens::records::product_form::ProductForm;
use brew_book_frontend::screens::records::product_list::ProductListScreen;
use brew_book_frontend::screens::records::purchase_detail::PurchaseDetailScreen;
use brew_book_frontend::screens::records::purchase_list::PurchaseListScreen;
use brew_book_frontend::screens::records::shop_form::ShopForm;
use brew_book_frontend::screens::records::shop_list::ShopListScreen;
use brew_book_frontend::screens::records::{RecordLoader, RecordPickerSheet};
use brew_book_frontend::ui::ListRow;
use dioxus::history::{History, MemoryHistory};
use dioxus::prelude::*;
use dioxus_router::components::HistoryProvider;
use dioxus_router::{Routable, Router};
use serde_json::{json, Value};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use support::web::{count, install_styles, mount, select, set_theme, tick};
use support::{
    FakeClock, FakeImageConverter, FakePasskeyClient, FakePhotoPicker, FakeTransport,
    FakeUploadTransport,
};

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    /// 一覧のテストが使う偽の送信の実装 (送った要求と返す応答を確かめる)。
    static LIST_TRANSPORT: RefCell<Option<Rc<FakeTransport>>> = const { RefCell::new(None) };

    /// 描く経路。
    static LIST_PATH: RefCell<String> = const { RefCell::new(String::new()) };
}

/// 一覧の依存を組む。API は thread_local の偽の送信の実装を使う。
fn list_services() -> (RecordServices, AuthServices) {
    let transport = LIST_TRANSPORT.with(|slot| {
        slot.borrow()
            .clone()
            .expect("the transport must be set before mounting")
    });
    let api = ApiClient::new(transport);
    let auth = AuthServices::new(api.clone(), Rc::new(FakePasskeyClient::new()));
    let uploader = PhotoUploader::new(
        RecordsApi::new(api.clone()),
        Rc::new(FakeUploadTransport::new()),
    );
    let records = RecordServices::new(
        api,
        Rc::new(FakeClock {
            now: LocalDateTime::new(2026, 10, 5, 12, 30),
            utc_offset_minutes: 540,
        }),
        Rc::new(FakePhotoPicker { photo: None }),
        Rc::new(FakeImageConverter),
        Rc::new(uploader),
    );
    (records, auth)
}

/// 画面の依存を配る探り (一覧と、一覧から開く詳細の検査に使う)。
#[component]
fn ListProbe() -> Element {
    let (services, auth) = list_services();
    let _services = use_context_provider(|| services);
    let _auth = use_context_provider(|| auth);
    let _session = use_context_provider(|| Signal::new(SessionStatus::SignedIn));
    let _notice = use_context_provider(|| Signal::new(None::<String>));
    let _revision = use_context_provider(|| Signal::new(0_u64));
    let path = LIST_PATH.with(|slot| slot.borrow().clone());
    rsx! {
        HistoryProvider {
            history: move |_| Rc::new(MemoryHistory::with_initial_path(&path)) as Rc<dyn History>,
            Router::<TestRoute> {}
        }
    }
}

/// 一覧の検査用の経路。一覧から開く画面は印だけを描く (開いたことを検査する)。
#[derive(Routable, Clone, PartialEq, Debug)]
enum TestRoute {
    #[route("/", HomeRoute)]
    Home {},
    #[route("/purchases", PurchasesRoute)]
    Purchases {},
    #[route("/products", ProductsRoute)]
    Products {},
    #[route("/shops", ShopsRoute)]
    Shops {},
    #[route("/brews/:id", BrewDetailRoute)]
    BrewDetail { id: String },
    #[route("/purchases/:id", PurchaseDetailRoute)]
    PurchaseDetail { id: String },
    #[route("/products/:id/edit", ProductEditRoute)]
    ProductEdit { id: String },
    #[route("/shops/:id/edit", ShopEditRoute)]
    ShopEdit { id: String },
}

#[component]
fn HomeRoute() -> Element {
    rsx! {
        HomeScreen {}
    }
}

#[component]
fn PurchasesRoute() -> Element {
    rsx! {
        PurchaseListScreen {}
    }
}

#[component]
fn ProductsRoute() -> Element {
    rsx! {
        ProductListScreen {}
    }
}

#[component]
fn ShopsRoute() -> Element {
    rsx! {
        ShopListScreen {}
    }
}

#[component]
fn BrewDetailRoute(id: String) -> Element {
    let _ = id;
    rsx! {
        div { class: "opened-detail" }
    }
}

#[component]
fn PurchaseDetailRoute(id: String) -> Element {
    let _ = id;
    rsx! {
        div { class: "opened-detail" }
    }
}

#[component]
fn ProductEditRoute(id: String) -> Element {
    let _ = id;
    rsx! {
        div { class: "opened-detail" }
    }
}

#[component]
fn ShopEditRoute(id: String) -> Element {
    let _ = id;
    rsx! {
        div { class: "opened-detail" }
    }
}

/// 詳細とフォームの検査用の経路 (実際の画面を描く)。
#[derive(Routable, Clone, PartialEq, Debug)]
enum DetailRoute {
    #[route("/", HomeRoute)]
    Home {},
    #[route("/brews/:id", RealBrewDetailRoute)]
    BrewDetail { id: String },
    #[route("/purchases/:id", RealPurchaseDetailRoute)]
    PurchaseDetail { id: String },
    #[route("/products/new", RealProductNewRoute)]
    ProductNew {},
    #[route("/products/:id/edit", RealProductEditRoute)]
    ProductEdit { id: String },
    #[route("/shops/new", RealShopNewRoute)]
    ShopNew {},
    #[route("/shops/:id/edit", RealShopEditRoute)]
    ShopEdit { id: String },
}

#[component]
fn RealProductNewRoute() -> Element {
    rsx! {
        ProductForm {}
    }
}

#[component]
fn RealShopNewRoute() -> Element {
    rsx! {
        ShopForm {}
    }
}

#[component]
fn RealBrewDetailRoute(id: String) -> Element {
    rsx! {
        BrewDetailScreen { id }
    }
}

#[component]
fn RealPurchaseDetailRoute(id: String) -> Element {
    rsx! {
        PurchaseDetailScreen { id }
    }
}

#[component]
fn RealProductEditRoute(id: String) -> Element {
    rsx! {
        ProductForm { id: Some(id) }
    }
}

#[component]
fn RealShopEditRoute(id: String) -> Element {
    rsx! {
        ShopForm { id: Some(id) }
    }
}

/// 詳細とフォームの検査用の探り。
#[component]
fn DetailProbe() -> Element {
    let (services, auth) = list_services();
    let _services = use_context_provider(|| services);
    let _auth = use_context_provider(|| auth);
    let _session = use_context_provider(|| Signal::new(SessionStatus::SignedIn));
    let _notice = use_context_provider(|| Signal::new(None::<String>));
    let _revision = use_context_provider(|| Signal::new(0_u64));
    let path = LIST_PATH.with(|slot| slot.borrow().clone());
    rsx! {
        HistoryProvider {
            history: move |_| Rc::new(MemoryHistory::with_initial_path(&path)) as Rc<dyn History>,
            Router::<DetailRoute> {}
        }
    }
}

/// 選択のシートの検査用の探り (依存と、シートを閉じる処理だけを持つ)。
#[component]
fn PickerProbe() -> Element {
    let (services, auth) = list_services();
    let _services = use_context_provider(|| services.clone());
    let _auth = use_context_provider(|| auth);
    let _session = use_context_provider(|| Signal::new(SessionStatus::SignedIn));
    let _notice = use_context_provider(|| Signal::new(None::<String>));
    let _revision = use_context_provider(|| Signal::new(0_u64));
    let load_services = services.clone();
    let load = RecordLoader::new(move |cursor, options| {
        let api = RecordsApi::new(load_services.api.clone());
        Box::pin(async move { api.shops(&options, cursor.as_deref()).await })
    });
    let row = Callback::new(|shop: brew_book_frontend::records::Shop| {
        rsx! {
            ListRow { title: shop.name.clone() }
        }
    });
    rsx! {
        RecordPickerSheet::<brew_book_frontend::records::Shop> {
            title: "Shops",
            load,
            row,
            on_close: move |_| {},
        }
    }
}

/// 応答を 1 つ作る。
fn response(body: &Value) -> Result<ApiResponse, TransportError> {
    FakeTransport::response(200, &body.to_string())
}

/// 経路を描き、読み込みの応答を待つ。
async fn mount_at(
    path: &str,
    responses: Vec<Result<ApiResponse, TransportError>>,
) -> (web_sys::Element, Rc<FakeTransport>) {
    let transport = Rc::new(FakeTransport::new(responses));
    LIST_TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport.clone()));
    LIST_PATH.with(|slot| *slot.borrow_mut() = path.to_string());
    let root = mount(ListProbe).await;
    for _ in 0..10 {
        tick().await;
    }
    (root, transport)
}

/// 詳細とフォームの経路を描き、読み込みの応答を待つ。
async fn mount_detail(
    path: &str,
    responses: Vec<Result<ApiResponse, TransportError>>,
) -> (web_sys::Element, Rc<FakeTransport>) {
    let transport = Rc::new(FakeTransport::new(responses));
    LIST_TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport.clone()));
    LIST_PATH.with(|slot| *slot.borrow_mut() = path.to_string());
    let root = mount(DetailProbe).await;
    for _ in 0..10 {
        tick().await;
    }
    (root, transport)
}

/// ブラウザのタスクを数回進める。
async fn settle() {
    for _ in 0..10 {
        tick().await;
    }
}

/// 要素を押す。
fn click(element: &web_sys::Element) {
    element.unchecked_ref::<web_sys::HtmlElement>().click();
}

/// 入れ物の中の一致を順に返す (NodeList の添字アクセス)。
fn elements(root: &web_sys::Element, selector: &str) -> Vec<web_sys::Element> {
    let list = root
        .query_selector_all(selector)
        .expect("the selector must be valid");
    let mut found = Vec::new();
    for index in 0..list.length() {
        let value = js_sys::Reflect::get(&list, &wasm_bindgen::JsValue::from_f64(f64::from(index)))
            .expect("the index must be readable");
        if let Ok(element) = value.dyn_into::<web_sys::Element>() {
            found.push(element);
        }
    }
    found
}

/// キーを押す (キーダウンだけを送る。実際のブラウザの既定の動作は合成のイベントでは起きない)。
fn press_key(element: &web_sys::Element, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
        .expect("the key event must be created");
    element
        .dispatch_event(&event)
        .expect("the key event must dispatch");
}

/// `select` の値を変えて change を送る。
fn set_select(root: &web_sys::Element, value: &str) {
    let element = select(root, ".sort-select");
    element
        .unchecked_ref::<web_sys::HtmlSelectElement>()
        .set_value(value);
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    let event = web_sys::Event::new_with_event_init_dict("change", &init)
        .expect("the change event must be created");
    element
        .dispatch_event(&event)
        .expect("the change event must dispatch");
}

/// `select` の選択肢の文言。
fn option_labels(root: &web_sys::Element) -> Vec<String> {
    let nodes = root
        .query_selector_all(".sort-select option")
        .expect("the selector must be valid");
    let mut labels = Vec::new();
    for index in 0..nodes.length() {
        if let Some(text) = nodes.item(index).and_then(|node| node.text_content()) {
            labels.push(text);
        }
    }
    labels
}

/// 店の応答の JSON。
fn shop_json(id: &str, name: &str, favorited_at: Option<&str>) -> Value {
    json!({
        "id": id,
        "user_id": "user",
        "name": name,
        "address": null,
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
        "favorited_at": favorited_at,
    })
}

/// 商品の応答の JSON。
fn product_json(id: &str, name: &str, favorited_at: Option<&str>) -> Value {
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
        "favorited_at": favorited_at,
    })
}

/// 購入の応答の JSON (商品と店をネストする)。
fn purchase_json(id: &str, product: &Value, favorited_at: Option<&str>) -> Value {
    json!({
        "id": id,
        "user_id": "user",
        "product_id": "p1",
        "shop_id": null,
        "purchased_on": "2026-10-01",
        "roast": null,
        "roast_date": null,
        "price_amount": null,
        "price_currency": null,
        "weight_grams": null,
        "photo_key": null,
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
        "favorited_at": favorited_at,
        "product": product,
        "shop": null,
    })
}

/// 抽出の応答の JSON (購入をネストする)。
fn brew_json(id: &str, purchase: &Value, favorited_at: Option<&str>) -> Value {
    json!({
        "id": id,
        "user_id": "user",
        "purchase_id": "b1",
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
        "favorited_at": favorited_at,
        "purchase": purchase,
    })
}

/// 4 つの一覧に並び順の選択と昇順/降順の切り替えとお気に入りのみの切り替えが出る (FR-20、FR-21)。
#[wasm_bindgen_test]
async fn the_four_lists_have_the_sort_and_favorite_toolbar() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let product = || product_json("p1", "Beans", None);
    let purchase = || purchase_json("b1", &product(), None);
    let cases: [(&str, Value, [&str; 3]); 4] = [
        (
            "/",
            json!({"brews": [brew_json("w1", &purchase(), None)], "next_cursor": null}),
            ["Brewed at", "Rating", "Dose"],
        ),
        (
            "/purchases",
            json!({"purchases": [purchase()], "next_cursor": null}),
            ["Purchased on", "Price", "Weight"],
        ),
        (
            "/products",
            json!({"products": [product()], "next_cursor": null}),
            ["Created", "Product name", "Updated"],
        ),
        (
            "/shops",
            json!({"shops": [shop_json("s1", "Shop", None)], "next_cursor": null}),
            ["Created", "Shop name", "Updated"],
        ),
    ];
    for (path, body, expected) in cases {
        let (root, _) = mount_at(path, vec![response(&body)]).await;
        assert_eq!(count(&root, ".list-toolbar"), 1, "{path}");
        assert_eq!(count(&root, ".sort-select"), 1, "{path}");
        assert_eq!(option_labels(&root), expected.map(str::to_string), "{path}");
        let order = select(&root, ".list-toolbar .iconbtn");
        assert_eq!(
            order.get_attribute("aria-label").as_deref(),
            Some("Descending"),
            "{path}"
        );
        let chip = select(&root, ".list-toolbar .chip");
        assert_eq!(
            chip.text_content().as_deref(),
            Some("Favorites only"),
            "{path}"
        );
        assert_eq!(count(&root, ".list-toolbar .chip.on"), 0, "{path}");
    }
}

/// 並び順、方向、お気に入りのみを変えると、カーソルを捨てて新しい条件で先頭から読み直す
/// (FR-20、FR-21)。
#[wasm_bindgen_test]
async fn changing_the_sort_and_the_order_and_the_favorites_reloads_from_the_first_page() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let product = product_json("p1", "Beans", None);
    let purchase = purchase_json("b1", &product, None);
    let list = |favorited: Option<&str>| json!({"brews": [brew_json("w1", &purchase, favorited)], "next_cursor": null});
    let (root, transport) = mount_at(
        "/",
        vec![
            response(&list(None)),
            response(&list(None)),
            response(&list(None)),
            response(&list(None)),
        ],
    )
    .await;

    // 並び順を評価にする。
    set_select(&root, "rating");
    settle().await;
    // 昇順に切り替える。
    let order = select(&root, ".list-toolbar .iconbtn");
    click(&order);
    settle().await;
    assert_eq!(
        select(&root, ".list-toolbar .iconbtn")
            .get_attribute("aria-label")
            .as_deref(),
        Some("Ascending")
    );
    // お気に入りのみにする。
    let chip = select(&root, ".list-toolbar .chip");
    click(&chip);
    settle().await;
    assert_eq!(count(&root, ".list-toolbar .chip.on"), 1);

    let paths: Vec<String> = transport
        .requests()
        .into_iter()
        .map(|request| request.path)
        .collect();
    assert_eq!(
        paths,
        [
            "/api/brews?limit=50&sort=brewed_at&order=desc",
            "/api/brews?limit=50&sort=rating&order=desc",
            "/api/brews?limit=50&sort=rating&order=asc",
            "/api/brews?limit=50&sort=rating&order=asc&favorite=true",
        ]
    );
}

/// お気に入りのみの切り替えで、お気に入りでない行が表示されない (FR-21)。
#[wasm_bindgen_test]
async fn the_favorites_only_filter_hides_the_records_that_are_not_favorites() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let (root, transport) = mount_at(
        "/products",
        vec![
            response(&json!({
                "products": [
                    product_json("p1", "Favorite", Some("2026-10-05T00:00:00.000Z")),
                    product_json("p2", "Plain", None),
                ],
                "next_cursor": null,
            })),
            response(&json!({
                "products": [product_json("p1", "Favorite", Some("2026-10-05T00:00:00.000Z"))],
                "next_cursor": null,
            })),
        ],
    )
    .await;
    assert_eq!(count(&root, ".row"), 2);
    assert_eq!(count(&root, ".row .fav.on"), 1);

    let chip = select(&root, ".list-toolbar .chip");
    click(&chip);
    settle().await;

    assert_eq!(
        transport.last_request().path,
        "/api/products?limit=50&sort=created_at&order=desc&favorite=true"
    );
    assert_eq!(count(&root, ".row"), 1);
    let row = select(&root, ".row .name");
    assert_eq!(row.text_content().as_deref(), Some("Favorite"));
}

/// 選択のシートには並び順とお気に入りの操作と星が出ない (FR-20、FR-21)。
#[wasm_bindgen_test]
async fn the_picker_sheet_has_no_sort_or_favorite_controls() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let transport = Rc::new(FakeTransport::new(vec![response(&json!({
        "shops": [shop_json("s1", "Shop", None)],
        "next_cursor": null,
    }))]));
    LIST_TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport.clone()));
    let root = mount(PickerProbe).await;
    settle().await;

    assert_eq!(count(&root, ".sheet"), 1);
    assert_eq!(count(&root, ".list-toolbar"), 0);
    assert_eq!(count(&root, ".sort-select"), 0);
    assert_eq!(count(&root, ".row .fav"), 0);
    assert_eq!(count(&root, ".row"), 1);
    // 並び順を指定せず、API の既定に任せる。
    assert_eq!(
        transport.last_request().path,
        "/api/shops?limit=50&order=desc"
    );
}

/// 行の星の押下と Enter と Space でお気に入りを切り替えられ、詳細は開かない (FR-21)。
#[wasm_bindgen_test]
async fn the_star_on_a_row_toggles_the_favorite_without_opening_the_detail() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let product = product_json("p1", "Beans", None);
    let purchase = purchase_json("b1", &product, None);
    let list = |favorited: Option<&str>| json!({"brews": [brew_json("w1", &purchase, favorited)], "next_cursor": null});
    let (root, transport) = mount_at(
        "/",
        vec![
            response(&list(None)),
            // お気に入りの `PUT` の応答は更新後の抽出 1 件。
            response(&brew_json(
                "w1",
                &purchase_json("b1", &product_json("p1", "Beans", None), None),
                Some("2026-10-05T00:00:00.000Z"),
            )),
            response(&list(Some("2026-10-05T00:00:00.000Z"))),
            // 星の押下が行の押下を発火させた場合に、詳細の読み込みで止まらないようにする。
            response(&brew_json(
                "w1",
                &purchase_json("b1", &product_json("p1", "Beans", None), None),
                None,
            )),
        ],
    )
    .await;
    assert_eq!(count(&root, ".row .fav.on"), 0);

    // 星を押すとお気に入りを付ける (`PUT`)。
    let star = select(&root, ".row .fav .iconbtn");
    click(&star);
    settle().await;
    assert_eq!(transport.requests()[1].path, "/api/brews/w1/favorite");
    assert_eq!(transport.requests()[1].method.as_str(), "PUT");
    // 一覧は先頭から読み直され、星が付いた状態になる。
    assert_eq!(transport.requests().len(), 3);
    assert_eq!(count(&root, ".row .fav.on"), 1);
    assert_eq!(count(&root, ".detail"), 0);
    assert_eq!(count(&root, ".opened-detail"), 0);

    // 星の Enter と Space は行の押下 (詳細を開く) を発火させない。
    for key in ["Enter", " "] {
        let star = select(&root, ".row .fav .iconbtn");
        press_key(&star, key);
        settle().await;
        assert_eq!(count(&root, ".opened-detail"), 0, "{key}");
        assert_eq!(count(&root, ".detail"), 0, "{key}");
        // キーではお気に入りを切り替えない (ブラウザの既定の動作は合成のイベントでは起きない)。
        assert_eq!(transport.requests().len(), 3, "{key}");
    }
}

/// 商品と店の編集の星でお気に入りを切り替えられ、新規の登録には星が出ない (FR-21)。
#[wasm_bindgen_test]
async fn the_edit_forms_have_the_star_and_the_new_forms_do_not() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);

    // 商品の編集: 星が出て、押すと `PUT` を送る。
    let (root, transport) = mount_detail(
        "/products/p1/edit",
        vec![
            response(&product_json("p1", "Beans", None)),
            response(&product_json(
                "p1",
                "Beans",
                Some("2026-10-05T00:00:00.000Z"),
            )),
        ],
    )
    .await;
    assert_eq!(count(&root, ".appbar .acts > .iconbtn"), 2);
    let star = select(&root, ".appbar .acts > .iconbtn");
    assert_eq!(
        star.get_attribute("aria-label").as_deref(),
        Some("Add to favorites")
    );
    click(&star);
    settle().await;
    assert_eq!(transport.last_request().path, "/api/products/p1/favorite");
    assert_eq!(transport.last_request().method.as_str(), "PUT");
    // 押した後は星が塗りになる。
    assert_eq!(
        select(&root, ".appbar .acts > .iconbtn")
            .get_attribute("aria-label")
            .as_deref(),
        Some("Remove from favorites")
    );

    // 店の編集: 同じく星が出る。地図の設定 (FR-22) の応答も返す。
    let (root, _) = mount_detail(
        "/shops/s1/edit",
        vec![
            response(&shop_json("s1", "Shop", None)),
            response(&json!({"embed_api_key": null})),
        ],
    )
    .await;
    assert_eq!(count(&root, ".appbar .acts > .iconbtn"), 2);

    // 新規の登録: 星は出ない (保存の文字ボタンだけ)。
    let (root, _) = mount_detail("/products/new", vec![]).await;
    assert_eq!(count(&root, ".appbar .acts > .iconbtn"), 0);
    assert_eq!(count(&root, ".appbar .acts > .btn"), 1);
    let (root, _) = mount_detail(
        "/shops/new",
        vec![response(&json!({"embed_api_key": null}))],
    )
    .await;
    assert_eq!(count(&root, ".appbar .acts > .iconbtn"), 0);
    assert_eq!(count(&root, ".appbar .acts > .btn"), 1);
}

/// 抽出と購入の詳細の星でお気に入りを切り替えられる (FR-21)。
#[wasm_bindgen_test]
async fn the_detail_screens_have_the_star() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let product = product_json("p1", "Beans", None);
    let purchase = purchase_json("b1", &product, None);

    // 抽出の詳細: 星と編集の 2 つ。押すと `PUT` を送る。
    let (root, transport) = mount_detail(
        "/brews/w1",
        vec![
            response(&brew_json("w1", &purchase, None)),
            response(&brew_json(
                "w1",
                &purchase,
                Some("2026-10-05T00:00:00.000Z"),
            )),
            // お気に入りの `PUT` の後、詳細は記録の変更の通知で読み直される。
            response(&brew_json(
                "w1",
                &purchase,
                Some("2026-10-05T00:00:00.000Z"),
            )),
        ],
    )
    .await;
    assert_eq!(count(&root, ".appbar .acts > .iconbtn"), 3);
    let star = select(&root, ".appbar .acts > .iconbtn");
    assert_eq!(
        star.get_attribute("aria-label").as_deref(),
        Some("Add to favorites")
    );
    click(&star);
    settle().await;
    // お気に入りの `PUT` の後に、詳細が先頭から読み直される。
    assert_eq!(transport.requests()[1].path, "/api/brews/w1/favorite");
    assert_eq!(transport.requests()[1].method.as_str(), "PUT");
    assert!(transport.requests().len() >= 2);
    assert_eq!(
        select(&root, ".appbar .acts > .iconbtn")
            .get_attribute("aria-label")
            .as_deref(),
        Some("Remove from favorites")
    );

    // 購入の詳細: 星と編集の 2 つ (評価の推移の取得も待つ)。
    let (root, _) = mount_detail(
        "/purchases/b1",
        vec![response(&purchase), response(&json!({"ratings": []}))],
    )
    .await;
    assert_eq!(count(&root, ".appbar .acts > .iconbtn"), 3);
    assert_eq!(
        select(&root, ".appbar .acts > .iconbtn")
            .get_attribute("aria-label")
            .as_deref(),
        Some("Add to favorites")
    );
}

/// 削除は確認の後にだけ行われ、取り消しでは API を呼ばない (0056)。
#[wasm_bindgen_test]
async fn the_delete_confirmation_gates_the_request() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let product = product_json("p1", "Beans", None);
    let purchase = purchase_json("b1", &product, None);
    let (root, transport) = mount_detail(
        "/brews/w1",
        vec![
            response(&brew_json("w1", &purchase, None)),
            // 確認の後にだけ削除の API を呼ぶ。
            FakeTransport::response(204, ""),
            // 削除の通知で詳細が読み直される場合に備える (404)。
            FakeTransport::response(
                404,
                &json!({"error": {"code": "not_found", "message": "x"}}).to_string(),
            ),
        ],
    )
    .await;
    let icons = elements(&root, ".appbar .acts > .iconbtn");
    assert_eq!(icons.len(), 3);

    // 削除を押すと確認が出て、削除の API はまだ呼ばれない。
    click(&icons[2]);
    settle().await;
    assert_eq!(count(&root, ".dialog"), 1);
    assert_eq!(
        transport.requests().len(),
        1,
        "the delete must wait for the confirmation"
    );

    // 取り消すとダイアログが閉じ、削除の API は呼ばれない。
    click(&select(&root, ".dialog .btn.text"));
    settle().await;
    assert_eq!(count(&root, ".dialog"), 0);
    assert_eq!(transport.requests().len(), 1);

    // 確認すると削除の API を送る。
    click(&elements(&root, ".appbar .acts > .iconbtn")[2]);
    settle().await;
    click(&select(&root, ".dialog .btn.danger"));
    settle().await;
    assert!(transport.requests().len() >= 2);
    assert_eq!(transport.requests()[1].path, "/api/brews/w1");
    assert_eq!(transport.requests()[1].method.as_str(), "DELETE");
}

/// 購入の削除の確認には、連鎖で消える抽出の件数が出る (0056)。
#[wasm_bindgen_test]
async fn the_purchase_delete_shows_the_brew_count_and_deletes_on_the_confirmation() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let product = product_json("p1", "Beans", None);
    let purchase = purchase_json("b1", &product, None);
    let (root, transport) = mount_detail(
        "/purchases/b1",
        vec![
            response(&purchase),
            response(&json!({"ratings": []})),
            response(&json!({"brews": 2})),
            FakeTransport::response(204, ""),
            // 削除の通知で詳細が読み直される場合に備える (404)。
            FakeTransport::response(
                404,
                &json!({"error": {"code": "not_found", "message": "x"}}).to_string(),
            ),
            FakeTransport::response(
                404,
                &json!({"error": {"code": "not_found", "message": "x"}}).to_string(),
            ),
        ],
    )
    .await;
    let icons = elements(&root, ".appbar .acts > .iconbtn");
    assert_eq!(icons.len(), 3);
    click(&icons[2]);
    settle().await;
    // 削除の影響を引いてから、件数入りの確認を出す。
    assert_eq!(transport.requests().len(), 3);
    assert_eq!(
        transport.requests()[2].path,
        "/api/purchases/b1/delete-impact"
    );
    assert_eq!(count(&root, ".dialog"), 1);
    let message = select(&root, ".dialog p")
        .text_content()
        .unwrap_or_default();
    assert!(message.contains("2 brews"), "{message}");

    click(&select(&root, ".dialog .btn.danger"));
    settle().await;
    assert!(transport.requests().len() >= 4);
    assert_eq!(transport.requests()[3].path, "/api/purchases/b1");
    assert_eq!(transport.requests()[3].method.as_str(), "DELETE");
}

/// 店の編集フォームの削除は、店の指定が外れる購入の件数を出して削除する (0056)。
#[wasm_bindgen_test]
async fn the_shop_form_delete_shows_the_affected_purchases() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let (root, transport) = mount_detail(
        "/shops/s1/edit",
        vec![
            response(&shop_json("s1", "Shop", None)),
            response(&json!({"embed_api_key": null})),
            response(&json!({"purchases": 3})),
            FakeTransport::response(204, ""),
            // 削除の通知でフォームが読み直される場合に備える (404)。
            FakeTransport::response(
                404,
                &json!({"error": {"code": "not_found", "message": "x"}}).to_string(),
            ),
        ],
    )
    .await;
    // 星と削除の 2 つのアイコンと、保存の文字ボタン。
    let icons = elements(&root, ".appbar .acts > .iconbtn");
    assert_eq!(icons.len(), 2);
    click(&icons[1]);
    settle().await;
    assert_eq!(transport.requests()[2].path, "/api/shops/s1/delete-impact");
    assert_eq!(count(&root, ".dialog"), 1);
    let message = select(&root, ".dialog p")
        .text_content()
        .unwrap_or_default();
    assert!(message.contains("3 purchases"), "{message}");

    click(&select(&root, ".dialog .btn.danger"));
    settle().await;
    assert!(transport.requests().len() >= 4);
    assert_eq!(transport.requests()[3].path, "/api/shops/s1");
    assert_eq!(transport.requests()[3].method.as_str(), "DELETE");
}

/// 2 段組では、削除の後に右の面が閉じて一覧が読み直される (0056)。
#[wasm_bindgen_test]
async fn the_wide_layout_closes_the_detail_pane_after_the_delete() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let product = product_json("p1", "Beans", None);
    let purchase = purchase_json("b1", &product, None);
    let list = json!({"brews": [brew_json("w1", &purchase, None)], "next_cursor": null});
    let (root, transport) = mount_at(
        "/",
        vec![
            response(&list),
            response(&brew_json("w1", &purchase, None)),
            FakeTransport::response(204, ""),
            response(&list),
            // 意図しない読み直しがあっても止まらないようにする (404)。
            FakeTransport::response(
                404,
                &json!({"error": {"code": "not_found", "message": "x"}}).to_string(),
            ),
        ],
    )
    .await;
    // 行を押すと右の面に詳細が出る (テストの窓は 1280 px)。
    click(&select(&root, ".row"));
    settle().await;
    assert_eq!(count(&root, ".wide-detail .detail"), 1);

    // 削除を確認すると、削除の API を送り、右の面を閉じて一覧を読み直す。
    let icons = elements(&root, ".wide-detail .appbar .acts > .iconbtn");
    assert_eq!(icons.len(), 3);
    click(&icons[2]);
    settle().await;
    click(&select(&root, ".dialog .btn.danger"));
    settle().await;
    assert!(transport.requests().len() >= 4);
    assert_eq!(transport.requests()[2].path, "/api/brews/w1");
    assert_eq!(transport.requests()[2].method.as_str(), "DELETE");
    assert_eq!(count(&root, ".wide-detail .detail"), 0);
    assert_eq!(count(&root, ".snack"), 1);
    // 一覧は先頭から読み直される。
    assert_eq!(
        transport.requests()[3].path,
        "/api/brews?limit=50&sort=brewed_at&order=desc"
    );
}
