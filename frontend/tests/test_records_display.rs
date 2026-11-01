//! `records::display` の単体テスト (0041)。
//!
//! 一覧の行の 2 行目、詳細の参照先のタイル、値の表示を確かめる (FR-11、UC-6。完了条件 3)。

use brew_book_frontend::i18n::{text, Key, Language};
use brew_book_frontend::records::display::{
    brew_reference_tiles, brew_row_subtitle, purchase_reference_tiles, purchase_row_subtitle,
    purchase_tile_name, shop_row_subtitle,
};
use brew_book_frontend::records::{Brew, Purchase, Shop};

/// 店を作る。
fn shop(id: &str, name: &str, address: Option<&str>) -> Shop {
    Shop {
        id: id.to_string(),
        name: name.to_string(),
        address: address.map(str::to_string),
        created_at: "2026-10-01T00:00:00.000Z".to_string(),
        updated_at: "2026-10-01T00:00:00.000Z".to_string(),
        favorited_at: None,
    }
}

/// 購入を作る (商品の項目も購入が持つ。ADR-0022)。
fn purchase(with_shop: bool) -> Purchase {
    Purchase {
        id: "b1".to_string(),
        shop_id: with_shop.then(|| "s1".to_string()),
        name: "豆".to_string(),
        producer: Some("生産者".to_string()),
        origin: Some("エチオピア".to_string()),
        region: Some("イルガチェフェ".to_string()),
        process: Some("ウォッシュド".to_string()),
        variety: Some("在来種".to_string()),
        flavor_notes: Vec::new(),
        taste_profile: None,
        purchased_on: "2026-10-01".to_string(),
        roast: None,
        roast_date: None,
        price_amount: Some(1200),
        price_currency: Some("JPY".to_string()),
        weight_grams: Some(200),
        photo_key: None,
        created_at: "2026-10-01T00:00:00.000Z".to_string(),
        updated_at: "2026-10-01T00:00:00.000Z".to_string(),
        favorited_at: None,
        shop: with_shop.then(|| shop("s1", "店", None)),
    }
}

/// 抽出を作る。
fn brew(with_shop: bool) -> Brew {
    Brew {
        id: "w1".to_string(),
        purchase_id: "b1".to_string(),
        brewed_at: "2026-10-01T00:30:00.000Z".to_string(),
        dose_grams: Some(15.0),
        water_grams: None,
        water_temp_c: None,
        brew_time_seconds: None,
        method: None,
        grind_setting: None,
        rating: Some(4),
        notes: None,
        created_at: "2026-10-01T00:00:00.000Z".to_string(),
        updated_at: "2026-10-01T00:00:00.000Z".to_string(),
        favorited_at: None,
        purchase: purchase(with_shop),
    }
}

#[test]
fn the_brew_row_subtitle_shows_the_local_time_and_the_shop() {
    // 日本標準時 (UTC+9) の 09:30 は UTC の 00:30。
    assert_eq!(
        brew_row_subtitle(&brew(true), Language::Japanese, 540),
        "2026/10/1 09:30 / 店"
    );
    assert_eq!(
        brew_row_subtitle(&brew(false), Language::Japanese, 540),
        "2026/10/1 09:30"
    );
    assert_eq!(
        brew_row_subtitle(&brew(true), Language::English, 540),
        "10/1/2026 09:30 / 店"
    );
}

#[test]
fn the_purchase_row_subtitle_shows_the_day_and_the_shop() {
    assert_eq!(
        purchase_row_subtitle(&purchase(true), Language::Japanese),
        "2026/10/1 / 店"
    );
    assert_eq!(
        purchase_row_subtitle(&purchase(false), Language::Japanese),
        "2026/10/1"
    );
}

#[test]
fn the_shop_row_subtitle_shows_the_address_or_the_unset_label() {
    assert_eq!(
        shop_row_subtitle(&shop("s1", "店", Some("住所")), Language::Japanese),
        "住所"
    );
    assert_eq!(
        shop_row_subtitle(&shop("s1", "店", None), Language::Japanese),
        text(Language::Japanese, Key::AddressUnset)
    );
}

#[test]
fn the_brew_reference_tiles_follow_the_purchase_and_the_shop() {
    let tiles = brew_reference_tiles(&brew(true), Language::Japanese);
    assert_eq!(tiles.len(), 2);
    assert_eq!(tiles[0].0, Key::PurchaseLabel);
    assert_eq!(tiles[0].1, "豆 / 2026/10/1 / 200 g / 1200 JPY");
    assert_eq!(tiles[1], (Key::ShopLabel, "店".to_string()));

    // 店が無い購入では店のタイルを省く (FR-9、UC-6)。
    let tiles = brew_reference_tiles(&brew(false), Language::Japanese);
    assert_eq!(tiles.len(), 1);
    assert_eq!(tiles[0].0, Key::PurchaseLabel);
}

#[test]
fn the_purchase_reference_tiles_follow_only_the_shop() {
    let tiles = purchase_reference_tiles(&purchase(true));
    assert_eq!(tiles, vec![(Key::ShopLabel, "店".to_string())]);
    // 店が無い購入では参照先が無い (商品は購入に統合した。ADR-0022)。
    assert!(purchase_reference_tiles(&purchase(false)).is_empty());
}

#[test]
fn the_purchase_tile_name_shows_the_name_the_day_the_weight_and_the_price() {
    assert_eq!(
        purchase_tile_name(&purchase(true), Language::Japanese),
        "豆 / 2026/10/1 / 200 g / 1200 JPY"
    );
    let mut bare = purchase(false);
    bare.weight_grams = None;
    bare.price_amount = None;
    bare.price_currency = None;
    assert_eq!(
        purchase_tile_name(&bare, Language::Japanese),
        "豆 / 2026/10/1"
    );
}
