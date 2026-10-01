//! `suggestion` の PBT (FR-19、ADR-0016)。
//!
//! 任意のテキストと、型の違う値の組み合わせからなる応答の JSON を解析し、結果が検証の条件
//! (長さ、形式、範囲) を満たすことを確認する。形式ごとの個別の検査は単体テスト
//! (`test_suggestion.rs`) が担う。

use brew_book_core::datetime::is_valid_date;
use brew_book_core::suggestion::{
    output_text, parse_response, ProductSuggestion, PurchaseSuggestion, MAX_FLAVOR_NOTES,
    MAX_NAME_CHARS, MAX_TEXT_CHARS,
};
use proptest::prelude::*;

/// 解析の結果が検証の条件を満たすことを確かめる。
///
/// 商品名は 200 文字、その他の文字列は 100 文字、Flavor Notes は 20 件、Roast Date は
/// `YYYY-MM-DD` の実在する日付、価格と重量は 0 以上で保存できる整数である (FR-19)。
fn assert_valid(suggestion: &PurchaseSuggestion) {
    let text_is_valid = |text: &Option<String>, max_chars: usize| {
        text.as_ref().is_none_or(|text| {
            !text.trim().is_empty() && text.chars().count() <= max_chars && text == text.trim()
        })
    };
    if let Some(product) = &suggestion.product {
        assert!(
            !product.is_empty(),
            "an empty product must become null: {suggestion:?}"
        );
        assert!(
            text_is_valid(&product.name, MAX_NAME_CHARS),
            "{suggestion:?}"
        );
        for text in [
            &product.producer,
            &product.origin,
            &product.region,
            &product.process,
            &product.variety,
        ] {
            assert!(text_is_valid(text, MAX_TEXT_CHARS), "{suggestion:?}");
        }
        assert!(
            product.flavor_notes.len() <= MAX_FLAVOR_NOTES,
            "{suggestion:?}"
        );
        for note in &product.flavor_notes {
            assert!(
                !note.trim().is_empty() && note.chars().count() <= MAX_TEXT_CHARS,
                "a flavor note must be a short non-empty string: {suggestion:?}"
            );
        }
    }
    assert!(
        text_is_valid(&suggestion.roast, MAX_TEXT_CHARS),
        "{suggestion:?}"
    );
    assert!(
        suggestion.roast_date.as_deref().is_none_or(is_valid_date),
        "{suggestion:?}"
    );
    for value in [suggestion.price_amount, suggestion.weight_grams] {
        assert!(
            value.is_none_or(|value| (0..=i64::from(i32::MAX)).contains(&value)),
            "{suggestion:?}"
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
        (-10_000i64..0).prop_map(|value| value.to_string()),
        Just("true".to_owned()),
        Just("[1, 2]".to_owned()),
        Just("{}".to_owned()),
    ]
}

/// 応答の JSON を、値の型の組み合わせとして作る。
fn response_json() -> impl Strategy<Value = String> {
    (
        proptest::collection::vec(json_fragment(), 0..4),
        json_fragment(),
        json_fragment(),
        json_fragment(),
        json_fragment(),
        json_fragment(),
    )
        .prop_map(|(product, roast, roast_date, price, weight, extra)| {
            format!(
                r#"{{"product": [{}], "roast": {roast}, "roast_date": {roast_date}, "price_amount": {price}, "weight_grams": {weight}, "confidence": {extra}}}"#,
                product.join(", ")
            )
        })
}

/// 検証を通る商品の候補を作る。名前を必ず持つため、空にならない。
fn valid_product() -> impl Strategy<Value = ProductSuggestion> {
    (
        clean_text(30),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        proptest::collection::vec(clean_text(20), 0..=MAX_FLAVOR_NOTES),
    )
        .prop_map(
            |(name, producer, origin, region, process, variety, flavor_notes)| ProductSuggestion {
                name: Some(name),
                producer,
                origin,
                region,
                process,
                variety,
                flavor_notes,
            },
        )
}

/// 実在する日付 (`YYYY-MM-DD`) を作る。日は 28 日までにして、どの月でも実在するようにする。
fn valid_date() -> impl Strategy<Value = String> {
    (2000i32..2100, 1u32..=12, 1u32..=28)
        .prop_map(|(year, month, day)| format!("{year:04}-{month:02}-{day:02}"))
}

/// 検証を通る応答を作る。
fn valid_suggestion() -> impl Strategy<Value = PurchaseSuggestion> {
    (
        prop::option::of(valid_product()),
        prop::option::of(clean_text(30)),
        prop::option::of(valid_date()),
        prop::option::of(0i64..=i64::from(i32::MAX)),
        prop::option::of(0i64..=i64::from(i32::MAX)),
    )
        .prop_map(|(product, roast, roast_date, price_amount, weight_grams)| {
            PurchaseSuggestion {
                product,
                roast,
                roast_date,
                price_amount,
                weight_grams,
            }
        })
}

/// 応答を、モデルが返すのと同じ形の JSON のテキストにする。
///
/// 生成する値は英数字だけなので、引用符の中のエスケープは要らない。
fn to_json(suggestion: &PurchaseSuggestion) -> String {
    let text = |value: &Option<String>| match value {
        Some(text) => format!("\"{text}\""),
        None => "null".to_owned(),
    };
    let product = match &suggestion.product {
        None => "null".to_owned(),
        Some(product) => {
            let notes: Vec<String> = product
                .flavor_notes
                .iter()
                .map(|note| format!("\"{note}\""))
                .collect();
            format!(
                r#"{{"name": {}, "producer": {}, "origin": {}, "region": {}, "process": {}, "variety": {}, "flavor_notes": [{}]}}"#,
                text(&product.name),
                text(&product.producer),
                text(&product.origin),
                text(&product.region),
                text(&product.process),
                text(&product.variety),
                notes.join(", ")
            )
        }
    };
    let number = |value: Option<i64>| match value {
        Some(value) => value.to_string(),
        None => "null".to_owned(),
    };
    format!(
        r#"{{"product": {product}, "roast": {}, "roast_date": {}, "price_amount": {}, "weight_grams": {}}}"#,
        text(&suggestion.roast),
        text(&suggestion.roast_date),
        number(suggestion.price_amount),
        number(suggestion.weight_grams)
    )
}

/// 応答の JSON のキー。`output_text` が読むキー (`response`、`choices`、`message`、`content`) を
/// 混ぜる (任意の文字列だけでは、これらのキーが実質的に生成されないため)。
fn json_key() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("response".to_owned()),
        Just("choices".to_owned()),
        Just("message".to_owned()),
        Just("content".to_owned()),
        any::<String>(),
    ]
}

/// 任意の JSON の値 (深さ 3 まで) を作る。AI バインディングの応答の形を模す。
fn json_value() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(|value| serde_json::json!(value)),
        any::<String>().prop_map(serde_json::Value::String),
    ];
    leaf.prop_recursive(3, 16, 4, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..4).prop_map(serde_json::Value::Array),
            proptest::collection::vec((json_key(), inner), 0..4)
                .prop_map(|entries| { serde_json::Value::Object(entries.into_iter().collect()) }),
        ]
    })
}

/// AI バインディングの応答の形に寄せた JSON の値を作る。
///
/// `response` に文字列で返る形、OpenAI 互換の `choices[0].message.content` で返る形、
/// 両方のキーを持つ形、どちらも無い形、任意の JSON の値を混ぜる。
fn response_value() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        any::<String>().prop_map(|text| serde_json::json!({ "response": text })),
        any::<String>().prop_map(|text| serde_json::json!({
            "choices": [{ "index": 0, "message": { "role": "assistant", "content": text } }],
            "usage": { "total_tokens": 100 }
        })),
        any::<String>().prop_map(|text| serde_json::json!({
            "response": text,
            "choices": [{ "message": { "content": text } }]
        })),
        Just(serde_json::json!({ "result": { "text": "hello" } })),
        json_value(),
    ]
}

proptest! {
    /// 任意のテキストを解析しても、結果は検証の条件を満たす。
    #[test]
    fn any_text_parses_into_a_valid_suggestion(text in any::<String>()) {
        assert_valid(&parse_response(&text));
    }

    /// 型の違う値が混ざった応答の JSON を解析しても、結果は検証の条件を満たす。
    #[test]
    fn any_response_json_parses_into_a_valid_suggestion(json in response_json()) {
        assert_valid(&parse_response(&json));
    }

    /// 任意の形の AI バインディングの応答から出力を取り出して解析しても、
    /// 結果は検証の条件を満たす (実経路は `parse_response(output_text(v))` である。FR-19)。
    #[test]
    fn any_model_result_parses_into_a_valid_suggestion(result in response_value()) {
        assert_valid(&parse_response(&output_text(&result)));
    }

    /// 検証を通る応答は、モデルが返す形の JSON にしてから解析すると元に戻る。
    #[test]
    fn a_valid_suggestion_round_trips(suggestion in valid_suggestion()) {
        prop_assert_eq!(parse_response(&to_json(&suggestion)), suggestion);
    }
}
