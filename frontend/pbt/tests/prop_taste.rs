//! `TasteProfile` の PBT (FR-24、ADR-0023)。
//!
//! 任意の JSON のオブジェクトを応答として読んでもパニックせず、正しい型の応答は
//! そのままの値になることを確かめる。応答の形式の個別の検査は `test_records_api.rs` が担う。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::records::{RecordError, TasteProfile};
use proptest::prelude::*;
use serde_json::{json, Map, Value};

/// アルファベットと数字だけの文字列。
fn clean_text(max_chars: usize) -> impl Strategy<Value = String> {
    proptest::collection::vec("[a-zA-Z0-9]", 0..max_chars).prop_map(|chars| chars.concat())
}

/// 任意の JSON の値。
fn json_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        clean_text(12).prop_map(Value::String),
        any::<i64>().prop_map(|value| json!(value)),
        any::<bool>().prop_map(Value::Bool),
        Just(Value::Null),
        prop::collection::vec(clean_text(8).prop_map(Value::String), 0..3).prop_map(Value::Array),
    ]
}

/// 正しい型の推定の応答。
fn valid_profile() -> impl Strategy<Value = Value> {
    (
        clean_text(40),
        proptest::collection::vec(clean_text(12), 0..4),
        prop::option::of(clean_text(20)),
        prop::option::of(clean_text(20)),
        prop::option::of(clean_text(20)),
        prop::option::of(clean_text(20)),
        prop::option::of(clean_text(20)),
        prop::option::of(clean_text(20)),
        prop::option::of(clean_text(20)),
        prop::option::of(clean_text(20)),
        prop::option::of(clean_text(20)),
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
            )| {
                json!({
                    "summary": summary,
                    "flavor_notes": flavor_notes,
                    "acidity": acidity,
                    "sweetness": sweetness,
                    "bitterness": bitterness,
                    "body": body,
                    "aroma": aroma,
                    "brewing_method": brewing_method,
                    "grind_setting": grind_setting,
                    "process_description": process_description,
                    "region_characteristics": region_characteristics,
                })
            },
        )
}

proptest! {
    /// 正しい型の応答は、そのままの値で読める。
    #[test]
    fn a_valid_profile_is_read(value in valid_profile()) {
        let object = value.as_object().expect("the profile must be an object");
        let profile = TasteProfile::from_json(object).expect("the profile must be read");
        prop_assert_eq!(&profile.summary, object["summary"].as_str().unwrap());
        prop_assert_eq!(
            profile.flavor_notes.len(),
            object["flavor_notes"].as_array().unwrap().len()
        );
        prop_assert_eq!(&profile.acidity, &object["acidity"].as_str().map(str::to_owned));
        prop_assert_eq!(&profile.sweetness, &object["sweetness"].as_str().map(str::to_owned));
        prop_assert_eq!(&profile.bitterness, &object["bitterness"].as_str().map(str::to_owned));
        prop_assert_eq!(&profile.body, &object["body"].as_str().map(str::to_owned));
        prop_assert_eq!(&profile.aroma, &object["aroma"].as_str().map(str::to_owned));
    }

    /// 任意の JSON のオブジェクトを読んでもパニックせず、読めた値は型が正しい。
    #[test]
    fn any_object_is_read_or_rejected_without_a_panic(
        summary in json_value(),
        flavor_notes in json_value(),
        traits in proptest::collection::vec(json_value(), 5),
    ) {
        let mut object = Map::new();
        object.insert("summary".to_string(), summary);
        object.insert("flavor_notes".to_string(), flavor_notes);
        for (key, value) in ["acidity", "sweetness", "bitterness", "body", "aroma"]
            .into_iter()
            .zip(traits)
        {
            object.insert(key.to_string(), value);
        }
        match TasteProfile::from_json(&object) {
            Ok(profile) => {
                prop_assert!(object["summary"].is_string());
                prop_assert!(
                    object["flavor_notes"].is_array() || object["flavor_notes"].is_null()
                );
                prop_assert_eq!(&profile.acidity, &object["acidity"].as_str().map(str::to_owned));
                prop_assert_eq!(&profile.sweetness, &object["sweetness"].as_str().map(str::to_owned));
                prop_assert_eq!(&profile.bitterness, &object["bitterness"].as_str().map(str::to_owned));
                prop_assert_eq!(&profile.body, &object["body"].as_str().map(str::to_owned));
                prop_assert_eq!(&profile.aroma, &object["aroma"].as_str().map(str::to_owned));
            }
            Err(RecordError::Format(_)) => {}
            Err(error) => prop_assert!(false, "unexpected error: {error:?}"),
        }
    }
}
