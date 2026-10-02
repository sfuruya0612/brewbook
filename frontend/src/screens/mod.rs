//! 画面 (0038、0040、0041、0042)。
//!
//! 認証の画面 (ログイン、登録) と、記録の画面 (ホーム、抽出、購入、商品、店) と、統計の画面を
//! 入れる。設定の画面は 0043 が入れるまで仮の画面 ([`Placeholder`]) にする。

use dioxus::prelude::*;
use dioxus_router::{navigator, use_route};

use crate::i18n::{t, Key};
use crate::router::{fallback_destination, Route};

mod login;
mod register;

pub mod records;
pub mod stats;

pub use login::LoginScreen;
pub use records::{
    brew_detail::BrewDetailScreen,
    brew_form::{BrewEditScreen, BrewFormScreen},
    home::HomeScreen,
    product_form::{ProductEditScreen, ProductFormScreen},
    product_list::ProductListScreen,
    purchase_detail::PurchaseDetailScreen,
    purchase_form::{PurchaseEditScreen, PurchaseFormScreen},
    purchase_list::PurchaseListScreen,
    shop_form::{ShopEditScreen, ShopFormScreen},
    shop_list::ShopListScreen,
};
pub use register::{register_submit, RegisterScreen, RegisterSubmit};
pub use stats::StatsScreen;

/// 画面の実装が入るまでの仮の画面。経路の題を表示する。
///
/// 設定の経路 (0043 が入れる) だけが使う。経路の動的な値 (`segments`) は、Dioxus の
/// ルーターが経路の型から渡すために prop として持つ。
#[component]
pub fn Placeholder(#[props(default)] segments: Vec<String>) -> Element {
    let _ = segments;
    let route = use_route::<Route>();
    let title = t(title_key(&route));
    rsx! {
        div { id: "screen",
            h1 { "{title}" }
        }
    }
}

/// 未知の経路の画面 (Dioxus のルーターの fallback)。ホームへ戻す (0038)。
#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let _ = segments;
    let navigator = navigator();
    use_effect(move || {
        let _ = navigator.replace(fallback_destination());
    });
    rsx! {
        div { id: "not-found" }
    }
}

/// 画面の題の翻訳のキー。
fn title_key(route: &Route) -> Key {
    match route {
        Route::Login {} => Key::LoginTitle,
        Route::Register { .. } => Key::RegisterTitle,
        Route::Home {} => Key::HomeTitle,
        Route::Settings {} => Key::SettingsTitle,
        Route::BrewNew {} => Key::BrewNewTitle,
        Route::BrewDetail { .. } => Key::BrewDetailTitle,
        Route::BrewEdit { .. } => Key::BrewEditTitle,
        Route::Purchases {} => Key::PurchasesTitle,
        Route::PurchaseNew {} => Key::PurchaseNewTitle,
        Route::PurchaseDetail { .. } => Key::PurchaseDetailTitle,
        Route::PurchaseEdit { .. } => Key::PurchaseEditTitle,
        Route::Products {} => Key::ProductsTitle,
        Route::ProductNew {} => Key::ProductNewTitle,
        Route::ProductEdit { .. } => Key::ProductEditTitle,
        Route::Shops {} => Key::ShopsTitle,
        Route::ShopNew {} => Key::ShopNewTitle,
        Route::ShopEdit { .. } => Key::ShopEditTitle,
        Route::Stats {} => Key::StatsTitle,
        Route::NotFound { .. } => Key::HomeTitle,
    }
}
