//! `records` の PBT。前後の空白の扱いと、Flavor Notes の正規化の性質を検査する。

use coffee_log_core::records::{
    trim_optional, trim_text, validate_flavor_notes, validate_name, NameError, TagNameError,
};
use proptest::prelude::*;

/// 前後の空白に使う値。全角の空白と改行とタブを含める。
fn whitespace() -> impl Strategy<Value = String> {
    prop::sample::select(vec![" ", "\t", "\n", "  ", "\u{3000}", "\u{00a0}"]).prop_map(String::from)
}

/// 名前や自由記述に使う単語。内部に空白を含む値も生成する
/// (前後の空白だけを除き、内部の空白を変えないことの検査のため)。
fn word() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z0-9]{1,6}",
        "[ぁ-ん]{1,4}",
        "[一-龠]{1,4}",
        "[a-zA-Z0-9]{1,3}[ \u{3000}]{1,2}[a-zA-Z0-9]{1,3}",
    ]
    .prop_map(String::from)
}

/// 前後に空白が付いた単語。空白だけの値も生成できる (単語の長さを 0 にする)。
fn padded_word() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(whitespace(), 0..3),
        prop::option::of(word()),
        prop::collection::vec(whitespace(), 0..3),
    )
        .prop_map(|(before, word, after)| {
            format!(
                "{}{}{}",
                before.concat(),
                word.unwrap_or_default(),
                after.concat()
            )
        })
}

proptest! {
    /// 名前の検証は、前後の空白を除いた値をそのまま返す。
    #[test]
    fn a_name_is_rejected_only_when_the_trimmed_value_is_empty(name in padded_word()) {
        let trimmed = name.trim();
        let validated = validate_name(&name);
        if trimmed.is_empty() {
            prop_assert_eq!(validated, Err(NameError::Empty));
        } else {
            prop_assert_eq!(validated.as_deref(), Ok(trimmed));
        }
    }

    /// 空白だけの名前は必ず拒否する。
    #[test]
    fn a_blank_name_is_rejected(parts in prop::collection::vec(whitespace(), 0..4)) {
        prop_assert_eq!(validate_name(&parts.concat()), Err(NameError::Empty));
    }

    /// 自由記述の正規化は前後の空白を除くだけで、もう一度かけても変わらない。
    #[test]
    fn the_trimmed_text_is_stable(text in padded_word()) {
        let trimmed = trim_text(&text);
        prop_assert_eq!(trimmed.as_str(), text.trim());
        prop_assert_eq!(trim_text(&trimmed), trimmed);
    }

    /// 任意の値の正規化は、無い値 (None) をそのまま保つ。
    #[test]
    fn the_optional_text_keeps_the_missing_value(text in prop::option::of(padded_word())) {
        let expected = text.as_deref().map(str::trim).map(str::to_owned);
        prop_assert_eq!(trim_optional(text.as_deref()), expected);
    }

    /// タグ名は、空白だけの名前が 1 つでもあれば拒否し、そうでなければ前後の空白を除いて
    /// 重複を除いた名前の昇順の並びを返す。
    #[test]
    fn the_flavor_notes_are_normalized_sorted_and_unique(
        names in prop::collection::vec(padded_word(), 0..6),
    ) {
        let mut expected = Vec::new();
        let mut rejected = false;
        for name in &names {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                rejected = true;
            } else {
                expected.push(trimmed.to_owned());
            }
        }
        expected.sort_unstable();
        expected.dedup();

        let validated = validate_flavor_notes(&names);
        if rejected {
            prop_assert_eq!(validated, Err(TagNameError::Empty));
        } else {
            prop_assert_eq!(validated, Ok(expected));
        }
    }

    /// 正規化したタグ名は空でなく、前後の空白を持たず、重複しない。
    #[test]
    fn the_normalized_flavor_notes_are_clean(names in prop::collection::vec(padded_word(), 0..6)) {
        if let Ok(validated) = validate_flavor_notes(&names) {
            prop_assert!(validated.iter().all(|name| !name.is_empty()));
            prop_assert!(validated.iter().all(|name| name.as_str() == name.trim()));
            prop_assert!(validated.windows(2).all(|pair| pair[0] < pair[1]));
        }
    }
}
