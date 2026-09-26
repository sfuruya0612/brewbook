//! 表示名の検証 (`brew_book_admin/src/input.rs`) の PBT (FR-17)。

use brew_book_admin::input::{validate_display_name, DISPLAY_NAME_MAX_CHARS};
use proptest::prelude::*;

/// 任意の文字列を作る (制御文字や空白も含む)。
fn any_text() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..80)
        .prop_map(|characters| characters.into_iter().collect::<String>())
}

proptest! {
    /// 前後の空白を除いた結果が 1 文字以上 50 文字以下なら、その文字列を返す。
    #[test]
    fn validate_returns_the_trimmed_name_when_in_range(input in any_text()) {
        let trimmed = input.trim();
        let count = trimmed.chars().count();
        let result = validate_display_name(&input);
        if count == 0 {
            prop_assert!(result.is_err(), "{input:?} must be rejected");
        } else if count > DISPLAY_NAME_MAX_CHARS {
            prop_assert!(result.is_err(), "{input:?} must be rejected");
        } else {
            prop_assert_eq!(result.expect("a valid name must be accepted"), trimmed);
        }
    }

    /// 前後の空白を除くと空になる入力は拒否する。
    #[test]
    fn validate_rejects_whitespace_only(input in "[ \t\n\r]*") {
        prop_assert!(validate_display_name(&input).is_err(), "{input:?} must be rejected");
    }

    /// 前後の空白を除いて 50 文字を超える入力は拒否する。
    ///
    /// 前後の空白は文字数に数えないため、超過する本体の前後に空白を付けた入力も拒否する。
    #[test]
    fn validate_rejects_names_over_the_limit(
        (prefix, name, suffix) in ("[\\s]*", "[^\\s]{51,80}", "[\\s]*")
    ) {
        let input = format!("{prefix}{name}{suffix}");
        prop_assert!(
            validate_display_name(&input).is_err(),
            "a name over the limit must be rejected: {input:?}"
        );
    }
}
