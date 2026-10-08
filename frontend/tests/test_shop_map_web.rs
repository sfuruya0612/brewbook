//! 店のフォームの地図と住所の検索のブラウザテスト (FR-22、ADR-0019)。
//!
//! 店のフォームを実際に描き、次を検査する。
//!
//! - 住所があるときは地図 (Maps Embed API の iframe) を出し、住所が無いときとキーの無いときは出さない。
//! - 店名から住所を検索でき、候補を選ぶと住所の欄に入り、店名は変わらない。
//! - 検索は「住所を検索」の操作でだけ行い、入力のたびには呼ばない。
//! - 候補が無いときの案内と、失敗の再試行の案内を出す。
//!
//! ブラウザで動かすため `frontend:test-web` (wasm-bindgen-test) で実行する。

#![cfg(target_arch = "wasm32")]

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use brew_book_frontend::api::{ApiClient, ApiResponse, TransportError};
use brew_book_frontend::auth::{AuthServices, SessionStatus};
use brew_book_frontend::i18n::{set_language, t, Key, Language};
use brew_book_frontend::records::values::LocalDateTime;
use brew_book_frontend::records::{PhotoUploader, RecordServices, RecordsApi};
use brew_book_frontend::screens::records::shop_form::ShopForm;
use dioxus::history::{History, MemoryHistory};
use dioxus::prelude::*;
use dioxus_router::components::HistoryProvider;
use dioxus_router::{Routable, Router};
use serde_json::{json, Value};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use support::web::{count, install_styles, mount, select, tick};
use support::{
    FakeClock, FakeImageConverter, FakePasskeyClient, FakePhotoPicker, FakeTransport,
    FakeUploadTransport,
};

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    /// 店のフォームのテストが使う偽の送信の実装 (送った要求と返す応答を確かめる)。
    static MAP_TRANSPORT: RefCell<Option<Rc<FakeTransport>>> = const { RefCell::new(None) };

    /// 描く経路。
    static MAP_PATH: RefCell<String> = const { RefCell::new(String::new()) };
}

/// 店のフォームの依存を組む。API は thread_local の偽の送信の実装を使う。
fn map_services() -> (RecordServices, AuthServices) {
    let transport = MAP_TRANSPORT.with(|slot| {
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
            now: LocalDateTime::new(2026, 10, 7, 12, 0),
            utc_offset_minutes: 540,
        }),
        Rc::new(FakePhotoPicker { photo: None }),
        Rc::new(FakeImageConverter),
        Rc::new(uploader),
    );
    (records, auth)
}

/// 店のフォームの検査用の経路。
#[derive(Routable, Clone, PartialEq, Debug)]
enum TestRoute {
    #[route("/shops/new", ShopNewRoute)]
    ShopNew {},
    #[route("/shops/:id/edit", ShopEditRoute)]
    ShopEdit { id: String },
}

#[component]
fn ShopNewRoute() -> Element {
    rsx! {
        ShopForm {}
    }
}

#[component]
fn ShopEditRoute(id: String) -> Element {
    rsx! {
        ShopForm { id: Some(id) }
    }
}

/// 画面の依存を配る探り。
#[component]
fn MapProbe() -> Element {
    let (services, auth) = map_services();
    let _services = use_context_provider(|| services);
    let _auth = use_context_provider(|| auth);
    let _session = use_context_provider(|| Signal::new(SessionStatus::SignedIn));
    let _notice = use_context_provider(|| Signal::new(None::<String>));
    let _revision = use_context_provider(|| Signal::new(0_u64));
    let path = MAP_PATH.with(|slot| slot.borrow().clone());
    rsx! {
        HistoryProvider {
            history: move |_| Rc::new(MemoryHistory::with_initial_path(&path)) as Rc<dyn History>,
            Router::<TestRoute> {}
        }
    }
}

/// 応答を 1 つ作る。
fn response(body: &Value) -> Result<ApiResponse, TransportError> {
    FakeTransport::response(200, &body.to_string())
}

/// 店の応答 (FR-6)。
fn shop_json(id: &str, name: &str, address: Option<&str>) -> Value {
    json!({
        "id": id,
        "user_id": "user",
        "name": name,
        "address": address,
        "created_at": "2026-10-01T00:00:00.000Z",
        "updated_at": "2026-10-01T00:00:00.000Z",
    })
}

/// 経路を描き、応答を待つ。
async fn mount_form(
    path: &str,
    responses: Vec<Result<ApiResponse, TransportError>>,
) -> (web_sys::Element, Rc<FakeTransport>) {
    let transport = Rc::new(FakeTransport::new(responses));
    MAP_TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport.clone()));
    MAP_PATH.with(|slot| *slot.borrow_mut() = path.to_string());
    let root = mount(MapProbe).await;
    settle().await;
    (root, transport)
}

/// ブラウザのタスクを数回進める。
async fn settle() {
    for _ in 0..10 {
        tick().await;
    }
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

/// 要素を押す。
fn click(element: &web_sys::Element) {
    element.unchecked_ref::<web_sys::HtmlElement>().click();
}

/// 入力欄の値を変えて input を送る。
fn type_value(element: &web_sys::Element, value: &str) {
    set_value(element, value);
    dispatch(element, "input");
}

/// 入力欄の値を変えて change を送る (値の確定)。
fn change_value(element: &web_sys::Element, value: &str) {
    set_value(element, value);
    dispatch(element, "change");
}

/// 入力欄の値を設定する。
fn set_value(element: &web_sys::Element, value: &str) {
    element
        .unchecked_ref::<web_sys::HtmlInputElement>()
        .set_value(value);
}

/// 要素にイベントを送る (バブリングあり)。
fn dispatch(element: &web_sys::Element, name: &str) {
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    let event =
        web_sys::Event::new_with_event_init_dict(name, &init).expect("the event must be created");
    element
        .dispatch_event(&event)
        .expect("the event must dispatch");
}

/// 店名と住所の入力欄を返す。
fn fields(root: &web_sys::Element) -> (web_sys::Element, web_sys::Element) {
    let inputs = elements(root, ".form .field .box input");
    assert_eq!(inputs.len(), 2, "the shop form must have two inputs");
    (inputs[0].clone(), inputs[1].clone())
}

#[wasm_bindgen_test]
async fn the_shop_form_shows_the_map_for_the_saved_address() {
    set_language(Language::Japanese);
    install_styles();
    let (root, transport) = mount_form(
        "/shops/s1/edit",
        vec![
            response(&shop_json("s1", "丸山珈琲", Some("長野県北佐久郡軽井沢町"))),
            response(&json!({"embed_api_key": "test-key"})),
        ],
    )
    .await;
    // 編集の読み込みと地図の設定の 2 つを呼ぶ。
    let requests = transport.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, "/api/shops/s1");
    assert_eq!(requests[1].path, "/api/maps/config");

    // 住所があるため地図を出し、住所と言語を URL に入れる (FR-22)。
    let iframe = select(&root, ".map iframe");
    assert_eq!(
        iframe.get_attribute("src").unwrap_or_default(),
        "https://www.google.com/maps/embed/v1/place?key=test-key&q=%E9%95%B7%E9%87%8E%E7%9C%8C%E5%8C%97%E4%BD%90%E4%B9%85%E9%83%A1%E8%BB%BD%E4%BA%95%E6%B2%A2%E7%94%BA&language=ja"
    );
    assert_eq!(
        iframe.get_attribute("title").as_deref(),
        Some(t(Key::MapTitle))
    );
    assert_eq!(iframe.get_attribute("loading").as_deref(), Some("lazy"));
}

#[wasm_bindgen_test]
async fn the_shop_form_searches_the_address_from_the_name() {
    set_language(Language::Japanese);
    install_styles();
    let (root, transport) = mount_form(
        "/shops/new",
        vec![
            response(&json!({"embed_api_key": "test-key"})),
            response(&json!({
                "candidates": [
                    {"name": "丸山珈琲", "address": "長野県北佐久郡軽井沢町"},
                ]
            })),
        ],
    )
    .await;
    // 店名を入力しても検索は呼ばない (操作でだけ呼ぶ。FR-22)。
    let (name_input, _) = fields(&root);
    type_value(&name_input, "丸山珈琲");
    settle().await;
    assert_eq!(
        transport.requests().len(),
        1,
        "only the config must be called"
    );

    // 「住所を検索」を押すと検索の経路を呼ぶ。
    click(&select(&root, ".field-actions .btn"));
    settle().await;
    let requests = transport.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[1].path,
        "/api/place-search?q=%E4%B8%B8%E5%B1%B1%E7%8F%88%E7%90%B2&lang=ja"
    );

    // 候補は名前と住所の 2 行で出す。
    assert_eq!(count(&root, ".candidates button"), 1);
    assert_eq!(
        select(&root, ".candidates .n").text_content().as_deref(),
        Some("丸山珈琲")
    );
    assert_eq!(
        select(&root, ".candidates .s").text_content().as_deref(),
        Some("長野県北佐久郡軽井沢町")
    );

    // 候補を選ぶと住所の欄に入り、店名は変わらない。候補は閉じ、地図が出る。
    click(&select(&root, ".candidates button"));
    settle().await;
    let (name_input, address_input) = fields(&root);
    assert_eq!(input_value(&address_input), "長野県北佐久郡軽井沢町");
    assert_eq!(input_value(&name_input), "丸山珈琲");
    assert_eq!(count(&root, ".candidates button"), 0);
    let src = select(&root, ".map iframe")
        .get_attribute("src")
        .unwrap_or_default();
    assert!(src.contains("q=%E9%95%B7%E9%87%8E%E7%9C%8C"), "{src}");
}

#[wasm_bindgen_test]
async fn the_shop_form_shows_the_empty_and_failed_states_of_the_search() {
    set_language(Language::Japanese);
    install_styles();
    let (root, transport) = mount_form(
        "/shops/new",
        vec![
            response(&json!({"embed_api_key": "test-key"})),
            response(&json!({"candidates": []})),
            FakeTransport::failure("the network is down"),
            response(&json!({
                "candidates": [{"name": "店", "address": "住所"}]
            })),
        ],
    )
    .await;
    let (name_input, _) = fields(&root);
    type_value(&name_input, "店");
    settle().await;

    // 候補が無いときは案内を出し、手入力を続けられる (FR-22)。
    click(&select(&root, ".field-actions .btn"));
    settle().await;
    assert!(
        root.text_content()
            .unwrap_or_default()
            .contains(t(Key::AddressSearchEmpty)),
        "the empty state must be shown"
    );

    // 失敗のときは再試行の案内を出す。
    click(&select(&root, ".field-actions .btn"));
    settle().await;
    assert_eq!(count(&root, ".banner"), 1);
    assert_eq!(count(&root, ".banner .act"), 1);

    // 再試行で検索をやり直す。
    click(&select(&root, ".banner .act"));
    settle().await;
    assert_eq!(transport.requests().len(), 4);
    assert_eq!(count(&root, ".candidates button"), 1);
}

#[wasm_bindgen_test]
async fn the_shop_form_hides_the_map_without_the_key() {
    set_language(Language::Japanese);
    install_styles();
    let (root, _) = mount_form(
        "/shops/new",
        vec![response(&json!({"embed_api_key": null}))],
    )
    .await;
    // 住所を入力して確定しても、キーが無いときは地図を出さない (FR-22)。
    let (_, address_input) = fields(&root);
    change_value(&address_input, "東京都渋谷区");
    settle().await;
    assert_eq!(count(&root, ".map"), 0);
}
