//! デザインシステムのブラウザテスト (0039)。
//!
//! `docs/design/tokens.css` と `src/ui/design.css` をテストのページに注入し、部品を実際に
//! 描いて計算済みスタイルを取り、原本の値 (docs/design/tokens.css) と比べる。ブラウザで動かす
//! ため `dioxus:test-web` (wasm-bindgen-test) で実行する。
//!
//! 画面の幅は `frontend/webdriver.json` の `--window-size` で 1280 px にし、2 段組の
//! メディアクエリ (840 px 以上) を効かせる。

#![cfg(target_arch = "wasm32")]

use brew_book_frontend::ui::{
    AppBar, Banner, Button, ButtonVariant, ChartFrame, ChartSection, Chip, ChipVariant,
    ConfirmDialog, Fab, Field, Icon, IconButton, Ledger, LedgerRow, ListRow, ListThumb,
    NavigationRail, PickerTile, RailItem, Rating, RatingInput, ReferenceChain, ReferenceTile,
    RowValue, Snackbar, StatTile, StatTiles, TagChip, TextField, WideLayout, WidePage,
};
use dioxus::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// デザイントークンの原本 (docs/design/tokens.css)。
const TOKENS_CSS: &str = include_str!("../../docs/design/tokens.css");
/// 部品の CSS (docs/design/components/bundle.css の写し)。
const DESIGN_CSS: &str = include_str!("../src/ui/design.css");

/// テストのページの document。
fn document() -> web_sys::Document {
    web_sys::window()
        .expect("the test must run in a browser")
        .document()
        .expect("the page must have a document")
}

/// 原本の CSS をページに 1 回だけ注入する。
///
/// 原本は Tailwind のビルドと同じく `@layer base` に入れる。原本が持つプレビュー用の型の
/// クラス (`.wordmark` など) が部品のクラス (components レイヤー) に勝たないようにするため。
fn install_styles() {
    let document = document();
    if document.get_element_by_id("design-system-styles").is_some() {
        return;
    }
    let style = document
        .create_element("style")
        .expect("a style element must be creatable");
    style.set_id("design-system-styles");
    style.set_text_content(Some(&format!(
        "@layer base {{\n{TOKENS_CSS}\n}}\n{DESIGN_CSS}"
    )));
    document
        .head()
        .expect("the page must have a head")
        .append_child(&style)
        .expect("the style must be attached");
}

/// テーマを data-theme で切り替える (paper と night)。
fn set_theme(theme: &str) {
    document()
        .document_element()
        .expect("the page must have a root element")
        .set_attribute("data-theme", theme)
        .expect("data-theme must be settable");
}

/// ブラウザのタスクを 1 つ進める。
async fn tick() {
    let promise = js_sys::Promise::resolve(&wasm_bindgen::JsValue::UNDEFINED);
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// 部品を body の下に描き、その入れ物を返す。
///
/// body の下に置くのは、計算済みスタイルを取るために要素が文書に入っている必要があるため。
async fn mount(component: fn() -> Element) -> web_sys::Element {
    let document = document();
    let container = document
        .create_element("div")
        .expect("a container must be creatable");
    document
        .body()
        .expect("the page must have a body")
        .append_child(&container)
        .expect("the container must be attached");
    let vdom = VirtualDom::new(component);
    dioxus::web::launch::launch_virtual_dom(
        vdom,
        dioxus::web::Config::new().rootelement(container.clone()),
    );
    for _ in 0..5 {
        tick().await;
    }
    container
}

/// 要素の計算済みスタイルの値を取る。
fn computed(element: &web_sys::Element, property: &str) -> String {
    web_sys::window()
        .expect("the test must run in a browser")
        .get_computed_style(element)
        .expect("getComputedStyle must succeed")
        .expect("the element must be in the document")
        .get_property_value(property)
        .expect("the property must be readable")
}

/// 要素の疑似要素の計算済みスタイルの値を取る。
fn computed_pseudo(element: &web_sys::Element, pseudo: &str, property: &str) -> String {
    web_sys::window()
        .expect("the test must run in a browser")
        .get_computed_style_with_pseudo_elt(element, pseudo)
        .expect("getComputedStyle must succeed")
        .expect("the element must be in the document")
        .get_property_value(property)
        .expect("the property must be readable")
}

/// 入れ物の中の最初の一致を返す。
fn select(root: &web_sys::Element, selector: &str) -> web_sys::Element {
    root.query_selector(selector)
        .expect("the selector must be valid")
        .unwrap_or_else(|| panic!("the element must match {selector}"))
}

/// 入れ物の中の一致の数を返す。
fn count(root: &web_sys::Element, selector: &str) -> u32 {
    root.query_selector_all(selector)
        .expect("the selector must be valid")
        .length()
}

#[component]
fn PrimaryButtonProbe() -> Element {
    rsx! {
        Button { label: "x", variant: ButtonVariant::Primary }
    }
}

/// ボタンの色、余白、角丸、書体が原本の値と一致することを検査する (Paper)。
#[wasm_bindgen_test]
async fn the_button_matches_the_paper_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(PrimaryButtonProbe).await;
    let button = select(&root, ".btn.primary");

    // --roast #4a2f1c、--on-roast #f7efe3。
    assert_eq!(computed(&button, "background-color"), "rgb(74, 47, 28)");
    assert_eq!(computed(&button, "color"), "rgb(247, 239, 227)");
    // 高さ 48 px、角丸 --radius-md 8 px、左右の余白 --space-6 24 px。
    assert_eq!(computed(&button, "height"), "48px");
    assert_eq!(computed(&button, "border-top-left-radius"), "8px");
    assert_eq!(computed(&button, "padding-left"), "24px");
    // 書体 --font-sans の 15 px の 500。
    assert!(computed(&button, "font-family").contains("IBM Plex Sans JP"));
    assert_eq!(computed(&button, "font-size"), "15px");
    assert_eq!(computed(&button, "font-weight"), "500");
}

/// data-theme の切り替えで Night の値になることを検査する。
#[wasm_bindgen_test]
async fn the_data_theme_switches_between_paper_and_night() {
    install_styles();
    set_theme("paper");
    let root = mount(PrimaryButtonProbe).await;
    let button = select(&root, ".btn.primary");
    assert_eq!(computed(&button, "background-color"), "rgb(74, 47, 28)");

    set_theme("night");
    // Night の --roast #dbb890、--on-roast #1b1511。
    assert_eq!(computed(&button, "background-color"), "rgb(219, 184, 144)");
    assert_eq!(computed(&button, "color"), "rgb(27, 21, 17)");
    assert_eq!(computed(&button, "height"), "48px");
    set_theme("paper");
}

#[component]
fn FieldProbe() -> Element {
    rsx! {
        Field { label: "x", required: true,
            TextField { value: "15.0", mono: true, unit: "g".to_string() }
        }
    }
}

/// 入力欄の色、余白、角丸、書体が原本の値と一致することを検査する。
#[wasm_bindgen_test]
async fn the_field_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(FieldProbe).await;
    let box_ = select(&root, ".field .box");
    let input = select(&root, ".field .box .in");

    // 地 --paper-sunken #eae0d1、枠 --line-strong #94826a、角丸 --radius-sm 4 px、高さ 48 px。
    assert_eq!(computed(&box_, "background-color"), "rgb(234, 224, 209)");
    assert_eq!(computed(&box_, "border-top-color"), "rgb(148, 130, 106)");
    assert_eq!(computed(&box_, "border-top-left-radius"), "4px");
    assert_eq!(computed(&box_, "height"), "48px");
    assert_eq!(computed(&box_, "padding-left"), "12px");
    // 数値は --font-mono と tabular-nums (docs/design/README.md のタイポグラフィ)。
    assert!(computed(&input, "font-family").contains("IBM Plex Mono"));
    assert_eq!(computed(&input, "font-variant-numeric"), "tabular-nums");
    // placeholder は原本の .ph と同じ --ink-faint #6f5f4e。
    assert_eq!(
        computed_pseudo(&input, "::placeholder", "color"),
        "rgb(111, 95, 78)"
    );
    // 単位は caption の 12 px の --ink-muted #6a5847。
    let unit = select(&root, ".field .box .unit");
    assert_eq!(computed(&unit, "font-size"), "12px");
    assert_eq!(computed(&unit, "color"), "rgb(106, 88, 71)");
    // 項目名は label の 13 px の --ink-muted。
    let label = select(&root, ".field .lbl");
    assert_eq!(computed(&label, "font-size"), "13px");
    assert_eq!(computed(&label, "color"), "rgb(106, 88, 71)");
}

#[component]
fn LedgerProbe() -> Element {
    rsx! {
        Ledger {
            LedgerRow { label: "a", value: "15.0".to_string(), unit: "g".to_string() }
            LedgerRow { label: "b" }
        }
    }
}

/// 帳簿の値が等幅で右揃え、未設定が --ink-faint になることを検査する。
#[wasm_bindgen_test]
async fn the_ledger_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(LedgerProbe).await;
    let ledger = select(&root, ".ledger");
    let value = select(&root, ".ledger .v");

    assert_eq!(computed(&ledger, "display"), "grid");
    assert!(computed(&value, "font-family").contains("IBM Plex Mono"));
    assert_eq!(computed(&value, "font-variant-numeric"), "tabular-nums");
    assert_eq!(computed(&value, "text-align"), "right");
    assert_eq!(computed(&value, "font-size"), "15px");
    // 単位は sans の caption 12 px の --ink-muted。
    let unit = select(&root, ".ledger .v .u");
    assert!(computed(&unit, "font-family").contains("IBM Plex Sans JP"));
    assert_eq!(computed(&unit, "font-size"), "12px");
    assert_eq!(computed(&unit, "color"), "rgb(106, 88, 71)");
    // 未設定は --ink-faint #6f5f4e の sans。
    let unset = select(&root, ".ledger .v.unset");
    assert_eq!(computed(&unset, "color"), "rgb(111, 95, 78)");
    assert!(computed(&unset, "font-family").contains("IBM Plex Sans JP"));
    // 項目名は label の 13 px の --ink-muted。
    let key = select(&root, ".ledger .k");
    assert_eq!(computed(&key, "font-size"), "13px");
    assert_eq!(computed(&key, "color"), "rgb(106, 88, 71)");
}

#[component]
fn IconProbe() -> Element {
    rsx! {
        Icon { name: "settings" }
        Icon { name: "settings", muted: true }
    }
}

/// アイコンが Material Icons Outlined の 24 px で、色が --ink と --ink-muted になることを検査する。
#[wasm_bindgen_test]
async fn the_icon_is_material_icons_outlined_at_24_px() {
    install_styles();
    set_theme("paper");
    let root = mount(IconProbe).await;
    let icon = select(&root, ".icon");
    let muted = select(&root, ".icon.muted");

    assert!(
        computed(&icon, "font-family").contains("Material Icons Outlined"),
        "the icon must use Material Icons Outlined: {}",
        computed(&icon, "font-family")
    );
    assert_eq!(computed(&icon, "font-size"), "24px");
    assert_eq!(computed(&icon, "color"), "rgb(43, 29, 19)");
    assert_eq!(computed(&muted, "color"), "rgb(106, 88, 71)");
}

#[component]
fn ListRowProbe() -> Element {
    rsx! {
        ListRow { title: "x", subtitle: rsx! { RowValue { text: "2026-10-02" } } }
        ListRow { title: "y", archived: true, selected: true }
    }
}

/// 一覧の行の高さと余白と罫線が原本の値と一致することを検査する。
#[wasm_bindgen_test]
async fn the_list_row_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(ListRowProbe).await;
    let row = select(&root, ".row");

    assert_eq!(computed(&row, "min-height"), "64px");
    assert_eq!(computed(&row, "padding-top"), "12px");
    assert_eq!(computed(&row, "padding-left"), "16px");
    assert_eq!(computed(&row, "border-bottom-width"), "1px");
    assert_eq!(computed(&row, "border-bottom-color"), "rgb(214, 200, 180)");
    assert_eq!(computed(&row, "background-color"), "rgb(244, 237, 226)");
    // 2 行目の日付は等幅。
    let value = select(&root, ".row .sub .v");
    assert!(computed(&value, "font-family").contains("IBM Plex Mono"));
    assert_eq!(computed(&value, "font-variant-numeric"), "tabular-nums");
    // 選択中の地は --roast-soft #e6d6c2、アーカイブ済みの名前は --ink-muted。
    let selected = select(&root, ".row.selected");
    assert_eq!(
        computed(&selected, "background-color"),
        "rgb(230, 214, 194)"
    );
    let archived = select(&root, ".row.archived .name");
    assert_eq!(computed(&archived, "color"), "rgb(106, 88, 71)");
    // バッジは高さ 20 px、角丸 4 px、地 --paper-sunken。
    let badge = select(&root, ".badge");
    assert_eq!(computed(&badge, "height"), "20px");
    assert_eq!(computed(&badge, "border-top-left-radius"), "4px");
    assert_eq!(computed(&badge, "background-color"), "rgb(234, 224, 209)");
    assert_eq!(computed(&badge, "color"), "rgb(106, 88, 71)");
}

#[component]
fn AppBarProbe() -> Element {
    rsx! {
        AppBar {
            title: "x",
            wordmark: true,
            leading_icon: "close",
            leading_label: "y",
            actions: rsx! { IconButton { name: "more_vert", label: "z" } },
        }
    }
}

/// AppBar の高さと罫線と書体が原本の値と一致することを検査する。
#[wasm_bindgen_test]
async fn the_app_bar_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(AppBarProbe).await;
    let bar = select(&root, ".appbar");

    assert_eq!(computed(&bar, "height"), "56px");
    assert_eq!(computed(&bar, "border-bottom-width"), "1px");
    assert_eq!(computed(&bar, "border-bottom-color"), "rgb(214, 200, 180)");
    assert_eq!(computed(&bar, "background-color"), "rgb(244, 237, 226)");
    assert_eq!(computed(&bar, "padding-left"), "16px");
    assert_eq!(computed(&bar, "padding-right"), "8px");
    // ホームの題は wordmark (IBM Plex Serif の 24 px)。
    let title = select(&root, ".appbar .ttl.wordmark");
    assert!(computed(&title, "font-family").contains("IBM Plex Serif"));
    assert_eq!(computed(&title, "font-size"), "24px");
    assert_eq!(computed(&title, "font-weight"), "500");
    // 先頭の操作は 40 px 四方。
    let lead = select(&root, ".appbar .lead");
    assert_eq!(computed(&lead, "width"), "40px");
    assert_eq!(computed(&lead, "height"), "40px");
    // 末尾の操作は 40 px 四方の丸。
    let action = select(&root, ".appbar .iconbtn");
    assert_eq!(computed(&action, "width"), "40px");
    assert_eq!(computed(&action, "border-top-left-radius"), "9999px");
    assert_eq!(computed(&action, "background-color"), "rgba(0, 0, 0, 0)");
}

#[component]
fn ChipProbe() -> Element {
    rsx! {
        Chip { label: "a" }
        Chip { label: "b", selected: true }
        Chip { label: "c", variant: ChipVariant::Tag, on_remove: move |_| {} }
        TagChip { label: "d" }
    }
}

/// チップの高さと角丸と選択中の塗りが原本の値と一致することを検査する。
#[wasm_bindgen_test]
async fn the_chip_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(ChipProbe).await;
    let chip = select(&root, ".chip");

    assert_eq!(computed(&chip, "height"), "32px");
    assert_eq!(computed(&chip, "border-top-left-radius"), "9999px");
    assert_eq!(computed(&chip, "border-top-color"), "rgb(148, 130, 106)");
    assert_eq!(computed(&chip, "font-size"), "13px");
    assert_eq!(computed(&chip, "font-weight"), "500");
    // 選択中は --roast の塗りに --on-roast。
    let selected = select(&root, ".chip.on");
    assert_eq!(computed(&selected, "background-color"), "rgb(74, 47, 28)");
    assert_eq!(computed(&selected, "color"), "rgb(247, 239, 227)");
    // タグは --crema-soft #f3e0c4 の地。
    let tag = select(&root, ".chip.tag");
    assert_eq!(computed(&tag, "background-color"), "rgb(243, 224, 196)");
    // 削除アイコンは 14 px。
    let remove = select(&root, ".chip.tag .icon");
    assert_eq!(computed(&remove, "font-size"), "14px");
    // 一覧の中の小さなタグは高さ 22 px、文字 12 px。
    let mini = select(&root, ".chip.mini");
    assert_eq!(computed(&mini, "height"), "22px");
    assert_eq!(computed(&mini, "font-size"), "12px");
}

#[component]
fn RatingProbe() -> Element {
    rsx! {
        Rating { value: 4, show_value: true }
        RatingInput { value: 3, on_change: move |_| {} }
        RatingInput { value: None, on_change: move |_| {} }
    }
}

/// 評価の丸の寸法と色、等幅の値、未評価の色を検査する。
#[wasm_bindgen_test]
async fn the_rating_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(RatingProbe).await;

    // 一覧と詳細の丸は 10 px で、--line #d6c8b4 と --crema #b8742a。
    assert_eq!(count(&root, ".rating i"), 15);
    assert_eq!(count(&root, ".rating i.on"), 7);
    let dot = select(&root, ".rating i:not(.on)");
    assert_eq!(computed(&dot, "width"), "10px");
    assert_eq!(computed(&dot, "height"), "10px");
    assert_eq!(computed(&dot, "border-top-left-radius"), "9999px");
    assert_eq!(computed(&dot, "background-color"), "rgb(214, 200, 180)");
    let on = select(&root, ".rating i.on");
    assert_eq!(computed(&on, "background-color"), "rgb(184, 116, 42)");
    // 入力の丸は 28 px で、未選択は --line-strong の枠だけ。
    let large = select(&root, ".rating.lg i:not(.on)");
    assert_eq!(computed(&large, "width"), "28px");
    assert_eq!(computed(&large, "height"), "28px");
    assert_eq!(computed(&large, "border-top-width"), "1px");
    assert_eq!(computed(&large, "border-top-color"), "rgb(148, 130, 106)");
    assert_eq!(computed(&large, "background-color"), "rgba(0, 0, 0, 0)");
    // 「4 / 5」は --font-mono と tabular-nums。
    let number = select(&root, ".rating .num");
    assert!(computed(&number, "font-family").contains("IBM Plex Mono"));
    assert_eq!(computed(&number, "font-variant-numeric"), "tabular-nums");
    // 未評価は --ink-faint #6f5f4e。
    let none = select(&root, ".rating .num.none");
    assert_eq!(computed(&none, "color"), "rgb(111, 95, 78)");
}

#[component]
fn ReferenceTileProbe() -> Element {
    rsx! {
        ReferenceTile { kind: "a", name: "b" }
        PickerTile { kind: "c", placeholder: "d" }
    }
}

/// 参照先のタイルの地と角丸、ピッカーの枠が原本の値と一致することを検査する。
#[wasm_bindgen_test]
async fn the_reference_tile_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(ReferenceTileProbe).await;
    let tile = select(&root, ".tile");

    assert_eq!(computed(&tile, "background-color"), "rgb(230, 214, 194)");
    assert_eq!(computed(&tile, "border-top-left-radius"), "8px");
    assert_eq!(computed(&tile, "padding-top"), "12px");
    assert_eq!(computed(&tile, "padding-left"), "16px");
    let kind = select(&root, ".tile .k");
    assert_eq!(computed(&kind, "font-size"), "12px");
    assert_eq!(computed(&kind, "color"), "rgb(106, 88, 71)");
    let name = select(&root, ".tile .n");
    assert_eq!(computed(&name, "font-size"), "15px");
    assert_eq!(computed(&name, "font-weight"), "600");
    // ピッカーは Field と同じ枠。
    let picker = select(&root, ".picker");
    assert_eq!(computed(&picker, "background-color"), "rgb(234, 224, 209)");
    assert_eq!(computed(&picker, "border-top-color"), "rgb(148, 130, 106)");
    assert_eq!(computed(&picker, "border-top-left-radius"), "4px");
    let faint = select(&root, ".picker .faint");
    assert_eq!(computed(&faint, "color"), "rgb(111, 95, 78)");
}

#[component]
fn FeedbackProbe() -> Element {
    rsx! {
        Banner { message: "a", on_retry: Some(EventHandler::new(|_| {})) }
        Snackbar { message: "b" }
        ConfirmDialog { title: "c", message: "d", cancel_label: "e", confirm_label: "f", danger: true }
    }
}

/// バナー、スナックバー、ダイアログの色と角丸が原本の値と一致することを検査する。
#[wasm_bindgen_test]
async fn the_feedback_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(FeedbackProbe).await;
    let banner = select(&root, ".banner");

    assert_eq!(computed(&banner, "background-color"), "rgb(245, 220, 213)");
    assert_eq!(computed(&banner, "color"), "rgb(168, 57, 42)");
    assert_eq!(computed(&banner, "border-top-left-radius"), "8px");
    assert_eq!(computed(&banner, "padding-top"), "12px");
    assert_eq!(computed(&banner, "padding-left"), "16px");
    let retry = select(&root, ".banner .act");
    assert_eq!(computed(&retry, "color"), "rgb(168, 57, 42)");
    assert_eq!(computed(&retry, "font-weight"), "600");
    // スナックバーは --ink の地に --paper の文字。
    let snack = select(&root, ".snack");
    assert_eq!(computed(&snack, "background-color"), "rgb(43, 29, 19)");
    assert_eq!(computed(&snack, "color"), "rgb(244, 237, 226)");
    assert_ne!(computed(&snack, "box-shadow"), "none");
    // ダイアログは --paper-raised の地、内側 --space-6 24 px、角丸 8 px。
    let dialog = select(&root, ".dialog");
    assert_eq!(computed(&dialog, "background-color"), "rgb(252, 248, 241)");
    assert_eq!(computed(&dialog, "border-top-left-radius"), "8px");
    assert_eq!(computed(&dialog, "padding-top"), "24px");
    assert_eq!(computed(&dialog, "max-width"), "320px");
    // 取り消せない操作の肯定は --signal の塗り。
    let confirm = select(&root, ".dialog .btn.danger");
    assert_eq!(computed(&confirm, "background-color"), "rgb(168, 57, 42)");
}

#[component]
fn ChartsProbe() -> Element {
    rsx! {
        StatTiles {
            StatTile { label: "a", value: "12", unit: "杯".to_string() }
            StatTile { label: "b", value: "32" }
        }
        ChartSection { title: "c", unit: "杯 / 日",
            ChartFrame {
                text { x: "0", y: "12", "2" }
            }
        }
    }
}

/// 統計のタイルとグラフの枠の色と寸法が原本の値と一致することを検査する。
#[wasm_bindgen_test]
async fn the_charts_match_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(ChartsProbe).await;
    let tile = select(&root, ".stat-tile");

    assert_eq!(computed(&tile, "background-color"), "rgb(252, 248, 241)");
    assert_eq!(computed(&tile, "border-top-color"), "rgb(214, 200, 180)");
    assert_eq!(computed(&tile, "border-top-left-radius"), "8px");
    assert_eq!(computed(&tile, "padding-left"), "16px");
    // 合計は value-large (mono の 28 px の 500、tabular-nums)。
    let value = select(&root, ".stat-tile .v");
    assert!(computed(&value, "font-family").contains("IBM Plex Mono"));
    assert_eq!(computed(&value, "font-size"), "28px");
    assert_eq!(computed(&value, "font-weight"), "500");
    assert_eq!(computed(&value, "font-variant-numeric"), "tabular-nums");
    // グラフの区画は下に --line の罫線。
    let chart = select(&root, ".chart");
    assert_eq!(computed(&chart, "border-bottom-width"), "1px");
    assert_eq!(
        computed(&chart, "border-bottom-color"),
        "rgb(214, 200, 180)"
    );
    let heading = select(&root, ".chart .h .t");
    assert_eq!(computed(&heading, "font-size"), "16px");
    assert_eq!(computed(&heading, "font-weight"), "600");
    let unit = select(&root, ".chart .h .s");
    assert!(computed(&unit, "font-family").contains("IBM Plex Mono"));
    // 軸と目盛の色と、目盛の数字の書体。
    let axis = select(&root, ".chart svg .axis");
    assert_eq!(computed(&axis, "stroke"), "rgb(148, 130, 106)");
    assert_eq!(computed(&axis, "stroke-width"), "1px");
    let grid = select(&root, ".chart svg .grid");
    assert_eq!(computed(&grid, "stroke"), "rgb(214, 200, 180)");
    let text = select(&root, ".chart svg text");
    assert!(computed(&text, "font-family").contains("IBM Plex Mono"));
    assert_eq!(computed(&text, "font-size"), "11px");
    assert_eq!(computed(&text, "fill"), "rgb(106, 88, 71)");
}

#[component]
fn WideLayoutProbe() -> Element {
    rsx! {
        WideLayout {
            rail: rsx! {
                NavigationRail {
                    items: vec![RailItem {
                        label: "a".to_string(),
                        icon: "settings".to_string(),
                        selected: true,
                        on_click: EventHandler::new(|_| {}),
                    }],
                }
            },
            list: rsx! { div { class: "list" } },
            detail: rsx! { div { class: "detail-content" } },
        }
    }
}

/// 幅 840 px 以上でレール 88 px、一覧 400 px、詳細の内容が最大 720 px になることを検査する。
#[wasm_bindgen_test]
async fn the_wide_layout_matches_the_documented_widths() {
    install_styles();
    set_theme("paper");
    let root = mount(WideLayoutProbe).await;

    let layout = select(&root, ".wide-layout");
    assert_eq!(computed(&layout, "display"), "flex");
    // レールは 88 px。上に印、項目は 72 px。
    let rail = select(&root, ".rail");
    assert_eq!(computed(&rail, "width"), "88px");
    assert_eq!(computed(&rail, "display"), "flex");
    assert_eq!(computed(&rail, "border-right-width"), "1px");
    assert_eq!(computed(&rail, "border-right-color"), "rgb(214, 200, 180)");
    let mark = select(&root, ".rail .mark");
    assert_eq!(computed(&mark, "width"), "40px");
    assert_eq!(computed(&mark, "height"), "40px");
    let item = select(&root, ".rail .item.on");
    assert_eq!(computed(&item, "width"), "72px");
    assert_eq!(computed(&item, "background-color"), "rgb(230, 214, 194)");
    assert_eq!(computed(&item, "color"), "rgb(43, 29, 19)");
    // 一覧は 400 px で右端に --line の罫線。
    let list = select(&root, ".wide-list");
    assert_eq!(computed(&list, "width"), "400px");
    assert_eq!(computed(&list, "border-right-width"), "1px");
    assert_eq!(computed(&list, "border-right-color"), "rgb(214, 200, 180)");
    // 詳細は残りの幅で、内容は最大 720 px。
    let detail = select(&root, ".wide-detail");
    assert_eq!(computed(&detail, "display"), "block");
    let inner = select(&root, ".wide-detail-inner");
    assert_eq!(computed(&inner, "max-width"), "720px");
}

#[component]
fn FabProbe() -> Element {
    rsx! {
        Fab { label: "x", icon: Some("add".to_string()), onclick: move |_| {} }
    }
}

/// 拡張 FAB の色、余白、角丸、書体が原本の値と一致することを検査する。
#[wasm_bindgen_test]
async fn the_fab_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(FabProbe).await;
    let fab = select(&root, ".fab");

    // 地 --roast #4a2f1c、文字 --on-roast #f7efe3、高さ 56 px、角丸 --radius-lg 16 px。
    assert_eq!(computed(&fab, "background-color"), "rgb(74, 47, 28)");
    assert_eq!(computed(&fab, "color"), "rgb(247, 239, 227)");
    assert_eq!(computed(&fab, "height"), "56px");
    assert_eq!(computed(&fab, "border-top-left-radius"), "16px");
    assert_eq!(computed(&fab, "padding-left"), "16px");
    assert_eq!(computed(&fab, "font-size"), "15px");
    assert_eq!(computed(&fab, "font-weight"), "500");
    // 影は --shadow-float。
    assert!(
        !computed(&fab, "box-shadow").is_empty(),
        "the FAB must have the floating shadow"
    );
    // アイコンは 24 px の currentColor。
    let icon = select(&root, ".fab .icon");
    assert_eq!(computed(&icon, "font-size"), "24px");
}

#[component]
fn ListThumbProbe() -> Element {
    rsx! {
        ListThumb { alt: "x" }
        ListThumb { src: Some("photo.png".to_string()), alt: "x" }
    }
}

/// 写真の枠の大きさ、角丸、地、アイコンの色が原本の値と一致することを検査する。
#[wasm_bindgen_test]
async fn the_list_thumb_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(ListThumbProbe).await;
    let thumb = select(&root, ".thumb");

    // 44 px、角丸 --radius-sm 4 px、地 --paper-sunken #eae0d1。
    assert_eq!(computed(&thumb, "width"), "44px");
    assert_eq!(computed(&thumb, "height"), "44px");
    assert_eq!(computed(&thumb, "border-top-left-radius"), "4px");
    assert_eq!(computed(&thumb, "background-color"), "rgb(234, 224, 209)");
    // 写真が無いときの印は --ink-faint #6f5f4e の 22 px。
    let icon = select(&root, ".thumb .icon");
    assert_eq!(computed(&icon, "color"), "rgb(111, 95, 78)");
    assert_eq!(computed(&icon, "font-size"), "22px");
    // 写真があるときは枠に収める。
    let image = select(&root, ".thumb img");
    assert_eq!(computed(&image, "object-fit"), "cover");
}

#[component]
fn ReferenceChainProbe() -> Element {
    rsx! {
        ReferenceChain {
            ReferenceTile { kind: "a", name: "b" }
            div { class: "link" }
            ReferenceTile { kind: "c", name: "d" }
        }
    }
}

/// 参照の連鎖が縦に並び、間の線が原本の値になることを検査する。
#[wasm_bindgen_test]
async fn the_reference_chain_matches_the_tokens() {
    install_styles();
    set_theme("paper");
    let root = mount(ReferenceChainProbe).await;
    let chain = select(&root, ".chain");

    assert_eq!(computed(&chain, "display"), "flex");
    assert_eq!(computed(&chain, "flex-direction"), "column");
    assert_eq!(computed(&chain, "gap"), "8px");
    // 間の線は 2 px の --line-strong #94826a。
    let link = select(&root, ".chain .link");
    assert_eq!(computed(&link, "width"), "2px");
    assert_eq!(computed(&link, "height"), "12px");
    assert_eq!(computed(&link, "background-color"), "rgb(148, 130, 106)");
    assert_eq!(count(&root, ".chain .tile"), 2);
}

/// index.html の起動スクリプトが、OS の設定に応じて data-theme を決めることを検査する
/// (既定 Paper、`prefers-color-scheme: dark` のとき Night。2026-10-02 の所有者の決定)。
#[wasm_bindgen_test]
fn the_boot_script_sets_the_theme_from_the_system() {
    let window = web_sys::window().expect("the test must run in a browser");
    let key = wasm_bindgen::JsValue::from_str("matchMedia");
    let original = js_sys::Reflect::get(&window, &key).unwrap_or(wasm_bindgen::JsValue::UNDEFINED);
    for (dark, expected) in [(false, "paper"), (true, "night")] {
        let mock = js_sys::Function::new_with_args(
            "query",
            &format!("return {{ matches: {dark}, addEventListener: function () {{}} }};"),
        );
        js_sys::Reflect::set(&window, &key, &mock).expect("matchMedia must be replaceable");
        run_boot_script();
        let theme = document()
            .document_element()
            .expect("the page must have a root element")
            .get_attribute("data-theme");
        assert_eq!(
            theme.as_deref(),
            Some(expected),
            "the boot script must set the theme for prefers-color-scheme: dark = {dark}"
        );
    }
    // 差し替えた matchMedia を元に戻す。Window.prototype の関数は Reflect で取れないことが
    // あるため、取れなかったときは own のプロパティを消して元の状態に戻す (残すと以降の
    // テストの Dioxus の起動が壊れる)。
    if original.is_undefined() {
        js_sys::Reflect::delete_property(&window, &key).expect("matchMedia must be restorable");
    } else {
        js_sys::Reflect::set(&window, &key, &original).expect("matchMedia must be restorable");
    }
}

/// index.html の `<script>` の中身を取り出して実行する。
fn run_boot_script() {
    let html = include_str!("../index.html");
    let open = html
        .find("<script>")
        .expect("index.html must have a script");
    let start = open + "<script>".len();
    let end = html[start..]
        .find("</script>")
        .expect("the script must be closed")
        + start;
    let document = document();
    let script = document
        .create_element("script")
        .expect("a script element must be creatable");
    script.set_text_content(Some(&html[start..end]));
    document
        .head()
        .expect("the page must have a head")
        .append_child(&script)
        .expect("the script must be attached");
    script.remove();
}

#[component]
fn WidePageProbe() -> Element {
    rsx! {
        WidePage {
            rail: rsx! { div { class: "rail" } },
            div { class: "content" }
        }
    }
}

/// 1 面の配置 (レールと残り幅の内容) が原本の値になることを検査する (0042 のレビューの指摘)。
#[wasm_bindgen_test]
async fn the_wide_page_places_the_rail_and_the_content() {
    install_styles();
    set_theme("paper");
    let root = mount(WidePageProbe).await;

    let page = select(&root, ".wide-page");
    assert_eq!(computed(&page, "display"), "flex");
    // 高さは画面いっぱい (min-height: 100vh は計算済みのスタイルでは px になる)。
    assert_ne!(computed(&page, "min-height"), "0px");
    // レールは 88 px で、内容は残り幅いっぱい。
    let rail = select(&root, ".wide-page .rail");
    assert_eq!(computed(&rail, "width"), "88px");
    let main = select(&root, ".wide-page-main");
    assert_eq!(computed(&main, "flex-grow"), "1");
    assert_eq!(computed(&main, "min-width"), "0px");
}
