//! `records::stats_period` の PBT。期間と粒度の組み立て (ADR-0013)。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::records::stats_period::{
    stats_day_count, stats_period_for, StatsGranularity, StatsPeriodError, StatsPeriodPreset,
    STATS_DAY_GRANULARITY_MAX_DAYS,
};
use brew_book_frontend::records::values::{format_day, parse_day, LocalDate, LocalDateTime};
use proptest::prelude::*;

/// 実在する日付 (月と日はどの月にもある範囲にする)。年は値の型の範囲 (0 から 9999) の
/// 境界を含める。
fn valid_date() -> impl Strategy<Value = LocalDate> {
    (0..=9999i32, 1..=12u32, 1..=28u32)
        .prop_map(|(year, month, day)| LocalDate::new(year, month, day))
}

/// 端末のローカル日時。
fn valid_datetime() -> impl Strategy<Value = LocalDateTime> {
    (0..=9999i32, 1..=12u32, 1..=28u32, 0..24u32, 0..60u32).prop_map(
        |(year, month, day, hour, minute)| LocalDateTime::new(year, month, day, hour, minute),
    )
}

/// 端末のローカル日時 (0 年より前の丸めが起きない範囲)。
fn valid_datetime_from_year_one() -> impl Strategy<Value = LocalDateTime> {
    (1..=9999i32, 1..=12u32, 1..=28u32, 0..24u32, 0..60u32).prop_map(
        |(year, month, day, hour, minute)| LocalDateTime::new(year, month, day, hour, minute),
    )
}

proptest! {
    /// 当月は 1 日から当日までを日別で覆う。
    #[test]
    fn the_current_month_starts_at_the_first_day(now in valid_datetime()) {
        let period = stats_period_for(StatsPeriodPreset::CurrentMonth, now, None)
            .expect("the period must be built");
        prop_assert_eq!(period.start, Some(format!("{:04}-{:02}-01", now.year, now.month)));
        prop_assert_eq!(period.end, Some(format_day(now.date())));
        prop_assert_eq!(period.granularity, StatsGranularity::Day);
    }

    /// 直近の月数は、当月を含む古い方の月の 1 日から当日までを月別で覆う。
    /// 0 年より前の丸めが起きない範囲 (1 年以上) で検査する。期待値は実装の式を写さず、
    /// 「開始の月から当月までの月数が指定の月数と一致する」性質で検査する。
    #[test]
    fn the_recent_months_start_at_the_first_day_of_the_oldest_month(
        now in valid_datetime_from_year_one(),
        months in prop::sample::select(vec![3u32, 6, 12]),
    ) {
        let preset = match months {
            3 => StatsPeriodPreset::ThreeMonths,
            6 => StatsPeriodPreset::SixMonths,
            _ => StatsPeriodPreset::TwelveMonths,
        };
        let period = stats_period_for(preset, now, None).expect("the period must be built");
        let start = period.start.clone().expect("the start must be set");
        let start = parse_day(&start).expect("the start must be a day");
        prop_assert_eq!(start.day, 1, "the start must be the first day: {:?}", start);
        let start_months = i64::from(start.year) * 12 + i64::from(start.month);
        let now_months = i64::from(now.year) * 12 + i64::from(now.month);
        prop_assert_eq!(
            now_months - start_months + 1,
            i64::from(months),
            "the period must cover {} months: {:?}",
            months,
            period
        );
        prop_assert_eq!(period.end, Some(format_day(now.date())));
        prop_assert_eq!(period.granularity, StatsGranularity::Month);
    }

    /// 直近の月数は 0 年 1 月より前を指さず、開始は終了より後にならない (境界の丸めは
    /// `tests/test_stats_period.rs` の単体テストが固定する)。
    #[test]
    fn the_recent_months_never_start_before_the_year_zero(
        now in valid_datetime(),
        months in prop::sample::select(vec![3u32, 6, 12]),
    ) {
        let preset = match months {
            3 => StatsPeriodPreset::ThreeMonths,
            6 => StatsPeriodPreset::SixMonths,
            _ => StatsPeriodPreset::TwelveMonths,
        };
        let period = stats_period_for(preset, now, None).expect("the period must be built");
        let start = period.start.expect("the start must be set");
        let end = period.end.expect("the end must be set");
        prop_assert!(
            start.as_str() >= "0000-01-01",
            "the start must be in the value's range: {}",
            start
        );
        prop_assert!(
            start.as_str() <= end.as_str(),
            "the start must not be after the end: {} > {}",
            start,
            end
        );
    }

    /// 任意の期間は、62 日以下なら日別、それ以外は月別にする。
    #[test]
    fn a_custom_period_uses_the_day_granularity_up_to_62_days(
        start in valid_date(),
        end in valid_date(),
    ) {
        let now = LocalDateTime::new(2026, 10, 2, 0, 0);
        match stats_period_for(StatsPeriodPreset::Custom, now, Some((start, end))) {
            Ok(period) => {
                prop_assert!(start <= end);
                let days = stats_day_count(start, end);
                prop_assert_eq!(period.start, Some(format_day(start)));
                prop_assert_eq!(period.end, Some(format_day(end)));
                prop_assert_eq!(
                    period.granularity,
                    if days <= STATS_DAY_GRANULARITY_MAX_DAYS {
                        StatsGranularity::Day
                    } else {
                        StatsGranularity::Month
                    }
                );
            }
            Err(StatsPeriodError::EndBeforeStart) => prop_assert!(end < start),
            Err(error) => prop_assert!(false, "unexpected error: {error:?}"),
        }
    }

    /// 同じ日の日数は 1。
    #[test]
    fn the_day_count_of_the_same_day_is_one(date in valid_date()) {
        prop_assert_eq!(stats_day_count(date, date), 1);
    }

    /// 日数は区間の分割で足し合わされる (a <= b <= c)。
    #[test]
    fn the_day_count_is_additive(a in valid_date(), b in valid_date(), c in valid_date()) {
        let mut dates = [a, b, c];
        dates.sort_unstable();
        let [a, b, c] = dates;
        prop_assert_eq!(
            stats_day_count(a, c),
            stats_day_count(a, b) + stats_day_count(b, c) - 1
        );
    }
}
