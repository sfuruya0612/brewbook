//! フォームの入力の検証と、推測の適用 (FR-9、FR-11、FR-19)。
//!
//! 画面の signal から渡された文字列を読み、API の入力 ([`super::inputs`]) を組み立てる。
//! 推測 (FR-19) は推測できた項目を入力欄に入れ、入力済みの値も置き換える (ADR-0022)。
//! Dioxus に依存しない純粋なモジュールにして、native の単体テストと PBT で守る (ADR-0013)。
//! Flutter の `frontend/lib/screens/*_form_screen.dart` の判定と同じにする。

use crate::i18n::Key;

use super::inputs::{BrewInput, PurchaseInput, ShopInput};
use super::models::{Purchase, PurchaseSuggestion, Shop};
use super::values::{
    format_day, parse_count, parse_day, parse_decimal, parse_time, to_utc_iso8601, LocalDateTime,
};

/// 保存が新規か更新かの判断 (0041 のレビューの指摘で切り出した)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SaveTarget {
    /// 新規の登録。
    Create,

    /// 既存の更新。対象の ID。
    Update(String),
}

/// ID の有無から保存の対象を決める (画面はこれで API の呼び出しを選ぶ)。
pub fn save_target(id: Option<&str>) -> SaveTarget {
    match id {
        Some(id) => SaveTarget::Update(id.to_string()),
        None => SaveTarget::Create,
    }
}

/// 空の入力を API の null にする。
pub fn optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// 店のフォームの入力 (FR-6)。店名は必須。
pub fn validate_shop_form(name: &str, address: &str) -> Result<ShopInput, Key> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Key::ValidationRequired);
    }
    Ok(ShopInput {
        name: name.to_string(),
        address: optional_text(address),
    })
}

/// 購入のフォームの入力。商品の項目も購入のフォームが持つ (ADR-0022)。
pub struct PurchaseFormValues<'a> {
    /// 商品名の入力。必須 (FR-9)。
    pub name: &'a str,
    /// 生産者の入力。
    pub producer: &'a str,
    /// 生産国の入力。
    pub origin: &'a str,
    /// 地域の入力。
    pub region: &'a str,
    /// 精製方法の入力。
    pub process: &'a str,
    /// 品種の入力。
    pub variety: &'a str,
    /// Flavor Notes のタグ (FR-8)。
    pub flavor_notes: &'a [String],
    /// 選択中の店。任意 (FR-9)。
    pub shop: Option<&'a Shop>,
    /// 購入日の入力 (`YYYY-MM-DD`)。必須 (FR-9)。
    pub purchased_on: &'a str,
    /// 焙煎度の入力。
    pub roast: &'a str,
    /// 焙煎日の入力 (`YYYY-MM-DD`)。
    pub roast_date: &'a str,
    /// 価格の入力。
    pub price: &'a str,
    /// 選択中の通貨コード (FR-9)。
    pub currency: &'a str,
    /// 重量の入力。
    pub weight: &'a str,
}

/// 購入のフォームの検証の誤り。None の項目は誤りが無い。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PurchaseFormErrors {
    /// 商品名の誤り。
    pub name: Option<Key>,
    /// 購入日の誤り。
    pub purchased_on: Option<Key>,
    /// 焙煎日の誤り。
    pub roast_date: Option<Key>,
    /// 価格の誤り。
    pub price: Option<Key>,
    /// 重量の誤り。
    pub weight: Option<Key>,
}

impl PurchaseFormErrors {
    /// 誤りが 1 つでもあるか (画面の上のバナーを出す判定)。
    pub fn has_errors(&self) -> bool {
        self.name.is_some()
            || self.purchased_on.is_some()
            || self.roast_date.is_some()
            || self.price.is_some()
            || self.weight.is_some()
    }
}

/// 購入のフォームの入力を検証し、API の入力にする (FR-9)。
///
/// 商品名と購入日は必須。焙煎日、価格、重量は空なら None にし、値があるときは形式を検査する。
/// タグは前後の空白を除き、空のタグを落とし、同じ名前を 1 つにまとめる (FR-8)。
pub fn validate_purchase_form(
    values: PurchaseFormValues<'_>,
) -> Result<PurchaseInput, PurchaseFormErrors> {
    let name = values.name.trim();
    let purchased_on = parse_day(values.purchased_on);
    let roast_date_text = values.roast_date.trim();
    let roast_date = if roast_date_text.is_empty() {
        None
    } else {
        parse_day(roast_date_text)
    };
    let price_text = values.price.trim();
    let price = if price_text.is_empty() {
        None
    } else {
        parse_count(price_text).and_then(|value| i64::try_from(value).ok())
    };
    let weight_text = values.weight.trim();
    let weight = if weight_text.is_empty() {
        None
    } else {
        parse_count(weight_text).and_then(|value| i64::try_from(value).ok())
    };

    let errors = PurchaseFormErrors {
        name: name.is_empty().then_some(Key::ValidationRequired),
        purchased_on: purchased_on.is_none().then_some(Key::ValidationDay),
        roast_date: if roast_date_text.is_empty() || roast_date.is_some() {
            None
        } else {
            Some(Key::ValidationDay)
        },
        price: if price_text.is_empty() || price.is_some() {
            None
        } else {
            Some(Key::ValidationNumber)
        },
        weight: if weight_text.is_empty() || weight.is_some() {
            None
        } else {
            Some(Key::ValidationNumber)
        },
    };
    if errors.has_errors() {
        return Err(errors);
    }
    let Some(purchased_on) = purchased_on else {
        return Err(errors);
    };
    Ok(PurchaseInput {
        name: name.to_string(),
        producer: optional_text(values.producer),
        origin: optional_text(values.origin),
        region: optional_text(values.region),
        process: optional_text(values.process),
        variety: optional_text(values.variety),
        flavor_notes: dedup_tags(values.flavor_notes),
        shop_id: values.shop.map(|shop| shop.id.clone()),
        purchased_on: format_day(purchased_on),
        roast: optional_text(values.roast),
        roast_date: roast_date.map(format_day),
        price_amount: price,
        // 価格が無いときは通貨コードも送らない (API は組で扱う。FR-9)。
        price_currency: price.map(|_| values.currency.to_string()),
        weight_grams: weight,
    })
}

/// タグの前後の空白を除き、空のタグを落とし、同じ名前を 1 つにまとめる (FR-8)。
pub fn dedup_tags(flavor_notes: &[String]) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for note in flavor_notes {
        let note = note.trim();
        if note.is_empty() || tags.iter().any(|tag| tag == note) {
            continue;
        }
        tags.push(note.to_string());
    }
    tags
}

/// 推測を入れる購入のフォームの入力欄 (FR-19)。
pub struct PurchaseFormFields<'a> {
    /// 商品名。
    pub name: &'a mut String,
    /// 生産者。
    pub producer: &'a mut String,
    /// 生産国。
    pub origin: &'a mut String,
    /// 地域。
    pub region: &'a mut String,
    /// 精製方法。
    pub process: &'a mut String,
    /// 品種。
    pub variety: &'a mut String,
    /// Flavor Notes のタグ。
    pub flavor_notes: &'a mut Vec<String>,
    /// 焙煎度。
    pub roast: &'a mut String,
    /// 焙煎日。
    pub roast_date: &'a mut String,
    /// 価格。
    pub price: &'a mut String,
    /// 重量。
    pub weight: &'a mut String,
}

/// 推測できた項目を入力欄に入れる (FR-19、ADR-0022)。
///
/// 推測できた項目 (None でない項目) は入力済みの値も置き換える (写真の読み取りで入力を
/// やり直す使い方のため。0057)。推測できなかった項目 (None) と空の Flavor Notes は入力欄を
/// そのまま残す。通貨は推測しない (画面の既定値のまま。FR-19)。
pub fn apply_purchase_suggestion(fields: PurchaseFormFields<'_>, suggestion: &PurchaseSuggestion) {
    fn put(field: &mut String, value: &Option<String>) {
        if let Some(value) = value {
            *field = value.clone();
        }
    }
    put(fields.name, &suggestion.name);
    put(fields.producer, &suggestion.producer);
    put(fields.origin, &suggestion.origin);
    put(fields.region, &suggestion.region);
    put(fields.process, &suggestion.process);
    put(fields.variety, &suggestion.variety);
    if !suggestion.flavor_notes.is_empty() {
        *fields.flavor_notes = dedup_tags(&suggestion.flavor_notes);
    }
    put(fields.roast, &suggestion.roast);
    put(fields.roast_date, &suggestion.roast_date);
    if let Some(value) = suggestion.price_amount {
        *fields.price = value.to_string();
    }
    if let Some(value) = suggestion.weight_grams {
        *fields.weight = value.to_string();
    }
}

/// 過去の購入から新規のフォームへ引き継ぐ値 (FR-9、issue 0058)。
///
/// 商品の項目 (商品名、生産者、生産国、地域、精製方法、品種、Flavor Notes) と、
/// 店と焙煎度を引き継ぐ。購入日 (当日にする)、焙煎日、写真、価格、重量、通貨は引き継がない
/// (購入ごとに変わる値のため。issue 0058 の設計判断)。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PurchasePrefill {
    /// 商品名。
    pub name: String,
    /// 生産者。
    pub producer: String,
    /// 生産国。
    pub origin: String,
    /// 地域。
    pub region: String,
    /// 精製方法。
    pub process: String,
    /// 品種。
    pub variety: String,
    /// Flavor Notes のタグ (重複を除いたもの)。
    pub flavor_notes: Vec<String>,
    /// 店。
    pub shop: Option<Shop>,
    /// 焙煎度。
    pub roast: String,
}

/// 過去の購入から引き継ぐ値を組む (issue 0058)。
///
/// None の項目は空の入力にする。Flavor Notes は [`dedup_tags`] で整える (保存時の検証と
/// 同じ形にするため)。
pub fn purchase_prefill(purchase: &Purchase) -> PurchasePrefill {
    PurchasePrefill {
        name: purchase.name.clone(),
        producer: purchase.producer.clone().unwrap_or_default(),
        origin: purchase.origin.clone().unwrap_or_default(),
        region: purchase.region.clone().unwrap_or_default(),
        process: purchase.process.clone().unwrap_or_default(),
        variety: purchase.variety.clone().unwrap_or_default(),
        flavor_notes: dedup_tags(&purchase.flavor_notes),
        shop: purchase.shop.clone(),
        roast: purchase.roast.clone().unwrap_or_default(),
    }
}

/// 抽出のフォームの入力。
pub struct BrewFormValues<'a> {
    /// 選択中の購入。必須 (FR-11)。
    pub purchase: Option<&'a Purchase>,
    /// 抽出日の入力 (`YYYY-MM-DD`)。必須 (FR-11)。
    pub date: &'a str,
    /// 抽出時刻の入力 (`HH:MM`)。必須 (FR-11)。
    pub time: &'a str,
    /// 豆の量の入力。
    pub dose: &'a str,
    /// 湯量の入力。
    pub water: &'a str,
    /// 湯の温度の入力。
    pub water_temp: &'a str,
    /// 時間の入力。
    pub brew_time: &'a str,
    /// 抽出方法の入力。
    pub method: &'a str,
    /// 挽き目の入力。
    pub grind_setting: &'a str,
    /// 評価 (1 から 5)。
    pub rating: Option<u8>,
    /// 感想の入力。
    pub notes: &'a str,
    /// 端末のローカル時刻から UTC を引いた分数 (FR-18)。
    pub utc_offset_minutes: i32,
}

/// 抽出のフォームの検証の誤り。None の項目は誤りが無い。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct BrewFormErrors {
    /// 購入の誤り。
    pub purchase: Option<Key>,
    /// 抽出日の誤り。
    pub date: Option<Key>,
    /// 抽出時刻の誤り。
    pub time: Option<Key>,
    /// 豆の量の誤り。
    pub dose: Option<Key>,
    /// 湯量の誤り。
    pub water: Option<Key>,
    /// 湯の温度の誤り。
    pub water_temp: Option<Key>,
    /// 時間の誤り。
    pub brew_time: Option<Key>,
}

impl BrewFormErrors {
    /// 誤りが 1 つでもあるか (画面の上のバナーを出す判定)。
    pub fn has_errors(&self) -> bool {
        self.purchase.is_some()
            || self.date.is_some()
            || self.time.is_some()
            || self.dose.is_some()
            || self.water.is_some()
            || self.water_temp.is_some()
            || self.brew_time.is_some()
    }
}

/// 小数の入力を読む。空のときは None にし、読めないときは誤りにする (FR-11)。
fn decimal_input(value: &str) -> (Option<f64>, Option<Key>) {
    let text = value.trim();
    if text.is_empty() {
        return (None, None);
    }
    match parse_decimal(text) {
        Some(value) => (Some(value), None),
        None => (None, Some(Key::ValidationDecimal)),
    }
}

/// 整数の入力を読む。空のときは None にし、読めないときは誤りにする (FR-11)。
fn count_input(value: &str) -> (Option<i64>, Option<Key>) {
    let text = value.trim();
    if text.is_empty() {
        return (None, None);
    }
    match parse_count(text).and_then(|value| i64::try_from(value).ok()) {
        Some(value) => (Some(value), None),
        None => (None, Some(Key::ValidationNumber)),
    }
}

/// 抽出のフォームの入力を検証し、API の入力にする (FR-11)。
///
/// 購入と抽出日時は必須。抽出日時は端末のローカル時刻で入力し、送信の直前に UTC へ変換する。
pub fn validate_brew_form(values: BrewFormValues<'_>) -> Result<BrewInput, BrewFormErrors> {
    let purchase = values.purchase;
    let date = parse_day(values.date);
    let time = parse_time(values.time);
    let (dose, dose_error) = decimal_input(values.dose);
    let (water, water_error) = decimal_input(values.water);
    let (water_temp, water_temp_error) = decimal_input(values.water_temp);
    let (brew_time, brew_time_error) = count_input(values.brew_time);

    // 日付と時刻がそろっているときだけ UTC にする (片方でも欠ければ日付の誤りにする)。
    let brewed_at = match (date, time) {
        (Some(date), Some((hour, minute))) => to_utc_iso8601(
            LocalDateTime::new(date.year, date.month, date.day, hour, minute),
            values.utc_offset_minutes,
        ),
        _ => None,
    };
    let errors = BrewFormErrors {
        purchase: purchase.is_none().then_some(Key::ValidationPurchase),
        date: date.is_none().then_some(Key::ValidationDay),
        time: time.is_none().then_some(Key::ValidationTime),
        dose: dose_error,
        water: water_error,
        water_temp: water_temp_error,
        brew_time: brew_time_error,
    };
    if errors.has_errors() {
        return Err(errors);
    }
    let (Some(purchase), Some(brewed_at)) = (purchase, brewed_at) else {
        return Err(errors);
    };
    Ok(BrewInput {
        purchase_id: purchase.id.clone(),
        brewed_at,
        dose_grams: dose,
        water_grams: water,
        water_temp_c: water_temp,
        brew_time_seconds: brew_time,
        method: optional_text(values.method),
        grind_setting: optional_text(values.grind_setting),
        rating: values.rating,
        notes: optional_text(values.notes),
    })
}
