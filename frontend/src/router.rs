//! 画面の経路の台帳とルーター (ADR-0017)。
//!
//! 経路の名前は経路のパターンと同じにし、画面数の成功指標 (PRD の成功指標) で「ルーターに
//! 登録した画面の種類」を数えられるようにする (Flutter の `AppRoutes` と同じ)。
//!
//! 未知の経路はホームへ戻す (Dioxus のルーターの fallback)。Flutter の go_router は既定の
//! エラー画面を出すため挙動が違うが、意図的な変更として記録する (0038)。

use dioxus::prelude::*;
use dioxus_router::Routable;

use crate::app::AppShell;
use crate::screens::{
    BrewDetailScreen, BrewEditScreen, BrewFormScreen, HomeScreen, LoginScreen, NotFound,
    ProductEditScreen, ProductFormScreen, ProductListScreen, PurchaseDetailScreen,
    PurchaseEditScreen, PurchaseFormScreen, PurchaseListScreen, RegisterScreen, SettingsScreen,
    ShopEditScreen, ShopFormScreen, ShopListScreen, StatsScreen,
};

/// 画面の経路の台帳 (ADR-0017)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AppRoute {
    /// 経路の名前。経路のパターンと同じにする。
    pub name: &'static str,

    /// 経路のパターン。
    pub pattern: &'static str,
}

/// 台帳の 1 行を作る (名前とパターンを同じにする)。
const fn route(pattern: &'static str) -> AppRoute {
    AppRoute {
        name: pattern,
        pattern,
    }
}

/// Flutter の `AppRoutes` と同じ 18 経路 (0038)。
///
/// 順序は Flutter の台帳と同じにする。経路の照合の順序は Dioxus のルーターが特異度で決める。
pub const APP_ROUTES: [AppRoute; 18] = [
    route("/login"),
    route("/register"),
    route("/"),
    route("/settings"),
    route("/brews/new"),
    route("/brews/:id"),
    route("/brews/:id/edit"),
    route("/purchases"),
    route("/purchases/new"),
    route("/purchases/:id"),
    route("/purchases/:id/edit"),
    route("/products"),
    route("/products/new"),
    route("/products/:id/edit"),
    route("/shops"),
    route("/shops/new"),
    route("/shops/:id/edit"),
    route("/stats"),
];

/// 画面の経路 (Dioxus のルーター)。
///
/// 全ての経路を実装済みの画面に割り当てる (0038、0040、0041、0042、0043)。
#[derive(Routable, Clone, PartialEq, Debug)]
pub enum Route {
    #[layout(AppShell)]
    /// ログイン (FR-2)。
    #[route("/login", LoginScreen)]
    Login {},
    /// 登録用トークンによるパスキーの登録 (FR-1)。トークンはクエリで渡す。
    #[route("/register?:token", RegisterScreen)]
    Register { token: Option<String> },
    /// ホーム (抽出の一覧。FR-11)。
    #[route("/", HomeScreen)]
    Home {},
    /// 設定 (パスキーの管理、エクスポート、アカウントの削除、ログアウト。FR-3、FR-4、FR-14、FR-15)。
    #[route("/settings", SettingsScreen)]
    Settings {},
    /// 抽出の登録 (FR-11)。
    #[route("/brews/new", BrewFormScreen)]
    BrewNew {},
    /// 抽出の詳細 (FR-11)。
    #[route("/brews/:id", BrewDetailScreen)]
    BrewDetail { id: String },
    /// 抽出の編集 (FR-11)。
    #[route("/brews/:id/edit", BrewEditScreen)]
    BrewEdit { id: String },
    /// 購入の一覧 (FR-9)。
    #[route("/purchases", PurchaseListScreen)]
    Purchases {},
    /// 購入の登録 (FR-9)。
    #[route("/purchases/new", PurchaseFormScreen)]
    PurchaseNew {},
    /// 購入の詳細 (FR-9、FR-10、FR-18)。
    #[route("/purchases/:id", PurchaseDetailScreen)]
    PurchaseDetail { id: String },
    /// 購入の編集 (FR-9)。
    #[route("/purchases/:id/edit", PurchaseEditScreen)]
    PurchaseEdit { id: String },
    /// 商品の一覧 (FR-7)。
    #[route("/products", ProductListScreen)]
    Products {},
    /// 商品の登録 (FR-7)。
    #[route("/products/new", ProductFormScreen)]
    ProductNew {},
    /// 商品の編集 (FR-7)。
    #[route("/products/:id/edit", ProductEditScreen)]
    ProductEdit { id: String },
    /// 店の一覧 (FR-6)。
    #[route("/shops", ShopListScreen)]
    Shops {},
    /// 店の登録 (FR-6)。
    #[route("/shops/new", ShopFormScreen)]
    ShopNew {},
    /// 店の編集 (FR-6)。
    #[route("/shops/:id/edit", ShopEditScreen)]
    ShopEdit { id: String },
    /// 統計 (FR-18)。
    #[route("/stats", StatsScreen)]
    Stats {},
    /// 未知の経路 (Dioxus のルーターの fallback)。ホームへ戻す (0038)。
    #[route("/:..segments", NotFound)]
    NotFound { segments: Vec<String> },
}

/// 未知の経路 (fallback) の遷移先。ホームへ戻す (0038)。
pub fn fallback_destination() -> Route {
    Route::Home {}
}

/// 現在の経路を `data-route` 属性に出す名前 (0044)。
///
/// 画面数の成功指標 (PRD の成功指標) で「表示した画面の種類」を数えるため、経路のパターンを
/// 返す。`APP_ROUTES` の名前と一致させ、動的な経路もパターンの形 (`/brews/:id`) にする。
/// 未知の経路は `/:..segments` とする。
pub fn route_name(route: &Route) -> &'static str {
    match route {
        Route::Login {} => "/login",
        Route::Register { .. } => "/register",
        Route::Home {} => "/",
        Route::Settings {} => "/settings",
        Route::BrewNew {} => "/brews/new",
        Route::BrewDetail { .. } => "/brews/:id",
        Route::BrewEdit { .. } => "/brews/:id/edit",
        Route::Purchases {} => "/purchases",
        Route::PurchaseNew {} => "/purchases/new",
        Route::PurchaseDetail { .. } => "/purchases/:id",
        Route::PurchaseEdit { .. } => "/purchases/:id/edit",
        Route::Products {} => "/products",
        Route::ProductNew {} => "/products/new",
        Route::ProductEdit { .. } => "/products/:id/edit",
        Route::Shops {} => "/shops",
        Route::ShopNew {} => "/shops/new",
        Route::ShopEdit { .. } => "/shops/:id/edit",
        Route::Stats {} => "/stats",
        Route::NotFound { .. } => "/:..segments",
    }
}
