//! `cursor` の PBT。並び順のキーの符号化と復号の往復を検証する。

use brew_book_core::cursor::CursorKey;
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

/// カーソルが運ぶ ID (UUID v4 の文字列)。
fn id_text() -> impl Strategy<Value = String> {
    "[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}"
}

proptest! {
    /// 日時のカーソルは符号化して復号すると元に戻る。
    #[test]
    fn a_datetime_cursor_round_trips(at in datetime_text(), id in id_text()) {
        let cursor = CursorKey::DateTime { at, id };
        let encoded = cursor.encode();
        prop_assert!(is_unpadded_base64url(&encoded), "unexpected encoding {encoded}");
        prop_assert_eq!(CursorKey::decode(&encoded).unwrap(), cursor);
    }

    /// 日付のカーソルは符号化して復号すると元に戻る。
    #[test]
    fn a_date_cursor_round_trips(on in date_text(), id in id_text()) {
        let cursor = CursorKey::Date { on, id };
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
