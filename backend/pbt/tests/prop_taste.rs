//! `taste` の PBT (FR-24、ADR-0023)。
//!
//! 任意のテキストと、型の違う値の組み合わせからなる応答の JSON を解析し、結果が検証の条件
//! (長さ、件数、前後の空白) を満たすことを確認する。形式ごとの個別の検査は単体テスト
//! (`test_taste.rs`) が担う。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_core::taste::{
    parse_response, user_prompt, BeanInfo, TasteProfile, MAX_BREWING_METHOD_CHARS,
    MAX_GRIND_SETTING_CHARS, MAX_NOTES, MAX_NOTE_CHARS, MAX_PROCESS_DESCRIPTION_CHARS,
    MAX_PROMPT_FIELD_CHARS, MAX_PROMPT_NOTES, MAX_PROMPT_NOTE_CHARS,
    MAX_REGION_CHARACTERISTICS_CHARS, MAX_SUMMARY_CHARS, MAX_TRAIT_CHARS,
};
use proptest::prelude::*;

/// 解析の結果が検証の条件を満たすことを確かめる。
fn assert_valid(profile: &TasteProfile) {
    assert!(
        profile.summary.chars().count() <= MAX_SUMMARY_CHARS,
        "{profile:?}"
    );
    assert_eq!(profile.summary, profile.summary.trim(), "{profile:?}");
    assert!(profile.flavor_notes.len() <= MAX_NOTES, "{profile:?}");
    for note in &profile.flavor_notes {
        assert!(
            !note.trim().is_empty()
                && note.chars().count() <= MAX_NOTE_CHARS
                && note == note.trim(),
            "a flavor note must be a short trimmed string: {profile:?}"
        );
    }
    for trait_text in [
        &profile.acidity,
        &profile.sweetness,
        &profile.bitterness,
        &profile.body,
        &profile.aroma,
    ] {
        assert!(
            trait_text.as_ref().is_none_or(|text| {
                !text.trim().is_empty()
                    && text.chars().count() <= MAX_TRAIT_CHARS
                    && text == text.trim()
            }),
            "{profile:?}"
        );
    }
    for (text, max_chars) in [
        (&profile.brewing_method, MAX_BREWING_METHOD_CHARS),
        (&profile.grind_setting, MAX_GRIND_SETTING_CHARS),
        (&profile.process_description, MAX_PROCESS_DESCRIPTION_CHARS),
        (
            &profile.region_characteristics,
            MAX_REGION_CHARACTERISTICS_CHARS,
        ),
    ] {
        assert!(
            text.as_ref().is_none_or(|value| {
                !value.trim().is_empty()
                    && value.chars().count() <= max_chars
                    && value == value.trim()
            }),
            "{profile:?}"
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
        proptest::collection::vec(json_fragment(), 12),
        proptest::collection::vec(json_fragment(), 0..5),
    )
        .prop_map(|(values, notes)| {
            format!(
                r#"{{"summary": {}, "flavor_notes": [{}], "acidity": {}, "sweetness": {}, "bitterness": {}, "body": {}, "aroma": {}, "brewing_method": {}, "grind_setting": {}, "process_description": {}, "region_characteristics": {}, "confidence": {}}}"#,
                values[0],
                notes.join(", "),
                values[1],
                values[2],
                values[3],
                values[4],
                values[5],
                values[6],
                values[7],
                values[8],
                values[9],
                values[10],
            )
        })
}

/// 検証を通る推定を作る。
fn valid_profile() -> impl Strategy<Value = TasteProfile> {
    (
        clean_text(60),
        proptest::collection::vec(clean_text(20), 0..5),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
        prop::option::of(clean_text(30)),
    )
        .prop_map(
            |(
                summary,
                flavor_notes,
                acidity,
                sweetness,
                bitterness,
                body,
                aroma,
                brewing_method,
                grind_setting,
                process_description,
                region_characteristics,
            )| TasteProfile {
                summary,
                flavor_notes,
                acidity,
                sweetness,
                bitterness,
                body,
                aroma,
                brewing_method,
                grind_setting,
                process_description,
                region_characteristics,
            },
        )
}

/// 任意のテキスト。
fn any_text() -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<char>(), 0..300)
        .prop_map(|characters| characters.into_iter().collect())
}

/// 任意のテキストからなる豆の情報。`BeanInfo` は参照を取るため、所有する値として持つ。
#[derive(Debug)]
struct Bean {
    name: String,
    producer: Option<String>,
    origin: Option<String>,
    region: Option<String>,
    process: Option<String>,
    variety: Option<String>,
    flavor_notes: Vec<String>,
    roast: Option<String>,
}

impl Bean {
    /// 豆の情報を借用した形にする。
    fn as_info(&self) -> BeanInfo<'_> {
        BeanInfo {
            name: &self.name,
            producer: self.producer.as_deref(),
            origin: self.origin.as_deref(),
            region: self.region.as_deref(),
            process: self.process.as_deref(),
            variety: self.variety.as_deref(),
            flavor_notes: &self.flavor_notes,
            roast: self.roast.as_deref(),
        }
    }
}

/// 任意の豆の情報。
fn any_bean() -> impl Strategy<Value = Bean> {
    (
        any_text(),
        prop::option::of(any_text()),
        prop::option::of(any_text()),
        prop::option::of(any_text()),
        prop::option::of(any_text()),
        prop::option::of(any_text()),
        proptest::collection::vec(any_text(), 0..15),
        prop::option::of(any_text()),
    )
        .prop_map(
            |(name, producer, origin, region, process, variety, flavor_notes, roast)| Bean {
                name,
                producer,
                origin,
                region,
                process,
                variety,
                flavor_notes,
                roast,
            },
        )
}

/// プロンプトに埋め込まれた豆の情報を、`<bean_data>` で囲まれた JSON として読む。
fn bean_data(user: &str) -> serde_json::Value {
    let body = user
        .split_once("<bean_data>\n")
        .expect("the user message must open the bean data")
        .1
        .split_once("\n</bean_data>")
        .expect("the user message must close the bean data")
        .0;
    serde_json::from_str(body).expect("the bean data must be JSON")
}

/// プロンプトのデータの範囲を壊す文字か。
fn breaks_prompt_data(character: char) -> bool {
    character.is_control() || matches!(character, '<' | '>' | '`')
}

proptest! {
    /// 任意のテキストを解析しても、結果は検証の条件を満たす。
    #[test]
    fn any_text_parses_into_a_valid_profile(text in any::<String>()) {
        assert_valid(&parse_response(&text));
    }

    /// 型の違う値が混ざった応答の JSON を解析しても、結果は検証の条件を満たす。
    #[test]
    fn any_response_json_parses_into_a_valid_profile(json in response_json()) {
        assert_valid(&parse_response(&json));
    }

    /// 検証を通る応答は、モデルが返す形の JSON にしてから解析すると元に戻る。
    #[test]
    fn a_valid_profile_round_trips(profile in valid_profile()) {
        let json = serde_json::to_string(&profile).expect("the profile must be JSON");
        prop_assert_eq!(parse_response(&json), profile);
    }

    /// 任意の豆の情報から組み立てた user メッセージは、データの境界を壊さず、項目の上限を守る。
    #[test]
    fn any_bean_builds_a_bounded_user_prompt(bean in any_bean()) {
        let user = user_prompt(&bean.as_info());
        // 入力の文字列からタグを作り出せないため、データを囲むタグはちょうど 1 組になる。
        prop_assert_eq!(user.matches("<bean_data>").count(), 1);
        prop_assert_eq!(user.matches("</bean_data>").count(), 1);
        let data = bean_data(&user);
        prop_assert!(data.is_object());
        let mut total = 0;
        for key in [
            "name",
            "producer",
            "origin",
            "region",
            "process",
            "variety",
            "roast",
        ] {
            let Some(value) = data[key].as_str() else {
                prop_assert!(data[key].is_null());
                continue;
            };
            prop_assert!(value.chars().count() <= MAX_PROMPT_FIELD_CHARS);
            prop_assert_eq!(value, value.trim());
            prop_assert!(!value.chars().any(breaks_prompt_data));
            prop_assert!(!value.contains("  "));
            total += value.chars().count();
        }
        let notes = data["flavor_notes"]
            .as_array()
            .expect("the flavor notes must be an array");
        prop_assert!(notes.len() <= MAX_PROMPT_NOTES);
        for note in notes {
            let note = note.as_str().expect("a flavor note must be a string");
            prop_assert!(!note.is_empty());
            prop_assert!(note.chars().count() <= MAX_PROMPT_NOTE_CHARS);
            prop_assert_eq!(note, note.trim());
            prop_assert!(!note.chars().any(breaks_prompt_data));
            prop_assert!(!note.contains("  "));
            total += note.chars().count();
        }
        prop_assert!(
            total <= MAX_PROMPT_FIELD_CHARS * 7 + MAX_PROMPT_NOTE_CHARS * MAX_PROMPT_NOTES
        );
    }
}
