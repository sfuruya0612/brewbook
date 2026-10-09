//! ヘッダーのハンバーガーメニューと、認証後の画面のヘッダー (0047)。
//!
//! ハンバーガーボタンとメニューの面は [`AppNav`] が担う。メニューの面は
//! `docs/design/components/AppBar` のガイドラインどおり、ハンバーガーボタンの下に開く
//! 左寄せのドロップダウンにする。認証後の画面は [`ScreenAppBar`] を使い、経路の操作と
//! ログアウトを組む。認証前の画面 (登録) は [`crate::ui::AppBar`] をそのまま使う
//! (ハンバーガーを出さない)。

use dioxus::prelude::*;
use dioxus_router::{navigator, router};

use crate::auth::{logout, message_key, AuthServices, SessionStatus};
use crate::i18n::{t, Key};
use crate::router::Route;
use crate::screens::records::NAV_ENTRIES;
use crate::ui::{AppBar, IconButton};

/// ハンバーガーメニューの 1 項目。
#[derive(Clone, PartialEq)]
pub struct AppNavItem {
    /// 表示する名前 (ARB から取る)。
    pub label: String,
    /// 現在の経路の項目か。押しても遷移しない。
    pub current: bool,
    /// 押したときの動き。
    pub on_click: EventHandler<MouseEvent>,
}

/// メニューの項目を組む (現在の経路と同じ項目は `current` を true にする)。
pub fn nav_items(current: &Route, on_navigate: EventHandler<Route>) -> Vec<AppNavItem> {
    NAV_ENTRIES
        .iter()
        .map(|(key, _icon, route)| AppNavItem {
            label: t(*key).to_string(),
            current: route == current,
            on_click: {
                let route = route.clone();
                EventHandler::new(move |_| on_navigate.call(route.clone()))
            },
        })
        .collect()
}

/// ハンバーガーボタンと、その下に開く左寄せのメニューの面 (0047)。
#[component]
pub fn AppNav(
    /// 現在の経路。同じ項目を押したときは遷移せず、メニューだけ閉じる。
    current: Route,
    /// 経路へ移るときの動き。
    #[props(default)]
    on_navigate: EventHandler<Route>,
    /// ログアウトの動き。
    #[props(default)]
    on_logout: EventHandler<MouseEvent>,
) -> Element {
    let mut open = use_signal(|| false);
    let items = nav_items(&current, on_navigate);
    let menu = open().then(|| {
        rsx! {
            div { class: "menu left",
                for item in items {
                    {
                        let label = item.label;
                        let current = item.current;
                        let on_click = item.on_click;
                        rsx! {
                            div {
                                onclick: move |event| {
                                    open.set(false);
                                    if !current {
                                        on_click.call(event);
                                    }
                                },
                                "{label}"
                            }
                        }
                    }
                }
                div { class: "sep" }
                div {
                    onclick: move |event| {
                        open.set(false);
                        on_logout.call(event);
                    },
                    "{t(Key::LogoutButton)}"
                }
            }
        }
    });
    rsx! {
        div { class: "nav",
            IconButton {
                name: "menu".to_string(),
                label: t(Key::MenuTooltip).to_string(),
                onclick: move |_| open.toggle(),
            }
            {menu}
        }
    }
}

/// 認証後の画面のヘッダー (0047)。
///
/// [`crate::ui::AppBar`] にハンバーガーメニュー ([`AppNav`]) と、印を押してホームへ戻る動きを
/// 組む。最上位の画面はメニューと印を出し、詳細とフォームは出さない (0054)。現在の経路が
/// ホームのときは何もしない (戻るで元の画面に戻れるように、それ以外はホームを積む)。
#[component]
pub fn ScreenAppBar(
    /// 画面の題 (ARB の `*Title` から取る)。
    title: String,
    /// 先頭の操作のアイコン (戻るは `arrow_back_ios_new`、フォームは `close`)。
    #[props(default)]
    leading_icon: Option<String>,
    /// 先頭の操作の名前 (読み上げ用。ARB から取る)。
    #[props(default)]
    leading_label: Option<String>,
    /// 先頭の操作を押したときの動き。
    #[props(default)]
    on_leading: EventHandler<MouseEvent>,
    /// 末尾の操作 (アイコンと文字ボタンで最大 3 つ)。
    #[props(default)]
    actions: Option<Element>,
    /// ハンバーガーメニューを出すか (最上位の画面だけ true)。
    #[props(default = true)]
    menu: bool,
    /// アプリの印を出すか (最上位の画面だけ true)。
    #[props(default = false)]
    brand: bool,
) -> Element {
    let navigator = navigator();
    let router = router();
    let current = router.current::<Route>();
    let auth = use_context::<AuthServices>();
    let mut session = use_context::<Signal<SessionStatus>>();
    let mut notice = use_context::<Signal<Option<String>>>();

    let on_navigate = EventHandler::new(move |route: Route| {
        let _ = navigator.push(route);
    });
    let home = current.clone();
    let on_home = EventHandler::new(move |_| {
        if !matches!(home, Route::Home {}) {
            let _ = navigator.push(Route::Home {});
        }
    });
    let on_logout = EventHandler::new(move |_| {
        let auth = auth.clone();
        spawn(async move {
            match logout(&auth).await {
                Ok(()) => session.set(SessionStatus::SignedOut),
                Err(error) => notice.set(Some(t(message_key(&error)).to_string())),
            }
        });
    });
    let menu = menu.then(|| {
        rsx! {
            AppNav { current, on_navigate, on_logout }
        }
    });
    rsx! {
        AppBar {
            title,
            leading_icon,
            leading_label,
            on_leading,
            actions,
            menu,
            brand,
            on_home: Some(on_home),
        }
    }
}
