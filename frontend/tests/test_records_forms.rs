//! `records::forms` の単体テスト (0041)。
//!
//! フォームの検証と、推測の適用 (FR-19。完了条件 6)、自由記述の項目のサジェストの状態
//! (FR-13。完了条件 7) を確かめる。

use brew_book_frontend::i18n::Key;
use brew_book_frontend::records::{
    product_match, suggested_product_name, validate_brew_form, validate_product_form,
    validate_purchase_form, validate_shop_form, BrewFormValues, Product, ProductMatch,
    ProductSuggestion, Purchase, PurchaseFormValues, PurchaseSuggestion, SuggestionState,
};

/// 商品を作る。
fn product(id: &str, name: &str) -> Product {
    Product {
        id: id.to_string(),
        name: name.to_string(),
        producer: None,
        origin: None,
        region: None,
        process: None,
        variety: None,
        flavor_notes: Vec::new(),
        created_at: "2026-10-01T00:00:00.000Z".to_string(),
        updated_at: "2026-10-01T00:00:00.000Z".to_string(),
        favorited_at: None,
    }
}

/// 購入を作る。
fn purchase(id: &str) -> Purchase {
    Purchase {
        id: id.to_string(),
        product_id: "p1".to_string(),
        shop_id: None,
        purchased_on: "2026-10-01".to_string(),
        roast: None,
        roast_date: None,
        price_amount: None,
        price_currency: None,
        weight_grams: None,
        photo_key: None,
        created_at: "2026-10-01T00:00:00.000Z".to_string(),
        updated_at: "2026-10-01T00:00:00.000Z".to_string(),
        favorited_at: None,
        product: product("p1", "豆"),
        shop: None,
    }
}

#[test]
fn the_shop_form_requires_a_name() {
    assert_eq!(validate_shop_form("  ", ""), Err(Key::ValidationRequired));
    let input = validate_shop_form(" 店 ", " 住所 ").expect("the input must be valid");
    assert_eq!(input.name, "店");
    assert_eq!(input.address.as_deref(), Some("住所"));
    let input = validate_shop_form("店", "").expect("the address is optional");
    assert_eq!(input.address, None);
}

#[test]
fn the_product_form_requires_a_name_and_normalizes_the_tags() {
    assert_eq!(
        validate_product_form("", "", "", "", "", "", &[]),
        Err(Key::ValidationRequired)
    );
    let input = validate_product_form(
        " 豆 ",
        " 生産者 ",
        "",
        "",
        "",
        "",
        &[
            " 甘い ".to_string(),
            "甘い".to_string(),
            "  ".to_string(),
            "酸っぱい".to_string(),
        ],
    )
    .expect("the input must be valid");
    assert_eq!(input.name, "豆");
    assert_eq!(input.producer.as_deref(), Some("生産者"));
    assert_eq!(
        input.flavor_notes,
        vec!["甘い".to_string(), "酸っぱい".to_string()]
    );
}

#[test]
fn the_purchase_form_requires_a_product_and_a_valid_day() {
    let errors = validate_purchase_form(PurchaseFormValues {
        product: None,
        shop: None,
        purchased_on: "2026-02-30",
        roast: "",
        roast_date: "",
        price: "",
        currency: "JPY",
        weight: "",
    })
    .expect_err("the form must be rejected");
    assert_eq!(errors.product, Some(Key::ValidationProduct));
    assert_eq!(errors.purchased_on, Some(Key::ValidationDay));
    assert!(errors.has_errors());

    let input = validate_purchase_form(PurchaseFormValues {
        product: Some(&product("p1", "豆")),
        shop: None,
        purchased_on: "2026-10-01",
        roast: " 中煎り ",
        roast_date: "",
        price: "1200",
        currency: "JPY",
        weight: "200",
    })
    .expect("the form must be accepted");
    assert_eq!(input.product_id, "p1");
    assert_eq!(input.roast.as_deref(), Some("中煎り"));
    assert_eq!(input.price_amount, Some(1200));
    assert_eq!(input.price_currency.as_deref(), Some("JPY"));
    assert_eq!(input.weight_grams, Some(200));
}

#[test]
fn the_purchase_form_rejects_broken_optional_fields() {
    let errors = validate_purchase_form(PurchaseFormValues {
        product: Some(&product("p1", "豆")),
        shop: None,
        purchased_on: "2026-10-01",
        roast: "",
        roast_date: "2026-13-01",
        price: "-1",
        currency: "JPY",
        weight: "1.5",
    })
    .expect_err("the form must be rejected");
    assert_eq!(errors.roast_date, Some(Key::ValidationDay));
    assert_eq!(errors.price, Some(Key::ValidationNumber));
    assert_eq!(errors.weight, Some(Key::ValidationNumber));

    // 価格が無いときは通貨コードを送らない (API は組で扱う。FR-9)。
    let input = validate_purchase_form(PurchaseFormValues {
        product: Some(&product("p1", "豆")),
        shop: None,
        purchased_on: "2026-10-01",
        roast: "",
        roast_date: "",
        price: "",
        currency: "JPY",
        weight: "",
    })
    .expect("the form must be accepted");
    assert_eq!(input.price_amount, None);
    assert_eq!(input.price_currency, None);
}

#[test]
fn the_brew_form_requires_a_purchase_and_converts_the_time_to_utc() {
    let errors = validate_brew_form(BrewFormValues {
        purchase: None,
        date: "2026-10-01",
        time: "25:00",
        dose: "",
        water: "",
        water_temp: "",
        brew_time: "",
        method: "",
        grind_setting: "",
        rating: None,
        notes: "",
        utc_offset_minutes: 540,
    })
    .expect_err("the form must be rejected");
    assert_eq!(errors.purchase, Some(Key::ValidationPurchase));
    assert_eq!(errors.time, Some(Key::ValidationTime));

    let input = validate_brew_form(BrewFormValues {
        purchase: Some(&purchase("b1")),
        date: "2026-10-01",
        time: "09:30",
        dose: "15.0",
        water: "",
        water_temp: "92.0",
        brew_time: "165",
        method: " ドリッパー ",
        grind_setting: "",
        rating: Some(4),
        notes: "",
        utc_offset_minutes: 540,
    })
    .expect("the form must be accepted");
    assert_eq!(input.purchase_id, "b1");
    // 日本標準時 (UTC+9) の 09:30 は UTC の 00:30。
    assert_eq!(input.brewed_at, "2026-10-01T00:30:00.000Z");
    assert_eq!(input.dose_grams, Some(15.0));
    assert_eq!(input.water_temp_c, Some(92.0));
    assert_eq!(input.brew_time_seconds, Some(165));
    assert_eq!(input.method.as_deref(), Some("ドリッパー"));
    assert_eq!(input.rating, Some(4));
}

#[test]
fn the_brew_form_rejects_broken_numbers() {
    let errors = validate_brew_form(BrewFormValues {
        purchase: Some(&purchase("b1")),
        date: "2026-10-01",
        time: "09:30",
        dose: "15.05",
        water: "-1",
        water_temp: "92.0",
        brew_time: "1.5",
        method: "",
        grind_setting: "",
        rating: None,
        notes: "",
        utc_offset_minutes: 540,
    })
    .expect_err("the form must be rejected");
    assert_eq!(errors.dose, Some(Key::ValidationDecimal));
    assert_eq!(errors.water, Some(Key::ValidationDecimal));
    assert_eq!(errors.brew_time, Some(Key::ValidationNumber));
}

/// 日付と時刻を空にすると必須の誤りになる (完了条件 4)。date input と time input を空に
/// したときの保存の検証を確かめる。
#[test]
fn an_empty_day_or_time_is_a_required_error() {
    let errors = validate_purchase_form(PurchaseFormValues {
        product: Some(&product("p1", "豆")),
        shop: None,
        purchased_on: "",
        roast: "",
        roast_date: "",
        price: "",
        currency: "JPY",
        weight: "",
    })
    .expect_err("the form must be rejected");
    assert_eq!(errors.purchased_on, Some(Key::ValidationDay));

    let errors = validate_brew_form(BrewFormValues {
        purchase: Some(&purchase("b1")),
        date: "",
        time: "09:30",
        dose: "",
        water: "",
        water_temp: "",
        brew_time: "",
        method: "",
        grind_setting: "",
        rating: None,
        notes: "",
        utc_offset_minutes: 540,
    })
    .expect_err("the form must be rejected");
    assert_eq!(errors.date, Some(Key::ValidationDay));

    let errors = validate_brew_form(BrewFormValues {
        purchase: Some(&purchase("b1")),
        date: "2026-10-01",
        time: "",
        dose: "",
        water: "",
        water_temp: "",
        brew_time: "",
        method: "",
        grind_setting: "",
        rating: None,
        notes: "",
        utc_offset_minutes: 540,
    })
    .expect_err("the form must be rejected");
    assert_eq!(errors.time, Some(Key::ValidationTime));
}

#[test]
fn the_product_match_selects_only_when_no_product_is_selected() {
    let suggestion = PurchaseSuggestion {
        product: Some(ProductSuggestion {
            name: Some(" 豆 ".to_string()),
            ..ProductSuggestion::default()
        }),
        ..PurchaseSuggestion::default()
    };
    assert_eq!(
        suggested_product_name(suggestion.product.as_ref().expect("a product must exist"))
            .as_deref(),
        Some("豆")
    );

    // 一致する商品があるときは、商品が未選択のときだけ選ぶ (FR-19)。
    match product_match(false, &suggestion, Some(product("p1", "豆"))) {
        ProductMatch::Select(selected) => assert_eq!(selected.id, "p1"),
        other => panic!("the product must be selected: {other:?}"),
    }
    assert_eq!(
        product_match(true, &suggestion, Some(product("p1", "豆"))),
        ProductMatch::None
    );

    // 一致が無いときは、推測値を引き継いだ登録の導線を出す (FR-19)。
    match product_match(false, &suggestion, None) {
        ProductMatch::Register(suggested) => {
            assert_eq!(suggested.name.as_deref(), Some(" 豆 "));
        }
        other => panic!("the register affordance must be shown: {other:?}"),
    }

    // 推測した商品名が無いときは何もしない。
    let empty = PurchaseSuggestion {
        product: Some(ProductSuggestion::default()),
        ..PurchaseSuggestion::default()
    };
    assert_eq!(product_match(false, &empty, None), ProductMatch::None);
}

#[test]
fn the_suggestion_state_shows_selects_and_ignores_stale_responses() {
    let mut state = SuggestionState::new();
    let first = state.begin("中");
    assert!(!state.is_open());

    // 古い世代の応答は捨てる (FR-13)。
    assert!(!state.apply_options(first + 1, vec!["深煎り".to_string()]));
    assert!(state.options().is_empty());

    let second = state.begin("中");
    assert!(state.apply_options(second, vec!["中煎り".to_string()]));
    assert!(state.is_open());
    assert_eq!(state.options(), ["中煎り".to_string()]);

    // 候補を選ぶと入力の値になり、表示を閉じる。
    assert_eq!(state.select("中煎り"), "中煎り");
    assert_eq!(state.query(), "中煎り");
    assert!(!state.is_open());

    // 候補を引けなくても入力は続けられる。
    let third = state.begin("深");
    assert!(state.apply_options(third, Vec::new()));
    assert!(!state.is_open());
    assert_eq!(state.query(), "深");
}

#[test]
fn the_highlight_splits_the_matched_prefix() {
    assert_eq!(
        SuggestionState::new().highlight("中煎り"),
        (String::new(), "中煎り".to_string())
    );

    let mut state = SuggestionState::new();
    let _ = state.begin(" 中 ");
    assert_eq!(
        state.highlight("中煎り"),
        ("中".to_string(), "煎り".to_string())
    );
    // 一致しない候補はそのまま出す。
    assert_eq!(
        state.highlight("深煎り"),
        (String::new(), "深煎り".to_string())
    );
}

/// 保存が新規か更新かの判断 (0041 のレビューの指摘)。
#[test]
fn a_save_target_depends_on_the_id() {
    use brew_book_frontend::records::{save_target, SaveTarget};

    assert_eq!(save_target(None), SaveTarget::Create);
    assert_eq!(
        save_target(Some("p1")),
        SaveTarget::Update("p1".to_string())
    );
}
