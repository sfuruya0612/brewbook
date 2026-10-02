//! ブラウザで動かすテスト (wasm-bindgen-test) が共有する補助 (0042)。
//!
//! `docs/design/tokens.css` と `src/ui/design.css` をテストのページに注入し、部品を実際に
//! 描いて計算済みスタイルを取る (0039 の `test_ui_web.rs` と同じ手順)。画面の幅は
//! `frontend/webdriver.json` の `--window-size` で 1280 px にする。

#![cfg(target_arch = "wasm32")]

use dioxus::prelude::*;

/// デザイントークンの原本 (docs/design/tokens.css)。
pub const TOKENS_CSS: &str = include_str!("../../../docs/design/tokens.css");

/// 部品の CSS (docs/design/components/bundle.css の写し)。
pub const DESIGN_CSS: &str = include_str!("../../src/ui/design.css");

/// テストのページの document。
pub fn document() -> web_sys::Document {
    web_sys::window()
        .expect("the test must run in a browser")
        .document()
        .expect("the page must have a document")
}

/// 原本の CSS をページに 1 回だけ注入する。
///
/// 原本は Tailwind のビルドと同じく `@layer base` に入れる。原本が持つプレビュー用の型の
/// クラス (`.wordmark` など) が部品のクラス (components レイヤー) に勝たないようにするため。
pub fn install_styles() {
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
pub fn set_theme(theme: &str) {
    document()
        .document_element()
        .expect("the page must have a root element")
        .set_attribute("data-theme", theme)
        .expect("data-theme must be settable");
}

/// ブラウザのタスクを 1 つ進める。
pub async fn tick() {
    let promise = js_sys::Promise::resolve(&wasm_bindgen::JsValue::UNDEFINED);
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// 部品を body の下に描き、その入れ物を返す。
///
/// body の下に置くのは、計算済みスタイルを取るために要素が文書に入っている必要があるため。
pub async fn mount(component: fn() -> Element) -> web_sys::Element {
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
pub fn computed(element: &web_sys::Element, property: &str) -> String {
    web_sys::window()
        .expect("the test must run in a browser")
        .get_computed_style(element)
        .expect("getComputedStyle must succeed")
        .expect("the element must be in the document")
        .get_property_value(property)
        .expect("the property must be readable")
}

/// 入れ物の中の最初の一致を返す。
pub fn select(root: &web_sys::Element, selector: &str) -> web_sys::Element {
    root.query_selector(selector)
        .expect("the selector must be valid")
        .unwrap_or_else(|| panic!("the element must match {selector}"))
}

/// 入れ物の中の一致の数を返す。
pub fn count(root: &web_sys::Element, selector: &str) -> u32 {
    root.query_selector_all(selector)
        .expect("the selector must be valid")
        .length()
}
