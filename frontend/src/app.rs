//! アプリの起動と依存の配線 (0038)。

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_router::{navigator, Outlet};

#[cfg(target_arch = "wasm32")]
use dioxus_router::Router;

use crate::auth::{check_session, SessionStatus};
#[cfg(target_arch = "wasm32")]
use crate::i18n::{browser_language, resolve_language, set_language};
use crate::records::RecordServices;
use crate::router::Route;

/// 画面のルート。言語を決め、記録の依存を束ねて画面に配り、ルーターを組み立てる (ADR-0017)。
///
/// Web だけを対象にするため (ADR-0017)、ブラウザの実装を束ねるこの画面は wasm のビルドだけに
/// 含める。
#[cfg(target_arch = "wasm32")]
#[component]
pub fn App() -> Element {
    // UI の言語を決める (FR-16)。ブラウザの言語が日本語なら日本語、それ以外は英語。
    set_language(resolve_language(browser_language().as_deref()));

    // 記録の依存を束ねて画面に配る (0041 が使う)。
    let _services = use_context_provider(|| RecordServices::web(crate::api::create_client()));
    // 起動時のセッション確認の結果。画面はこれを見て遷移を決める。
    let _session = use_context_provider(|| Signal::new(SessionStatus::Checking));

    rsx! {
        Router::<Route> {}
    }
}

/// 401 の応答で遷移させる経路 (FR-2)。テストで固定するために切り出す。
pub fn unauthorized_route() -> Route {
    Route::Login {}
}

/// 全ての画面を包む (経路の台帳の `#[layout]`)。
///
/// 起動時に `GET /api/passkeys` でセッションを確認し (FR-1、FR-2)、401 の応答ではログイン
/// 画面へ遷移させる (ADR-0007)。
#[component]
pub fn AppShell() -> Element {
    let services = use_context::<RecordServices>();
    let mut session = use_context::<Signal<SessionStatus>>();
    let navigator = navigator();
    let api = services.api.clone();
    use_hook(move || {
        let navigator = navigator;
        api.set_on_unauthorized(Rc::new(move || {
            let _ = navigator.replace(unauthorized_route());
        }));
    });
    use_future(move || {
        let api = services.api.clone();
        async move {
            session.set(check_session(&api).await);
        }
    });
    rsx! {
        Outlet::<Route> {}
    }
}
