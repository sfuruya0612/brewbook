//! 一覧と詳細の表示に使う値の組み立て (FR-9、FR-11、FR-16、UC-6)。
//!
//! 行の 2 行目、詳細の参照先のタイル、詳細の見出しの値を、純粋な関数で組み立てる。画面の
//! `rsx!` はここで作った文字列を置くだけにし、表示の規則を native のテストで守れるようにする。

use crate::i18n::{text_args, Key, Language};

use super::models::{Brew, Product, Purchase, Shop};
use super::values::{display_day, display_timestamp, format_number, parse_day, parse_utc_to_local};

/// 抽出の一覧の行の 2 行目 (抽出日時と店。FR-11、FR-16)。
///
/// 抽出日時は API の UTC を端末のタイムゾーンに直して表示する。店が無い購入では店を付けない。
pub fn brew_row_subtitle(brew: &Brew, language: Language, utc_offset_minutes: i32) -> String {
    let date = parse_utc_to_local(&brew.brewed_at, utc_offset_minutes)
        .map(|datetime| display_timestamp(datetime, language))
        .unwrap_or_else(|| brew.brewed_at.clone());
    match brew.purchase.shop.as_ref() {
        Some(shop) => text_args(
            language,
            Key::BrewRowSubtitle,
            &[("date", &date), ("shop", &shop.name)],
        ),
        None => text_args(language, Key::BrewRowSubtitleNoShop, &[("date", &date)]),
    }
}

/// 購入の一覧の行の 2 行目 (購入日と店。FR-9、FR-16)。
pub fn purchase_row_subtitle(purchase: &Purchase, language: Language) -> String {
    let date = parse_day(&purchase.purchased_on)
        .map(|day| display_day(day, language))
        .unwrap_or_else(|| purchase.purchased_on.clone());
    match purchase.shop.as_ref() {
        Some(shop) => format!(
            "{date}{}{}",
            text_args(language, Key::RowSubtitleSeparator, &[]),
            shop.name
        ),
        None => date,
    }
}

/// 商品の一覧の行の 2 行目 (生産地、精製方法、品種)。1 つも無ければ None。
pub fn product_row_subtitle(product: &Product) -> Option<String> {
    let location = [product.region.as_deref(), product.origin.as_deref()]
        .into_iter()
        .flatten()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let mut parts = Vec::new();
    if !location.is_empty() {
        parts.push(location);
    }
    for value in [&product.process, &product.variety]
        .into_iter()
        .flatten()
        .filter(|value| !value.is_empty())
    {
        parts.push(value.clone());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" / "))
    }
}

/// 店の一覧の行の 2 行目 (住所)。住所が無いときは「未設定」の文言を返す (FR-6)。
pub fn shop_row_subtitle(shop: &Shop, language: Language) -> String {
    match shop.address.as_deref() {
        Some(address) if !address.is_empty() => address.to_string(),
        _ => text_args(language, Key::AddressUnset, &[]),
    }
}

/// 購入の一覧の行の右端の重量 (未設定のときは None)。
pub fn purchase_weight_text(purchase: &Purchase) -> Option<String> {
    purchase
        .weight_grams
        .map(|weight| format_number(weight as f64))
}

/// 購入の一覧の行の右端の価格 (未設定のときは None)。
pub fn purchase_price_text(purchase: &Purchase) -> Option<String> {
    purchase.price_amount.map(|amount| {
        format!(
            "{} {}",
            amount,
            purchase.price_currency.as_deref().unwrap_or("")
        )
    })
}

/// 抽出の詳細の「使った豆」の参照先 (購入、商品、店)。店が無い購入では店を省く (UC-6)。
pub fn brew_reference_tiles(brew: &Brew, language: Language) -> Vec<(Key, String)> {
    let purchase = &brew.purchase;
    let mut tiles = vec![(Key::PurchaseLabel, purchase_tile_name(purchase, language))];
    tiles.push((Key::ProductLabel, purchase.product.name.clone()));
    if let Some(shop) = purchase.shop.as_ref() {
        tiles.push((Key::ShopLabel, shop.name.clone()));
    }
    tiles
}

/// 購入の詳細の参照先 (商品、店)。店が無い購入では店を省く (UC-6)。
pub fn purchase_reference_tiles(purchase: &Purchase) -> Vec<(Key, String)> {
    let mut tiles = vec![(Key::ProductLabel, purchase.product.name.clone())];
    if let Some(shop) = purchase.shop.as_ref() {
        tiles.push((Key::ShopLabel, shop.name.clone()));
    }
    tiles
}

/// 購入のタイルの名前 (購入日 / 重量 / 価格)。商品名は別のタイルに出すので繰り返さない (UC-6)。
pub fn purchase_tile_name(purchase: &Purchase, language: Language) -> String {
    let mut parts = Vec::new();
    if let Some(day) = parse_day(&purchase.purchased_on) {
        parts.push(display_day(day, language));
    } else {
        parts.push(purchase.purchased_on.clone());
    }
    if let Some(weight) = purchase.weight_grams {
        parts.push(text_args(
            language,
            Key::GramsValue,
            &[("value", &format_number(weight as f64))],
        ));
    }
    if let Some(amount) = purchase.price_amount {
        parts.push(text_args(
            language,
            Key::PriceValue,
            &[
                ("amount", &amount.to_string()),
                ("currency", purchase.price_currency.as_deref().unwrap_or("")),
            ],
        ));
    }
    parts.join(" / ")
}

/// 評価の表示 (「4 / 5」。未評価のときは None)。
pub fn rating_text(rating: Option<u8>, language: Language) -> Option<String> {
    rating.map(|rating| {
        text_args(
            language,
            Key::RatingValue,
            &[("value", &rating.to_string())],
        )
    })
}

/// 小数の値を表示用にする。無いときは None (Ledger が「未設定」を出す)。
pub fn number_text(value: Option<f64>) -> Option<String> {
    value.map(format_number)
}

/// 整数の値を表示用にする。無いときは None。
pub fn count_text(value: Option<i64>) -> Option<String> {
    value.map(|value| value.to_string())
}
