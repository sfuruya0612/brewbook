//! `maps` の単体テスト (FR-22、ADR-0019)。
//!
//! 検索の入力の検証 (店名の空と長さ、言語) と、Places API (New) の Text Search の応答の解析
//! (正常な応答、0 件、名前か住所の欠け、型違い、不正な JSON、上限) を確認する。
//! 解析の結果が検証の条件を満たすことは PBT (`prop_maps.rs`) が担う。

use brew_book_core::maps::{
    parse_search_response, referer_header, search_request_body, validate_search_input, Candidate,
    MapsConfigResponse, ParseError, SearchInputError, SearchResponse, MAX_CANDIDATES,
    MAX_QUERY_CHARS, SUPPORTED_LANGUAGES,
};

/// 候補が 2 件ある応答 (Google の例と同じ形)。
const TWO_PLACES: &str = r#"{
    "places": [
        {
            "formattedAddress": "367 Pitt St, Sydney NSW 2000, Australia",
            "displayName": { "text": "Mother Chu's Vegetarian Kitchen", "languageCode": "en" }
        },
        {
            "formattedAddress": "175 First Ave, Five Dock NSW 2046, Australia",
            "displayName": { "text": "Veggo Sizzle", "languageCode": "en" }
        }
    ]
}"#;

#[test]
fn a_valid_response_parses_into_candidates() {
    let candidates = parse_search_response(TWO_PLACES).expect("the response must parse");
    assert_eq!(
        candidates,
        vec![
            Candidate {
                name: "Mother Chu's Vegetarian Kitchen".to_owned(),
                address: "367 Pitt St, Sydney NSW 2000, Australia".to_owned(),
            },
            Candidate {
                name: "Veggo Sizzle".to_owned(),
                address: "175 First Ave, Five Dock NSW 2046, Australia".to_owned(),
            },
        ]
    );
}

#[test]
fn an_empty_response_parses_into_no_candidates() {
    // 検索の結果が無いときの応答は places を持たない (候補 0 件にする)。
    assert_eq!(parse_search_response("{}"), Ok(Vec::new()));
    assert_eq!(parse_search_response(r#"{"places": []}"#), Ok(Vec::new()));
}

#[test]
fn places_without_a_name_or_an_address_are_skipped() {
    let body = r#"{
        "places": [
            { "displayName": { "text": "名前だけ" } },
            { "formattedAddress": "住所だけ" },
            { "displayName": { "text": "   " }, "formattedAddress": "空白の名前" },
            { "displayName": { "text": "空白の住所" }, "formattedAddress": "   " },
            { "displayName": { "languageCode": "ja" }, "formattedAddress": "テキストが無い名前" },
            { "displayName": { "text": "正しい候補" }, "formattedAddress": "東京都渋谷区" }
        ]
    }"#;
    assert_eq!(
        parse_search_response(body),
        Ok(vec![Candidate {
            name: "正しい候補".to_owned(),
            address: "東京都渋谷区".to_owned(),
        }])
    );
}

#[test]
fn names_and_addresses_are_trimmed() {
    let body = r#"{
        "places": [
            { "displayName": { "text": "  丸山珈琲  " }, "formattedAddress": " 長野県軽井沢町 " }
        ]
    }"#;
    assert_eq!(
        parse_search_response(body),
        Ok(vec![Candidate {
            name: "丸山珈琲".to_owned(),
            address: "長野県軽井沢町".to_owned(),
        }])
    );
}

#[test]
fn the_candidates_are_capped_at_the_limit() {
    let places: Vec<String> = (0..MAX_CANDIDATES + 3)
        .map(|index| {
            format!(
                r#"{{ "displayName": {{ "text": "店 {index}" }}, "formattedAddress": "住所 {index}" }}"#
            )
        })
        .collect();
    let body = format!(r#"{{ "places": [{}] }}"#, places.join(","));
    let candidates = parse_search_response(&body).expect("the response must parse");
    assert_eq!(candidates.len(), MAX_CANDIDATES);
    assert_eq!(candidates[0].name, "店 0");
    assert_eq!(candidates[MAX_CANDIDATES - 1].name, "店 4");
}

#[test]
fn an_invalid_response_is_an_error() {
    assert_eq!(
        parse_search_response("not JSON"),
        Err(ParseError::InvalidJson)
    );
    assert_eq!(
        parse_search_response(r#"{"places": "not an array"}"#),
        Err(ParseError::InvalidShape)
    );
    // JSON ではあるがオブジェクトでない応答も形の誤りにする。
    assert_eq!(parse_search_response("[]"), Err(ParseError::InvalidShape));
}

#[test]
fn the_search_input_is_validated() {
    // 前後の空白は除く。
    assert_eq!(
        validate_search_input("  丸山珈琲  ", "ja"),
        Ok(("丸山珈琲".to_owned(), "ja"))
    );
    assert_eq!(
        validate_search_input("", "ja"),
        Err(SearchInputError::EmptyQuery)
    );
    assert_eq!(
        validate_search_input("   ", "ja"),
        Err(SearchInputError::EmptyQuery)
    );
    // 長さは文字数で数える (バイト数ではない)。
    let just_fits = "あ".repeat(MAX_QUERY_CHARS);
    let too_long = "あ".repeat(MAX_QUERY_CHARS + 1);
    assert!(validate_search_input(&just_fits, "ja").is_ok());
    assert_eq!(
        validate_search_input(&too_long, "ja"),
        Err(SearchInputError::QueryTooLong)
    );
    // 言語は 2 つだけを受け付ける。
    assert_eq!(SUPPORTED_LANGUAGES, ["ja", "en"]);
    assert_eq!(
        validate_search_input("店", "en"),
        Ok(("店".to_owned(), "en"))
    );
    for lang in ["fr", "JA", "japanese", ""] {
        assert_eq!(
            validate_search_input("店", lang),
            Err(SearchInputError::UnsupportedLanguage),
            "{lang}"
        );
    }
}

#[test]
fn the_error_messages_are_english() {
    for error in [
        SearchInputError::EmptyQuery,
        SearchInputError::QueryTooLong,
        SearchInputError::UnsupportedLanguage,
    ] {
        let message = error.message();
        assert!(
            message.is_ascii() && !message.is_empty(),
            "the message must be English: {message}"
        );
    }
    for error in [ParseError::InvalidJson, ParseError::InvalidShape] {
        let message = error.message();
        assert!(
            message.is_ascii() && !message.is_empty(),
            "the message must be English: {message}"
        );
    }
}

#[test]
fn the_search_request_body_fixes_the_query_and_the_language() {
    // Google に渡す本体の形 (FR-22)。pageSize は候補の上限と同じにする。
    // キーの順序は serde_json の feature (preserve_order) に依存するため、値で比べる。
    let body: serde_json::Value = serde_json::from_str(&search_request_body("丸山珈琲", "ja"))
        .expect("the body must be JSON");
    assert_eq!(
        body,
        serde_json::json!({
            "textQuery": "丸山珈琲",
            "pageSize": 5,
            "languageCode": "ja",
        })
    );
    let body: serde_json::Value =
        serde_json::from_str(&search_request_body("Maruyama Coffee", "en"))
            .expect("the body must be JSON");
    assert_eq!(
        body,
        serde_json::json!({
            "textQuery": "Maruyama Coffee",
            "pageSize": 5,
            "languageCode": "en",
        })
    );
    assert_eq!(MAX_CANDIDATES, 5);
}

#[test]
fn the_referer_header_is_the_origin_with_a_slash() {
    // キーは 1 つで、リファラの制限をブラウザと Worker の両方で通す (ADR-0019)。
    assert_eq!(
        referer_header("https://brewbook.example.workers.dev"),
        "https://brewbook.example.workers.dev/"
    );
    // 末尾の `/` は重ねない。
    assert_eq!(
        referer_header("https://brewbook.example.workers.dev/"),
        "https://brewbook.example.workers.dev/"
    );
    assert_eq!(
        referer_header("http://localhost:8787"),
        "http://localhost:8787/"
    );
}

#[test]
fn the_response_shapes_are_fixed() {
    let search = SearchResponse {
        candidates: vec![Candidate {
            name: "店".to_owned(),
            address: "住所".to_owned(),
        }],
    };
    assert_eq!(
        serde_json::to_value(&search).expect("the response must serialize"),
        serde_json::json!({
            "candidates": [{ "name": "店", "address": "住所" }]
        })
    );
    let config = MapsConfigResponse {
        embed_api_key: None,
    };
    assert_eq!(
        serde_json::to_value(&config).expect("the response must serialize"),
        serde_json::json!({ "embed_api_key": null })
    );
}
