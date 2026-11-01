//! `taste` の単体テスト (FR-24、ADR-0023)。
//!
//! プロンプトの組み立て、言語の検証、モデルの応答の解析と検証を覆う。
//! AI の呼び出しは Worker 側の責務のため、ここでは扱わない。

use brew_book_core::taste::{
    self, BeanInfo, LanguageError, TasteProfile, MAX_BREWING_METHOD_CHARS, MAX_GRIND_SETTING_CHARS,
    MAX_NOTES, MAX_NOTE_CHARS, MAX_PROCESS_DESCRIPTION_CHARS, MAX_REGION_CHARACTERISTICS_CHARS,
    MAX_SUMMARY_CHARS, MAX_TRAIT_CHARS,
};

/// 推定に使う豆の情報。テストの間だけ生きる Flavor Notes を解放して参照にする。
fn bean_info() -> BeanInfo<'static> {
    let flavor_notes: &'static [String] =
        Box::leak(vec!["ピーチ".to_string(), "ジャスミン".to_string()].into_boxed_slice());
    BeanInfo {
        name: "Ethiopia Guji Natural",
        producer: Some("Guji Highland Farm"),
        origin: Some("Ethiopia"),
        region: Some("Guji"),
        process: Some("Natural"),
        variety: Some("Heirloom"),
        flavor_notes,
        roast: Some("浅煎り"),
    }
}

#[test]
fn the_supported_languages_are_japanese_and_english() {
    assert_eq!(taste::validate_language("ja"), Ok("Japanese"));
    assert_eq!(taste::validate_language("en"), Ok("English"));
    assert_eq!(taste::validate_language("JA"), Ok("Japanese"));
    assert_eq!(taste::validate_language("EN"), Ok("English"));
    for lang in ["", "fr", "japanese", "ja-JP", "en-US"] {
        assert_eq!(
            taste::validate_language(lang),
            Err(LanguageError::UnsupportedLanguage),
            "{lang}"
        );
        assert_eq!(
            LanguageError::UnsupportedLanguage.message(),
            "the language must be ja or en"
        );
    }
}

#[test]
fn the_prompt_contains_the_bean_information_and_the_language() {
    let info = bean_info();
    let prompt = taste::prompt(&info, "Japanese");
    for value in [
        "Ethiopia Guji Natural",
        "Guji Highland Farm",
        "Ethiopia",
        "Guji",
        "Natural",
        "Heirloom",
        "ピーチ",
        "ジャスミン",
        "浅煎り",
        "Write the answer in Japanese.",
        "exactly these keys",
        "at most 400 characters",
        "at most 10 short flavor descriptions",
        "brewing_method",
        "grind_setting",
        "process_description",
        "region_characteristics",
        "at most 100 characters",
        "at most 200 characters",
        "Do not add any keys other than the ones above.",
    ] {
        assert!(prompt.contains(value), "the prompt must contain {value}");
    }
    assert!(!prompt.contains("Write the answer in English."));
    assert!(taste::prompt(&info, "English").contains("English"));
}

#[test]
fn the_prompt_embeds_the_user_values_as_json() {
    let notes = Vec::new();
    let info = BeanInfo {
        name: "name with \"quotes\" and\nnewlines",
        producer: Some("producer"),
        origin: None,
        region: None,
        process: None,
        variety: None,
        flavor_notes: &notes,
        roast: None,
    };
    let prompt = taste::prompt(&info, "English");
    // 改行や引用符は JSON としてエスケープされ、プロンプトの行を壊さない。
    assert!(prompt.contains(r#""name with \"quotes\" and\nnewlines""#));
    assert!(prompt.contains(r#""origin":null"#));
    assert!(prompt.contains(r#""flavor_notes":[]"#));
}

#[test]
fn a_full_response_is_parsed() {
    let profile = taste::parse_response(
        r#"Here is the result.
```json
{
  "summary": "明るい酸と桃のような甘さがあり、冷めるとジャスミンの香りが立つ。",
  "flavor_notes": ["ピーチ", "ジャスミン", " 黒糖 "],
  "acidity": "明るい柑橘の酸",
  "sweetness": "桃のような甘さ",
  "bitterness": "控えめ",
  "body": "軽い",
  "aroma": "ジャスミン",
  "brewing_method": "V60 のハンドドリップ",
  "grind_setting": "中細",
  "process_description": "ナチュラル精製は果実の甘さを残しやすい。",
  "region_characteristics": "イルガチェフェは花のような香りと明るい酸が特徴。"
}
```
"#,
    );
    assert_eq!(
        profile.summary,
        "明るい酸と桃のような甘さがあり、冷めるとジャスミンの香りが立つ。"
    );
    assert_eq!(profile.flavor_notes, vec!["ピーチ", "ジャスミン", "黒糖"]);
    assert_eq!(profile.acidity.as_deref(), Some("明るい柑橘の酸"));
    assert_eq!(profile.sweetness.as_deref(), Some("桃のような甘さ"));
    assert_eq!(profile.bitterness.as_deref(), Some("控えめ"));
    assert_eq!(profile.body.as_deref(), Some("軽い"));
    assert_eq!(profile.aroma.as_deref(), Some("ジャスミン"));
    assert_eq!(
        profile.brewing_method.as_deref(),
        Some("V60 のハンドドリップ")
    );
    assert_eq!(profile.grind_setting.as_deref(), Some("中細"));
    assert_eq!(
        profile.process_description.as_deref(),
        Some("ナチュラル精製は果実の甘さを残しやすい。")
    );
    assert_eq!(
        profile.region_characteristics.as_deref(),
        Some("イルガチェフェは花のような香りと明るい酸が特徴。")
    );
    assert!(profile.has_content());
}

#[test]
fn invalid_fields_are_dropped_individually() {
    let profile = taste::parse_response(
        r#"{
          "summary": "  まとめ  ",
          "flavor_notes": ["ok", 1, " ", "too long too long too long too long too long", null],
          "acidity": 3,
          "sweetness": "",
          "bitterness": "   ",
          "body": "  中程度  ",
          "aroma": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "brewing_method": 1,
          "grind_setting": "  ",
          "process_description": "   "
        }"#,
    );
    assert_eq!(profile.summary, "まとめ");
    assert_eq!(profile.flavor_notes, vec!["ok"]);
    assert_eq!(profile.acidity, None);
    assert_eq!(profile.sweetness, None);
    assert_eq!(profile.bitterness, None);
    assert_eq!(profile.body.as_deref(), Some("中程度"));
    assert_eq!(profile.aroma, None);
    assert_eq!(profile.brewing_method, None);
    assert_eq!(profile.grind_setting, None);
    assert_eq!(profile.process_description, None);
}

#[test]
fn the_limits_are_enforced() {
    let summary = "a".repeat(MAX_SUMMARY_CHARS);
    let too_long_summary = "a".repeat(MAX_SUMMARY_CHARS + 1);
    let trait_text = "b".repeat(MAX_TRAIT_CHARS);
    let too_long_trait = "b".repeat(MAX_TRAIT_CHARS + 1);
    let note = "c".repeat(MAX_NOTE_CHARS);
    let too_long_note = "c".repeat(MAX_NOTE_CHARS + 1);
    let notes = std::iter::repeat_n(note.clone(), MAX_NOTES + 3)
        .chain(std::iter::once(too_long_note))
        .collect::<Vec<_>>();
    let value = serde_json::json!({
        "summary": summary,
        "flavor_notes": notes,
        "acidity": trait_text.clone(),
        "sweetness": too_long_trait,
    })
    .to_string();
    let profile = taste::parse_response(&value);
    assert_eq!(profile.summary.chars().count(), MAX_SUMMARY_CHARS);
    assert_eq!(profile.flavor_notes.len(), MAX_NOTES);
    assert!(profile
        .flavor_notes
        .iter()
        .all(|note| note.chars().count() <= MAX_NOTE_CHARS));
    assert_eq!(profile.acidity.as_deref(), Some(trait_text.as_str()));
    assert_eq!(profile.sweetness, None);

    let profile =
        taste::parse_response(&serde_json::json!({"summary": too_long_summary}).to_string());
    assert!(profile.summary.is_empty());
    assert!(!profile.has_content());
}

#[test]
fn the_recommendation_limits_are_enforced() {
    let method = "d".repeat(MAX_BREWING_METHOD_CHARS);
    let grind = "e".repeat(MAX_GRIND_SETTING_CHARS);
    let process = "f".repeat(MAX_PROCESS_DESCRIPTION_CHARS);
    let region = "g".repeat(MAX_REGION_CHARACTERISTICS_CHARS);
    let profile = taste::parse_response(
        &serde_json::json!({
            "summary": "まとめ",
            "brewing_method": method,
            "grind_setting": grind,
            "process_description": process,
            "region_characteristics": region,
        })
        .to_string(),
    );
    assert_eq!(
        profile.brewing_method.as_deref().map(str::len),
        Some(method.len())
    );
    assert_eq!(
        profile.grind_setting.as_deref().map(str::len),
        Some(grind.len())
    );
    assert_eq!(
        profile.process_description.as_deref().map(str::len),
        Some(process.len())
    );
    assert_eq!(
        profile.region_characteristics.as_deref().map(str::len),
        Some(region.len())
    );
    let overlong = taste::parse_response(
        &serde_json::json!({
            "summary": "まとめ",
            "brewing_method": format!("{method}x"),
            "grind_setting": format!("{grind}x"),
            "process_description": format!("{process}x"),
            "region_characteristics": format!("{region}x"),
        })
        .to_string(),
    );
    assert_eq!(overlong.brewing_method, None);
    assert_eq!(overlong.grind_setting, None);
    assert_eq!(overlong.process_description, None);
    assert_eq!(overlong.region_characteristics, None);
}

#[test]
fn a_broken_response_is_an_empty_profile() {
    for text in [
        "",
        "no json here",
        "{not json}",
        "[1, 2, 3]",
        r#"{"summary": null, "flavor_notes": "not an array"}"#,
    ] {
        let profile = taste::parse_response(text);
        assert_eq!(profile, TasteProfile::default(), "{text:?}");
        assert!(!profile.has_content(), "{text:?}");
    }
}

#[test]
fn content_is_detected_from_any_field() {
    let mut profile = TasteProfile::default();
    assert!(!profile.has_content());
    profile.summary = "まとめ".to_string();
    assert!(profile.has_content());
    profile.summary.clear();
    profile.flavor_notes = vec!["甘い".to_string()];
    assert!(profile.has_content());
    profile.flavor_notes.clear();
    profile.brewing_method = Some("V60".to_string());
    assert!(profile.has_content());
    profile.brewing_method = None;
    profile.grind_setting = Some("中細".to_string());
    assert!(profile.has_content());
    profile.grind_setting = None;
    profile.process_description = Some("ナチュラル".to_string());
    assert!(profile.has_content());
    profile.process_description = None;
    profile.region_characteristics = Some("花の香り".to_string());
    assert!(profile.has_content());
    profile.region_characteristics = None;
    profile.aroma = Some("花".to_string());
    assert!(profile.has_content());
}
