//! 統計のグラフ (0039)。
//!
//! docs/design/components/Charts のガイドライン。色は `chart-count` (件数と金額。`roast`) と
//! `chart-grams` (グラム。`crema`) の 2 つだけ。この issue ではグラフの枠 (題、単位、軸) までを
//! 作り、棒、点、線の中身は 0042 が描く。

use dioxus::prelude::*;

/// グラフの区画。見出しと単位を上に置き、下に `line` の罫線を引いて次と区切る。
#[component]
pub fn ChartSection(
    /// 区画の見出し (ARB から取る)。
    title: String,
    /// 右端の単位 (「杯 / 日」「g / 月」など)。無いときは出さない。
    #[props(default)]
    unit: Option<String>,
    /// 下に罫線を引くか。
    #[props(default = true)]
    divider: bool,
    /// グラフ (棒、散布図、折れ線) か、記録が無いときの表示。
    children: Element,
) -> Element {
    let class = if divider { "chart" } else { "chart nb" };
    rsx! {
        div { class,
            div { class: "h",
                span { class: "t", "{title}" }
                if let Some(unit) = unit {
                    span { class: "s", "{unit}" }
                }
            }
            {children}
        }
    }
}

/// グラフの軸の枠。軸と目盛の横罫 (最大と半分) だけを描く。棒、点、線は子として渡す。
#[component]
pub fn ChartFrame(
    /// グラフの中身 (棒、点、線)。
    children: Element,
) -> Element {
    rsx! {
        svg { class: "chart-frame", view_box: "0 0 340 120",
            line { class: "grid", x1: "28", y1: "8", x2: "340", y2: "8" }
            line { class: "grid", x1: "28", y1: "50", x2: "340", y2: "50" }
            line { class: "axis", x1: "28", y1: "92", x2: "340", y2: "92" }
            {children}
        }
    }
}

/// 期間の合計のタイル (`value-large` と単位)。
#[component]
pub fn StatTile(
    /// 項目名 (ARB から取る)。
    label: String,
    /// 値。数値は整形済みの文字列にする。
    value: String,
    /// 単位 (杯、g など)。無いときは出さない。
    #[props(default)]
    unit: Option<String>,
) -> Element {
    rsx! {
        div { class: "stat-tile",
            div { class: "k", "{label}" }
            div { class: "v",
                "{value}"
                if let Some(unit) = unit {
                    span { class: "u", "{unit}" }
                }
            }
        }
    }
}

/// 合計のタイルを 2 列に並べる。
#[component]
pub fn StatTiles(
    /// 並べるタイル。
    children: Element,
) -> Element {
    rsx! {
        div { class: "stat-tiles", {children} }
    }
}
