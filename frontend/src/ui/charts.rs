//! 統計のグラフ (0039、0042)。
//!
//! docs/design/components/Charts のガイドライン。色は `chart-count` (件数と金額。`roast`) と
//! `chart-grams` (グラム。`crema`) の 2 つだけ。0039 が枠 (題、単位、軸) を作り、0042 が棒、
//! 点、線の中身 ([`ChartBars`]、[`ChartScatter`]、[`ChartLine`]) を足した。座標の計算は
//! [`crate::records::chart`] の純粋な関数が行う (ADR-0013)。

use dioxus::prelude::*;

use crate::records::chart;
use crate::records::values::format_number;

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

/// グラフの系列の色 (docs/design/components/Charts)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChartSeries {
    /// 件数と金額 (`chart-count`。`roast`)。
    Count,
    /// グラム (`chart-grams`。`crema`)。
    Grams,
}

impl ChartSeries {
    /// CSS のクラス名 (docs/design/components/bundle.css と同じ)。
    fn class(self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Grams => "grams",
        }
    }
}

/// 棒グラフの棒と軸の目盛り。[`ChartFrame`] の子として置く。
///
/// 目盛りの数字 (最大、半分、0) と、下の軸のラベル (先頭、中央、末尾だけ) も描く。
#[component]
pub fn ChartBars(
    /// 区間ごとの値。呼び出し側が空のときは呼ばない (「記録がありません」を出す)。
    values: Vec<f64>,
    /// 下の軸のラベル (区間のキー)。`values` と同じ数にする。
    labels: Vec<String>,
    /// 色の系列。
    kind: ChartSeries,
    /// 棒をまとめる `g` の id (テストの選択子)。
    id: String,
) -> Element {
    let max_value = values.iter().copied().fold(0.0, f64::max);
    let ticks = chart::axis_ticks(max_value);
    let slots = chart::bar_slots(values.len());
    let label_positions = chart::label_indices(values.len());
    let class = kind.class();
    rsx! {
        g { id: "{id}",
            for (index, value) in ticks.iter().enumerate() {
                {
                    let y = chart::tick_y(index as f64 / 2.0) + 4.0;
                    rsx! {
                        text { x: "0", y: "{y}", "{chart::format_axis(*value)}" }
                    }
                }
            }
            for (index, value) in values.iter().enumerate() {
                {
                    let (x, width) = slots[index];
                    let (y, height) = chart::bar_rect(*value, max_value);
                    rsx! {
                        rect { class: "{class}", x: "{x}", y: "{y}", width: "{width}", height: "{height}", rx: "1" }
                    }
                }
            }
            for index in label_positions {
                {
                    let (x, width) = slots[index];
                    let (x, anchor) = if index == 0 {
                        (chart::PLOT_LEFT, "start")
                    } else if index + 1 == values.len() {
                        (chart::PLOT_RIGHT, "end")
                    } else {
                        (x + width / 2.0, "middle")
                    };
                    let label = labels[index].clone();
                    rsx! {
                        text { x: "{x}", y: "116", "text-anchor": "{anchor}", "{label}" }
                    }
                }
            }
        }
    }
}

/// 散布図の枠。左の縦軸と、上の横罫 (評価の最大) と、下の軸 (評価の 1) を描く。
#[component]
pub fn ChartScatterFrame(
    /// グラフの中身 (点)。
    children: Element,
) -> Element {
    rsx! {
        svg { class: "chart-frame", view_box: "0 0 340 120",
            line { class: "grid", x1: "22", y1: "8", x2: "340", y2: "8" }
            line { class: "axis", x1: "22", y1: "92", x2: "340", y2: "92" }
            line { class: "axis", x1: "22", y1: "8", x2: "22", y2: "92" }
            {children}
        }
    }
}

/// 散布図の点 (条件と評価)。[`ChartScatterFrame`] の子として置く。
///
/// 条件が null の抽出は呼び出し側が除く (FR-18)。横軸の両端に最小と最大の値を単位付きで出す。
#[component]
pub fn ChartScatter(
    /// 点 (条件の値、評価 1 から 5)。
    points: Vec<(f64, f64)>,
    /// 横軸の単位 (g、℃、秒)。
    unit: String,
    /// 点をまとめる `g` の id (テストの選択子)。
    id: String,
) -> Element {
    if points.is_empty() {
        return rsx! {
            g { id: "{id}" }
        };
    }
    let min = points.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
    let max = points
        .iter()
        .map(|(x, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let (min, max) = chart::scatter_x_range(min, max);
    let min_label = format!("{} {}", format_number(min), unit);
    let max_label = format!("{} {}", format_number(max), unit);
    rsx! {
        g { id: "{id}",
            text { x: "0", y: "12", "{chart::format_axis(chart::RATING_MAX)}" }
            text { x: "0", y: "96", "{chart::format_axis(chart::RATING_MIN)}" }
            for (value, rating) in points {
                {
                    let cx = chart::scatter_x(value, min, max);
                    let cy = chart::rating_y(rating);
                    rsx! {
                        circle { class: "dot", cx: "{cx}", cy: "{cy}", r: "3.5", opacity: ".85" }
                    }
                }
            }
            text { x: "{chart::SCATTER_LEFT}", y: "118", "{min_label}" }
            text { x: "{chart::PLOT_RIGHT}", y: "118", "text-anchor": "end", "{max_label}" }
        }
    }
}

/// 折れ線 (評価の推移)。[`ChartFrame`] の子として置く。
///
/// 横軸は最初と最後の日付だけを出す (docs/design/components/Charts)。
#[component]
pub fn ChartLine(
    /// 点 (抽出日時の ISO 8601、評価 1 から 5)。日時は日付の部分だけを横軸に出す。
    entries: Vec<(String, u8)>,
    /// 線と点をまとめる `g` の id (テストの選択子)。
    id: String,
) -> Element {
    let count = entries.len();
    let points = entries
        .iter()
        .enumerate()
        .map(|(index, (_, rating))| {
            format!(
                "{},{}",
                chart::line_x(index, count),
                chart::rating_y(f64::from(*rating))
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        g { id: "{id}",
            text { x: "0", y: "12", "{chart::format_axis(chart::RATING_MAX)}" }
            text { x: "0", y: "54", "{chart::format_axis((chart::RATING_MIN + chart::RATING_MAX) / 2.0)}" }
            text { x: "0", y: "96", "{chart::format_axis(chart::RATING_MIN)}" }
            polyline { class: "line", points: "{points}" }
            for (index, (_, rating)) in entries.iter().enumerate() {
                {
                    let cx = chart::line_x(index, count);
                    let cy = chart::rating_y(f64::from(*rating));
                    rsx! {
                        circle { class: "pt", cx: "{cx}", cy: "{cy}", r: "4" }
                    }
                }
            }
            if let Some((first, _)) = entries.first() {
                text { x: "30", y: "112", "{chart::period_label(first)}" }
            }
            if count > 1 {
                if let Some((last, _)) = entries.last() {
                    text { x: "300", y: "112", "text-anchor": "end", "{chart::period_label(last)}" }
                }
            }
        }
    }
}
