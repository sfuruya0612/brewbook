//! 入力の値の読み書き (FR-9、FR-11、FR-16)。
//!
//! 日付は `YYYY-MM-DD`、時刻は `HH:MM`、抽出日時は ISO 8601 の UTC で API と受け渡しする。
//! 画面の入力は端末のローカル時刻で行い、送信の直前に UTC へ変換する (FR-11)。
//! Dioxus に依存しない純粋なモジュールにして、native の単体テストと PBT で守る (ADR-0013)。

use crate::i18n::Language;

/// 端末のタイムゾーンでの日付 (FR-9、FR-11、FR-18)。
///
/// 年月日は 0 から 9999 の範囲を扱う (Backend の日時の範囲と同じ)。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct LocalDate {
    /// 年。
    pub year: i32,
    /// 月 (1 から 12)。
    pub month: u32,
    /// 日 (1 から 31)。
    pub day: u32,
}

impl LocalDate {
    /// 年月日から作る。実在しない日付は [`LocalDate::is_valid`] が false になる。
    pub const fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    /// 実在する日付か (うるう年を考慮する)。
    pub fn is_valid(self) -> bool {
        (0..=9999).contains(&self.year)
            && (1..=12).contains(&self.month)
            && (1..=days_in_month(self.year, self.month)).contains(&self.day)
    }
}

/// 端末のタイムゾーンでの日時 (FR-11)。
///
/// 画面の入力は分までなので、秒とミリ秒は持たない。API の応答の秒は [`parse_utc_to_local`] が
/// 切り捨てる。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LocalDateTime {
    /// 年。
    pub year: i32,
    /// 月 (1 から 12)。
    pub month: u32,
    /// 日 (1 から 31)。
    pub day: u32,
    /// 時 (0 から 23)。
    pub hour: u32,
    /// 分 (0 から 59)。
    pub minute: u32,
}

impl LocalDateTime {
    /// 年月日と時分から作る。
    pub const fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
        }
    }

    /// 日付の部分。
    pub const fn date(self) -> LocalDate {
        LocalDate::new(self.year, self.month, self.day)
    }
}

/// 日付 (`YYYY-MM-DD`) にする。
pub fn format_day(day: LocalDate) -> String {
    format!("{:04}-{:02}-{:02}", day.year, day.month, day.day)
}

/// 日付 (`YYYY-MM-DD`) を読む。実在しない日付と形式の違反は None にする。
pub fn parse_day(text: &str) -> Option<LocalDate> {
    let bytes = text.trim().as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year = parse_digits(&bytes[0..4])? as i32;
    let month = parse_digits(&bytes[5..7])? as u32;
    let day = parse_digits(&bytes[8..10])? as u32;
    let date = LocalDate::new(year, month, day);
    date.is_valid().then_some(date)
}

/// 時刻 (`HH:MM`) を読む。読めなければ None にする。
pub fn parse_time(text: &str) -> Option<(u32, u32)> {
    let bytes = text.trim().as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return None;
    }
    let hour = parse_digits(&bytes[0..2])? as u32;
    let minute = parse_digits(&bytes[3..5])? as u32;
    if hour > 23 || minute > 59 {
        return None;
    }
    Some((hour, minute))
}

/// 時刻を `HH:MM` にする。
pub fn format_time(hour: u32, minute: u32) -> String {
    format!("{hour:02}:{minute:02}")
}

/// 0 以上の整数を読む。読めなければ None にする (FR-9、FR-11)。
///
/// 小数と符号と指数と、`u64` に入らない値は受け付けない。
pub fn parse_count(text: &str) -> Option<u64> {
    let value = text.trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse::<u64>().ok()
}

/// 0 以上で小数第 1 位までの数を読む。読めなければ None にする (FR-11)。
///
/// API の検証と同じ条件 (`^\d+(\.\d)?$`) にする。符号、指数、2 桁以上の小数は受け付けない。
pub fn parse_decimal(text: &str) -> Option<f64> {
    let value = text.trim();
    let (integer, fraction) = match value.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (value, None),
    };
    if integer.is_empty() || !integer.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = fraction {
        if fraction.len() != 1 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
    }
    let parsed = value.parse::<f64>().ok()?;
    // f64 で表せない桁数の入力は inf になる。Backend の validate_decimal は非有限を拒否するため、
    // クライアントも拒否する (0040 以降の入力検証がサーバーと食い違わないようにする)。
    if !parsed.is_finite() {
        return None;
    }
    Some(parsed)
}

/// 数を表示用の文字列にする。整数のときは小数部を付けない (FR-11)。
pub fn format_number(value: f64) -> String {
    if value == value.round() {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// 端末のローカル時刻の日時を、送信用の ISO 8601 の UTC (`YYYY-MM-DDTHH:MM:SS.mmmZ`) にする
/// (FR-11)。範囲の外 (0 年より前、9999 年より後) は None にする。
pub fn to_utc_iso8601(datetime: LocalDateTime, utc_offset_minutes: i32) -> Option<String> {
    let days = days_from_civil(datetime.year, datetime.month, datetime.day);
    let minutes =
        days * MINUTES_PER_DAY + i64::from(datetime.hour) * 60 + i64::from(datetime.minute)
            - i64::from(utc_offset_minutes);
    let utc_days = minutes.div_euclid(MINUTES_PER_DAY);
    let minutes_of_day = minutes.rem_euclid(MINUTES_PER_DAY);
    let (year, month, day) = civil_from_days(utc_days);
    if !(0..=9999).contains(&year) {
        return None;
    }
    let hour = minutes_of_day / 60;
    let minute = minutes_of_day % 60;
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:00.000Z"
    ))
}

/// ISO 8601 の UTC (`YYYY-MM-DDTHH:MM:SS.mmmZ`、Backend と同じ 24 文字) を、端末の
/// ローカル時刻にする (表示用。FR-11)。
///
/// 秒とミリ秒は切り捨てる (画面の入力は分まで)。
pub fn parse_utc_to_local(text: &str, utc_offset_minutes: i32) -> Option<LocalDateTime> {
    let bytes = text.as_bytes();
    const SEPARATORS: [(usize, u8); 6] = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'.'),
    ];
    if bytes.len() != 24 || bytes[23] != b'Z' {
        return None;
    }
    for (index, separator) in SEPARATORS {
        if bytes[index] != separator {
            return None;
        }
    }
    let year = parse_digits(&bytes[0..4])? as i32;
    let month = parse_digits(&bytes[5..7])? as u32;
    let day = parse_digits(&bytes[8..10])? as u32;
    let hour = parse_digits(&bytes[11..13])? as u32;
    let minute = parse_digits(&bytes[14..16])? as u32;
    let second = parse_digits(&bytes[17..19])? as u32;
    let millis = parse_digits(&bytes[20..23])? as u32;
    let date = LocalDate::new(year, month, day);
    if !date.is_valid() || hour > 23 || minute > 59 || second > 59 || millis > 999 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let minutes = days * MINUTES_PER_DAY
        + i64::from(hour) * 60
        + i64::from(minute)
        + i64::from(utc_offset_minutes);
    let local_days = minutes.div_euclid(MINUTES_PER_DAY);
    let minutes_of_day = minutes.rem_euclid(MINUTES_PER_DAY);
    let (year, month, day) = civil_from_days(local_days);
    if !(0..=9999).contains(&year) {
        return None;
    }
    Some(LocalDateTime::new(
        year,
        month,
        day,
        (minutes_of_day / 60) as u32,
        (minutes_of_day % 60) as u32,
    ))
}

/// 2 つの日付の間の経過日数 (開始日から終了日まで。終了日が前なら負)。
///
/// 日付だけの値で数えるため、夏時間の切り替えの影響を受けない。
pub fn days_between(start: LocalDate, end: LocalDate) -> i64 {
    days_from_civil(end.year, end.month, end.day)
        - days_from_civil(start.year, start.month, start.day)
}

/// 日付を言語に合わせて表示する (FR-16)。
///
/// 日本語は `2026/10/2`、英語は `10/2/2026` にする。入力と送信の形式 (`YYYY-MM-DD`) とは
/// 別に、一覧と詳細の表示だけに使う。
pub fn display_day(day: LocalDate, language: Language) -> String {
    match language {
        Language::Japanese => format!("{}/{}/{}", day.year, day.month, day.day),
        Language::English => format!("{}/{}/{}", day.month, day.day, day.year),
    }
}

/// UTC の日時を端末のタイムゾーンと言語で表示する (FR-11、FR-16)。
///
/// 時刻は 24 時間の `HH:MM` にする (デザインの規約)。
pub fn display_timestamp(datetime: LocalDateTime, language: Language) -> String {
    format!(
        "{} {:02}:{:02}",
        display_day(datetime.date(), language),
        datetime.hour,
        datetime.minute
    )
}

/// その年月の日数。うるう年はグレゴリオ暦の規則で判定する。
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// うるう年か。
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// ASCII の数字だけの並びを数にする。空と数字以外は None。
fn parse_digits(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() {
        return None;
    }
    let mut value = 0_u64;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value * 10 + u64::from(byte - b'0');
    }
    Some(value)
}

const MINUTES_PER_DAY: i64 = 24 * 60;

/// 西暦の年月日を、1970-01-01 からの経過日数にする。
/// Howard Hinnant の `days_from_civil` (グレゴリオ暦、負の年も扱える)。
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(if month <= 2 { year - 1 } else { year });
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// 1970-01-01 からの経過日数を、西暦の年月日にする。
/// Howard Hinnant の `civil_from_days`。
fn civil_from_days(days: i64) -> (i32, u32, u32) {
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
    (year as i32, month as u32, day as u32)
}
