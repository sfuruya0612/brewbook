//! 記録のフォームのブラウザテスト (日付と時刻の入力)。
//!
//! 購入と抽出の登録と編集のフォームを実際に描き、日付は date input、時刻は time input で、
//! 値が `YYYY-MM-DD` と `HH:MM` になることを検査する。ブラウザで動かすため `frontend:test-web`
//! (wasm-bindgen-test) で実行する。表示の形式 (ブラウザの言語による yyyy/mm/dd などの見え方)
//! は検査しない。内部の値の形式だけを確かめる。過去の購入をもとにした登録のフォーム
//! (引き継ぐ値、元の購入が無いとき、読み込みの失敗と再試行) も検査する (issue 0058)。

#![cfg(target_arch = "wasm32")]

mod support;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use brew_book_frontend::api::ApiClient;
use brew_book_frontend::auth::{AuthServices, SessionStatus};
use brew_book_frontend::i18n::{set_language, Language};
use brew_book_frontend::records::values::LocalDateTime;
use brew_book_frontend::records::{PhotoUploader, RecordServices, RecordsApi};
use brew_book_frontend::screens::records::brew_form::BrewForm;
use brew_book_frontend::screens::records::purchase_form::PurchaseForm;
use dioxus::history::{History, MemoryHistory};
use dioxus::prelude::*;
use dioxus_router::components::HistoryProvider;
use dioxus_router::{Routable, Router};
use serde_json::json;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use support::web::{count, install_styles, mount, select, set_theme, tick};
use support::{
    FakeClock, FakeImageConverter, FakePasskeyClient, FakePhotoPicker, FakeTransport,
    FakeUploadTransport,
};

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    /// フォームのテストが使う偽の送信の実装 (要求を確かめる)。
    static FORMS_TRANSPORT: RefCell<Option<Rc<FakeTransport>>> = const { RefCell::new(None) };

    /// 描くフォームの経路。
    static FORMS_PATH: RefCell<String> = const { RefCell::new(String::new()) };

    /// フォームを閉じたか (破棄の確認の検査に使う)。
    static FORM_CLOSED: Cell<bool> = const { Cell::new(false) };

    /// フォームが保存できたか (引き継いだ登録の保存の検査に使う。issue 0058)。
    static FORM_SAVED: Cell<bool> = const { Cell::new(false) };
}

/// フォームの依存を組む。API は thread_local の偽の送信の実装を使う。
fn forms_services() -> (RecordServices, AuthServices) {
    let transport = FORMS_TRANSPORT.with(|slot| {
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
            now: LocalDateTime::new(2026, 9, 25, 12, 30),
            utc_offset_minutes: 540,
        }),
        Rc::new(FakePhotoPicker { photo: None }),
        Rc::new(FakeImageConverter),
        Rc::new(uploader),
    );
    (records, auth)
}

/// フォームだけを持つテスト用の経路 (ルーターの文脈を用意する)。
#[derive(Routable, Clone, PartialEq, Debug)]
enum TestRoute {
    #[route("/purchases/new", PurchaseNewRoute)]
    PurchaseNew {},
    #[route("/purchases/new-from", PurchaseNewFromRoute)]
    PurchaseNewFrom {},
    #[route("/purchases/new-from-empty", PurchaseNewFromEmptyRoute)]
    PurchaseNewFromEmpty {},
    #[route("/purchases/:id/edit", PurchaseEditRoute)]
    PurchaseEdit { id: String },
    #[route("/brews/new", BrewNewRoute)]
    BrewNew {},
    #[route("/brews/new-close", BrewNewCloseRoute)]
    BrewNewClose {},
    #[route("/brews/:id/edit", BrewEditRoute)]
    BrewEdit { id: String },
}

#[component]
fn PurchaseNewRoute() -> Element {
    rsx! {
        PurchaseForm { id: None }
    }
}

/// 購入 `p1` をもとにした登録のフォーム (閉じる動きと保存の動きを受け取る。issue 0058)。
#[component]
fn PurchaseNewFromRoute() -> Element {
    rsx! {
        PurchaseForm {
            id: None,
            from: Some("p1".to_string()),
            on_close: Some(EventHandler::new(|_| {
                FORM_CLOSED.with(|cell| cell.set(true));
            })),
            on_saved: Some(EventHandler::new(|_| {
                FORM_SAVED.with(|cell| cell.set(true));
            })),
        }
    }
}

/// 空の ID をもとにした登録のフォーム (`/purchases/new?from=` の形。issue 0058)。
#[component]
fn PurchaseNewFromEmptyRoute() -> Element {
    rsx! {
        PurchaseForm {
            id: None,
            from: Some(String::new()),
        }
    }
}

#[component]
fn PurchaseEditRoute(id: String) -> Element {
    rsx! {
        PurchaseForm { id: Some(id) }
    }
}

#[component]
fn BrewNewRoute() -> Element {
    rsx! {
        BrewForm { id: None }
    }
}

/// 閉じる動きを受け取るフォーム (破棄の確認の検査用)。
#[component]
fn BrewNewCloseRoute() -> Element {
    rsx! {
        BrewForm {
            id: None,
            on_close: Some(EventHandler::new(|_| {
                FORM_CLOSED.with(|cell| cell.set(true));
            })),
        }
    }
}

#[component]
fn BrewEditRoute(id: String) -> Element {
    rsx! {
        BrewForm { id: Some(id) }
    }
}

/// フォームを、ルーターの文脈と依存を与えて描く探り。
#[component]
fn FormProbe() -> Element {
    let (services, auth) = forms_services();
    let _services = use_context_provider(|| services);
    let _auth = use_context_provider(|| auth);
    let _session = use_context_provider(|| Signal::new(SessionStatus::SignedIn));
    let _notice = use_context_provider(|| Signal::new(None::<String>));
    let _revision = use_context_provider(|| Signal::new(0_u64));
    let path = FORMS_PATH.with(|slot| slot.borrow().clone());
    rsx! {
        HistoryProvider {
            history: move |_| Rc::new(MemoryHistory::with_initial_path(&path)) as Rc<dyn History>,
            Router::<TestRoute> {}
        }
    }
}

/// フォームを 1 つ描き、読み込みの応答を待つ。
async fn mount_form(path: &str, transport: Rc<FakeTransport>) -> web_sys::Element {
    FORMS_TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport));
    FORMS_PATH.with(|slot| *slot.borrow_mut() = path.to_string());
    let root = mount(FormProbe).await;
    for _ in 0..10 {
        tick().await;
    }
    root
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

/// input の値を読む。
fn input_value(element: &web_sys::Element) -> String {
    element.unchecked_ref::<web_sys::HtmlInputElement>().value()
}

/// placeholder に値が入っていないか (date input と time input では使わない)。
fn has_no_placeholder(element: &web_sys::Element) -> bool {
    element
        .get_attribute("placeholder")
        .unwrap_or_default()
        .is_empty()
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

/// 入力欄の値を変えて input を送る。
fn type_value(element: &web_sys::Element, value: &str) {
    element
        .unchecked_ref::<web_sys::HtmlInputElement>()
        .set_value(value);
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    let event = web_sys::Event::new_with_event_init_dict("input", &init)
        .expect("the input event must be created");
    element
        .dispatch_event(&event)
        .expect("the input event must dispatch");
}

/// 購入の応答 (購入日 `2026-10-01`、焙煎日 `2026-09-20`)。
fn purchase_json() -> serde_json::Value {
    json!({
        "id": "p1",
        "shop_id": null,
        "name": "豆",
        "producer": null,
        "origin": null,
        "region": null,
        "process": null,
        "variety": null,
        "flavor_notes": [],
        "purchased_on": "2026-10-01",
        "roast": null,
        "roast_date": "2026-09-20",
        "price_amount": null,
        "price_currency": null,
        "weight_grams": null,
        "photo_key": null,
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
        "shop": null,
    })
}

/// 項目の揃った購入の応答 (引き継ぎの検査用。issue 0058)。
fn filled_purchase_json() -> serde_json::Value {
    json!({
        "id": "p1",
        "shop_id": "s1",
        "name": "Ethiopia Guji",
        "producer": "Producer",
        "origin": "Ethiopia",
        "region": "Guji",
        "process": "Natural",
        "variety": "Heirloom",
        "flavor_notes": ["berry", "floral"],
        "purchased_on": "2026-10-01",
        "roast": "Medium",
        "roast_date": "2026-09-20",
        "price_amount": 1980,
        "price_currency": "USD",
        "weight_grams": 200,
        "photo_key": "photos/p1",
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
        "favorited_at": "2026-10-02T00:00:00.000Z",
        "shop": {
            "id": "s1",
            "name": "Shop One",
            "address": null,
            "created_at": "2026-10-01T00:00:00.000Z",
            "updated_at": "2026-10-01T00:00:00.000Z",
            "favorited_at": null,
        },
    })
}

/// 404 の応答 (元の購入が無いとき)。
fn not_found_json() -> String {
    json!({"error": {"code": "not_found", "message": "x"}}).to_string()
}

/// ラベルの欄の input を探す。ラベルの先頭の文字で比べる (必須の印は数えない)。
fn field_input(root: &web_sys::Element, label: &str) -> web_sys::Element {
    elements(root, ".form .field")
        .into_iter()
        .find(|field| {
            field
                .query_selector(".lbl")
                .ok()
                .flatten()
                .and_then(|lbl| lbl.first_child())
                .and_then(|text| text.text_content())
                .is_some_and(|text| text.trim() == label)
        })
        .and_then(|field| field.query_selector("input").ok().flatten())
        .unwrap_or_else(|| panic!("the field {label} must have an input"))
}

/// Flavor Notes のチップの文字 (先頭の文字だけ。削除のアイコンの文字は数えない)。
fn chip_labels(root: &web_sys::Element) -> Vec<String> {
    elements(root, ".form .chips .chip")
        .iter()
        .map(|chip| {
            chip.first_child()
                .and_then(|text| text.text_content())
                .unwrap_or_default()
                .trim()
                .to_string()
        })
        .collect()
}

/// 抽出の応答 (`2026-10-01T00:30:00.000Z` は +540 分で 2026-10-01 09:30)。
fn brew_json() -> serde_json::Value {
    json!({
        "id": "b1",
        "purchase_id": "p1",
        "brewed_at": "2026-10-01T00:30:00.000Z",
        "dose_grams": null,
        "water_grams": null,
        "water_temp_c": null,
        "brew_time_seconds": null,
        "method": null,
        "grind_setting": null,
        "rating": null,
        "notes": null,
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
        "purchase": purchase_json(),
    })
}

/// 購入の登録のフォームが、購入日と焙煎日を date input で描く (完了条件 1、3)。
#[wasm_bindgen_test]
async fn the_purchase_new_form_draws_the_dates_with_the_native_picker() {
    install_styles();
    set_theme("paper");
    let root = mount_form("/purchases/new", Rc::new(FakeTransport::new(vec![]))).await;

    assert_eq!(count(&root, "input[type='date']"), 2);
    assert_eq!(count(&root, "input[type='time']"), 0);
    let dates = elements(&root, "input[type='date']");
    // 購入日は端末のタイムゾーンでの当日を既定値にする (FR-9)。焙煎日は空。
    assert_eq!(input_value(&dates[0]), "2026-09-25");
    assert_eq!(input_value(&dates[1]), "");
    // 形式の案内は help に出し、placeholder と icon は使わない (0049 の原本の更新による)。
    assert_eq!(count(&root, ".form .field .help"), 2);
    assert!(dates.iter().all(has_no_placeholder));
    assert_eq!(count(&root, "input[type='date'] ~ .icon"), 0);
}

/// 過去の購入をもとにした登録のフォームは、商品の項目と店と焙煎度を引き継ぎ、購入日は当日、
/// 焙煎日、写真、価格、重量は空にする。引き継いだだけでは変更に数えず、確認なしで閉じる
/// (issue 0058)。
#[wasm_bindgen_test]
async fn the_purchase_form_from_a_purchase_prefills_the_product_fields_the_shop_and_the_roast() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    FORM_CLOSED.with(|cell| cell.set(false));
    let transport = Rc::new(FakeTransport::new(vec![FakeTransport::response(
        200,
        &filled_purchase_json().to_string(),
    )]));
    let root = mount_form("/purchases/new-from", transport.clone()).await;

    // 元の購入を読む。題は登録のまま。
    assert_eq!(transport.requests().len(), 1);
    assert_eq!(transport.requests()[0].path, "/api/purchases/p1");
    assert_eq!(transport.requests()[0].method.as_str(), "GET");
    let title = select(&root, ".appbar .ttl")
        .text_content()
        .unwrap_or_default();
    assert_eq!(title.trim(), "Log a purchase");
    assert_eq!(count(&root, ".banner"), 0);

    // 商品の項目と店と焙煎度を引き継ぐ。
    assert_eq!(
        input_value(&field_input(&root, "Product name")),
        "Ethiopia Guji"
    );
    assert_eq!(input_value(&field_input(&root, "Producer")), "Producer");
    assert_eq!(input_value(&field_input(&root, "Origin")), "Ethiopia");
    assert_eq!(input_value(&field_input(&root, "Region")), "Guji");
    assert_eq!(input_value(&field_input(&root, "Process")), "Natural");
    assert_eq!(input_value(&field_input(&root, "Variety")), "Heirloom");
    assert_eq!(
        chip_labels(&root),
        vec!["berry".to_string(), "floral".to_string()]
    );
    let shop = select(&root, ".form .picker .n")
        .text_content()
        .unwrap_or_default();
    assert_eq!(shop.trim(), "Shop One");
    assert_eq!(input_value(&field_input(&root, "Roast")), "Medium");

    // 購入日は当日、焙煎日、価格、重量は空、写真は無い。通貨は既定値。
    let dates = elements(&root, "input[type='date']");
    assert_eq!(input_value(&dates[0]), "2026-09-25");
    assert_eq!(input_value(&dates[1]), "");
    assert_eq!(input_value(&field_input(&root, "Price")), "");
    assert_eq!(input_value(&field_input(&root, "Weight")), "");
    assert_eq!(count(&root, ".form .photo img"), 0);
    let currency = select(&root, ".form select");
    assert_eq!(
        currency
            .unchecked_ref::<web_sys::HtmlSelectElement>()
            .value(),
        "JPY"
    );

    // 引き継いだだけの状態は変更に数えず、確認を出さずに閉じる。
    click(&select(&root, "button[aria-label='Cancel']"));
    settle().await;
    assert_eq!(count(&root, ".dialog"), 0);
    assert!(
        FORM_CLOSED.with(|cell| cell.get()),
        "the prefilled form without changes must close"
    );
}

/// 引き継いだ後に値を変えると、閉じる前に確認を出す (issue 0058、0054)。
#[wasm_bindgen_test]
async fn the_prefilled_purchase_form_confirms_before_discarding_the_changes() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    FORM_CLOSED.with(|cell| cell.set(false));
    let transport = Rc::new(FakeTransport::new(vec![FakeTransport::response(
        200,
        &filled_purchase_json().to_string(),
    )]));
    let root = mount_form("/purchases/new-from", transport).await;
    type_value(&field_input(&root, "Product name"), "Ethiopia Guji 2");
    settle().await;
    click(&select(&root, "button[aria-label='Cancel']"));
    settle().await;
    assert_eq!(count(&root, ".dialog"), 1);
    assert!(!FORM_CLOSED.with(|cell| cell.get()));
    click(&select(&root, ".dialog .acts .btn.primary"));
    settle().await;
    assert!(FORM_CLOSED.with(|cell| cell.get()));
}

/// 引き継いだ登録のフォームの保存は、新しい購入の登録 (POST) になり、元の購入は更新しない
/// (issue 0058)。
#[wasm_bindgen_test]
async fn the_prefilled_purchase_form_saves_a_new_purchase() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    FORM_SAVED.with(|cell| cell.set(false));
    let mut created = filled_purchase_json();
    created["id"] = serde_json::json!("p2");
    let transport = Rc::new(FakeTransport::new(vec![
        FakeTransport::response(200, &filled_purchase_json().to_string()),
        FakeTransport::response(201, &created.to_string()),
    ]));
    let root = mount_form("/purchases/new-from", transport.clone()).await;
    assert_eq!(
        input_value(&field_input(&root, "Product name")),
        "Ethiopia Guji"
    );
    click(&select(&root, ".appbar .acts .btn.primary"));
    settle().await;
    let requests = transport.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].method.as_str(), "POST");
    assert_eq!(requests[1].path, "/api/purchases");
    assert!(
        FORM_SAVED.with(|cell| cell.get()),
        "the prefilled form must report the save"
    );
}

/// 空の ID (`/purchases/new?from=`) は引き継ぐ元が無いものとして扱い、何も読まずに空の
/// 登録のフォームになる (issue 0058)。
#[wasm_bindgen_test]
async fn the_purchase_form_from_an_empty_source_is_empty_without_a_request() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let transport = Rc::new(FakeTransport::new(vec![]));
    let root = mount_form("/purchases/new-from-empty", transport.clone()).await;
    assert_eq!(transport.requests().len(), 0);
    assert_eq!(count(&root, ".banner"), 0);
    assert_eq!(count(&root, ".empty"), 0);
    assert_eq!(count(&root, ".form"), 1);
    let title = select(&root, ".appbar .ttl")
        .text_content()
        .unwrap_or_default();
    assert_eq!(title.trim(), "Log a purchase");
    assert_eq!(input_value(&field_input(&root, "Product name")), "");
}

/// 元の購入が無いとき (404) は空の登録のフォームになり、案内は出ない (issue 0058)。
#[wasm_bindgen_test]
async fn the_purchase_form_from_a_missing_purchase_is_empty_without_a_banner() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let transport = Rc::new(FakeTransport::new(vec![FakeTransport::response(
        404,
        &not_found_json(),
    )]));
    let root = mount_form("/purchases/new-from", transport.clone()).await;
    assert_eq!(transport.requests().len(), 1);
    assert_eq!(count(&root, ".banner"), 0);
    assert_eq!(count(&root, ".form"), 1);
    assert_eq!(input_value(&field_input(&root, "Product name")), "");
    assert_eq!(chip_labels(&root), Vec::<String>::new());
    let shop = select(&root, ".form .picker .n")
        .text_content()
        .unwrap_or_default();
    assert_eq!(shop.trim(), "No shop");
    let dates = elements(&root, "input[type='date']");
    assert_eq!(input_value(&dates[0]), "2026-09-25");
}

/// 404 以外の失敗では空のフォームの上に再試行のバナーを出し、再試行で引き継ぐ
/// (issue 0058)。
#[wasm_bindgen_test]
async fn the_purchase_form_from_a_failed_load_retries_and_then_prefills() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let transport = Rc::new(FakeTransport::new(vec![
        FakeTransport::response(500, "{}"),
        FakeTransport::response(200, &filled_purchase_json().to_string()),
    ]));
    let root = mount_form("/purchases/new-from", transport.clone()).await;
    // 空のフォームと再試行のバナー。
    assert_eq!(count(&root, ".form"), 1);
    assert_eq!(count(&root, ".form .banner"), 1);
    assert_eq!(count(&root, ".form .banner .act"), 1);
    assert_eq!(input_value(&field_input(&root, "Product name")), "");

    // 再試行で読み直し、引き継ぐ。
    click(&select(&root, ".form .banner .act"));
    settle().await;
    assert_eq!(transport.requests().len(), 2);
    assert_eq!(transport.requests()[1].path, "/api/purchases/p1");
    assert_eq!(count(&root, ".banner"), 0);
    assert_eq!(
        input_value(&field_input(&root, "Product name")),
        "Ethiopia Guji"
    );
    assert_eq!(input_value(&field_input(&root, "Roast")), "Medium");
}

/// 失敗の後に入力を始めていたら、再試行で読み直しても入力を上書きしない (issue 0058)。
#[wasm_bindgen_test]
async fn the_purchase_form_retry_does_not_overwrite_the_typed_values() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let transport = Rc::new(FakeTransport::new(vec![
        FakeTransport::response(500, "{}"),
        FakeTransport::response(200, &filled_purchase_json().to_string()),
    ]));
    let root = mount_form("/purchases/new-from", transport.clone()).await;
    assert_eq!(count(&root, ".form .banner"), 1);
    type_value(&field_input(&root, "Product name"), "Typed");
    settle().await;

    click(&select(&root, ".form .banner .act"));
    settle().await;
    assert_eq!(transport.requests().len(), 2);
    assert_eq!(count(&root, ".banner"), 0);
    assert_eq!(input_value(&field_input(&root, "Product name")), "Typed");
    assert_eq!(input_value(&field_input(&root, "Producer")), "");
    assert_eq!(chip_labels(&root), Vec::<String>::new());
    assert_eq!(input_value(&field_input(&root, "Roast")), "");
    // 入力は変更に数えたまま。
    click(&select(&root, "button[aria-label='Cancel']"));
    settle().await;
    assert_eq!(count(&root, ".dialog"), 1);
}

/// 購入の編集のフォームが、読み込んだ購入日と焙煎日を date input で描く (完了条件 1、3)。
#[wasm_bindgen_test]
async fn the_purchase_edit_form_draws_the_dates_with_the_native_picker() {
    install_styles();
    set_theme("paper");
    let root = mount_form(
        "/purchases/p1/edit",
        Rc::new(FakeTransport::new(vec![FakeTransport::response(
            200,
            &purchase_json().to_string(),
        )])),
    )
    .await;

    assert_eq!(count(&root, "input[type='date']"), 2);
    let dates = elements(&root, "input[type='date']");
    assert_eq!(input_value(&dates[0]), "2026-10-01");
    assert_eq!(input_value(&dates[1]), "2026-09-20");
    assert!(dates.iter().all(has_no_placeholder));
    assert_eq!(count(&root, "input[type='date'] ~ .icon"), 0);
}

/// 抽出の登録のフォームが、抽出日を date input、時刻を time input で描く (完了条件 1、2、3)。
#[wasm_bindgen_test]
async fn the_brew_new_form_draws_the_date_and_time_with_the_native_pickers() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let root = mount_form("/brews/new", Rc::new(FakeTransport::new(vec![]))).await;

    assert_eq!(count(&root, "input[type='date']"), 1);
    assert_eq!(count(&root, "input[type='time']"), 1);
    // 既定値は端末のタイムゾーンでの現在の日時 (FR-11)。
    let date = select(&root, "input[type='date']");
    let time = select(&root, "input[type='time']");
    assert_eq!(input_value(&date), "2026-09-25");
    assert_eq!(input_value(&time), "12:30");
    // 抽出方法には例をプレースホルダーで出す (0054)。
    assert_eq!(count(&root, "input[placeholder='e.g. Pour over']"), 1);
    // 形式の案内は help に出し、placeholder と icon は使わない (0049 の原本の更新による)。
    assert_eq!(count(&root, ".form .field .help"), 2);
    assert!(has_no_placeholder(&date));
    assert!(has_no_placeholder(&time));
    assert_eq!(count(&root, "input[type='date'] ~ .icon"), 0);
    assert_eq!(count(&root, "input[type='time'] ~ .icon"), 0);
    // 日付と時刻の枠と案内の縦の位置が揃う (0054 の修正)。
    let boxes = elements(&root, ".form .grid2 .field .box");
    assert_eq!(boxes.len(), 6);
    let date_box = boxes[0].get_bounding_client_rect();
    let time_box = boxes[1].get_bounding_client_rect();
    assert!(
        (date_box.top() - time_box.top()).abs() < 1.0,
        "the date and time boxes must align: date={date_box:?} time={time_box:?}"
    );
    let helps = elements(&root, ".form .field .help");
    assert_eq!(helps.len(), 2);
    assert!(
        (helps[0].get_bounding_client_rect().top() - helps[1].get_bounding_client_rect().top())
            .abs()
            < 1.0,
        "the date and time help texts must align"
    );
}

/// 抽出の編集のフォームが、読み込んだ抽出日時を date input と time input で描く
/// (完了条件 1、2、3)。
#[wasm_bindgen_test]
async fn the_brew_edit_form_draws_the_date_and_time_with_the_native_pickers() {
    install_styles();
    set_theme("paper");
    let root = mount_form(
        "/brews/b1/edit",
        Rc::new(FakeTransport::new(vec![FakeTransport::response(
            200,
            &brew_json().to_string(),
        )])),
    )
    .await;

    assert_eq!(count(&root, "input[type='date']"), 1);
    assert_eq!(count(&root, "input[type='time']"), 1);
    let date = select(&root, "input[type='date']");
    let time = select(&root, "input[type='time']");
    assert_eq!(input_value(&date), "2026-10-01");
    assert_eq!(input_value(&time), "09:30");
    assert!(has_no_placeholder(&date));
    assert!(has_no_placeholder(&time));
    assert_eq!(count(&root, "input[type='date'] ~ .icon"), 0);
    assert_eq!(count(&root, "input[type='time'] ~ .icon"), 0);
}

/// フォームに変更があるときは、閉じる前に破棄の確認を出す (0054)。
#[wasm_bindgen_test]
async fn the_brew_form_confirms_before_discarding_the_changes() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    FORM_CLOSED.with(|cell| cell.set(false));
    let root = mount_form("/brews/new-close", Rc::new(FakeTransport::new(vec![]))).await;

    // 変更が無いときは確認を出さずに閉じる。
    click(&select(&root, "button[aria-label='Cancel']"));
    settle().await;
    assert_eq!(count(&root, ".dialog"), 0);
    assert!(
        FORM_CLOSED.with(|cell| cell.get()),
        "the form without changes must close"
    );

    // 値を変えると、閉じる前に確認を出す。
    FORM_CLOSED.with(|cell| cell.set(false));
    let dose = &elements(&root, ".form input.in")[2];
    type_value(dose, "15");
    settle().await;
    click(&select(&root, "button[aria-label='Cancel']"));
    settle().await;
    assert_eq!(count(&root, ".dialog"), 1);
    let dialog = select(&root, ".dialog").text_content().unwrap_or_default();
    assert!(dialog.contains("Close without saving?"), "{dialog}");
    assert!(
        !FORM_CLOSED.with(|cell| cell.get()),
        "the form must not close before the confirmation"
    );

    // キャンセルすると閉じない。
    click(&select(&root, ".dialog .acts .btn.text"));
    settle().await;
    assert_eq!(count(&root, ".dialog"), 0);
    assert_eq!(count(&root, ".screen"), 1);
    assert!(!FORM_CLOSED.with(|cell| cell.get()));

    // 破棄すると閉じる。
    click(&select(&root, "button[aria-label='Cancel']"));
    settle().await;
    click(&select(&root, ".dialog .acts .btn.primary"));
    settle().await;
    assert_eq!(count(&root, ".dialog"), 0);
    assert!(
        FORM_CLOSED.with(|cell| cell.get()),
        "the discard must close the form"
    );
}
