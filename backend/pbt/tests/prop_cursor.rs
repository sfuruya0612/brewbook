//! `cursor` の PBT。並び順のキーの符号化と復号の往復を検証する。

use brew_book_core::cursor::{CursorKey, CursorValue, SortKey, SortOrder};
use brew_book_core::datetime::{format_epoch_millis, MAX_EPOCH_MILLIS, MIN_EPOCH_MILLIS};
use proptest::prelude::*;

/// カーソルが運ぶ日時 (ISO 8601 UTC)。
fn datetime_text() -> impl Strategy<Value = String> {
    (MIN_EPOCH_MILLIS..=MAX_EPOCH_MILLIS)
        .prop_map(|epoch_millis| format_epoch_millis(epoch_millis).unwrap())
}

/// カーソルが運ぶ日付 (`YYYY-MM-DD`)。日はどの月にもある 28 日までにする。
fn date_text() -> impl Strategy<Value = String> {
    (0_i64..=9999, 1_u32..=12, 1_u32..=28)
        .prop_map(|(year, month, day)| format!("{year:04}-{month:02}-{day:02}"))
}

/// カーソルが運ぶ名前 (文字列)。
fn name_text() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9ぁ-ん一-龠 ]{0,12}".prop_map(String::from)
}

/// カーソルが運ぶ整数のキーの値。
fn integer_value() -> impl Strategy<Value = i64> {
    -1_000_000_i64..=1_000_000
}

/// カーソルが運ぶ小数のキーの値。
fn real_value() -> impl Strategy<Value = f64> {
    -1_000_000.0_f64..1_000_000.0
}

/// カーソルが運ぶ ID (UUID v4 の文字列)。
fn id_text() -> impl Strategy<Value = String> {
    "[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}"
}

/// 並び順のキーと、そのキーの型に合う値 (NULL を含む)。
fn keyed_value() -> impl Strategy<Value = (SortKey, Option<CursorValue>)> {
    prop_oneof![
        prop::option::of(datetime_text().prop_map(CursorValue::Text))
            .prop_map(|value| (SortKey::BrewedAt, value)),
        prop::option::of(datetime_text().prop_map(CursorValue::Text))
            .prop_map(|value| (SortKey::CreatedAt, value)),
        prop::option::of(datetime_text().prop_map(CursorValue::Text))
            .prop_map(|value| (SortKey::UpdatedAt, value)),
        prop::option::of(date_text().prop_map(CursorValue::Text))
            .prop_map(|value| (SortKey::PurchasedOn, value)),
        prop::option::of(name_text().prop_map(CursorValue::Text))
            .prop_map(|value| (SortKey::Name, value)),
        prop::option::of(integer_value().prop_map(CursorValue::Integer))
            .prop_map(|value| (SortKey::Rating, value)),
        prop::option::of(integer_value().prop_map(CursorValue::Integer))
            .prop_map(|value| (SortKey::PriceAmount, value)),
        prop::option::of(integer_value().prop_map(CursorValue::Integer))
            .prop_map(|value| (SortKey::WeightGrams, value)),
        prop::option::of(real_value().prop_map(CursorValue::Real))
            .prop_map(|value| (SortKey::DoseGrams, value)),
    ]
}

/// 並び順の方向。
fn order() -> impl Strategy<Value = SortOrder> {
    prop_oneof![Just(SortOrder::Asc), Just(SortOrder::Desc)]
}

proptest! {
    /// どのキーと方向と値のカーソルも、符号化して復号すると元に戻る。
    #[test]
    fn every_cursor_round_trips(
        (sort, value) in keyed_value(),
        order in order(),
        id in id_text(),
    ) {
        let cursor = CursorKey { sort, order, value, id };
        let encoded = cursor.encode();
        prop_assert!(is_unpadded_base64url(&encoded), "unexpected encoding {encoded}");
        prop_assert_eq!(CursorKey::decode(&encoded).unwrap(), cursor);
    }
}

/// パディング無しの base64url の文字だけからなるか。
fn is_unpadded_base64url(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}
