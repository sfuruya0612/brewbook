//! アプリの起動と依存の配線 (0038、0040)。

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_router::{navigator, router, Outlet};

#[cfg(target_arch = "wasm32")]
use dioxus_router::Router;

#[cfg(target_arch = "wasm32")]
use crate::auth::AuthServices;
use crate::auth::{check_session, SessionStatus};
#[cfg(target_arch = "wasm32")]
use crate::i18n::{browser_language, resolve_language, set_language};
use crate::records::RecordServices;
use crate::router::Route;
#[cfg(target_arch = "wasm32")]
use crate::settings::SettingsServices;

/// 画面のルート。言語を決め、画面の依存を束ねて配り、ルーターを組み立てる (ADR-0017)。
///
/// Web だけを対象にするため (ADR-0017)、ブラウザの実装を束ねるこの画面は wasm のビルドだけに
/// 含める。
#[cfg(target_arch = "wasm32")]
#[component]
pub fn App() -> Element {
    // UI の言語を決める (FR-16)。ブラウザの言語が日本語なら日本語、それ以外は英語。
    set_language(resolve_language(browser_language().as_deref()));

    // 記録の依存を束ねて画面に配る (0041 が使う)。
    let api = crate::api::create_client();
    let _services = use_context_provider(|| RecordServices::web(api.clone()));
    // 設定の依存を束ねて画面に配る (0043 が使う)。
    let _settings = use_context_provider(|| SettingsServices::web(api.clone()));
    // 認証の依存を束ねて画面に配る (0040 が使う)。
    let _auth = use_context_provider(|| AuthServices::web(api));
    // 起動時のセッション確認の結果。画面はこれを見て遷移を決める。
    let _session = use_context_provider(|| Signal::new(SessionStatus::Checking));
    // 記録の変更の通知 (0041)。一覧はこれを受けて先頭から読み直す。
    let _revision = use_context_provider(|| Signal::new(0_u64));
    // 保存などの通知 (スナックバー)。画面が文言を入れ、数秒後に消える。
    let _notice = use_context_provider(|| Signal::new(None::<String>));

    rsx! {
        Router::<Route> {}
    }
}

/// ログインの状態に応じた遷移先を決める (FR-1、FR-2)。遷移が不要なときは None。
///
/// Flutter の go_router の redirect と同じ判定にする。確認中と確認できなかったときは
/// 遷移させず (ホームが再試行を促す)、未認証ではログインと登録以外をログイン画面へ戻し、
/// ログイン済みでログインと登録を開いたときはホームへ戻す。
pub fn guard_destination(route: &Route, status: SessionStatus) -> Option<Route> {
    let on_login = matches!(route, Route::Login {});
    let on_register = matches!(route, Route::Register { .. });
    match status {
        SessionStatus::Checking | SessionStatus::Unknown => None,
        SessionStatus::SignedOut if on_login || on_register => None,
        SessionStatus::SignedOut => Some(Route::Login {}),
        SessionStatus::SignedIn if on_login || on_register => Some(Route::Home {}),
        SessionStatus::SignedIn => None,
    }
}

/// 全ての画面を包む (経路の台帳の `#[layout]`)。
///
/// 起動時に `GET /api/passkeys` でセッションを確認し (FR-1、FR-2)、401 の応答では
/// セッションが失われたことを記録し、遷移の判定がログイン画面へ戻す (ADR-0007)。
#[component]
pub fn AppShell() -> Element {
    let services = use_context::<RecordServices>();
    let mut session = use_context::<Signal<SessionStatus>>();
    let navigator = navigator();
    let router = router();
    let api = services.api.clone();
    use_hook(move || {
        api.set_on_unauthorized(Rc::new(move || {
            // Signal は Copy のため、Fn の closure では可変の束縛を作ってから設定する。
            let mut session = session;
            session.set(SessionStatus::SignedOut);
        }));
    });
    use_future(move || {
        let api = services.api.clone();
        async move {
            session.set(check_session(&api).await);
        }
    });
    // 経路の変更とセッションの変更のたびに遷移の判定をやり直す (FR-1、FR-2)。
    use_effect(move || {
        let route = router.current::<Route>();
        // 画面数の成功指標 (PRD の成功指標) のため、現在の経路をアプリのルート要素に出す (0044)。
        #[cfg(target_arch = "wasm32")]
        set_data_route(crate::router::route_name(&route));
        if let Some(destination) = guard_destination(&route, session()) {
            let _ = navigator.replace(destination);
        }
    });
    rsx! {
        Outlet::<Route> {}
    }
}

/// アプリのルート要素 (`#main`) に現在の経路を出す (0044)。
///
/// E2E (`frontend:test-same-origin`) が画面数の成功指標を数えるために読む。
#[cfg(target_arch = "wasm32")]
fn set_data_route(name: &str) {
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("main"))
    {
        let _ = element.set_attribute("data-route", name);
    }
}
