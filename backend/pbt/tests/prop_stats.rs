//! `stats` の PBT。期間の端を UTC の瞬間に直す往復、日時関数の修飾子の組み立て、粒度の検証、
//! 組み立てた SQL の不変条件を検査する。

use brew_book_core::datetime::{format_epoch_millis, parse_epoch_millis};
use brew_book_core::query::{Statement, Value};
use brew_book_core::stats::{self, Granularity, Period, StatsError};
use proptest::prelude::*;

/// 検査に使う利用者 ID。
const USER_ID: &str = "9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60";
/// 1 分のミリ秒。
const MILLIS_PER_MINUTE: i64 = 60_000;
/// 1 日のミリ秒。
const MILLIS_PER_DAY: i64 = 86_400_000;

/// 扱える日時の範囲の内側の日付 (1833 年ごろから 2243 年ごろまで)。
/// 端に近い日付はオフセットの加算で範囲の外になるため、検査には使わない。
fn a_date() -> impl Strategy<Value = String> {
    (-50_000_i64..100_000).prop_map(|days| {
        let text = format_epoch_millis(days * MILLIS_PER_DAY).expect("the day must format");
        text[..10].to_owned()
    })
}

/// オフセットの範囲 (-840 から 840) の値。
fn an_offset() -> impl Strategy<Value = i32> {
    -840_i32..=840
}

/// 区間の粒度。
fn a_granularity() -> impl Strategy<Value = Granularity> {
    prop_oneof![Just(Granularity::Day), Just(Granularity::Month)]
}

/// 開始日が終了日より後にならない期間。
fn a_period() -> impl Strategy<Value = Period> {
    (a_date(), a_date()).prop_map(|(first, second)| {
        let (start, end) = if first <= second {
            (first, second)
        } else {
            (second, first)
        };
        Period {
            start: Some(start),
            end: Some(end),
        }
    })
}

/// 日付の 00:00 の epoch ミリ秒。
fn midnight(date: &str) -> i64 {
    parse_epoch_millis(&format!("{date}T00:00:00.000Z")).expect("the date must parse")
}

/// 組み立てた文から、プレースホルダに束縛した文字列を取り出す。
fn bound_text(statement: &Statement, index: usize) -> String {
    match statement.params.get(index) {
        Some(Value::Text(text)) => text.clone(),
        other => panic!("the bound value must be a text: {other:?}"),
    }
}

proptest! {
    /// オフセットは範囲内の整数だけを受け付ける。
    #[test]
    fn an_offset_in_the_range_is_accepted_and_others_are_rejected(value in any::<i32>()) {
        let text = value.to_string();
        if (-840..=840).contains(&value) {
            prop_assert_eq!(stats::parse_offset_minutes(Some(&text)), Ok(value));
        } else {
            prop_assert_eq!(stats::parse_offset_minutes(Some(&text)), Err(StatsError::Offset));
        }
    }

    /// 修飾子は符号と数字と単位だけを持ち、整数に戻せる。
    #[test]
    fn the_offset_modifier_carries_only_the_sign_and_the_digits(offset in an_offset()) {
        let modifier = stats::offset_modifier(offset);
        let number = modifier
            .strip_suffix(" minutes")
            .expect("the modifier must end with the unit");
        prop_assert!(number.starts_with('+') || number.starts_with('-'));
        prop_assert_eq!(number.parse::<i32>(), Ok(offset));
        prop_assert!(!modifier.contains('?'));
    }

    /// 開始日の端は、オフセットを戻すと同じ日のローカル時刻の 00:00 になる。
    #[test]
    fn the_start_of_a_period_maps_back_to_the_same_local_midnight(
        date in a_date(),
        offset in an_offset(),
    ) {
        let period = Period { start: Some(date.clone()), end: None };
        let statement = stats::brews_stats(USER_ID, Granularity::Day, offset, &period)
            .expect("a date inside the supported range must convert");
        let utc = parse_epoch_millis(&bound_text(&statement, 1))
            .expect("the bound must be a fixed UTC timestamp");
        let local = format_epoch_millis(utc + i64::from(offset) * MILLIS_PER_MINUTE)
            .expect("the local time must format");
        prop_assert_eq!(&local[..10], date.as_str());
        prop_assert_eq!(&local[11..], "00:00:00.000Z");
    }

    /// 終了日の端は、オフセットを戻すと翌日のローカル時刻の 00:00 になる (終了日を含める)。
    #[test]
    fn the_end_of_a_period_maps_back_to_the_next_local_midnight(
        date in a_date(),
        offset in an_offset(),
    ) {
        let period = Period { start: None, end: Some(date.clone()) };
        let statement = stats::brews_stats(USER_ID, Granularity::Day, offset, &period)
            .expect("a date inside the supported range must convert");
        let utc = parse_epoch_millis(&bound_text(&statement, 1))
            .expect("the bound must be a fixed UTC timestamp");
        let local = format_epoch_millis(utc + i64::from(offset) * MILLIS_PER_MINUTE)
            .expect("the local time must format");
        let next_day = format_epoch_millis(midnight(&date) + MILLIS_PER_DAY)
            .expect("the next day must format");
        prop_assert_eq!(local, next_day);
    }

    /// 期間は、開始日が終了日より後でなければ受け付け、後なら拒否する。
    #[test]
    fn a_period_is_rejected_exactly_when_the_start_is_after_the_end(
        first in a_date(),
        second in a_date(),
    ) {
        let (start, end) = if first <= second {
            (first.clone(), second.clone())
        } else {
            (second.clone(), first.clone())
        };
        prop_assert!(stats::parse_period(Some(&start), Some(&end)).is_ok());
        if start != end {
            prop_assert_eq!(
                stats::parse_period(Some(&end), Some(&start)),
                Err(StatsError::ReversedPeriod)
            );
        }
    }

    /// 組み立てた文はどれも利用者 ID の条件を持ち、値がプレースホルダで渡る。
    #[test]
    fn every_generated_stats_query_keeps_the_user_condition(
        granularity in a_granularity(),
        offset in an_offset(),
        period in a_period(),
    ) {
        let statements = [
            stats::brews_stats(USER_ID, granularity, offset, &period)
                .expect("a date inside the supported range must convert"),
            stats::brew_ratings(USER_ID, offset, &period)
                .expect("a date inside the supported range must convert"),
            stats::purchases_stats(USER_ID, granularity, &period),
            stats::rating_history(USER_ID, USER_ID),
        ];
        for statement in statements {
            prop_assert!(statement.sql.contains("user_id = ?"), "{}", statement.sql);
            prop_assert!(statement.params.contains(&Value::Text(USER_ID.to_owned())));
            prop_assert_eq!(statement.sql.matches('?').count(), statement.params.len());
            // 利用者の入力の文字列は SQL に連結しない (ADR-0006)。
            prop_assert!(!statement.sql.contains(USER_ID), "{}", statement.sql);
        }
    }

    /// 区間のキーの形式は粒度で決まる。
    #[test]
    fn the_period_key_format_follows_the_granularity(
        granularity in a_granularity(),
        offset in an_offset(),
    ) {
        let format = match granularity {
            Granularity::Day => "%Y-%m-%d",
            Granularity::Month => "%Y-%m",
        };
        let key = format!("strftime('{format}'");
        let statement = stats::brews_stats(USER_ID, granularity, offset, &Period::default())
            .expect("an omitted period has no bound to convert");
        prop_assert!(statement.sql.contains(&key), "{}", statement.sql);
        let purchases = stats::purchases_stats(USER_ID, granularity, &Period::default());
        prop_assert!(purchases.sql.contains(&key), "{}", purchases.sql);
    }
}
