//! `records` の単体テスト。入力の誤りと応答の対応、日付と日時と数値の検証の境界を確認する。
//!
//! 値の扱いの性質 (前後の空白の除去、空白だけの値の拒否、タグ名の正規化) は PBT
//! (`prop_records.rs`) が担う。

use coffee_log_core::records::{
    validate_count, validate_currency, validate_day, validate_decimal, validate_rating,
    validate_timestamp, CountError, CurrencyError, DayError, DecimalError, NameError, RatingError,
    TagNameError, TimestampError,
};

/// 検証に使う JSON の数値の文字列表現。検査は表記に対して行う (0007 の設計判断)。
/// Worker は `serde_json` の数値を `to_string` した値を渡す。
fn decimal(text: &str) -> Result<f64, DecimalError> {
    validate_decimal(text)
}

#[test]
fn every_input_error_maps_to_a_bad_request() {
    assert_eq!(NameError::Empty.code().status(), 400);
    assert_eq!(TagNameError::Empty.code().status(), 400);
    assert_eq!(DayError::Invalid.code().status(), 400);
    assert_eq!(TimestampError::Invalid.code().status(), 400);
    assert_eq!(CountError::Negative.code().status(), 400);
    assert_eq!(CountError::TooLarge.code().status(), 400);
    assert_eq!(RatingError::OutOfRange.code().status(), 400);
    assert_eq!(CurrencyError::NotThreeUppercaseLetters.code().status(), 400);
    assert_eq!(DecimalError::Negative.code().status(), 400);
    assert_eq!(DecimalError::NotOneDecimalPlace.code().status(), 400);
    assert_eq!(DecimalError::OutOfRange.code().status(), 400);
    assert!(!NameError::Empty.message().is_empty());
    assert!(!DayError::Invalid.message().is_empty());
    assert!(!TimestampError::Invalid.message().is_empty());
    assert!(!CountError::Negative.message().is_empty());
    assert!(!RatingError::OutOfRange.message().is_empty());
    assert!(!CurrencyError::NotThreeUppercaseLetters.message().is_empty());
    assert!(!DecimalError::Negative.message().is_empty());
}

#[test]
fn the_day_validation_accepts_existing_dates_only() {
    // 実在する日付だけを受け付ける (うるう年を考慮する)。
    for text in [
        "2026-09-21",
        "2024-02-29",
        "2000-02-29",
        "0001-01-01",
        "9999-12-31",
    ] {
        assert_eq!(validate_day(text), Ok(()), "{text} must be accepted");
    }
    for text in [
        "",
        // 実在しない日付。
        "2026-02-30",
        "2026-04-31",
        "2026-02-29",
        "2100-02-29",
        "2026-13-01",
        "2026-00-10",
        "2026-09-00",
        "0000-00-00",
        // 形式が違う値。
        "2026-9-1",
        "2026-09-1",
        "26-09-21",
        "2026/09/21",
        "2026-09-21T00:00:00.000Z",
        "2026-09-21 ",
        " 2026-09-21",
        "2026-09-2",
        "2026-09-211",
    ] {
        assert_eq!(
            validate_day(text),
            Err(DayError::Invalid),
            "{text} must be rejected"
        );
    }
}

#[test]
fn the_timestamp_validation_accepts_the_fixed_utc_form_only() {
    // 末尾が `Z` で小数秒がミリ秒 3 桁の固定長だけを受け付ける (ADR-0002)。
    for text in [
        "2026-09-21T00:00:00.000Z",
        "2024-02-29T23:59:59.999Z",
        "0000-01-01T00:00:00.000Z",
        "9999-12-31T23:59:59.999Z",
    ] {
        assert_eq!(validate_timestamp(text), Ok(()), "{text} must be accepted");
    }
    for text in [
        "",
        // オフセット付きの日時は受け付けない。
        "2026-09-21T00:00:00.000+09:00",
        "2026-09-21T09:00:00.000+09:00",
        "2026-09-21T00:00:00.000-05:00",
        // 小数秒の桁数が違う値、末尾が `Z` でない値。
        "2026-09-21T00:00:00Z",
        "2026-09-21T00:00:00.00Z",
        "2026-09-21T00:00:00.0000Z",
        "2026-09-21T00:00:00.000",
        "2026-09-21T00:00:00.000z",
        "2026-09-21 00:00:00.000Z",
        "2026-09-21t00:00:00.000Z",
        // 実在しない日時。
        "2026-02-30T00:00:00.000Z",
        "2026-09-21T24:00:00.000Z",
        "2026-09-21T00:60:00.000Z",
        "2026-09-21T00:00:60.000Z",
        "2026-09-21",
        "2026-09-21T00:00:00.000Z ",
    ] {
        assert_eq!(
            validate_timestamp(text),
            Err(TimestampError::Invalid),
            "{text} must be rejected"
        );
    }
}

#[test]
fn the_count_validation_accepts_zero_and_rejects_negative_and_too_large_values() {
    // 価格、重量、時間は 0 以上の整数だけを受け付ける (FR-9、FR-11)。
    assert_eq!(validate_count(0), Ok(0));
    assert_eq!(validate_count(1200), Ok(1200));
    assert_eq!(validate_count(i64::from(i32::MAX)), Ok(i64::from(i32::MAX)));
    assert_eq!(validate_count(-1), Err(CountError::Negative));
    assert_eq!(
        validate_count(i64::from(i32::MAX) + 1),
        Err(CountError::TooLarge)
    );
    assert_eq!(validate_count(i64::MAX), Err(CountError::TooLarge));
}

#[test]
fn the_rating_validation_accepts_one_to_five_only() {
    for value in 1..=5 {
        assert_eq!(validate_rating(value), Ok(value));
    }
    for value in [0, 6, -1, i64::MAX] {
        assert_eq!(validate_rating(value), Err(RatingError::OutOfRange));
    }
}

#[test]
fn the_currency_validation_accepts_three_uppercase_letters_only() {
    // ISO 4217 の 3 文字の英大文字だけを受け付ける (FR-9)。
    for text in ["JPY", "USD", "EUR", "ABC", "XXX"] {
        assert_eq!(validate_currency(text), Ok(text.to_owned()), "{text}");
    }
    for text in [
        "",
        " ",
        "JP",
        "JPYY",
        "jpy",
        "Jpy",
        "Usd",
        "JP1",
        "12A",
        "あいう",
        "JPY ",
    ] {
        assert_eq!(
            validate_currency(text),
            Err(CurrencyError::NotThreeUppercaseLetters),
            "{text} must be rejected"
        );
    }
}

#[test]
fn the_decimal_validation_accepts_values_with_at_most_one_decimal_place() {
    // 0 以上で小数第 1 位までの値だけを受け付ける (FR-11)。
    assert_eq!(decimal("0"), Ok(0.0));
    assert_eq!(decimal("0.0"), Ok(0.0));
    assert_eq!(decimal("0.1"), Ok(0.1));
    assert_eq!(decimal("15.5"), Ok(15.5));
    assert_eq!(decimal("250"), Ok(250.0));
    assert_eq!(decimal("92.5"), Ok(92.5));
    assert_eq!(decimal("1200"), Ok(1200.0));
    // 負の値。
    assert_eq!(decimal("-1"), Err(DecimalError::Negative));
    assert_eq!(decimal("-0.1"), Err(DecimalError::Negative));
    // 小数第 2 位以下を持つ値。
    assert_eq!(decimal("1.25"), Err(DecimalError::NotOneDecimalPlace));
    assert_eq!(decimal("0.01"), Err(DecimalError::NotOneDecimalPlace));
    assert_eq!(decimal("92.55"), Err(DecimalError::NotOneDecimalPlace));
    // 指数表記は小数第 1 位までという条件を表記で判定できないため受け付けない。
    assert_eq!(decimal("1e300"), Err(DecimalError::NotOneDecimalPlace));
    // 数値の表記でない値。
    assert_eq!(decimal(""), Err(DecimalError::NotOneDecimalPlace));
    assert_eq!(decimal("1.2.3"), Err(DecimalError::NotOneDecimalPlace));
    assert_eq!(decimal(" 1.5"), Err(DecimalError::NotOneDecimalPlace));
    assert_eq!(decimal("1.5 "), Err(DecimalError::NotOneDecimalPlace));
    // 桁が大きすぎて f64 に収まらない値 (防御の分岐)。
    let huge = format!("1{}", "0".repeat(400));
    assert_eq!(decimal(&huge), Err(DecimalError::OutOfRange));
}
