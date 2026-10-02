//! `records::values` の PBT。値の変換の往復と受理の条件 (ADR-0013)。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::records::values::{
    format_day, format_number, format_time, parse_count, parse_day, parse_decimal, parse_time,
    parse_utc_to_local, to_utc_iso8601, LocalDate, LocalDateTime,
};
use proptest::prelude::*;

/// 実在する日付 (月と日はどの月にもある範囲にする)。
fn valid_date() -> impl Strategy<Value = LocalDate> {
    (1..=9998i32, 1..=12u32, 1..=28u32)
        .prop_map(|(year, month, day)| LocalDate::new(year, month, day))
}

/// 端末のローカル日時。
fn valid_datetime() -> impl Strategy<Value = LocalDateTime> {
    (1..=9998i32, 1..=12u32, 1..=28u32, 0..24u32, 0..60u32).prop_map(
        |(year, month, day, hour, minute)| LocalDateTime::new(year, month, day, hour, minute),
    )
}

proptest! {
    /// 日付の整形と解釈の往復。
    #[test]
    fn a_formatted_day_is_parsed_back(date in valid_date()) {
        let text = format_day(date);
        prop_assert_eq!(text.len(), 10);
        prop_assert_eq!(&text[4..5], "-");
        prop_assert_eq!(&text[7..8], "-");
        prop_assert_eq!(parse_day(&text), Some(date));
    }

    /// 解釈できた日付は、元の文字列に戻る。
    #[test]
    fn a_parsed_day_round_trips_through_the_format(
        year in 0..=9999i32,
        month in 0..=99u32,
        day in 0..=99u32,
    ) {
        let text = format!("{year:04}-{month:02}-{day:02}");
        if let Some(date) = parse_day(&text) {
            prop_assert_eq!(format_day(date), text);
            prop_assert!((1..=12).contains(&date.month));
            prop_assert!((1..=31).contains(&date.day));
        }
    }

    /// 時刻の整形と解釈の往復。
    #[test]
    fn a_formatted_time_is_parsed_back(hour in 0..24u32, minute in 0..60u32) {
        prop_assert_eq!(parse_time(&format_time(hour, minute)), Some((hour, minute)));
    }

    /// 0 以上の整数の整形と解釈の往復。
    #[test]
    fn a_count_is_parsed_back(value in any::<u64>()) {
        prop_assert_eq!(parse_count(&value.to_string()), Some(value));
    }

    /// 数字以外を含む文字列は整数として読まない (前後の空白は除いてから読む)。
    ///
    /// 空白を除いた後に数字以外が残るときだけ拒否する (0042 の作業中に見つけた既存の
    /// テストの誤り。実装は `parse_count` のとおりで、期待値の側が除く前の文字列を見ていた)。
    #[test]
    fn a_count_with_a_non_digit_is_rejected(
        prefix in "[^0-9]*",
        digit in "[0-9]",
        suffix in "[^0-9]*",
    ) {
        let text = format!("{prefix}{digit}{suffix}");
        prop_assert_eq!(
            parse_count(&text).is_some(),
            text.trim().bytes().all(|byte| byte.is_ascii_digit()),
            "the parse must match the digits after trimming: {:?}",
            text
        );
    }

    /// 小数第 1 位までの数の整形と解釈の往復。
    #[test]
    fn a_number_is_formatted_and_parsed_back(integer in 0..1_000_000u64, tenth in 0..10u32) {
        let value = integer as f64 + f64::from(tenth) / 10.0;
        let expected = if tenth == 0 {
            integer.to_string()
        } else {
            format!("{integer}.{tenth}")
        };
        prop_assert_eq!(format_number(value), expected.clone());
        let parsed = parse_decimal(&expected).expect("the formatted number must be accepted");
        prop_assert!((parsed - value).abs() < 1e-9);
    }

    /// UTC の ISO 8601 の整形と解釈の往復 (タイムゾーンのオフセットを含む)。
    #[test]
    fn a_utc_string_is_read_back_as_the_local_datetime(
        datetime in valid_datetime(),
        offset in -840..=840i32,
    ) {
        let text = to_utc_iso8601(datetime, offset).expect("the datetime must be in range");
        prop_assert_eq!(text.len(), 24);
        prop_assert!(text.ends_with('Z'));
        prop_assert_eq!(parse_utc_to_local(&text, offset), Some(datetime));
    }
}
