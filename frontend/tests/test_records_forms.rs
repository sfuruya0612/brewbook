//! `records::forms` の単体テスト (0041)。
//!
//! フォームの検証と、推測の適用 (FR-19。完了条件 6)、自由記述の項目のサジェストの状態
//! (FR-13。完了条件 7) を確かめる。商品の項目は購入のフォームが持つ (ADR-0022)。

use brew_book_frontend::i18n::Key;
use brew_book_frontend::records::{
    apply_purchase_suggestion, dedup_tags, purchase_prefill, validate_brew_form,
    validate_purchase_form, validate_shop_form, BrewFormValues, Purchase, PurchaseFormFields,
    PurchaseFormValues, PurchasePrefill, PurchaseSuggestion, Shop, SuggestionState,
};

/// 購入を作る。
fn purchase(id: &str) -> Purchase {
    Purchase {
        id: id.to_string(),
        shop_id: None,
        name: "豆".to_string(),
        producer: None,
        origin: None,
        region: None,
        process: None,
        variety: None,
        flavor_notes: Vec::new(),
        taste_profile: None,
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
        shop: None,
    }
}

/// 購入のフォームの入力 (商品名と購入日だけを持ち、他は空)。
fn purchase_values<'a>(name: &'a str, purchased_on: &'a str) -> PurchaseFormValues<'a> {
    PurchaseFormValues {
        name,
        producer: "",
        origin: "",
        region: "",
        process: "",
        variety: "",
        flavor_notes: &[],
        shop: None,
        purchased_on,
        roast: "",
        roast_date: "",
        price: "",
        currency: "JPY",
        weight: "",
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
fn the_purchase_form_requires_a_name_and_a_valid_day() {
    let errors = validate_purchase_form(purchase_values("  ", "2026-02-30"))
        .expect_err("the form must be rejected");
    assert_eq!(errors.name, Some(Key::ValidationRequired));
    assert_eq!(errors.purchased_on, Some(Key::ValidationDay));
    assert!(errors.has_errors());

    let input = validate_purchase_form(PurchaseFormValues {
        name: " 豆 ",
        producer: " 生産者 ",
        origin: "",
        region: " イルガチェフェ ",
        process: "",
        variety: "",
        flavor_notes: &[],
        shop: None,
        purchased_on: "2026-10-01",
        roast: " 中煎り ",
        roast_date: "",
        price: "1200",
        currency: "JPY",
        weight: "200",
    })
    .expect("the form must be accepted");
    assert_eq!(input.name, "豆");
    assert_eq!(input.producer.as_deref(), Some("生産者"));
    assert_eq!(input.origin, None);
    assert_eq!(input.region.as_deref(), Some("イルガチェフェ"));
    assert_eq!(input.process, None);
    assert_eq!(input.variety, None);
    assert!(input.flavor_notes.is_empty());
    assert_eq!(input.roast.as_deref(), Some("中煎り"));
    assert_eq!(input.price_amount, Some(1200));
    assert_eq!(input.price_currency.as_deref(), Some("JPY"));
    assert_eq!(input.weight_grams, Some(200));
}

/// タグは前後の空白を除き、空のタグを落とし、同じ名前を 1 つにまとめる (FR-8)。
#[test]
fn the_purchase_form_normalizes_the_tags() {
    let tags = [
        " 甘い ".to_string(),
        "甘い".to_string(),
        "  ".to_string(),
        "酸っぱい".to_string(),
    ];
    assert_eq!(
        dedup_tags(&tags),
        vec!["甘い".to_string(), "酸っぱい".to_string()]
    );
    let input = validate_purchase_form(PurchaseFormValues {
        flavor_notes: &tags,
        ..purchase_values("豆", "2026-10-01")
    })
    .expect("the form must be accepted");
    assert_eq!(
        input.flavor_notes,
        vec!["甘い".to_string(), "酸っぱい".to_string()]
    );
    // 空の配列も入力として送る (全て外す操作を表す)。
    let json = input.to_json();
    assert_eq!(
        json["flavor_notes"],
        serde_json::json!(["甘い", "酸っぱい"])
    );
    let empty = validate_purchase_form(purchase_values("豆", "2026-10-01"))
        .expect("the form must be accepted");
    assert_eq!(empty.to_json()["flavor_notes"], serde_json::json!([]));
    assert!(empty.to_json().get("product_id").is_none());
}

#[test]
fn the_purchase_form_rejects_broken_optional_fields() {
    let errors = validate_purchase_form(PurchaseFormValues {
        roast_date: "2026-13-01",
        price: "-1",
        weight: "1.5",
        ..purchase_values("豆", "2026-10-01")
    })
    .expect_err("the form must be rejected");
    assert_eq!(errors.roast_date, Some(Key::ValidationDay));
    assert_eq!(errors.price, Some(Key::ValidationNumber));
    assert_eq!(errors.weight, Some(Key::ValidationNumber));

    // 価格が無いときは通貨コードを送らない (API は組で扱う。FR-9)。
    let input = validate_purchase_form(purchase_values("豆", "2026-10-01"))
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
    let errors =
        validate_purchase_form(purchase_values("豆", "")).expect_err("the form must be rejected");
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

/// 推測の適用の入力欄。
struct Fields {
    name: String,
    producer: String,
    origin: String,
    region: String,
    process: String,
    variety: String,
    flavor_notes: Vec<String>,
    roast: String,
    roast_date: String,
    price: String,
    weight: String,
}

impl Fields {
    /// 全ての入力欄に値が入った状態。
    fn filled() -> Self {
        Self {
            name: "入力した豆".to_string(),
            producer: "入力した生産者".to_string(),
            origin: "入力した生産国".to_string(),
            region: "入力した地域".to_string(),
            process: "入力した精製方法".to_string(),
            variety: "入力した品種".to_string(),
            flavor_notes: vec!["入力したタグ".to_string()],
            roast: "入力した焙煎度".to_string(),
            roast_date: "2026-09-01".to_string(),
            price: "100".to_string(),
            weight: "50".to_string(),
        }
    }

    /// 推測を入れる。
    fn apply(&mut self, suggestion: &PurchaseSuggestion) {
        apply_purchase_suggestion(
            PurchaseFormFields {
                name: &mut self.name,
                producer: &mut self.producer,
                origin: &mut self.origin,
                region: &mut self.region,
                process: &mut self.process,
                variety: &mut self.variety,
                flavor_notes: &mut self.flavor_notes,
                roast: &mut self.roast,
                roast_date: &mut self.roast_date,
                price: &mut self.price,
                weight: &mut self.weight,
            },
            suggestion,
        );
    }
}

/// 推測できた項目は入力済みの値も置き換え、推測できなかった項目は残す (FR-19、ADR-0022)。
#[test]
fn the_suggestion_overwrites_the_filled_fields_and_keeps_the_unsuggested_ones() {
    let suggestion = PurchaseSuggestion {
        name: Some("推測した豆".to_string()),
        producer: None,
        origin: Some("エチオピア".to_string()),
        region: None,
        process: Some("ウォッシュド".to_string()),
        variety: None,
        flavor_notes: vec![" フローラル ".to_string(), "フローラル".to_string()],
        roast: Some("中煎り".to_string()),
        roast_date: None,
        price_amount: Some(1200),
        weight_grams: None,
    };
    let mut fields = Fields::filled();
    fields.apply(&suggestion);
    assert_eq!(fields.name, "推測した豆");
    assert_eq!(fields.producer, "入力した生産者");
    assert_eq!(fields.origin, "エチオピア");
    assert_eq!(fields.region, "入力した地域");
    assert_eq!(fields.process, "ウォッシュド");
    assert_eq!(fields.variety, "入力した品種");
    assert_eq!(fields.flavor_notes, vec!["フローラル".to_string()]);
    assert_eq!(fields.roast, "中煎り");
    assert_eq!(fields.roast_date, "2026-09-01");
    assert_eq!(fields.price, "1200");
    assert_eq!(fields.weight, "50");

    // 空の推測は何も変えない (空の Flavor Notes も入力したタグを残す)。
    let mut fields = Fields::filled();
    fields.apply(&PurchaseSuggestion::default());
    assert_eq!(fields.name, "入力した豆");
    assert_eq!(fields.flavor_notes, vec!["入力したタグ".to_string()]);
    assert_eq!(fields.weight, "50");
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

/// 過去の購入から引き継ぐのは商品の項目と店と焙煎度だけで、購入日、焙煎日、価格、重量、
/// 写真は引き継がない (issue 0058)。
#[test]
fn the_prefill_copies_the_product_fields_the_shop_and_the_roast_only() {
    let shop = Shop {
        id: "s1".to_string(),
        name: "店".to_string(),
        address: Some("住所".to_string()),
        created_at: "2026-10-01T00:00:00.000Z".to_string(),
        updated_at: "2026-10-01T00:00:00.000Z".to_string(),
        favorited_at: None,
    };
    let mut source = purchase("p1");
    source.shop_id = Some("s1".to_string());
    source.producer = Some("生産者".to_string());
    source.origin = Some("Ethiopia".to_string());
    source.region = Some("Guji".to_string());
    source.process = Some("Natural".to_string());
    source.variety = Some("Heirloom".to_string());
    source.flavor_notes = vec![
        " berry ".to_string(),
        String::new(),
        "berry".to_string(),
        "floral".to_string(),
    ];
    source.roast = Some("中煎り".to_string());
    source.roast_date = Some("2026-09-20".to_string());
    source.price_amount = Some(1980);
    source.price_currency = Some("USD".to_string());
    source.weight_grams = Some(200);
    source.photo_key = Some("photos/p1".to_string());
    source.favorited_at = Some("2026-10-02T00:00:00.000Z".to_string());
    source.shop = Some(shop.clone());

    assert_eq!(
        purchase_prefill(&source),
        PurchasePrefill {
            name: "豆".to_string(),
            producer: "生産者".to_string(),
            origin: "Ethiopia".to_string(),
            region: "Guji".to_string(),
            process: "Natural".to_string(),
            variety: "Heirloom".to_string(),
            flavor_notes: vec!["berry".to_string(), "floral".to_string()],
            shop: Some(shop),
            roast: "中煎り".to_string(),
        }
    );
}

/// 項目の無い購入からの引き継ぎは、商品名だけを持ち、他は空 (issue 0058)。
#[test]
fn the_prefill_of_a_bare_purchase_has_only_the_name() {
    assert_eq!(
        purchase_prefill(&purchase("p1")),
        PurchasePrefill {
            name: "豆".to_string(),
            ..PurchasePrefill::default()
        }
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
