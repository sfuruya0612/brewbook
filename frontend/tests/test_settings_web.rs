//! 設定の画面の配線のブラウザテスト (0043)。
//!
//! ダウンロードのリンクの組み立て (FR-14) と、最後の 1 つの削除の禁止とアカウントの削除の
//! 確認の配線 (FR-3、FR-15) を、偽の依存を差した画面の描画で確かめる。

#![cfg(target_arch = "wasm32")]

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use brew_book_frontend::api::{ApiClient, Method};
use brew_book_frontend::auth::{AuthServices, SessionStatus};
use brew_book_frontend::records::{PhotoUploader, RecordServices, RecordsApi};
use brew_book_frontend::screens::SettingsScreen;
use brew_book_frontend::settings::{download_link, SettingsServices};
use dioxus::history::{History, MemoryHistory};
use dioxus::prelude::*;
use dioxus_router::components::HistoryProvider;
use dioxus_router::{Routable, Router};
use serde_json::json;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use support::web::{computed, count, install_styles, mount, select, set_theme, tick};
use support::{
    FakeClock, FakeFileDownload, FakeImageConverter, FakePasskeyClient, FakePhotoPicker,
    FakeTransport, FakeUploadTransport,
};

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    /// 画面の API の応答 (テストごとに設定する)。
    static SETTINGS_TRANSPORT: RefCell<Option<Rc<FakeTransport>>> = const { RefCell::new(None) };
}

/// 偽の依存を束ねた設定の画面の依存。
fn settings_services() -> (AuthServices, SettingsServices, RecordServices) {
    let transport = SETTINGS_TRANSPORT.with(|slot| {
        slot.borrow()
            .clone()
            .expect("the transport must be set before mounting")
    });
    let api = ApiClient::new(transport);
    let auth = AuthServices::new(api.clone(), Rc::new(FakePasskeyClient::new()));
    let settings = SettingsServices::new(api.clone(), Rc::new(FakeFileDownload::new()));
    let uploader = PhotoUploader::new(
        RecordsApi::new(api.clone()),
        Rc::new(FakeUploadTransport::new()),
    );
    let records = RecordServices::new(
        api,
        Rc::new(FakeClock {
            now: brew_book_frontend::records::values::LocalDateTime::new(2026, 10, 2, 12, 0),
            utc_offset_minutes: 540,
        }),
        Rc::new(FakePhotoPicker { photo: None }),
        Rc::new(FakeImageConverter),
        Rc::new(uploader),
    );
    (auth, settings, records)
}

/// 設定の画面だけを持つテスト用の経路 (ルーターの文脈を用意する)。
#[derive(Routable, Clone, PartialEq, Debug)]
enum TestRoute {
    #[route("/settings", SettingsRoute)]
    Settings {},
}

#[component]
fn SettingsRoute() -> Element {
    rsx! {
        SettingsScreen {}
    }
}

#[component]
fn SettingsProbe() -> Element {
    let (auth, settings, records) = settings_services();
    let _auth = use_context_provider(|| auth);
    let _settings = use_context_provider(|| settings);
    let _records = use_context_provider(|| records);
    let _session = use_context_provider(|| Signal::new(SessionStatus::SignedIn));
    let _notice = use_context_provider(|| Signal::new(None::<String>));
    rsx! {
        HistoryProvider {
            history: move |_| Rc::new(MemoryHistory::with_initial_path("/settings")) as Rc<dyn History>,
            Router::<TestRoute> {}
        }
    }
}

/// ボタンを押す。
fn click(element: &web_sys::Element) {
    element
        .dyn_ref::<web_sys::HtmlElement>()
        .expect("the element must be clickable")
        .click();
}

#[wasm_bindgen_test]
fn the_download_link_points_to_the_object_url_with_the_file_name() {
    let link = download_link("blob:http://localhost/abc", "brewbook-export.json")
        .expect("the download link must be built");

    assert_eq!(link.href(), "blob:http://localhost/abc");
    assert_eq!(link.download(), "brewbook-export.json");
}

/// 最後の 1 つの削除が無効で API を呼ばず、アカウントの削除が確認の後だけ呼ばれることを
/// 検査する (FR-3、FR-15。0043 のレビューの指摘)。
#[wasm_bindgen_test]
async fn the_settings_screen_gates_the_delete_actions() {
    install_styles();
    set_theme("paper");
    let transport = Rc::new(FakeTransport::new(vec![
        // パスキーの一覧 (1 件)。最後の 1 つなので削除は無効。
        FakeTransport::response(
            200,
            &json!({
                "passkeys": [{
                    "id": "k1",
                    "name": "マイキー",
                    "created_at": "2026-10-01T00:00:00.000Z",
                    "last_used_at": null,
                }],
            })
            .to_string(),
        ),
        // アカウントの削除 (確認の後だけ届く)。
        FakeTransport::response(204, ""),
    ]));
    SETTINGS_TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport.clone()));
    let root = mount(SettingsProbe).await;
    for _ in 0..10 {
        tick().await;
    }

    // 一覧が読まれ、1 件の行が出る。
    assert_eq!(count(&root, ".pk"), 1);
    // 最後の 1 つなので削除は無効で、グリフも --ink-faint に落ちる。
    let delete = select(&root, ".pk .iconbtn[disabled]");
    assert_eq!(computed(&delete, "color"), "rgb(111, 95, 78)");
    assert_eq!(
        computed(&select(&root, ".pk .iconbtn[disabled] .icon"), "color"),
        "rgb(111, 95, 78)"
    );
    click(&delete);
    for _ in 0..5 {
        tick().await;
    }
    assert_eq!(
        transport.requests().len(),
        1,
        "the disabled delete must not call the API"
    );

    // アカウントの削除は、確認のダイアログを出すだけでは API を呼ばない。
    click(&select(&root, ".btn.danger-outline"));
    for _ in 0..5 {
        tick().await;
    }
    assert_eq!(count(&root, ".dialog"), 1, "the confirmation must open");
    assert_eq!(
        transport.requests().len(),
        1,
        "the confirmation must not call the API yet"
    );

    // 確認のボタンを押したときだけ呼ぶ。
    click(&select(&root, ".dialog .btn.danger"));
    for _ in 0..10 {
        tick().await;
    }
    let requests = transport.requests();
    assert_eq!(requests.len(), 2, "{requests:?}");
    assert_eq!(requests[1].method, Method::Delete);
    assert_eq!(requests[1].path, "/api/account");
}
