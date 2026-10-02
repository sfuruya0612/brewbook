//! `records::values` の単体テスト。往復と受理の条件は PBT (`prop_values.rs`) が担う。

use brew_book_frontend::i18n::Language;
use brew_book_frontend::records::values::{
    display_day, display_timestamp, format_day, format_time, parse_count, parse_day, parse_decimal,
    parse_time, parse_utc_to_local, to_utc_iso8601, LocalDate, LocalDateTime,
};

#[test]
fn a_day_is_formatted_with_four_digit_year_and_two_digit_month_and_day() {
    assert_eq!(format_day(LocalDate::new(2026, 10, 2)), "2026-10-02");
    assert_eq!(format_day(LocalDate::new(7, 1, 9)), "0007-01-09");
}

#[test]
fn a_day_is_parsed_and_trimmed() {
    assert_eq!(parse_day("2026-10-02"), Some(LocalDate::new(2026, 10, 2)));
    assert_eq!(parse_day(" 2026-10-02 "), Some(LocalDate::new(2026, 10, 2)));
}

#[test]
fn a_day_that_does_not_exist_is_rejected() {
    assert_eq!(parse_day("2026-02-30"), None);
    assert_eq!(parse_day("2026-04-31"), None);
    assert_eq!(parse_day("2026-13-01"), None);
    assert_eq!(parse_day("2026-00-10"), None);
    assert_eq!(parse_day("2026-01-00"), None);
    assert_eq!(parse_day("2026-1-01"), None);
    assert_eq!(parse_day("2026/01/01"), None);
    assert_eq!(parse_day(""), None);
}

#[test]
fn a_leap_day_is_accepted_only_in_a_leap_year() {
    assert_eq!(parse_day("2024-02-29"), Some(LocalDate::new(2024, 2, 29)));
    assert_eq!(parse_day("2023-02-29"), None);
    assert_eq!(parse_day("2000-02-29"), Some(LocalDate::new(2000, 2, 29)));
    assert_eq!(parse_day("1900-02-29"), None);
}

#[test]
fn a_time_is_formatted_with_two_digits() {
    assert_eq!(format_time(9, 5), "09:05");
    assert_eq!(format_time(23, 59), "23:59");
}

#[test]
fn a_time_with_surrounding_spaces_is_accepted() {
    // 前後の空白を除く規則は入力の受理の規則であり、PBT の性質ではない (0040 以降の入力で使う)。
    assert_eq!(parse_time(" 23:59 "), Some((23, 59)));
    assert_eq!(parse_day(" 2026-10-02 "), Some(LocalDate::new(2026, 10, 2)));
}

#[test]
fn a_time_that_does_not_exist_is_rejected() {
    assert_eq!(parse_time("24:00"), None);
    assert_eq!(parse_time("12:60"), None);
    assert_eq!(parse_time("9:05"), None);
    assert_eq!(parse_time("0905"), None);
}

#[test]
fn a_count_accepts_only_digits() {
    assert_eq!(parse_count("0"), Some(0));
    assert_eq!(parse_count(" 42 "), Some(42));
    assert_eq!(parse_count("18446744073709551615"), Some(u64::MAX));
    assert_eq!(parse_count("18446744073709551616"), None);
    assert_eq!(parse_count("1.5"), None);
    assert_eq!(parse_count("-1"), None);
    assert_eq!(parse_count("+1"), None);
    assert_eq!(parse_count("1e3"), None);
    assert_eq!(parse_count(""), None);
    assert_eq!(parse_count("１２"), None);
}

#[test]
fn a_decimal_accepts_one_fraction_digit() {
    assert_eq!(parse_decimal("0"), Some(0.0));
    assert_eq!(parse_decimal(" 12.5 "), Some(12.5));
    assert_eq!(parse_decimal("12.05"), None);
    assert_eq!(parse_decimal(".5"), None);
    assert_eq!(parse_decimal("12."), None);
    assert_eq!(parse_decimal("-1"), None);
    assert_eq!(parse_decimal("1e3"), None);
    assert_eq!(parse_decimal(""), None);
}

#[test]
fn a_decimal_with_too_many_digits_is_rejected() {
    // f64 で表せない桁数の入力は inf になる。Backend の validate_decimal と同じく拒否する。
    let too_many_digits = "9".repeat(400);
    assert_eq!(parse_decimal(&too_many_digits), None);
}

#[test]
fn a_local_datetime_is_sent_as_a_fixed_length_utc_string() {
    let datetime = LocalDateTime::new(2026, 10, 2, 12, 34);
    // 日本標準時 (+540 分) の 12:34 は UTC の 03:34。
    assert_eq!(
        to_utc_iso8601(datetime, 540),
        Some("2026-10-02T03:34:00.000Z".to_string())
    );
    assert_eq!(to_utc_iso8601(datetime, 0).unwrap().len(), 24);
}

#[test]
fn a_utc_string_is_read_back_in_the_local_timezone() {
    assert_eq!(
        parse_utc_to_local("2026-10-02T03:34:00.000Z", 540),
        Some(LocalDateTime::new(2026, 10, 2, 12, 34))
    );
    // 秒とミリ秒は切り捨てる (画面の入力は分まで)。
    assert_eq!(
        parse_utc_to_local("2026-10-02T03:34:59.999Z", 540),
        Some(LocalDateTime::new(2026, 10, 2, 12, 34))
    );
    // 日付をまたぐオフセット。
    assert_eq!(
        parse_utc_to_local("2026-10-02T23:30:00.000Z", 540),
        Some(LocalDateTime::new(2026, 10, 3, 8, 30))
    );
}

#[test]
fn a_utc_string_out_of_the_fixed_format_is_rejected() {
    assert_eq!(parse_utc_to_local("2026-10-02T03:34:00Z", 540), None);
    assert_eq!(parse_utc_to_local("2026-10-02T03:34:00.000", 540), None);
    assert_eq!(parse_utc_to_local("2026-10-02 03:34:00.000Z", 540), None);
    assert_eq!(parse_utc_to_local("2026-02-30T03:34:00.000Z", 540), None);
    assert_eq!(parse_utc_to_local("2026-10-02T24:00:00.000Z", 540), None);
    assert_eq!(parse_utc_to_local("", 540), None);
}

#[test]
fn the_supported_range_of_the_utc_string_is_year_zero_to_9999() {
    // 0 年 1 月 1 日の 00:00 より前は None。
    assert_eq!(to_utc_iso8601(LocalDateTime::new(0, 1, 1, 0, 0), 540), None);
    // 9999 年 12 月 31 日の 23:59 より後は None。
    assert_eq!(
        to_utc_iso8601(LocalDateTime::new(9999, 12, 31, 23, 59), -540),
        None
    );
}

#[test]
fn a_day_is_displayed_in_the_language_order() {
    let day = LocalDate::new(2026, 10, 2);
    assert_eq!(display_day(day, Language::Japanese), "2026/10/2");
    assert_eq!(display_day(day, Language::English), "10/2/2026");
}

#[test]
fn a_timestamp_is_displayed_with_24_hour_time() {
    let datetime = LocalDateTime::new(2026, 10, 2, 13, 45);
    assert_eq!(
        display_timestamp(datetime, Language::Japanese),
        "2026/10/2 13:45"
    );
    assert_eq!(
        display_timestamp(datetime, Language::English),
        "10/2/2026 13:45"
    );
}

#[test]
fn days_between_counts_the_elapsed_days() {
    use brew_book_frontend::records::values::days_between;
    assert_eq!(
        days_between(LocalDate::new(2026, 10, 1), LocalDate::new(2026, 10, 2)),
        1
    );
    assert_eq!(
        days_between(LocalDate::new(2026, 10, 2), LocalDate::new(2026, 10, 1)),
        -1
    );
    assert_eq!(
        days_between(LocalDate::new(2024, 2, 28), LocalDate::new(2024, 3, 1)),
        2
    );
}
