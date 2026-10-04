//! 記録のフォームのブラウザテスト (日付と時刻の入力)。
//!
//! 購入と抽出の登録と編集のフォームを実際に描き、日付は date input、時刻は time input で、
//! 値が `YYYY-MM-DD` と `HH:MM` になることを検査する。ブラウザで動かすため `frontend:test-web`
//! (wasm-bindgen-test) で実行する。表示の形式 (ブラウザの言語による yyyy/mm/dd などの見え方)
//! は検査しない。内部の値の形式だけを確かめる。

#![cfg(target_arch = "wasm32")]

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use brew_book_frontend::api::ApiClient;
use brew_book_frontend::auth::{AuthServices, SessionStatus};
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
    #[route("/purchases/:id/edit", PurchaseEditRoute)]
    PurchaseEdit { id: String },
    #[route("/brews/new", BrewNewRoute)]
    BrewNew {},
    #[route("/brews/:id/edit", BrewEditRoute)]
    BrewEdit { id: String },
}

#[component]
fn PurchaseNewRoute() -> Element {
    rsx! {
        PurchaseForm { id: None }
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

/// 購入の応答 (購入日 `2026-10-01`、焙煎日 `2026-09-20`)。
fn purchase_json() -> serde_json::Value {
    json!({
        "id": "p1",
        "product_id": "pr1",
        "shop_id": null,
        "purchased_on": "2026-10-01",
        "roast": null,
        "roast_date": "2026-09-20",
        "price_amount": null,
        "price_currency": null,
        "weight_grams": null,
        "photo_key": null,
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
        "archived_at": null,
        "product": {
            "id": "pr1",
            "name": "豆",
            "producer": null,
            "origin": null,
            "region": null,
            "process": null,
            "variety": null,
            "flavor_notes": [],
            "created_at": "2026-10-01T00:00:00.000Z",
            "updated_at": "2026-10-01T00:00:00.000Z",
            "archived_at": null,
        },
        "shop": null,
    })
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
        "archived_at": null,
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
    let root = mount_form("/brews/new", Rc::new(FakeTransport::new(vec![]))).await;

    assert_eq!(count(&root, "input[type='date']"), 1);
    assert_eq!(count(&root, "input[type='time']"), 1);
    // 既定値は端末のタイムゾーンでの現在の日時 (FR-11)。
    let date = select(&root, "input[type='date']");
    let time = select(&root, "input[type='time']");
    assert_eq!(input_value(&date), "2026-09-25");
    assert_eq!(input_value(&time), "12:30");
    // 形式の案内は help に出し、placeholder と icon は使わない (0049 の原本の更新による)。
    assert_eq!(count(&root, ".form .field .help"), 2);
    assert!(has_no_placeholder(&date));
    assert!(has_no_placeholder(&time));
    assert_eq!(count(&root, "input[type='date'] ~ .icon"), 0);
    assert_eq!(count(&root, "input[type='time'] ~ .icon"), 0);
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
