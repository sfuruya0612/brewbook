//! `records` の PBT (0041)。
//!
//! 一覧のページングの状態機械、推測の適用 (FR-19)、サジェストの状態 (FR-13) の性質を、
//! 任意の入力と操作の列で確かめる (ADR-0013)。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::records::{
    apply_purchase_suggestion, dedup_tags, PurchaseFormFields, PurchaseSuggestion, RecordError,
    RecordList, SuggestionState,
};
use proptest::prelude::*;

/// 一覧への操作。
#[derive(Clone, Debug)]
enum Op {
    /// 先頭から読み直す。
    Reset,
    /// 次のページを読む。
    LoadMore,
    /// ページが読めたことにする。
    FinishPage(usize, Option<String>),
    /// 読み込みが失敗したことにする。
    FinishError,
}

/// 任意の操作。
fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::Reset),
        Just(Op::LoadMore),
        (0..60usize, prop::option::of("[a-z0-9]{0,8}"))
            .prop_map(|(count, cursor)| Op::FinishPage(count, cursor)),
        Just(Op::FinishError),
    ]
}

proptest! {
    /// 読み込み中の要求は重ねない。
    #[test]
    fn a_page_is_never_requested_twice_at_once(ops in prop::collection::vec(op(), 0..40)) {
        let mut list = RecordList::new();
        for op in ops {
            let was_loading = list.is_loading();
            let request = match op {
                Op::Reset => list.reset(),
                Op::LoadMore => list.load_more(),
                Op::FinishPage(count, cursor) => {
                    list.finish_load(Ok((count, cursor)));
                    continue;
                }
                Op::FinishError => {
                    list.finish_load(Err(RecordError::Format("broken".to_string())));
                    continue;
                }
            };
            if request.is_some() {
                prop_assert!(!was_loading, "a page was requested while loading");
            } else {
                prop_assert!(was_loading || matches!(op, Op::LoadMore) || list.next_cursor().is_none());
            }
        }
    }

    /// 失敗しても状態は壊れず、再試行で先頭から読める。
    #[test]
    fn a_failed_load_keeps_the_list_usable(count in 0..60usize, cursor in prop::option::of("[a-z]{0,6}")) {
        let mut list = RecordList::new();
        let _ = list.reset();
        list.finish_load(Ok((count, cursor.clone())));
        let _ = list.load_more();
        list.finish_load(Err(RecordError::Format("broken".to_string())));
        prop_assert!(list.error().is_some());
        prop_assert_eq!(list.next_cursor(), cursor.as_deref());

        let request = list.retry().expect("the retry must be requested");
        prop_assert_eq!(request.cursor, None);
        prop_assert!(list.error().is_none());
        list.finish_load(Ok((1, None)));
        prop_assert_eq!(list.item_count(), 1);
        prop_assert!(list.is_loaded());
    }

    /// 推測のある項目は入力済みの値を上書きし、無い項目 (None と空のタグ) は入力を保つ
    /// (FR-19、0057)。
    #[test]
    fn the_suggestion_overwrites_the_suggested_fields_and_keeps_the_rest(
        texts in prop::collection::vec(".*", 10),
        tags in prop::collection::vec("[a-z ]{0,6}", 0..4),
        suggested_texts in prop::collection::vec(prop::option::of("[a-z]{0,8}"), 8),
        suggested_tags in prop::collection::vec("[a-z ]{0,6}", 0..4),
        suggested_price in prop::option::of(0i64..100_000),
        suggested_weight in prop::option::of(0i64..10_000),
    ) {
        let suggestion = PurchaseSuggestion {
            name: suggested_texts[0].clone(),
            producer: suggested_texts[1].clone(),
            origin: suggested_texts[2].clone(),
            region: suggested_texts[3].clone(),
            process: suggested_texts[4].clone(),
            variety: suggested_texts[5].clone(),
            flavor_notes: suggested_tags.clone(),
            roast: suggested_texts[6].clone(),
            roast_date: suggested_texts[7].clone(),
            price_amount: suggested_price,
            weight_grams: suggested_weight,
        };
        let mut values = texts.clone();
        let mut tag_values = tags.clone();
        let (head, tail) = values.split_at_mut(5);
        let [name, producer, origin, region, process] = head else { unreachable!() };
        let [variety, roast, roast_date, price, weight] = tail else { unreachable!() };
        apply_purchase_suggestion(
            PurchaseFormFields {
                name,
                producer,
                origin,
                region,
                process,
                variety,
                flavor_notes: &mut tag_values,
                roast,
                roast_date,
                price,
                weight,
            },
            &suggestion,
        );

        // 文字の 8 項目: 推測があればその値、無ければ入力のまま。
        for (index, suggested) in suggested_texts.iter().enumerate() {
            let expected = suggested.clone().unwrap_or_else(|| texts[index].clone());
            prop_assert_eq!(&values[index], &expected, "field {}", index);
        }
        // 価格と重量: 推測があれば 10 進の文字、無ければ入力のまま。
        let expected_price = suggested_price.map(|v| v.to_string()).unwrap_or_else(|| texts[8].clone());
        prop_assert_eq!(&values[8], &expected_price);
        let expected_weight = suggested_weight.map(|v| v.to_string()).unwrap_or_else(|| texts[9].clone());
        prop_assert_eq!(&values[9], &expected_weight);
        // タグ: 推測が空なら入力のまま、あれば正規化した推測。
        if suggested_tags.is_empty() {
            prop_assert_eq!(&tag_values, &tags);
        } else {
            prop_assert_eq!(&tag_values, &dedup_tags(&suggested_tags));
        }
    }

    /// タグの正規化は、空白を除いた空でない文字を、最初の出現の順で 1 つずつ残す (0057)。
    #[test]
    fn the_tags_are_trimmed_and_deduplicated_in_order(
        tags in prop::collection::vec("[ a-z]{0,6}", 0..8),
    ) {
        let normalized = dedup_tags(&tags);
        // 空と重複が無く、どれも前後に空白を持たない。
        for (index, tag) in normalized.iter().enumerate() {
            prop_assert!(!tag.is_empty());
            prop_assert_eq!(tag.trim(), tag.as_str());
            prop_assert!(!normalized[..index].contains(tag), "the tag {:?} appears twice", tag);
        }
        // 入力の空でないタグはすべて残り、順序は最初の出現の順。
        let mut expected: Vec<String> = Vec::new();
        for tag in &tags {
            let tag = tag.trim();
            if !tag.is_empty() && !expected.iter().any(|seen| seen == tag) {
                expected.push(tag.to_string());
            }
        }
        prop_assert_eq!(normalized, expected);
        // 冪等。
        let again = dedup_tags(&dedup_tags(&tags));
        prop_assert_eq!(again, dedup_tags(&tags));
    }

    /// サジェストの状態は、古い世代の応答を反映しない (FR-13)。
    #[test]
    fn a_stale_suggestion_response_is_ignored(
        inputs in prop::collection::vec(
            ("[a-z]{0,6}", prop::collection::vec("[a-z]{0,6}", 0..4)),
            1..6,
        ),
    ) {
        let mut state = SuggestionState::new();
        let mut tokens = Vec::new();
        for (query, _) in &inputs {
            tokens.push(state.begin(query));
        }
        // 最後より前の世代の応答は捨てる。
        for (index, token) in tokens.iter().enumerate().take(tokens.len() - 1) {
            let applied = state.apply_options(*token, inputs[index].1.clone());
            prop_assert!(!applied, "a stale response was applied");
        }
        let last = *tokens.last().expect("at least one query");
        let last_values = inputs[tokens.len() - 1].1.clone();
        prop_assert!(state.apply_options(last, last_values.clone()));
        prop_assert_eq!(state.options(), last_values.as_slice());
        prop_assert_eq!(state.is_open(), !last_values.is_empty());
    }
}
