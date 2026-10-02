//! brewbook の印 (0039)。
//!
//! docs/design/assets/Marks/mark.svg。ドリッパーの一滴が記録の行に落ちる形。ログインの画面と、
//! 記録が無いときの表示、広い画面のナビゲーションレールの上に使う。塗りは `currentColor` に
//! し、CSS の `.mark` が `roast` を当てる (Night では tokens.css の `roast` が明るくなる)。

use dioxus::prelude::*;

/// 印。既定は 40 px 四方 (ナビゲーションレールと同じ大きさ)。
#[component]
pub fn BrewbookMark(
    /// 一辺の大きさ (px)。
    #[props(default = 40)]
    size: u32,
) -> Element {
    rsx! {
        svg {
            class: "mark",
            view_box: "100 106 312 368",
            width: "{size}",
            height: "{size}",
            "aria-hidden": "true",
            rect { x: "118", y: "124", width: "276", height: "34", rx: "17", fill: "currentColor" }
            path {
                d: "M148 158 H364 L298 286 H214 Z",
                fill: "currentColor",
                stroke: "currentColor",
                "stroke-width": "8",
                "stroke-linejoin": "round",
            }
            path {
                d: "M256 298 C256 298 228 332 228 352 A28 28 0 0 0 284 352 C284 332 256 298 256 298 Z",
                fill: "currentColor",
            }
            rect { x: "118", y: "386", width: "276", height: "26", rx: "13", fill: "currentColor" }
            rect { x: "118", y: "430", width: "168", height: "26", rx: "13", fill: "currentColor" }
        }
    }
}
