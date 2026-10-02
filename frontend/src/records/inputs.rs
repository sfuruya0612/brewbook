//! 記録の登録と更新の入力 (FR-6 から FR-13)。
//!
//! 画面が組み立てた値を API の本文の形に直す。更新では画面が持っている全ての項目を送り、
//! 空にした任意の項目は `null` として送る (API は項目が無いときは変更しない。FR-9)。
//! Flutter の `frontend/lib/api/record_inputs.dart` と同じ形にする。

use serde_json::{json, Value};

/// 店の登録と更新の入力 (FR-6)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ShopInput {
    /// 店名。必須。
    pub name: String,
    /// 住所。空にしたときは None。
    pub address: Option<String>,
}

impl ShopInput {
    /// API の本文にする。
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "address": self.address,
        })
    }
}

/// 商品の登録と更新の入力 (FR-7、FR-8)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProductInput {
    /// 商品名。必須。
    pub name: String,
    /// 生産者。
    pub producer: Option<String>,
    /// 生産国。
    pub origin: Option<String>,
    /// 地域。
    pub region: Option<String>,
    /// 精製方法。
    pub process: Option<String>,
    /// 品種。
    pub variety: Option<String>,
    /// Flavor Notes のタグ名。更新ではこの配列で置き換える (FR-8)。
    pub flavor_notes: Vec<String>,
}

impl ProductInput {
    /// API の本文にする。タグは空の配列でも送り、全て外す操作を表せるようにする (FR-8)。
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "producer": self.producer,
            "origin": self.origin,
            "region": self.region,
            "process": self.process,
            "variety": self.variety,
            "flavor_notes": self.flavor_notes,
        })
    }
}

/// 購入の登録と更新の入力 (FR-9)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PurchaseInput {
    /// 商品の ID。必須。
    pub product_id: String,
    /// 店の ID。店を指定しないときは None。
    pub shop_id: Option<String>,
    /// 購入日 (`YYYY-MM-DD`)。必須。
    pub purchased_on: String,
    /// 焙煎度。
    pub roast: Option<String>,
    /// 焙煎日 (`YYYY-MM-DD`)。
    pub roast_date: Option<String>,
    /// 価格 (通貨の最小単位)。
    pub price_amount: Option<i64>,
    /// ISO 4217 の通貨コード。価格が無いときは None にする (API は組で扱う。FR-9)。
    pub price_currency: Option<String>,
    /// 重量 (グラム)。
    pub weight_grams: Option<i64>,
}

impl PurchaseInput {
    /// API の本文にする。
    ///
    /// 価格が無いときは通貨コードも null にする (価格だけを持つ組は API が 400 で拒否する)。
    pub fn to_json(&self) -> Value {
        json!({
            "product_id": self.product_id,
            "shop_id": self.shop_id,
            "purchased_on": self.purchased_on,
            "roast": self.roast,
            "roast_date": self.roast_date,
            "price_amount": self.price_amount,
            "price_currency": if self.price_amount.is_none() { None } else { self.price_currency.clone() },
            "weight_grams": self.weight_grams,
        })
    }
}

/// 抽出の登録と更新の入力 (FR-11)。
#[derive(Clone, PartialEq, Debug)]
pub struct BrewInput {
    /// 購入の ID。必須。
    pub purchase_id: String,
    /// 抽出日時 (ISO 8601 の UTC)。必須。
    pub brewed_at: String,
    /// 豆の量 (グラム)。
    pub dose_grams: Option<f64>,
    /// 湯量 (グラム)。
    pub water_grams: Option<f64>,
    /// 湯の温度 (摂氏)。
    pub water_temp_c: Option<f64>,
    /// 時間 (秒)。
    pub brew_time_seconds: Option<i64>,
    /// 抽出方法。
    pub method: Option<String>,
    /// 挽き目。
    pub grind_setting: Option<String>,
    /// 評価 (1 から 5)。
    pub rating: Option<u8>,
    /// 感想。
    pub notes: Option<String>,
}

impl BrewInput {
    /// API の本文にする。
    pub fn to_json(&self) -> Value {
        json!({
            "purchase_id": self.purchase_id,
            "brewed_at": self.brewed_at,
            "dose_grams": self.dose_grams,
            "water_grams": self.water_grams,
            "water_temp_c": self.water_temp_c,
            "brew_time_seconds": self.brew_time_seconds,
            "method": self.method,
            "grind_setting": self.grind_setting,
            "rating": self.rating,
            "notes": self.notes,
        })
    }
}
