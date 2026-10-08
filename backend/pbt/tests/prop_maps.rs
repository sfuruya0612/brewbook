//! `maps` の PBT (FR-22、ADR-0019)。
//!
//! 任意のテキストと、型の違う値の組み合わせからなる応答の JSON を解析し、結果が検証の条件
//! (候補の上限、前後の空白を除いた非空の名前と住所) を満たすことを確認する。
//! 形式ごとの個別の検査は単体テスト (`test_maps.rs`) が担う。

use brew_book_core::maps::{
    parse_search_response, validate_search_input, Candidate, MAX_CANDIDATES, MAX_QUERY_CHARS,
    SUPPORTED_LANGUAGES,
};
use proptest::prelude::*;

/// 解析の結果が検証の条件を満たすことを確かめる (FR-22)。
fn assert_valid(candidates: &[Candidate]) {
    assert!(candidates.len() <= MAX_CANDIDATES, "{candidates:?}");
    for candidate in candidates {
        assert!(
            !candidate.name.is_empty() && candidate.name == candidate.name.trim(),
            "the name must be a trimmed non-empty string: {candidates:?}"
        );
        assert!(
            !candidate.address.is_empty() && candidate.address == candidate.address.trim(),
            "the address must be a trimmed non-empty string: {candidates:?}"
        );
    }
}

/// アルファベットと数字だけの、前後の空白を除いても変わらない文字列。
fn clean_text(max_chars: usize) -> impl Strategy<Value = String> {
    proptest::collection::vec("[a-zA-Z0-9]", 1..max_chars).prop_map(|chars| chars.concat())
}

/// JSON の断片 (応答の値を壊すために使う)。
fn json_fragment() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("null".to_owned()),
        clean_text(8).prop_map(|text| format!("{text:?}")),
        (0i64..10_000).prop_map(|value| value.to_string()),
        Just("true".to_owned()),
        Just("[1, 2]".to_owned()),
        Just("{}".to_owned()),
    ]
}

/// `places` の要素 1 件の JSON を、値の型の組み合わせとして作る。
fn place_json() -> impl Strategy<Value = String> {
    let display = prop_oneof![
        clean_text(12).prop_map(|text| format!(r#"{{ "text": {text:?} }}"#)),
        json_fragment(),
    ];
    let address = prop_oneof![
        clean_text(12).prop_map(|text| format!("{text:?}")),
        json_fragment(),
    ];
    (display, address).prop_map(|(display, address)| {
        format!(r#"{{ "displayName": {display}, "formattedAddress": {address} }}"#)
    })
}

/// 応答の JSON を、`places` の型と要素の組み合わせとして作る。
fn response_json() -> impl Strategy<Value = String> {
    prop_oneof![
        proptest::collection::vec(place_json(), 0..MAX_CANDIDATES + 3)
            .prop_map(|places| format!(r#"{{ "places": [{}] }}"#, places.join(", "))),
        // `places` の型が違う応答。
        json_fragment().prop_map(|places| format!(r#"{{ "places": {places} }}"#)),
    ]
}

/// 検証を通る候補を作る。名前と住所は前後の空白を含まない非空の文字列になる。
fn valid_candidate() -> impl Strategy<Value = Candidate> {
    (clean_text(30), clean_text(40)).prop_map(|(name, address)| Candidate { name, address })
}

/// 候補を、Google の応答の形の JSON のテキストにする。
///
/// 生成する値は英数字だけなので、引用符の中のエスケープは要らない。
fn to_json(candidates: &[Candidate]) -> String {
    let places: Vec<String> = candidates
        .iter()
        .map(|candidate| {
            format!(
                r#"{{ "displayName": {{ "text": {:?} }}, "formattedAddress": {:?} }}"#,
                candidate.name, candidate.address
            )
        })
        .collect();
    format!(r#"{{ "places": [{}] }}"#, places.join(", "))
}

proptest! {
    /// 任意のテキストを解析しても、結果は検証の条件を満たすか、形の誤りになる。
    #[test]
    fn any_text_parses_into_valid_candidates_or_an_error(text in any::<String>()) {
        if let Ok(candidates) = parse_search_response(&text) {
            assert_valid(&candidates);
        }
    }

    /// 型の違う値が混ざった応答の JSON を解析しても、結果は検証の条件を満たすか、形の誤りになる。
    #[test]
    fn any_response_json_parses_into_valid_candidates_or_an_error(json in response_json()) {
        if let Ok(candidates) = parse_search_response(&json) {
            assert_valid(&candidates);
        }
    }

    /// 検証を通る候補は、Google の応答の形の JSON にしてから解析すると元に戻る。
    #[test]
    fn valid_candidates_round_trip(candidates in proptest::collection::vec(valid_candidate(), 0..MAX_CANDIDATES)) {
        prop_assert_eq!(parse_search_response(&to_json(&candidates)), Ok(candidates));
    }

    /// 検証を通る入力は、前後の空白を除いた店名と、受け付ける言語のコードになる。
    #[test]
    fn any_valid_input_becomes_a_trimmed_query_and_a_supported_language(
        query in clean_text(MAX_QUERY_CHARS),
        lang in prop_oneof![
            Just("ja".to_owned()),
            Just("en".to_owned()),
        ],
    ) {
        let (query, language) = validate_search_input(&query, &lang).expect("the input must be valid");
        prop_assert!(!query.is_empty());
        prop_assert!(query == query.trim());
        prop_assert!(query.chars().count() <= MAX_QUERY_CHARS);
        prop_assert!(SUPPORTED_LANGUAGES.contains(&language));
    }

    /// 任意の入力の検証は panic せず、通ったときは検証の条件を満たす。
    #[test]
    fn any_input_is_validated(query in any::<String>(), lang in any::<String>()) {
        if let Ok((query, language)) = validate_search_input(&query, &lang) {
            prop_assert!(!query.is_empty());
            prop_assert!(query == query.trim());
            prop_assert!(query.chars().count() <= MAX_QUERY_CHARS);
            prop_assert!(SUPPORTED_LANGUAGES.contains(&language));
        }
    }
}
