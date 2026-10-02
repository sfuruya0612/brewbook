//! `records` の PBT (0041)。
//!
//! 一覧のページングの状態機械、推測の適用 (FR-19)、サジェストの状態 (FR-13) の性質を、
//! 任意の入力と操作の列で確かめる (ADR-0013)。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::records::{
    apply_purchase_suggestion, product_match, Product, ProductMatch, ProductSuggestion,
    PurchaseSuggestion, RecordError, RecordList, SuggestionState,
};
use proptest::prelude::*;

/// 一覧への操作。
#[derive(Clone, Debug)]
enum Op {
    /// 先頭から読み直す。
    Reset,
    /// 次のページを読む。
    LoadMore,
    /// アーカイブ済みを含める切り替えを反転する。
    Toggle,
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
        Just(Op::Toggle),
        (0..60usize, prop::option::of("[a-z0-9]{0,8}"))
            .prop_map(|(count, cursor)| Op::FinishPage(count, cursor)),
        Just(Op::FinishError),
    ]
}

proptest! {
    /// 読み込み中の要求は重ねず、要求のアーカイブの指定は常に現在の切り替えと同じにする。
    #[test]
    fn a_page_is_never_requested_twice_at_once(ops in prop::collection::vec(op(), 0..40)) {
        let mut list = RecordList::new();
        for op in ops {
            let was_loading = list.is_loading();
            let include_archived = list.include_archived();
            let request = match op {
                Op::Reset => list.reset(),
                Op::LoadMore => list.load_more(),
                Op::Toggle => list.toggle_include_archived(),
                Op::FinishPage(count, cursor) => {
                    list.finish_load(Ok((count, cursor)));
                    continue;
                }
                Op::FinishError => {
                    list.finish_load(Err(RecordError::Format("broken".to_string())));
                    continue;
                }
            };
            if let Some(request) = request {
                prop_assert!(!was_loading, "a page was requested while loading");
                // 要求のアーカイブの指定は、反転の直後の状態と一致する。
                let expected = if matches!(op, Op::Toggle) { !include_archived } else { include_archived };
                prop_assert_eq!(request.include_archived, expected);
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

    /// 推測は空の入力欄にだけ入り、入力済みの値は上書きしない (FR-19)。
    #[test]
    fn the_suggestion_never_overwrites_an_input(
        roast in ".*",
        roast_date in ".*",
        price in ".*",
        weight in ".*",
        suggested_roast in prop::option::of("[a-z]{0,8}"),
        suggested_date in prop::option::of("[0-9]{4}-[0-9]{2}-[0-9]{2}"),
        suggested_price in prop::option::of(0i64..100_000),
        suggested_weight in prop::option::of(0i64..10_000),
    ) {
        let suggestion = PurchaseSuggestion {
            product: None,
            roast: suggested_roast.clone(),
            roast_date: suggested_date.clone(),
            price_amount: suggested_price,
            weight_grams: suggested_weight,
        };
        let mut roast_value = roast.clone();
        let mut roast_date_value = roast_date.clone();
        let mut price_value = price.clone();
        let mut weight_value = weight.clone();
        apply_purchase_suggestion(
            &mut roast_value,
            &mut roast_date_value,
            &mut price_value,
            &mut weight_value,
            &suggestion,
        );

        if roast.trim().is_empty() {
            if let Some(value) = suggested_roast { prop_assert_eq!(roast_value, value); }
        } else {
            prop_assert_eq!(roast_value, roast);
        }
        if roast_date.trim().is_empty() {
            if let Some(value) = suggested_date { prop_assert_eq!(roast_date_value, value); }
        } else {
            prop_assert_eq!(roast_date_value, roast_date);
        }
        if price.trim().is_empty() {
            if let Some(value) = suggested_price { prop_assert_eq!(price_value, value.to_string()); }
        } else {
            prop_assert_eq!(price_value, price);
        }
        if weight.trim().is_empty() {
            if let Some(value) = suggested_weight { prop_assert_eq!(weight_value, value.to_string()); }
        } else {
            prop_assert_eq!(weight_value, weight);
        }
    }

    /// 商品が選択済みのときは何もしない (FR-19)。
    #[test]
    fn a_selected_product_is_never_replaced(
        name in prop::option::of("[a-zA-Z ]{0,6}"),
        matched in any::<bool>(),
    ) {
        let suggestion = PurchaseSuggestion {
            product: Some(ProductSuggestion { name, ..ProductSuggestion::default() }),
            ..PurchaseSuggestion::default()
        };
        let matched_product = matched.then(|| Product {
            id: "p1".to_string(),
            name: "豆".to_string(),
            producer: None,
            origin: None,
            region: None,
            process: None,
            variety: None,
            flavor_notes: Vec::new(),
            created_at: "2026-10-01T00:00:00.000Z".to_string(),
            updated_at: "2026-10-01T00:00:00.000Z".to_string(),
            archived_at: None,
        });
        prop_assert_eq!(product_match(true, &suggestion, matched_product), ProductMatch::None);
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
