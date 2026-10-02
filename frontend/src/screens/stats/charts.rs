//! 統計の画面のグラフ (FR-18)。
//!
//! 集計の結果をデザインの Charts の部品で描く。色は `chart-count` (件数と金額) と
//! `chart-grams` (グラム、散布図の点、折れ線) の 2 つだけにする (docs/design/components/Charts)。
//! 座標の計算は [`crate::records::chart`] の純粋な関数が行う (ADR-0013)。

use dioxus::prelude::*;

use crate::i18n::{t, t_args, Key};
use crate::records::chart;
use crate::records::stats::{BrewPeriod, BrewRating, PurchasePeriod, RatingHistoryEntry};
use crate::ui::{
    ChartBars, ChartFrame, ChartLine, ChartScatter, ChartScatterFrame, ChartSection, ChartSeries,
    StatTile, StatTiles,
};

/// 区間のキーから単位の期間 (日 / 月) を返す。記録が無いときは日別にする。
fn period_unit(first: Option<&str>) -> &'static str {
    match first {
        Some(key) if !chart::is_daily_key(key) => t(Key::StatsPeriodMonth),
        _ => t(Key::StatsPeriodDay),
    }
}

/// 記録が無い区画の表示 (Charts のガイドライン)。
fn no_records() -> Element {
    rsx! {
        div { class: "empty", "{t(Key::NoRecords)}" }
    }
}

/// 抽出回数と豆の消費量の棒グラフ (FR-18)。
#[component]
pub fn BrewStatsCharts(
    /// 区間ごとの集計の結果。記録の無い区間は含まれない。
    periods: Vec<BrewPeriod>,
) -> Element {
    let keys: Vec<String> = periods.iter().map(|period| period.period.clone()).collect();
    let period_unit = period_unit(keys.first().map(String::as_str));
    let brew_count: u64 = periods.iter().map(|period| period.brew_count).sum();
    let dose: f64 = periods.iter().map(|period| period.dose_grams).sum();
    let count_unit = t_args(
        Key::StatsChartUnit,
        &[("unit", t(Key::StatsUnitCups)), ("period", period_unit)],
    );
    let dose_unit = t_args(
        Key::StatsChartUnit,
        &[("unit", t(Key::GramUnit)), ("period", period_unit)],
    );
    let count_values: Vec<f64> = periods
        .iter()
        .map(|period| period.brew_count as f64)
        .collect();
    let dose_values: Vec<f64> = periods.iter().map(|period| period.dose_grams).collect();
    rsx! {
        div { class: "stats-section",
            StatTiles {
                StatTile {
                    label: t(Key::StatsBrewCountTitle).to_string(),
                    value: brew_count.to_string(),
                    unit: Some(t(Key::StatsUnitCups).to_string()),
                }
                StatTile {
                    label: t(Key::StatsBrewDoseTitle).to_string(),
                    value: format!("{dose:.1}"),
                    unit: Some(t(Key::GramUnit).to_string()),
                }
            }
            ChartSection {
                title: t(Key::StatsBrewCountTitle).to_string(),
                unit: Some(count_unit),
                if periods.is_empty() {
                    {no_records()}
                } else {
                    ChartFrame {
                        ChartBars {
                            id: "stats-brew-count-chart".to_string(),
                            values: count_values,
                            labels: keys.clone(),
                            kind: ChartSeries::Count,
                        }
                    }
                }
            }
            ChartSection {
                title: t(Key::StatsBrewDoseTitle).to_string(),
                unit: Some(dose_unit),
                divider: false,
                if periods.is_empty() {
                    {no_records()}
                } else {
                    ChartFrame {
                        ChartBars {
                            id: "stats-brew-dose-chart".to_string(),
                            values: dose_values,
                            labels: keys.clone(),
                            kind: ChartSeries::Grams,
                        }
                    }
                }
            }
        }
    }
}

/// 購入金額と重量の棒グラフ (通貨コードごとに分ける。FR-18)。
///
/// 為替換算はせず、通貨コードごとに別のグラフにする (PRD のやらないこと)。通貨はグラフを
/// 分けて題に付ける (色で分けない)。
#[component]
pub fn PurchaseStatsCharts(
    /// 区間と通貨コードの組ごとの集計の結果。記録の無い区間は含まれない。
    purchases: Vec<PurchasePeriod>,
) -> Element {
    if purchases.is_empty() {
        return rsx! {
            div { class: "stats-section",
                ChartSection {
                    title: t(Key::StatsPurchaseAmountLabel).to_string(),
                    {no_records()}
                }
                ChartSection {
                    title: t(Key::StatsPurchaseWeightLabel).to_string(),
                    divider: false,
                    {no_records()}
                }
            }
        };
    }
    // 通貨コードごとに分ける。同じ通貨の行は区間ごとに 1 つだけ返る。
    let mut groups: Vec<(Option<String>, Vec<PurchasePeriod>)> = Vec::new();
    for row in purchases {
        match groups
            .iter_mut()
            .find(|(currency, _)| *currency == row.price_currency)
        {
            Some((_, rows)) => rows.push(row),
            None => groups.push((row.price_currency.clone(), vec![row])),
        }
    }
    rsx! {
        div { class: "stats-section",
            for (currency, rows) in groups {
                {
                    let keys: Vec<String> = rows.iter().map(|row| row.period.clone()).collect();
                    let period_unit = period_unit(keys.first().map(String::as_str));
                    let label = currency
                        .clone()
                        .unwrap_or_else(|| t(Key::StatsCurrencyNone).to_string());
                    let key = currency.unwrap_or_else(|| "none".to_string());
                    let amount_title =
                        t_args(Key::StatsPurchaseAmountTitle, &[("currency", &label)]);
                    let weight_title =
                        t_args(Key::StatsPurchaseWeightTitle, &[("currency", &label)]);
                    let amount_unit =
                        t_args(Key::StatsChartUnit, &[("unit", &label), ("period", period_unit)]);
                    let weight_unit = t_args(
                        Key::StatsChartUnit,
                        &[("unit", t(Key::GramUnit)), ("period", period_unit)],
                    );
                    let amounts: Vec<f64> =
                        rows.iter().map(|row| row.price_amount as f64).collect();
                    let weights: Vec<f64> =
                        rows.iter().map(|row| row.weight_grams as f64).collect();
                    rsx! {
                        ChartSection {
                            title: amount_title,
                            unit: Some(amount_unit),
                            ChartFrame {
                                ChartBars {
                                    id: format!("stats-purchase-amount-chart-{key}"),
                                    values: amounts,
                                    labels: keys.clone(),
                                    kind: ChartSeries::Count,
                                }
                            }
                        }
                        ChartSection {
                            title: weight_title,
                            unit: Some(weight_unit),
                            ChartFrame {
                                ChartBars {
                                    id: format!("stats-purchase-weight-chart-{key}"),
                                    values: weights,
                                    labels: keys.clone(),
                                    kind: ChartSeries::Grams,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 抽出条件と評価の関係の散布図 (豆の量、湯量、湯の温度、時間。FR-18)。
#[component]
pub fn BrewRatingScatterCharts(
    /// 期間内の評価を持つ抽出。条件が null の項目はその散布図に描かない。
    ratings: Vec<BrewRating>,
) -> Element {
    let points = |value: fn(&BrewRating) -> Option<f64>| -> Vec<(f64, f64)> {
        ratings
            .iter()
            .filter_map(|rating| value(rating).map(|value| (value, f64::from(rating.rating))))
            .collect()
    };
    let sections = [
        (
            "stats-rating-dose-chart",
            t(Key::StatsRatingDoseTitle).to_string(),
            points(|rating| rating.dose_grams),
            t(Key::GramUnit).to_string(),
        ),
        (
            "stats-rating-water-chart",
            t(Key::StatsRatingWaterTitle).to_string(),
            points(|rating| rating.water_grams),
            t(Key::GramUnit).to_string(),
        ),
        (
            "stats-rating-temp-chart",
            t(Key::StatsRatingTempTitle).to_string(),
            points(|rating| rating.water_temp_c),
            t(Key::CelsiusUnit).to_string(),
        ),
        (
            "stats-rating-time-chart",
            t(Key::StatsRatingTimeTitle).to_string(),
            points(|rating| rating.brew_time_seconds.map(|value| value as f64)),
            t(Key::SecondUnit).to_string(),
        ),
    ];
    rsx! {
        div { class: "stats-scatter",
            div { class: "t-heading", "{t(Key::StatsScatterSection)}" }
            div { class: "grid2",
                for (id, title, points, unit) in sections {
                    div { class: "chart nb",
                        div { class: "t-label muted", "{title}" }
                        if points.is_empty() {
                            {no_records()}
                        } else {
                            ChartScatterFrame {
                                ChartScatter { id: id.to_string(), points, unit }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 購入ごとの評価の推移の折れ線グラフ (FR-18)。購入の詳細に足す (0041)。
#[component]
pub fn RatingHistoryChart(
    /// 購入に紐づく評価を持つ抽出。抽出日時の昇順で並ぶ。
    entries: Vec<RatingHistoryEntry>,
) -> Element {
    let count_label = format!("{} {}", entries.len(), t(Key::StatsUnitCups));
    let line_entries: Vec<(String, u8)> = entries
        .iter()
        .map(|entry| (entry.brewed_at.clone(), entry.rating))
        .collect();
    rsx! {
        ChartSection {
            title: t(Key::RatingHistoryTitle).to_string(),
            unit: (!entries.is_empty()).then_some(count_label),
            divider: false,
            if entries.is_empty() {
                {no_records()}
            } else {
                ChartFrame {
                    ChartLine {
                        id: "purchase-rating-history-chart".to_string(),
                        entries: line_entries,
                    }
                }
            }
        }
    }
}
