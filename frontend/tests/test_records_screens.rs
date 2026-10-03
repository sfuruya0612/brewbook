//! 記録と統計の画面の配線の単体テスト (0041、0042)。
//!
//! 画面の型と prop が存在すること、ルーターの経路が画面に繋がっていることを確かめる
//! (完了条件 1 の「画面があり」の native の確認)。ブラウザでの描画は 0044 の E2E が確かめる。

use std::fs;
use std::path::PathBuf;

use brew_book_frontend::i18n::Key;
use brew_book_frontend::screens::records::NAV_ENTRIES;
use brew_book_frontend::screens::records::{
    brew_detail::{BrewDetail, BrewDetailProps, BrewDetailScreen, BrewDetailScreenProps},
    brew_form::{BrewEditScreen, BrewEditScreenProps, BrewForm, BrewFormProps, BrewFormScreen},
    home::HomeScreen,
    product_form::{
        ProductEditScreen, ProductEditScreenProps, ProductForm, ProductFormProps, ProductFormScreen,
    },
    product_list::ProductListScreen,
    purchase_detail::{PurchaseDetailScreen, PurchaseDetailScreenProps},
    purchase_form::{
        PurchaseEditScreen, PurchaseEditScreenProps, PurchaseForm, PurchaseFormProps,
        PurchaseFormScreen,
    },
    purchase_list::PurchaseListScreen,
    shop_form::{ShopEditScreen, ShopEditScreenProps, ShopForm, ShopFormProps, ShopFormScreen},
    shop_list::ShopListScreen,
};
use brew_book_frontend::screens::stats::{
    charts::RatingHistoryChartProps, RatingHistoryChart, StatsScreen,
};
use dioxus::prelude::Element;

/// クレートのルート (frontend/)。
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// ルーターのソース。
fn router_source() -> String {
    fs::read_to_string(crate_dir().join("src/router.rs")).expect("the router must be readable")
}

#[test]
fn the_record_screens_have_the_expected_props() {
    // 経路に割り当てる画面は、ルーターが渡す prop で組めること (型の確認)。
    let _: fn() -> Element = HomeScreen;
    let _: fn() -> Element = StatsScreen;
    let _: fn() -> Element = BrewFormScreen;
    let _: fn(BrewEditScreenProps) -> Element = BrewEditScreen;
    let _: fn(BrewDetailScreenProps) -> Element = BrewDetailScreen;
    let _: fn() -> Element = PurchaseListScreen;
    let _: fn() -> Element = PurchaseFormScreen;
    let _: fn(PurchaseEditScreenProps) -> Element = PurchaseEditScreen;
    let _: fn(PurchaseDetailScreenProps) -> Element = PurchaseDetailScreen;
    let _: fn() -> Element = ProductListScreen;
    let _: fn() -> Element = ProductFormScreen;
    let _: fn(ProductEditScreenProps) -> Element = ProductEditScreen;
    let _: fn() -> Element = ShopListScreen;
    let _: fn() -> Element = ShopFormScreen;
    let _: fn(ShopEditScreenProps) -> Element = ShopEditScreen;
    // 2 段組の右の面にも出す共通のフォームと詳細。
    let _: fn(BrewFormProps) -> Element = BrewForm;
    let _: fn(BrewDetailProps) -> Element = BrewDetail;
    let _: fn(PurchaseFormProps) -> Element = PurchaseForm;
    let _: fn(ProductFormProps) -> Element = ProductForm;
    let _: fn(ShopFormProps) -> Element = ShopForm;

    // 編集の画面は ID を受け取り、2 段組の面に出せる (props の中身の確認)。
    let edit = BrewEditScreenProps {
        id: "b1".to_string(),
    };
    assert_eq!(edit.id, "b1");
    let detail = BrewDetailScreenProps {
        id: "b1".to_string(),
    };
    assert_eq!(detail.id, "b1");
    let form = BrewFormProps {
        id: Some("b1".to_string()),
        embedded: true,
        on_close: None,
        on_saved: None,
    };
    assert!(form.embedded);
    let detail = BrewDetailProps {
        id: "b1".to_string(),
        embedded: true,
        on_edit: None,
    };
    assert_eq!(detail.id, "b1");
    let shop_form = ShopFormProps {
        id: None,
        embedded: false,
        on_close: None,
        on_saved: None,
    };
    assert!(shop_form.id.is_none());
    let product_form = ProductFormProps {
        id: None,
        embedded: false,
        on_close: None,
        on_saved: None,
    };
    assert!(product_form.id.is_none());
    let purchase_form = PurchaseFormProps {
        id: None,
        embedded: false,
        on_close: None,
        on_saved: None,
    };
    assert!(purchase_form.id.is_none());
    let purchase_detail = PurchaseDetailScreenProps {
        id: "b1".to_string(),
    };
    assert_eq!(purchase_detail.id, "b1");
}

#[test]
fn the_record_routes_are_wired_to_the_record_screens() {
    let source = router_source();
    for (pattern, screen) in [
        ("/", "HomeScreen"),
        ("/brews/new", "BrewFormScreen"),
        ("/brews/:id", "BrewDetailScreen"),
        ("/brews/:id/edit", "BrewEditScreen"),
        ("/purchases", "PurchaseListScreen"),
        ("/purchases/new", "PurchaseFormScreen"),
        ("/purchases/:id", "PurchaseDetailScreen"),
        ("/purchases/:id/edit", "PurchaseEditScreen"),
        ("/products", "ProductListScreen"),
        ("/products/new", "ProductFormScreen"),
        ("/products/:id/edit", "ProductEditScreen"),
        ("/shops", "ShopListScreen"),
        ("/shops/new", "ShopFormScreen"),
        ("/shops/:id/edit", "ShopEditScreen"),
    ] {
        let route = format!("#[route(\"{pattern}\", {screen})]");
        assert!(source.contains(&route), "the router must have {route}");
    }
    // 統計は 0042 が入れた。設定の経路の配線は 0043 のテスト (test_settings.rs) が確かめる。
    assert!(source.contains("#[route(\"/stats\", StatsScreen)]"));
}

/// 認証後の画面 (記録、統計、設定) が ScreenAppBar を使い、登録の画面は ui::AppBar のままで
/// ハンバーガーを出さないことを確かめる (0047)。
#[test]
fn the_signed_in_screens_use_the_screen_app_bar() {
    for path in [
        "src/screens/records/home.rs",
        "src/screens/records/brew_detail.rs",
        "src/screens/records/brew_form.rs",
        "src/screens/records/purchase_list.rs",
        "src/screens/records/purchase_detail.rs",
        "src/screens/records/purchase_form.rs",
        "src/screens/records/product_list.rs",
        "src/screens/records/product_form.rs",
        "src/screens/records/shop_list.rs",
        "src/screens/records/shop_form.rs",
        "src/screens/stats/mod.rs",
        "src/screens/settings.rs",
    ] {
        let source = fs::read_to_string(crate_dir().join(path))
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert!(
            source.contains("ScreenAppBar {"),
            "{path} must use the ScreenAppBar"
        );
        // AppBar は ScreenAppBar の中にだけ現れる (ui::AppBar を直接使わない)。
        assert_eq!(
            source.matches("AppBar").count(),
            source.matches("ScreenAppBar").count(),
            "{path} must not use ui::AppBar directly"
        );
    }

    // 登録の画面は認証前なので ui::AppBar のままで、ハンバーガー (AppNav) を出さない。
    let register = fs::read_to_string(crate_dir().join("src/screens/register.rs"))
        .expect("the register screen must be readable");
    assert!(register.contains("use crate::ui::{AppBar"));
    assert!(register.contains("AppBar { title:"));
    assert!(!register.contains("ScreenAppBar"));
    assert!(!register.contains("AppNav"));

    // ホームの more_vert のメニューは削除し、行き先はハンバーガーメニューに一本化した (0047)。
    let home = fs::read_to_string(crate_dir().join("src/screens/records/home.rs"))
        .expect("the home screen must be readable");
    assert!(!home.contains("more_vert"), "the home menu must be removed");
}

/// ヘッダーのメニューとレールの 1 項目目 (ホーム) が「抽出」になっていることを確かめる (0047)。
///
/// 完了条件はソースの検査を挙げているが、`rail_items` と `AppNav` が共有する並びの実体
/// (`NAV_ENTRIES`) を実行時に読む方が、整形の変更に強く、値を直接確かめられる
/// (0047 のレビューの指摘)。
#[test]
fn the_navigation_entries_start_with_the_brews_label() {
    assert_eq!(
        NAV_ENTRIES[0].0,
        Key::BrewsLabel,
        "the first navigation entry must be the brews label"
    );
}

/// 購入の詳細が、評価の推移の折れ線を 0041 の画面に足していることを確かめる (FR-18)。
#[test]
fn the_purchase_detail_has_the_rating_history_line_chart() {
    let source = fs::read_to_string(crate_dir().join("src/screens/records/purchase_detail.rs"))
        .expect("the purchase detail must be readable");
    assert!(
        source.contains("RatingHistoryChart { entries: ratings() }"),
        "the purchase detail must render the rating history chart with the loaded entries"
    );
    assert!(
        source.contains("rating_history(&id)"),
        "the purchase detail must load the rating history for the purchase"
    );
    // 折れ線の部品の型が存在すること (型の確認)。
    let _: fn(RatingHistoryChartProps) -> Element = RatingHistoryChart;
    let props = RatingHistoryChartProps {
        entries: Vec::new(),
    };
    assert!(props.entries.is_empty());
}

/// プレビューの base64 の符号化が RFC 4648 の標準アルファベットとパディングに従う
/// (0041 のレビューの指摘)。
#[test]
fn the_photo_preview_base64_follows_the_standard() {
    use brew_book_frontend::screens::records::{base64_encode, photo_preview_url};

    // 長さ 0、1、2、3 の端数とパディング。
    assert_eq!(base64_encode(&[]), "");
    assert_eq!(base64_encode(b"f"), "Zg==");
    assert_eq!(base64_encode(b"fo"), "Zm8=");
    assert_eq!(base64_encode(b"foo"), "Zm9v");
    assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
    assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
    assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    // 全バイト (0x00 から 0xFF) でも落ちない。
    let all: Vec<u8> = (0..=255).collect();
    assert_eq!(base64_encode(&all).len(), 344);

    assert_eq!(photo_preview_url(b"f"), "data:image/jpeg;base64,Zg==");
}
