//! brewbook の Frontend (ADR-0017)。
//!
//! 0037 で作った最小のアプリで、ビルドとテストの配線だけを確認する。
//! 画面の中身は 0038 以降で実装する。

use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

/// 画面のルート。0037 では中身を持たない。
#[component]
fn App() -> Element {
    rsx! {
        div { id: "app" }
    }
}
