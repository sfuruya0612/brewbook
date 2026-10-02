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
use crate::screens::{LoginScreen, NotFound, Placeholder, RegisterScreen};

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
/// 画面の実装は 0040 以降が入れる。0038 では全ての経路を仮の画面 ([`Placeholder`]) に割り当て、
/// 経路の台帳とルーターの配線を確かめる。
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
    #[route("/", Placeholder)]
    Home {},
    /// 設定 (パスキーの管理、エクスポート、アカウントの削除、ログアウト。FR-3、FR-4、FR-14、FR-15)。
    #[route("/settings", Placeholder)]
    Settings {},
    /// 抽出の登録 (FR-11)。
    #[route("/brews/new", Placeholder)]
    BrewNew {},
    /// 抽出の詳細 (FR-11)。
    #[route("/brews/:id", Placeholder)]
    BrewDetail { id: String },
    /// 抽出の編集 (FR-11)。
    #[route("/brews/:id/edit", Placeholder)]
    BrewEdit { id: String },
    /// 購入の一覧 (FR-9)。
    #[route("/purchases", Placeholder)]
    Purchases {},
    /// 購入の登録 (FR-9)。
    #[route("/purchases/new", Placeholder)]
    PurchaseNew {},
    /// 購入の詳細 (FR-9、FR-10、FR-18)。
    #[route("/purchases/:id", Placeholder)]
    PurchaseDetail { id: String },
    /// 購入の編集 (FR-9)。
    #[route("/purchases/:id/edit", Placeholder)]
    PurchaseEdit { id: String },
    /// 商品の一覧 (FR-7)。
    #[route("/products", Placeholder)]
    Products {},
    /// 商品の登録 (FR-7)。
    #[route("/products/new", Placeholder)]
    ProductNew {},
    /// 商品の編集 (FR-7)。
    #[route("/products/:id/edit", Placeholder)]
    ProductEdit { id: String },
    /// 店の一覧 (FR-6)。
    #[route("/shops", Placeholder)]
    Shops {},
    /// 店の登録 (FR-6)。
    #[route("/shops/new", Placeholder)]
    ShopNew {},
    /// 店の編集 (FR-6)。
    #[route("/shops/:id/edit", Placeholder)]
    ShopEdit { id: String },
    /// 統計 (FR-18)。
    #[route("/stats", Placeholder)]
    Stats {},
    /// 未知の経路 (Dioxus のルーターの fallback)。ホームへ戻す (0038)。
    #[route("/:..segments", NotFound)]
    NotFound { segments: Vec<String> },
}

/// 未知の経路 (fallback) の遷移先。ホームへ戻す (0038)。
pub fn fallback_destination() -> Route {
    Route::Home {}
}
