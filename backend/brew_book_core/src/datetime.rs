//! 日時の ISO 8601 UTC の固定長文字列 (ADR-0002)。
//!
//! 形式は `YYYY-MM-DDTHH:MM:SS.mmmZ` の 24 文字に固定する。ミリ秒を 3 桁で必ず出し、
//! 末尾を `Z` にするため、文字列の辞書順の比較が時刻の順と一致する。
//! 現在時刻は呼び出し側 (Worker) が `worker::Date` から取り、このモジュールには epoch
//! ミリ秒で渡す。整形と比較だけなら `chrono` や `time` は要らない。

/// 対応する最も古い時刻 (`0000-01-01T00:00:00.000Z`) の epoch ミリ秒。
pub const MIN_EPOCH_MILLIS: i64 = -62_167_219_200_000;
/// 対応する最も新しい時刻 (`9999-12-31T23:59:59.999Z`) の epoch ミリ秒。
pub const MAX_EPOCH_MILLIS: i64 = 253_402_300_799_999;

/// 日時の整形と解釈の誤り。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTimeError {
    /// 対応する範囲 (0000 年から 9999 年) の外。
    OutOfRange,
    /// 形式または値が不正。
    Invalid,
}

const MILLIS_PER_SECOND: i64 = 1_000;
const MILLIS_PER_MINUTE: i64 = 60 * MILLIS_PER_SECOND;
const MILLIS_PER_HOUR: i64 = 60 * MILLIS_PER_MINUTE;
const MILLIS_PER_DAY: i64 = 24 * MILLIS_PER_HOUR;

/// epoch ミリ秒を ISO 8601 UTC の固定長文字列に整形する。
pub fn format_epoch_millis(epoch_millis: i64) -> Result<String, DateTimeError> {
    let days = epoch_millis.div_euclid(MILLIS_PER_DAY);
    let millis_of_day = epoch_millis.rem_euclid(MILLIS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    if !(0..=9999).contains(&year) {
        return Err(DateTimeError::OutOfRange);
    }
    let seconds_of_day = millis_of_day / MILLIS_PER_SECOND;
    let millis = millis_of_day % MILLIS_PER_SECOND;
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day / 60) % 60;
    let second = seconds_of_day % 60;
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z"
    ))
}

/// ISO 8601 UTC の固定長文字列 (`YYYY-MM-DDTHH:MM:SS.mmmZ`) を epoch ミリ秒に戻す。
///
/// 形式が固定長でない値と、範囲外の日時 (うるう年を考慮した日付を含む) は拒否する。
/// うるう秒は受け付けない (60 秒は不正とする)。
pub fn parse_epoch_millis(text: &str) -> Result<i64, DateTimeError> {
    let bytes = text.as_bytes();
    // YYYY-MM-DDTHH:MM:SS.mmmZ の 24 文字。区切り文字の位置も固定する。
    const SEPARATORS: [(usize, u8); 6] = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'.'),
    ];
    if bytes.len() != 24 || bytes[23] != b'Z' {
        return Err(DateTimeError::Invalid);
    }
    for (index, separator) in SEPARATORS {
        if bytes[index] != separator {
            return Err(DateTimeError::Invalid);
        }
    }
    let digits = |range: std::ops::Range<usize>| -> Option<i64> {
        let mut value = 0_i64;
        for byte in &bytes[range] {
            if !byte.is_ascii_digit() {
                return None;
            }
            value = value * 10 + i64::from(byte - b'0');
        }
        Some(value)
    };
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second), Some(millis)) = (
        digits(0..4),
        digits(5..7),
        digits(8..10),
        digits(11..13),
        digits(14..16),
        digits(17..19),
        digits(20..23),
    ) else {
        return Err(DateTimeError::Invalid);
    };
    let (month, day, hour, minute, second, millis) = (
        u32::try_from(month).map_err(|_| DateTimeError::Invalid)?,
        u32::try_from(day).map_err(|_| DateTimeError::Invalid)?,
        u32::try_from(hour).map_err(|_| DateTimeError::Invalid)?,
        u32::try_from(minute).map_err(|_| DateTimeError::Invalid)?,
        u32::try_from(second).map_err(|_| DateTimeError::Invalid)?,
        u32::try_from(millis).map_err(|_| DateTimeError::Invalid)?,
    );
    if !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(DateTimeError::Invalid);
    }
    let days = days_from_civil(year, month, day);
    Ok(days * MILLIS_PER_DAY
        + i64::from(hour) * MILLIS_PER_HOUR
        + i64::from(minute) * MILLIS_PER_MINUTE
        + i64::from(second) * MILLIS_PER_SECOND
        + i64::from(millis))
}

/// `YYYY-MM-DD` の日付として妥当か (うるう年を考慮する)。
pub fn is_valid_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| -> Option<i64> {
        let mut value = 0_i64;
        for byte in &bytes[range] {
            if !byte.is_ascii_digit() {
                return None;
            }
            value = value * 10 + i64::from(byte - b'0');
        }
        Some(value)
    };
    let (Some(year), Some(month), Some(day)) = (digits(0..4), digits(5..7), digits(8..10)) else {
        return false;
    };
    let (month, day) = (month as u32, day as u32);
    (1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day)
}

/// その年月の日数。うるう年はグレゴリオ暦の規則で判定する。
fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// 西暦の年月日を、1970-01-01 からの経過日数にする。
/// Howard Hinnant の `days_from_civil` (グレゴリオ暦、負の年も扱える)。
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// 1970-01-01 からの経過日数を、西暦の年月日にする。
/// Howard Hinnant の `civil_from_days`。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = if month_part < 10 {
        month_part + 3
    } else {
        month_part - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month as u32, day as u32)
}
