//! `datetime` の単体テスト。往復と単調性は PBT (`prop_datetime.rs`) が担う。

use brew_book_core::datetime::{
    format_epoch_millis, is_valid_date, parse_epoch_millis, DateTimeError, MAX_EPOCH_MILLIS,
    MIN_EPOCH_MILLIS,
};

#[test]
fn the_epoch_is_formatted_as_a_fixed_length_utc_string() {
    assert_eq!(format_epoch_millis(0).unwrap(), "1970-01-01T00:00:00.000Z");
    assert_eq!(format_epoch_millis(0).unwrap().len(), 24);
}

#[test]
fn milliseconds_keep_three_digits() {
    assert_eq!(format_epoch_millis(1).unwrap(), "1970-01-01T00:00:00.001Z");
    assert_eq!(
        format_epoch_millis(1_234).unwrap(),
        "1970-01-01T00:00:01.234Z"
    );
}

#[test]
fn a_negative_epoch_is_before_1970() {
    assert_eq!(format_epoch_millis(-1).unwrap(), "1969-12-31T23:59:59.999Z");
}

#[test]
fn the_supported_range_starts_at_year_zero_and_ends_at_year_9999() {
    assert_eq!(
        parse_epoch_millis("0000-01-01T00:00:00.000Z").unwrap(),
        MIN_EPOCH_MILLIS
    );
    assert_eq!(
        parse_epoch_millis("9999-12-31T23:59:59.999Z").unwrap(),
        MAX_EPOCH_MILLIS
    );
    assert_eq!(
        format_epoch_millis(MIN_EPOCH_MILLIS).unwrap(),
        "0000-01-01T00:00:00.000Z"
    );
    assert_eq!(
        format_epoch_millis(MAX_EPOCH_MILLIS).unwrap(),
        "9999-12-31T23:59:59.999Z"
    );
}

#[test]
fn formatting_rejects_times_outside_the_supported_range() {
    assert_eq!(
        format_epoch_millis(MAX_EPOCH_MILLIS + 1),
        Err(DateTimeError::OutOfRange)
    );
    assert_eq!(
        format_epoch_millis(MIN_EPOCH_MILLIS - 1),
        Err(DateTimeError::OutOfRange)
    );
}

#[test]
fn a_leap_day_is_valid_and_a_non_leap_day_is_not() {
    assert!(parse_epoch_millis("2024-02-29T00:00:00.000Z").is_ok());
    assert_eq!(
        parse_epoch_millis("2023-02-29T00:00:00.000Z"),
        Err(DateTimeError::Invalid)
    );
    // 100 で割り切れ 400 で割り切れない年はうるう年ではない。
    assert_eq!(
        parse_epoch_millis("1900-02-29T00:00:00.000Z"),
        Err(DateTimeError::Invalid)
    );
    assert!(parse_epoch_millis("2000-02-29T00:00:00.000Z").is_ok());
}

#[test]
fn parsing_rejects_a_value_that_is_not_the_fixed_length_form() {
    // ミリ秒が無い、末尾が Z でない、長さが違う、区切りが違う値は拒否する。
    for text in [
        "1970-01-01T00:00:00Z",
        "1970-01-01T00:00:00.000",
        "1970-01-01T00:00:00.000+00:00",
        "1970/01/01T00:00:00.000Z",
        "1970-01-01 00:00:00.000Z",
        "1970-01-01t00:00:00.000z",
        "1970-01-01T00:00:00.000Z ",
        "1970-01-01T00:00:00.00aZ",
        "",
    ] {
        assert_eq!(
            parse_epoch_millis(text),
            Err(DateTimeError::Invalid),
            "{text} must be rejected"
        );
    }
}

#[test]
fn parsing_rejects_values_out_of_the_calendar_range() {
    for text in [
        "1970-00-01T00:00:00.000Z",
        "1970-13-01T00:00:00.000Z",
        "1970-01-00T00:00:00.000Z",
        "1970-01-32T00:00:00.000Z",
        "2026-04-31T00:00:00.000Z",
        "1970-01-01T24:00:00.000Z",
        "1970-01-01T00:60:00.000Z",
        "1970-01-01T00:00:60.000Z",
    ] {
        assert_eq!(
            parse_epoch_millis(text),
            Err(DateTimeError::Invalid),
            "{text} must be rejected"
        );
    }
}

#[test]
fn date_validation_checks_the_shape_and_the_calendar() {
    for text in ["0000-01-01", "1970-01-01", "2024-02-29", "9999-12-31"] {
        assert!(is_valid_date(text), "{text} must be valid");
    }
    for text in [
        "2023-02-29",
        "2026-04-31",
        "2026-13-01",
        "2026-00-10",
        "2026-01-00",
        "2026-01-32",
        "2026-1-01",
        "2026-01-1",
        "2026-01-01T",
        "2026-01-01 ",
        "abcd-01-01",
        "",
    ] {
        assert!(!is_valid_date(text), "{text} must be invalid");
    }
}
