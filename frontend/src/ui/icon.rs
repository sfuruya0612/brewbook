//! 画面内のアイコン (0039)。
//!
//! Material Icons Outlined の Web フォント (Google Fonts) を 24 px で使い、色は `ink` か
//! `ink-muted` にする (docs/design/README.md の「アイコン」。Flutter の `Icons.*_outlined`
//! に対応する)。グリフの名前は Material Icons のリガチャ (例: `arrow_back_ios_new`) を渡す。

use dioxus::prelude::*;

/// 画面内のアイコン。既定は 24 px の `ink`。
#[component]
pub fn Icon(name: String, #[props(default = false)] muted: bool) -> Element {
    let class = if muted { "icon muted" } else { "icon" };
    rsx! {
        span {
            class,
            "aria-hidden": "true",
            "{name}"
        }
    }
}
