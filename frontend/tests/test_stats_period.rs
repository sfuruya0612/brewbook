//! `records::stats_period` の単体テスト。境界と組み立ての性質は PBT (`prop_stats_period.rs`) が担う。

use brew_book_frontend::records::stats_period::{
    stats_day_count, stats_period_for, StatsGranularity, StatsPeriodError, StatsPeriodPreset,
    STATS_DAY_GRANULARITY_MAX_DAYS,
};
use brew_book_frontend::records::values::{LocalDate, LocalDateTime};

fn period(
    preset: StatsPeriodPreset,
    now: LocalDateTime,
    custom: Option<(LocalDate, LocalDate)>,
) -> brew_book_frontend::records::stats_period::StatsPeriod {
    stats_period_for(preset, now, custom).expect("the period must be built")
}

#[test]
fn the_recent_months_do_not_go_before_the_year_zero() {
    // 値の型は 0 年から 9999 年までを扱う (values.rs)。直近の月が 0 年より前になる場合は
    // 0 年 1 月で止める (負の年は値の範囲と API の日付の形式の外になる)。現在の月 (2 月) では
    // なく 1 月で止まることを固定するため、月が 1 以外の入力にする。
    let result = period(
        StatsPeriodPreset::ThreeMonths,
        LocalDateTime::new(0, 2, 15, 0, 0),
        None,
    );
    assert_eq!(result.start.as_deref(), Some("0000-01-01"));
    assert_eq!(result.end.as_deref(), Some("0000-02-15"));
}

#[test]
fn the_recent_months_include_the_current_month() {
    let now = LocalDateTime::new(2026, 1, 15, 0, 0);
    let result = period(StatsPeriodPreset::ThreeMonths, now, None);
    // 2025 年 11 月から 2026 年 1 月まで。
    assert_eq!(result.start.as_deref(), Some("2025-11-01"));
    assert_eq!(result.end.as_deref(), Some("2026-01-15"));
    assert_eq!(result.granularity, StatsGranularity::Month);

    let result = period(StatsPeriodPreset::TwelveMonths, now, None);
    assert_eq!(result.start.as_deref(), Some("2025-02-01"));
}

#[test]
fn the_all_time_period_has_no_start_and_no_end() {
    let result = period(
        StatsPeriodPreset::AllTime,
        LocalDateTime::new(2026, 10, 2, 0, 0),
        None,
    );
    assert_eq!(result.start, None);
    assert_eq!(result.end, None);
    assert_eq!(result.granularity, StatsGranularity::Month);
}

#[test]
fn a_custom_period_needs_both_ends() {
    assert_eq!(
        stats_period_for(
            StatsPeriodPreset::Custom,
            LocalDateTime::new(2026, 10, 2, 0, 0),
            None
        ),
        Err(StatsPeriodError::MissingCustomRange)
    );
}

#[test]
fn a_custom_period_needs_the_end_on_or_after_the_start() {
    let now = LocalDateTime::new(2026, 10, 2, 0, 0);
    let start = LocalDate::new(2026, 10, 2);
    let end = LocalDate::new(2026, 10, 1);
    assert_eq!(
        stats_period_for(StatsPeriodPreset::Custom, now, Some((start, end))),
        Err(StatsPeriodError::EndBeforeStart)
    );
}

#[test]
fn a_custom_period_uses_the_day_granularity_up_to_62_days() {
    let now = LocalDateTime::new(2026, 10, 2, 0, 0);
    let start = LocalDate::new(2026, 1, 1);

    let last_day = LocalDate::new(2026, 3, 3); // 1 月 1 日から 62 日。
    assert_eq!(
        stats_day_count(start, last_day),
        STATS_DAY_GRANULARITY_MAX_DAYS
    );
    let result = period(StatsPeriodPreset::Custom, now, Some((start, last_day)));
    assert_eq!(result.granularity, StatsGranularity::Day);
    assert_eq!(result.start.as_deref(), Some("2026-01-01"));
    assert_eq!(result.end.as_deref(), Some("2026-03-03"));

    let over = LocalDate::new(2026, 3, 4); // 63 日。
    assert_eq!(
        stats_day_count(start, over),
        STATS_DAY_GRANULARITY_MAX_DAYS + 1
    );
    let result = period(StatsPeriodPreset::Custom, now, Some((start, over)));
    assert_eq!(result.granularity, StatsGranularity::Month);
}

#[test]
fn the_day_count_includes_both_ends() {
    assert_eq!(
        stats_day_count(LocalDate::new(2026, 10, 2), LocalDate::new(2026, 10, 2)),
        1
    );
    assert_eq!(
        stats_day_count(LocalDate::new(2026, 10, 1), LocalDate::new(2026, 10, 2)),
        2
    );
    assert_eq!(
        stats_day_count(LocalDate::new(2024, 2, 28), LocalDate::new(2024, 3, 1)),
        3
    );
}

#[test]
fn the_granularity_has_the_api_value() {
    assert_eq!(StatsGranularity::Day.api_value(), "day");
    assert_eq!(StatsGranularity::Month.api_value(), "month");
}
