//! 画面 (0038、0040、0041、0042、0043)。
//!
//! 認証の画面 (ログイン、登録)、記録の画面 (ホーム、抽出、購入、商品、店)、統計の画面、設定の
//! 画面を入れる。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::router::fallback_destination;

mod login;
mod register;

pub mod app_nav;
pub mod records;
pub mod settings;
pub mod stats;

pub use app_nav::{AppNav, AppNavItem, ScreenAppBar};
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
pub use settings::SettingsScreen;
pub use stats::StatsScreen;

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
