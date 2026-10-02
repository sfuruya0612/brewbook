//! 統計の画面 (FR-18)。
//!
//! 初期表示は端末のタイムゾーンでの当月 (1 日から当日まで) を日別で表示し、当月、3 か月、
//! 6 か月、12 か月、全期間、任意の開始日と終了日に切り替えられる。集計は Backend の API が
//! 行い、この画面は集計結果をグラフに描くだけにする (ADR-0007)。読み込みは 3 経路を順に
//! 呼び、届いた順に描き、期間を切り替えたら古い応答を捨てる (docs/design/components/Stats)。

pub mod charts;

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{t, t_args, Key};
use crate::records::stats::{BrewPeriod, BrewRating, PurchasePeriod};
use crate::records::stats_period::{
    stats_day_count, stats_period_for, StatsGranularity, StatsPeriodPreset,
};
use crate::records::values::{format_day, parse_day, LocalDate};
use crate::records::{RecordError, RecordServices, StatsApi};
use crate::screens::records::{rail_items, retryable_banner};
use crate::ui::{
    AppBar, Button, ButtonSize, ButtonVariant, Chip, Field, NavigationRail, TextField, WidePage,
};

pub use charts::{
    BrewRatingScatterCharts, BrewStatsCharts, PurchaseStatsCharts, RatingHistoryChart,
};

/// 統計の画面 (FR-18)。
#[component]
pub fn StatsScreen() -> Element {
    let services = use_context::<RecordServices>();
    let navigator = navigator();

    // 期間の切り替えと、任意の期間の入力 (FR-18)。
    let mut preset = use_signal(|| StatsPeriodPreset::CurrentMonth);
    let clock_services = services.clone();
    let mut custom = use_signal(move || {
        let now = clock_services.clock.now();
        (LocalDate::new(now.year, now.month, 1), now.date())
    });
    let mut start_text = use_signal(|| format_day(custom().0));
    let mut end_text = use_signal(|| format_day(custom().1));
    let mut start_error = use_signal(|| None::<Key>);
    let mut end_error = use_signal(|| None::<Key>);

    // 集計の結果 (FR-18)。
    let mut brews = use_signal(Vec::<BrewPeriod>::new);
    let mut purchases = use_signal(Vec::<PurchasePeriod>::new);
    let mut ratings = use_signal(Vec::<BrewRating>::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<RecordError>);

    // 読み込みの世代。古い読み込みの応答を捨てるために使う。
    let mut generation = use_signal(|| 0_u64);

    let load_services = services.clone();
    let load = EventHandler::new(move |_| {
        // 世代の読み書きは購読しない (peek)。この処理は期間の切り替えの effect からも呼ばれ、
        // 購読すると世代の更新が effect を再び起こして読み込みが止まらなくなる。
        let current = *generation.peek() + 1;
        generation.set(current);
        let period = match stats_period_for(preset(), load_services.clock.now(), Some(custom())) {
            Ok(period) => period,
            Err(_) => {
                // 期間の組み立てに失敗したときは、読み込み中のままにせずにエラーを出す
                // (0042 のレビューの指摘。現状の入力では到達しない防御)。
                loading.set(false);
                error.set(Some(RecordError::Validation(Key::ValidationPeriod)));
                return;
            }
        };
        let offset = load_services.clock.utc_offset_minutes();
        let api = StatsApi::new(load_services.api.clone());
        spawn(async move {
            loading.set(true);
            error.set(None);
            let start = period.start.as_deref();
            let end = period.end.as_deref();
            let result = async {
                let values = api.brews(start, end, period.granularity, offset).await?;
                if generation() != current {
                    return Ok(());
                }
                brews.set(values);
                let values = api.purchases(start, end, period.granularity).await?;
                if generation() != current {
                    return Ok(());
                }
                purchases.set(values);
                let values = api.brew_ratings(start, end, offset).await?;
                if generation() != current {
                    return Ok(());
                }
                ratings.set(values);
                Ok::<(), RecordError>(())
            }
            .await;
            if generation() == current {
                if let Err(failure) = result {
                    error.set(Some(failure));
                }
                loading.set(false);
            }
        });
    });

    // 期間の切り替えのたびに読み込む。任意の期間だけは「適用」を押すまで読み込まない。
    use_effect(move || {
        if preset() != StatsPeriodPreset::Custom {
            load.call(());
        }
    });

    let select = EventHandler::new(move |selected: StatsPeriodPreset| {
        preset.set(selected);
        start_error.set(None);
        end_error.set(None);
    });

    let apply = EventHandler::new(move |_| {
        let start = parse_day(&start_text());
        let end = parse_day(&end_text());
        start_error.set(start.is_none().then_some(Key::ValidationDay));
        end_error.set(match (start, end) {
            (_, None) => Some(Key::ValidationDay),
            (Some(start), Some(end)) if end < start => Some(Key::ValidationPeriod),
            _ => None,
        });
        if let (Some(start), Some(end)) = (start, end) {
            if start <= end {
                custom.set((start, end));
                load.call(());
            }
        }
    });

    let preset_entries = [
        (
            StatsPeriodPreset::CurrentMonth,
            Key::StatsPeriodCurrentMonth,
        ),
        (StatsPeriodPreset::ThreeMonths, Key::StatsPeriodThreeMonths),
        (StatsPeriodPreset::SixMonths, Key::StatsPeriodSixMonths),
        (
            StatsPeriodPreset::TwelveMonths,
            Key::StatsPeriodTwelveMonths,
        ),
        (StatsPeriodPreset::AllTime, Key::StatsPeriodAllTime),
        (StatsPeriodPreset::Custom, Key::StatsPeriodCustom),
    ];
    let chips = rsx! {
        div { class: "chips",
            for (value, label) in preset_entries {
                Chip {
                    label: t(label).to_string(),
                    selected: preset() == value,
                    onclick: move |_| select.call(value),
                }
            }
        }
    };

    // 選択中の範囲と粒度の 1 行 (caption)。
    let range = stats_period_for(preset(), services.clock.now(), Some(custom()));
    let caption = match range {
        Ok(period) => {
            let granularity = granularity_label(period.granularity);
            match (period.start.as_deref(), period.end.as_deref()) {
                (Some(start), Some(end)) => {
                    let suffix = t_args(Key::StatsRangeSuffix, &[("granularity", granularity)]);
                    rsx! {
                        div { class: "t-caption muted",
                            "{t(Key::StatsRangePrefix)}"
                            span { class: "t-value", style: "font-size: 12px", "{start}" }
                            "{t(Key::StatsRangeMiddle)}"
                            span { class: "t-value", style: "font-size: 12px", "{end}" }
                            "{suffix}"
                        }
                    }
                }
                _ => {
                    let all_time = t_args(Key::StatsRangeAllTime, &[("granularity", granularity)]);
                    rsx! {
                        div { class: "t-caption muted", "{all_time}" }
                    }
                }
            }
        }
        Err(_) => rsx! {
            div {}
        },
    };

    // 任意の期間の入力と、粒度の注記 (FR-18)。
    let note = {
        let (start, end) = custom();
        let days = stats_day_count(start, end);
        let granularity = stats_period_for(
            StatsPeriodPreset::Custom,
            services.clock.now(),
            Some((start, end)),
        )
        .map(|period| granularity_label(period.granularity))
        .unwrap_or_else(|_| t(Key::StatsGranularityMonthly));
        t_args(
            Key::StatsCustomGranularityNote,
            &[("days", &days.to_string()), ("granularity", granularity)],
        )
    };
    let custom_fields = rsx! {
        div { class: "stats-custom",
            div { class: "grid2",
                Field {
                    label: t(Key::StatsStartLabel).to_string(),
                    error: start_error().map(|key| t(key).to_string()),
                    TextField {
                        value: start_text(),
                        mono: true,
                        icon: Some("calendar_today".to_string()),
                        oninput: move |event: FormEvent| start_text.set(event.value()),
                    }
                }
                Field {
                    label: t(Key::StatsEndLabel).to_string(),
                    error: end_error().map(|key| t(key).to_string()),
                    TextField {
                        value: end_text(),
                        mono: true,
                        icon: Some("calendar_today".to_string()),
                        oninput: move |event: FormEvent| end_text.set(event.value()),
                    }
                }
            }
            div { class: "apply",
                Button {
                    label: t(Key::StatsApplyButton).to_string(),
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    onclick: move |_| apply.call(()),
                }
            }
            if start_error().is_none() && end_error().is_none() {
                div { class: "t-caption muted", "{note}" }
            }
        }
    };

    let content = rsx! {
        div { class: "screen",
            AppBar { title: t(Key::StatsTitle).to_string() }
            div { class: "body",
                div { class: "stats",
                    {chips}
                    {caption}
                    if preset() == StatsPeriodPreset::Custom {
                        {custom_fields}
                    }
                    if loading() {
                        div { class: "empty", "{t(Key::Loading)}" }
                    } else if let Some(failure) = error() {
                        {retryable_banner(&failure, load)}
                    } else {
                        div { class: "stats-columns",
                            BrewStatsCharts { periods: brews() }
                            PurchaseStatsCharts { purchases: purchases() }
                        }
                        BrewRatingScatterCharts { ratings: ratings() }
                    }
                }
            }
        }
    };

    rsx! {
        WidePage {
            rail: rsx! { NavigationRail { items: rail_items(navigator, 4) } },
            {content}
        }
    }
}

/// 粒度の文言 (日別 / 月別)。
fn granularity_label(granularity: StatsGranularity) -> &'static str {
    match granularity {
        StatsGranularity::Day => t(Key::StatsGranularityDaily),
        StatsGranularity::Month => t(Key::StatsGranularityMonthly),
    }
}
