//! 管理者画面の HTML エスケープ (`brew_book_admin/src/html.rs`) の PBT。

use brew_book_admin::html::escape;
use proptest::prelude::*;

/// エスケープが作る実体参照。
const ENTITIES: [&str; 5] = ["&amp;", "&lt;", "&gt;", "&quot;", "&#39;"];

proptest! {
    /// エスケープの結果には HTML の特殊文字が 1 つも残らない。
    #[test]
    fn escape_removes_the_html_special_characters(input in any::<String>()) {
        let escaped = escape(&input);
        prop_assert!(!escaped.contains('<'));
        prop_assert!(!escaped.contains('>'));
        prop_assert!(!escaped.contains('"'));
        prop_assert!(!escaped.contains('\''));
    }

    /// エスケープの結果の `&` は、必ず 5 つの実体参照の開始として現れる。
    #[test]
    fn escape_only_leaves_entities(input in any::<String>()) {
        let escaped = escape(&input);
        for (index, _) in escaped.match_indices('&') {
            prop_assert!(
                ENTITIES.iter().any(|entity| escaped[index..].starts_with(entity)),
                "a stray ampersand in {escaped:?}"
            );
        }
    }

    /// 特殊文字を含まない文字列は変化しない。
    #[test]
    fn escape_keeps_plain_text(input in "[^&<>\"']*") {
        prop_assert_eq!(escape(&input), input);
    }
}
