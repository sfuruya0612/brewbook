//! ヘッダーのハンバーガーメニューのブラウザテスト (0047)。
//!
//! 認証後の画面のヘッダー (`ScreenAppBar`) が組むハンバーガーボタンとメニューの面を実際に
//! 描き、開閉と項目の並びと位置を検査する。ブラウザで動かすため `frontend:test-web`
//! (wasm-bindgen-test) で実行する。

#![cfg(target_arch = "wasm32")]

mod support;

use brew_book_frontend::i18n::{set_language, Language};
use brew_book_frontend::router::Route;
use brew_book_frontend::screens::AppNav;
use brew_book_frontend::ui::{AppBar, IconButton};
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use support::web::{count, install_styles, mount, select, set_theme, tick};

wasm_bindgen_test_configure!(run_in_browser);

/// ハンバーガーメニューの検査用に、認証後の画面のヘッダーと同じ構成を組む。
#[component]
fn AppNavProbe() -> Element {
    rsx! {
        AppBar {
            title: "x",
            leading_icon: "close",
            leading_label: "y",
            actions: rsx! { IconButton { name: "more_vert", label: "z" } },
            menu: rsx! {
                AppNav {
                    current: Route::Home {},
                    on_navigate: move |_| {},
                    on_logout: move |_| {},
                }
            },
            on_home: move |_| {},
        }
    }
}

/// ハンバーガーボタンでメニューが開き、ハンバーガーの下に左寄せで 6 項目とログアウトが
/// 出ることを検査する (0047)。
#[wasm_bindgen_test]
async fn the_app_nav_opens_the_menu_under_the_hamburger() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let root = mount(AppNavProbe).await;
    assert_eq!(count(&root, ".menu"), 0);

    let hamburger = select(&root, ".appbar .nav .iconbtn");
    hamburger.unchecked_ref::<web_sys::HtmlElement>().click();
    for _ in 0..5 {
        tick().await;
    }

    assert_eq!(count(&root, ".menu"), 1);
    // 区切りの div は 1 つで、ログアウトは区切りの後の項目として並ぶ (原本と同じ組み方)。
    assert_eq!(count(&root, ".menu > div.sep"), 1);
    // 抽出、購入、商品、店、統計、設定の 6 項目とログアウトが並ぶ (区切りは除く)。
    let nodes = root
        .query_selector_all(".menu > div")
        .expect("the selector must be valid");
    let mut items = Vec::new();
    for index in 0..nodes.length() {
        if let Some(text) = nodes.item(index).and_then(|node| node.text_content()) {
            if !text.is_empty() {
                items.push(text);
            }
        }
    }
    assert_eq!(
        items,
        [
            "Brews",
            "Purchases",
            "Products",
            "Shops",
            "Stats",
            "Settings",
            "Log out"
        ]
    );
    // メニューはハンバーガーの下に、左端を揃えて開く。
    let button = hamburger.get_bounding_client_rect();
    let menu = select(&root, ".menu").get_bounding_client_rect();
    assert!(
        (menu.left() - button.left()).abs() < 1.0,
        "the menu must be left-aligned with the hamburger: menu={menu:?} button={button:?}"
    );
    assert!(
        menu.top() >= button.bottom() - 1.0,
        "the menu must open under the hamburger: menu={menu:?} button={button:?}"
    );
    // 項目を押すとメニューが閉じる。
    let products = (0..nodes.length())
        .filter_map(|index| nodes.item(index))
        .find(|node| node.text_content().as_deref() == Some("Products"))
        .expect("the Products item must be in the menu");
    products.unchecked_ref::<web_sys::HtmlElement>().click();
    for _ in 0..5 {
        tick().await;
    }
    assert_eq!(count(&root, ".menu"), 0);
}
