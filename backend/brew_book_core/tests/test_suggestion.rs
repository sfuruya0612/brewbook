//! `suggestion` の単体テスト (FR-19、ADR-0016)。
//!
//! プロンプトの生成、応答の解析 (正常な JSON、コードフェンス、説明文、不正な JSON、型違い、
//! 範囲外の値、未知のフィールド)、応答の形、画像の `data:` URL を確認する。
//! 解析の結果が検証の条件を満たすことは PBT (`prop_suggestion.rs`) が担う。

use brew_book_core::suggestion::{
    image_data_url, output_text, parse_response, prompt, ProductSuggestion, PurchaseSuggestion,
    MAX_FLAVOR_NOTES, MAX_NAME_CHARS, MAX_TEXT_CHARS,
};

/// 商品の項目が全て揃った応答 (issue の例と同じ形)。
const FULL_RESPONSE: &str = r#"{
    "product": {
        "name": "エチオピア イルガチェフェ",
        "producer": null,
        "origin": "エチオピア",
        "region": null,
        "process": "ウォッシュト",
        "variety": null,
        "flavor_notes": ["フローラル"]
    },
    "roast": "中煎り",
    "roast_date": "2026-09-20",
    "price_amount": null,
    "weight_grams": 200
}"#;

#[test]
fn the_prompt_fixes_the_json_shape_and_the_rules() {
    let text = prompt();
    // 応答の JSON の形を固定し、項目を増やさない (ADR-0016)。
    for key in [
        "product",
        "name",
        "producer",
        "origin",
        "region",
        "process",
        "variety",
        "flavor_notes",
        "roast",
        "roast_date",
        "price_amount",
        "weight_grams",
    ] {
        assert!(text.contains(key), "the prompt must ask for {key}: {text}");
    }
    // 解析が前提にする条件をプロンプトにも書く。
    assert!(text.contains("YYYY-MM-DD"), "{text}");
    assert!(text.contains("null"), "{text}");
    assert!(text.contains("at most 20"), "{text}");
    assert!(text.contains("Do not add any keys"), "{text}");
    // 同じプロンプトを返す (出力を決定的にする)。
    assert_eq!(prompt(), text);
}

#[test]
fn a_valid_response_parses_into_every_field() {
    let suggestion = parse_response(FULL_RESPONSE);
    assert_eq!(
        suggestion,
        PurchaseSuggestion {
            product: Some(ProductSuggestion {
                name: Some("エチオピア イルガチェフェ".to_owned()),
                producer: None,
                origin: Some("エチオピア".to_owned()),
                region: None,
                process: Some("ウォッシュト".to_owned()),
                variety: None,
                flavor_notes: vec!["フローラル".to_owned()],
            }),
            roast: Some("中煎り".to_owned()),
            roast_date: Some("2026-09-20".to_owned()),
            price_amount: None,
            weight_grams: Some(200),
        }
    );
}

#[test]
fn a_response_in_a_code_fence_parses() {
    let text = format!("```json\n{FULL_RESPONSE}\n```");
    assert_eq!(parse_response(&text), parse_response(FULL_RESPONSE));
}

#[test]
fn a_response_with_explanations_around_the_json_parses() {
    let text = format!(
        "Here is the information I read from the bag.\n{FULL_RESPONSE}\nI hope this helps."
    );
    assert_eq!(parse_response(&text), parse_response(FULL_RESPONSE));
    // 説明文の中の `{` は飛ばして、最初に読める JSON のオブジェクトを使う。
    let text = format!("Use the keys {{name, roast}}.\n{FULL_RESPONSE}");
    assert_eq!(parse_response(&text), parse_response(FULL_RESPONSE));
}

#[test]
fn an_unparsable_response_has_no_fields() {
    for text in [
        "",
        "I cannot read this photo.",
        "{",
        "{\"roast\": }",
        "null",
        "[1, 2, 3]",
    ] {
        assert_eq!(
            parse_response(text),
            PurchaseSuggestion::default(),
            "the response must be empty for {text:?}"
        );
    }
}

#[test]
fn fields_of_another_type_become_null() {
    let text = r#"{
        "product": "エチオピア",
        "roast": 12,
        "roast_date": 20260920,
        "price_amount": "1200",
        "weight_grams": [200]
    }"#;
    assert_eq!(parse_response(text), PurchaseSuggestion::default());
    // 商品がオブジェクトでも、項目ごとに型を検査する。
    let text = r#"{
        "product": {"name": 12, "producer": ["a"], "flavor_notes": "フローラル"}
    }"#;
    assert_eq!(parse_response(text), PurchaseSuggestion::default());
}

#[test]
fn out_of_range_values_become_null() {
    let text = r#"{
        "product": {"name": "豆"},
        "roast_date": "2026-02-29",
        "price_amount": -1,
        "weight_grams": 2147483648
    }"#;
    let suggestion = parse_response(text);
    assert_eq!(suggestion.roast_date, None, "2026-02-29 does not exist");
    assert_eq!(
        suggestion.price_amount, None,
        "a negative price is rejected"
    );
    assert_eq!(
        suggestion.weight_grams, None,
        "a weight that does not fit in the stored integer is rejected"
    );
    assert_eq!(
        suggestion.product.map(|product| product.name),
        Some(Some("豆".to_owned()))
    );
}

#[test]
fn long_strings_become_null_but_other_fields_survive() {
    let long_name = "あ".repeat(MAX_NAME_CHARS + 1);
    let long_text = "あ".repeat(MAX_TEXT_CHARS + 1);
    let text = format!(
        r#"{{"product": {{"name": "{long_name}", "origin": "エチオピア", "producer": "{long_text}"}}, "roast": "{long_text}", "roast_date": "2026-09-20"}}"#
    );
    let suggestion = parse_response(&text);
    let product = suggestion.product.expect("the origin must survive");
    assert_eq!(product.name, None);
    assert_eq!(product.producer, None);
    assert_eq!(product.origin, Some("エチオピア".to_owned()));
    assert_eq!(suggestion.roast, None);
    assert_eq!(suggestion.roast_date, Some("2026-09-20".to_owned()));
    // ちょうどの長さは受け付ける。
    let name = "あ".repeat(MAX_NAME_CHARS);
    let text = format!(r#"{{"product": {{"name": "{name}"}}}}"#);
    assert_eq!(
        parse_response(&text)
            .product
            .and_then(|product| product.name),
        Some(name)
    );
}

#[test]
fn strings_are_trimmed_and_blank_strings_become_null() {
    let text = r#"{
        "product": {"name": "  エチオピア  ", "origin": "   ", "producer": ""},
        "roast": " 中煎り ",
        "roast_date": " 2026-09-20 "
    }"#;
    let suggestion = parse_response(text);
    let product = suggestion.product.expect("the name must survive");
    assert_eq!(product.name, Some("エチオピア".to_owned()));
    assert_eq!(product.origin, None);
    assert_eq!(product.producer, None);
    assert_eq!(suggestion.roast, Some("中煎り".to_owned()));
    assert_eq!(suggestion.roast_date, Some("2026-09-20".to_owned()));
}

#[test]
fn a_product_without_any_field_becomes_null() {
    for text in [
        r#"{"product": null}"#,
        r#"{"product": {}}"#,
        r#"{"product": {"name": null, "flavor_notes": []}}"#,
        r#"{"product": {"name": "  ", "producer": 12}}"#,
    ] {
        assert_eq!(parse_response(text).product, None, "{text}");
    }
}

#[test]
fn unknown_fields_are_ignored() {
    let text = r#"{
        "product": {"name": "豆", "price_currency": "JPY", "unknown": {"a": 1}},
        "shop": "テスト店",
        "confidence": 0.9
    }"#;
    let suggestion = parse_response(text);
    assert_eq!(
        suggestion.product,
        Some(ProductSuggestion {
            name: Some("豆".to_owned()),
            ..ProductSuggestion::default()
        })
    );
    assert_eq!(suggestion.roast, None);
}

#[test]
fn flavor_notes_drop_invalid_items_and_stop_at_the_limit() {
    let text = r#"{
        "product": {"name": "豆", "flavor_notes": [" フローラル ", "", "   ", 12, null, "柑橘"]}
    }"#;
    let suggestion = parse_response(text);
    assert_eq!(
        suggestion
            .product
            .expect("the product must survive")
            .flavor_notes,
        vec!["フローラル".to_owned(), "柑橘".to_owned()]
    );

    // 21 件以上あっても 20 件までにする (FR-19)。
    let notes: Vec<String> = (0..MAX_FLAVOR_NOTES + 5)
        .map(|index| format!("\"メモ {index:02}\""))
        .collect();
    let text = format!(
        r#"{{"product": {{"name": "豆", "flavor_notes": [{}]}}}}"#,
        notes.join(", ")
    );
    let suggestion = parse_response(&text);
    assert_eq!(
        suggestion
            .product
            .expect("the product must survive")
            .flavor_notes
            .len(),
        MAX_FLAVOR_NOTES
    );
}

#[test]
fn an_integer_valued_number_is_accepted() {
    let text = r#"{"price_amount": 1200.0, "weight_grams": 200.5}"#;
    let suggestion = parse_response(text);
    assert_eq!(suggestion.price_amount, Some(1200));
    assert_eq!(
        suggestion.weight_grams, None,
        "a fraction is not an integer"
    );
}

#[test]
fn the_suggestion_serializes_into_the_response_shape() {
    // キーは既存の API の列名に揃え、商品は `product` の入れ子にする (FR-19)。
    let json =
        serde_json::to_value(PurchaseSuggestion::default()).expect("the response serializes");
    assert_eq!(
        json,
        serde_json::json!({
            "product": null,
            "roast": null,
            "roast_date": null,
            "price_amount": null,
            "weight_grams": null
        })
    );
    let json =
        serde_json::to_value(parse_response(FULL_RESPONSE)).expect("the response serializes");
    assert_eq!(
        json,
        serde_json::json!({
            "product": {
                "name": "エチオピア イルガチェフェ",
                "producer": null,
                "origin": "エチオピア",
                "region": null,
                "process": "ウォッシュト",
                "variety": null,
                "flavor_notes": ["フローラル"]
            },
            "roast": "中煎り",
            "roast_date": "2026-09-20",
            "price_amount": null,
            "weight_grams": 200
        })
    );
}

#[test]
fn the_image_data_url_is_standard_base64() {
    // RFC 4648 の Section 10 のテストベクタ。
    for (bytes, encoded) in [
        (b"".as_slice(), ""),
        (b"f".as_slice(), "Zg=="),
        (b"fo".as_slice(), "Zm8="),
        (b"foo".as_slice(), "Zm9v"),
        (b"foob".as_slice(), "Zm9vYg=="),
        (b"fooba".as_slice(), "Zm9vYmE="),
        (b"foobar".as_slice(), "Zm9vYmFy"),
    ] {
        assert_eq!(
            image_data_url(bytes),
            format!("data:image/jpeg;base64,{encoded}")
        );
    }
    // JPEG の先頭のバイト列 (0xFF 0xD8 0xFF) が標準のアルファベットになる。
    assert_eq!(
        image_data_url(&[0xFF, 0xD8, 0xFF]),
        "data:image/jpeg;base64,/9j/"
    );
}

#[test]
fn the_output_text_reads_the_native_response_shape() {
    // `response` に文字列で返る形 (古いモデル)。
    let result = serde_json::json!({ "response": "hello" });
    assert_eq!(output_text(&result), "hello");
}

#[test]
fn the_output_text_reads_the_openai_compatible_shape() {
    // OpenAI 互換の `choices[0].message.content` で返る形 (新しいモデル)。
    let result = serde_json::json!({
        "choices": [
            { "index": 0, "message": { "role": "assistant", "content": "hi" } }
        ],
        "usage": { "total_tokens": 10 }
    });
    assert_eq!(output_text(&result), "hi");
}

#[test]
fn the_output_text_prefers_the_native_response_shape() {
    // 両方あるときは `response` を優先する (古いモデルの既定の形)。
    let result = serde_json::json!({
        "response": "native",
        "choices": [{ "message": { "content": "openai" } }]
    });
    assert_eq!(output_text(&result), "native");
}

#[test]
fn the_output_text_falls_back_to_choices_when_the_native_response_is_empty() {
    // `response` が空の文字列のときは `choices` に落とす (両方のキーを持つモデルのため)。
    for response in ["", "  ", "\n"] {
        let result = serde_json::json!({
            "response": response,
            "choices": [{ "message": { "content": "openai" } }]
        });
        assert_eq!(
            output_text(&result),
            "openai",
            "the response was {response:?}"
        );
    }
}

#[test]
fn the_output_text_of_an_object_response_is_its_json() {
    // `response` がオブジェクトのときは、その JSON をそのまま解析にかける。
    let result = serde_json::json!({ "response": { "product": { "name": "Bean" } } });
    let text = output_text(&result);
    let suggestion = parse_response(&text);
    assert_eq!(
        suggestion.product.and_then(|product| product.name),
        Some("Bean".to_owned())
    );
}

#[test]
fn the_output_text_of_an_unknown_shape_is_empty() {
    // 形が分からない応答は空の文字列にする (解析の結果は全項目 null になる)。
    for result in [
        serde_json::json!({}),
        serde_json::json!({ "response": null }),
        serde_json::json!({ "choices": [] }),
        serde_json::json!({ "choices": [{ "message": { "content": null } }] }),
        serde_json::json!({ "choices": [{ "message": { "content": 3 } }] }),
        serde_json::json!({ "result": { "text": "hello" } }),
    ] {
        assert_eq!(output_text(&result), "", "the result was {result}");
        assert_eq!(
            parse_response(&output_text(&result)),
            PurchaseSuggestion::default()
        );
    }
}

#[test]
fn a_real_openai_compatible_response_parses_into_the_suggestion() {
    // 2026-09-30 に `@cf/mistralai/mistral-small-3.1-24b-instruct` が返した応答の形 (issue 0034 の比較)。
    // OpenAI 互換の `choices[0].message.content` に、プロンプトどおりの JSON が文字列で入る。
    let result = serde_json::json!({
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": r#"{"product":{"name":"ETHIOPIA YIRGACHEFFE NATURAL","producer":"アブデラ・ケベデ","origin":"エチオピア","region":"イルガチェフェ","process":"ナチュラル","variety":"ヒールーム","flavor_notes":["ブルーベリー","ジャスミン","シトラス"]},"roast":"中焙り (シティ)","roast_date":"2026-09-20","price_amount":null,"weight_grams":200}"#
            },
            "finish_reason": "stop"
        }],
        "usage": { "total_tokens": 1404, "neurons": 46.99 }
    });
    let suggestion = parse_response(&output_text(&result));
    let product = suggestion.product.expect("the product must be parsed");
    assert_eq!(
        product.name.as_deref(),
        Some("ETHIOPIA YIRGACHEFFE NATURAL")
    );
    assert_eq!(product.producer.as_deref(), Some("アブデラ・ケベデ"));
    assert_eq!(product.origin.as_deref(), Some("エチオピア"));
    assert_eq!(
        product.flavor_notes,
        vec![
            "ブルーベリー".to_owned(),
            "ジャスミン".to_owned(),
            "シトラス".to_owned()
        ]
    );
    assert_eq!(suggestion.roast.as_deref(), Some("中焙り (シティ)"));
    assert_eq!(suggestion.roast_date.as_deref(), Some("2026-09-20"));
    assert_eq!(suggestion.weight_grams, Some(200));
    assert_eq!(suggestion.price_amount, None);
}
