//! 統計の期間と粒度の組み立て (FR-18)。
//!
//! 画面の期間の切り替え (当月、3 か月、6 か月、12 か月、全期間、任意の開始日と終了日) を、
//! API に渡す開始日、終了日、粒度に変換する。日付は端末のタイムゾーンでのローカル日付とする。
//! Dioxus に依存しない純粋なモジュールにして、native の単体テストと PBT で守る (ADR-0013)。

use super::values::{self, format_day, LocalDate, LocalDateTime};

/// 任意の期間を日別で表示する上限の日数 (FR-18)。
pub const STATS_DAY_GRANULARITY_MAX_DAYS: i64 = 62;

/// 集計の粒度 (FR-18)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatsGranularity {
    /// 日別 (区間のキーは `YYYY-MM-DD`)。
    Day,
    /// 月別 (区間のキーは `YYYY-MM`)。
    Month,
}

impl StatsGranularity {
    /// API のクエリパラメータの値。
    pub fn api_value(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Month => "month",
        }
    }
}

/// 統計画面の期間の切り替えの種別 (FR-18)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatsPeriodPreset {
    /// 当月 (端末のタイムゾーンでの当月 1 日から当日まで。日別)。
    CurrentMonth,
    /// 当月を含む直近の 3 か月 (月別)。
    ThreeMonths,
    /// 当月を含む直近の 6 か月 (月別)。
    SixMonths,
    /// 当月を含む直近の 12 か月 (月別)。
    TwelveMonths,
    /// 全期間 (開始日と終了日を省略する。月別)。
    AllTime,
    /// 任意の開始日と終了日 (62 日以下なら日別、それ以外は月別)。
    Custom,
}

/// API に渡す期間と粒度 (FR-18)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StatsPeriod {
    /// 開始日 (`YYYY-MM-DD`)。全期間では None。
    pub start: Option<String>,
    /// 終了日 (`YYYY-MM-DD`)。全期間では None。
    pub end: Option<String>,
    /// 集計の粒度。
    pub granularity: StatsGranularity,
}

/// 期間の組み立ての誤り。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatsPeriodError {
    /// 任意の期間に開始日または終了日が無い。
    MissingCustomRange,
    /// 終了日が開始日より前。
    EndBeforeStart,
}

/// 切り替えの種別から期間と粒度を組み立てる (FR-18)。
///
/// `now` は端末のタイムゾーンでの現在の日時。`custom` は [`StatsPeriodPreset::Custom`] の
/// ときだけ使う。
pub fn stats_period_for(
    preset: StatsPeriodPreset,
    now: LocalDateTime,
    custom: Option<(LocalDate, LocalDate)>,
) -> Result<StatsPeriod, StatsPeriodError> {
    match preset {
        StatsPeriodPreset::CurrentMonth => Ok(StatsPeriod {
            start: Some(format_day(LocalDate::new(now.year, now.month, 1))),
            end: Some(format_day(now.date())),
            granularity: StatsGranularity::Day,
        }),
        StatsPeriodPreset::ThreeMonths => Ok(recent_months(now, 3)),
        StatsPeriodPreset::SixMonths => Ok(recent_months(now, 6)),
        StatsPeriodPreset::TwelveMonths => Ok(recent_months(now, 12)),
        StatsPeriodPreset::AllTime => Ok(StatsPeriod {
            start: None,
            end: None,
            granularity: StatsGranularity::Month,
        }),
        StatsPeriodPreset::Custom => {
            let (start, end) = custom.ok_or(StatsPeriodError::MissingCustomRange)?;
            if end < start {
                return Err(StatsPeriodError::EndBeforeStart);
            }
            Ok(StatsPeriod {
                start: Some(format_day(start)),
                end: Some(format_day(end)),
                granularity: if stats_day_count(start, end) <= STATS_DAY_GRANULARITY_MAX_DAYS {
                    StatsGranularity::Day
                } else {
                    StatsGranularity::Month
                },
            })
        }
    }
}

/// 開始日から終了日までの日数 (両端を含む)。終了日が開始日より前なら負になる。
///
/// 日付だけの値で数えるため、夏時間の切り替えの影響を受けない。
pub fn stats_day_count(start: LocalDate, end: LocalDate) -> i64 {
    values::days_between(start, end) + 1
}

/// 当月を含む直近 `months` か月の月別の期間。
fn recent_months(now: LocalDateTime, months: u32) -> StatsPeriod {
    let total_months = i64::from(now.year) * 12 + i64::from(now.month) - 1 - i64::from(months - 1);
    // 値の型は 0 年から 9999 年までを扱う (values.rs)。直近の月が 0 年より前になるときは
    // 0 年 1 月で止める (負の年は値の範囲と API の日付の形式の外になる)。
    let (year, month) = if total_months < 0 {
        (0, 1)
    } else {
        (
            total_months.div_euclid(12) as i32,
            total_months.rem_euclid(12) as u32 + 1,
        )
    };
    StatsPeriod {
        start: Some(format_day(LocalDate::new(year, month, 1))),
        end: Some(format_day(now.date())),
        granularity: StatsGranularity::Month,
    }
}
