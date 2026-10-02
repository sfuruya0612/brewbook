//! 画面 (0038、0040)。
//!
//! 0040 が認証の画面 (ログイン、登録) を入れ、それ以外の経路は仮の画面 ([`Placeholder`]) の
//! ままにする。残りの画面は 0041 以降が入れる。

use dioxus::prelude::*;
use dioxus_router::{navigator, use_route};

use crate::i18n::{t, Key};
use crate::router::{fallback_destination, Route};

mod login;
mod register;

pub use login::LoginScreen;
pub use register::{register_submit, RegisterScreen, RegisterSubmit};

/// 画面の実装が入るまでの仮の画面。経路の題を表示する。
///
/// 経路の動的な値 (`id`、`token`、`segments`) は、Dioxus のルーターが経路の型から渡すために
/// prop として持つ。0040 以降の画面が経路ごとの実装に置き換える。
#[component]
pub fn Placeholder(
    #[props(default)] id: String,
    #[props(default)] token: Option<String>,
    #[props(default)] segments: Vec<String>,
) -> Element {
    let _ = (id, token, segments);
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
