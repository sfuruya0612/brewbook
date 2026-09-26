//! `datetime` の PBT。整形と解釈の往復、辞書順の比較と時刻順の一致を検証する。

use brew_book_core::datetime::{
    format_epoch_millis, parse_epoch_millis, MAX_EPOCH_MILLIS, MIN_EPOCH_MILLIS,
};
use proptest::prelude::*;

proptest! {
    /// 整形して解釈し直すと同じ epoch ミリ秒に戻る。
    #[test]
    fn format_and_parse_round_trip(epoch_millis in MIN_EPOCH_MILLIS..=MAX_EPOCH_MILLIS) {
        let text = format_epoch_millis(epoch_millis).unwrap();
        prop_assert_eq!(parse_epoch_millis(&text).unwrap(), epoch_millis);
    }

    /// 文字列の辞書順の比較が時刻の順と一致する。
    #[test]
    fn the_string_order_matches_the_time_order(
        first in MIN_EPOCH_MILLIS..=MAX_EPOCH_MILLIS,
        second in MIN_EPOCH_MILLIS..=MAX_EPOCH_MILLIS,
    ) {
        let first_text = format_epoch_millis(first).unwrap();
        let second_text = format_epoch_millis(second).unwrap();
        prop_assert_eq!(first <= second, first_text <= second_text);
    }

    /// 整形した文字列は 24 文字の固定長で、ミリ秒 3 桁と末尾の Z を持つ。
    #[test]
    fn the_format_is_a_fixed_length_utc_string(epoch_millis in MIN_EPOCH_MILLIS..=MAX_EPOCH_MILLIS) {
        let text = format_epoch_millis(epoch_millis).unwrap();
        prop_assert_eq!(text.len(), 24);
        prop_assert_eq!(&text[10..11], "T");
        prop_assert_eq!(&text[19..20], ".");
        prop_assert_eq!(&text[23..24], "Z");
    }
}
