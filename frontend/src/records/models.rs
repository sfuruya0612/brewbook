//! 記録の API の応答の型 (FR-6 から FR-13)。
//!
//! 項目名はスキーマの列名に対応させ、API の応答をそのまま表す。画面はこの型から表示を作る。
//! 日付と日時は API の形式の文字列のまま保持し、端末のタイムゾーンへの変換は表示の直前に行う
//! (Flutter の `frontend/lib/api/models.dart` と同じ)。応答の形の違反は [`RecordError::Format`]
//! にする (形式の違反で一覧を静かに打ち切らない)。
//!
//! serde を依存に加えず、[`ApiClient`](crate::api::ApiClient) が返す `serde_json` の値から
//! 手で組み立てる (依存を増やさない。ADR-0007)。

use serde_json::{Map, Value};

use super::RecordError;

/// 店 (FR-6)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Shop {
    /// ID。
    pub id: String,
    /// 店名。
    pub name: String,
    /// 住所。無いときは None。
    pub address: Option<String>,
    /// 作成日時。
    pub created_at: String,
    /// 更新日時。
    pub updated_at: String,
    /// アーカイブした日時。アーカイブ済みでなければ None (FR-12)。
    pub archived_at: Option<String>,
}

impl Shop {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        Ok(Self {
            id: string_field(json, "id")?,
            name: string_field(json, "name")?,
            address: optional_string(json, "address")?,
            created_at: string_field(json, "created_at")?,
            updated_at: string_field(json, "updated_at")?,
            archived_at: optional_string(json, "archived_at")?,
        })
    }

    /// アーカイブ済みか (FR-12)。
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

/// 商品 (FR-7、FR-8)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Product {
    /// ID。
    pub id: String,
    /// 商品名。
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
    /// Flavor Notes のタグ名 (名前の昇順。FR-8)。
    pub flavor_notes: Vec<String>,
    /// 作成日時。
    pub created_at: String,
    /// 更新日時。
    pub updated_at: String,
    /// アーカイブした日時。アーカイブ済みでなければ None (FR-12)。
    pub archived_at: Option<String>,
}

impl Product {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        Ok(Self {
            id: string_field(json, "id")?,
            name: string_field(json, "name")?,
            producer: optional_string(json, "producer")?,
            origin: optional_string(json, "origin")?,
            region: optional_string(json, "region")?,
            process: optional_string(json, "process")?,
            variety: optional_string(json, "variety")?,
            flavor_notes: string_list(json, "flavor_notes")?,
            created_at: string_field(json, "created_at")?,
            updated_at: string_field(json, "updated_at")?,
            archived_at: optional_string(json, "archived_at")?,
        })
    }

    /// アーカイブ済みか (FR-12)。
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

/// 購入 (FR-9、FR-10)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Purchase {
    /// ID。
    pub id: String,
    /// 商品の ID。
    pub product_id: String,
    /// 店の ID。店が無い購入では None。
    pub shop_id: Option<String>,
    /// 購入日 (`YYYY-MM-DD`)。
    pub purchased_on: String,
    /// 焙煎度。
    pub roast: Option<String>,
    /// 焙煎日 (`YYYY-MM-DD`)。
    pub roast_date: Option<String>,
    /// 価格 (通貨の最小単位)。
    pub price_amount: Option<i64>,
    /// ISO 4217 の通貨コード。
    pub price_currency: Option<String>,
    /// 重量 (グラム)。
    pub weight_grams: Option<i64>,
    /// 写真のオブジェクトキー。写真が無ければ None (FR-10)。
    pub photo_key: Option<String>,
    /// 作成日時。
    pub created_at: String,
    /// 更新日時。
    pub updated_at: String,
    /// アーカイブした日時。アーカイブ済みでなければ None (FR-12)。
    pub archived_at: Option<String>,
    /// 商品。必須の参照のため常にある (FR-9)。
    pub product: Product,
    /// 店。店が無い購入では None (FR-9)。
    pub shop: Option<Shop>,
}

impl Purchase {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        let shop = optional_object(json, "shop")?
            .map(|shop| Shop::from_json(&shop))
            .transpose()?;
        Ok(Self {
            id: string_field(json, "id")?,
            product_id: string_field(json, "product_id")?,
            shop_id: optional_string(json, "shop_id")?,
            purchased_on: string_field(json, "purchased_on")?,
            roast: optional_string(json, "roast")?,
            roast_date: optional_string(json, "roast_date")?,
            price_amount: optional_i64(json, "price_amount")?,
            price_currency: optional_string(json, "price_currency")?,
            weight_grams: optional_i64(json, "weight_grams")?,
            photo_key: optional_string(json, "photo_key")?,
            created_at: string_field(json, "created_at")?,
            updated_at: string_field(json, "updated_at")?,
            archived_at: optional_string(json, "archived_at")?,
            product: Product::from_json(&object_field(json, "product")?)?,
            shop,
        })
    }

    /// アーカイブ済みか (FR-12)。
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

/// 抽出 (FR-11)。
#[derive(Clone, PartialEq, Debug)]
pub struct Brew {
    /// ID。
    pub id: String,
    /// 購入の ID。
    pub purchase_id: String,
    /// 抽出日時 (ISO 8601 の UTC)。表示のときに端末のタイムゾーンへ変換する (FR-11)。
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
    /// 作成日時。
    pub created_at: String,
    /// 更新日時。
    pub updated_at: String,
    /// アーカイブした日時。アーカイブ済みでなければ None (FR-12)。
    pub archived_at: Option<String>,
    /// 購入。必須の参照のため常にある。中に商品と店を含む (FR-11)。
    pub purchase: Purchase,
}

impl Brew {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        Ok(Self {
            id: string_field(json, "id")?,
            purchase_id: string_field(json, "purchase_id")?,
            brewed_at: string_field(json, "brewed_at")?,
            dose_grams: optional_f64(json, "dose_grams")?,
            water_grams: optional_f64(json, "water_grams")?,
            water_temp_c: optional_f64(json, "water_temp_c")?,
            brew_time_seconds: optional_i64(json, "brew_time_seconds")?,
            method: optional_string(json, "method")?,
            grind_setting: optional_string(json, "grind_setting")?,
            rating: optional_rating(json, "rating")?,
            notes: optional_string(json, "notes")?,
            created_at: string_field(json, "created_at")?,
            updated_at: string_field(json, "updated_at")?,
            archived_at: optional_string(json, "archived_at")?,
            purchase: Purchase::from_json(&object_field(json, "purchase")?)?,
        })
    }

    /// アーカイブ済みか (FR-12)。
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

/// 写真から推測した商品の項目 (FR-19)。推測できない項目は None。
///
/// 商品の応答 ([`Product`]) とは違い、ID と日時を持たない候補である。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ProductSuggestion {
    /// 商品名の候補。
    pub name: Option<String>,
    /// 生産者の候補。
    pub producer: Option<String>,
    /// 生産国の候補。
    pub origin: Option<String>,
    /// 地域の候補。
    pub region: Option<String>,
    /// 精製方法の候補。
    pub process: Option<String>,
    /// 品種の候補。
    pub variety: Option<String>,
    /// Flavor Notes の候補 (FR-8)。
    pub flavor_notes: Vec<String>,
}

impl ProductSuggestion {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        Ok(Self {
            name: optional_string(json, "name")?,
            producer: optional_string(json, "producer")?,
            origin: optional_string(json, "origin")?,
            region: optional_string(json, "region")?,
            process: optional_string(json, "process")?,
            variety: optional_string(json, "variety")?,
            flavor_notes: string_list(json, "flavor_notes")?,
        })
    }
}

/// 写真から推測した購入と商品の項目 (FR-19)。キーは購入の応答の列名に揃える。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PurchaseSuggestion {
    /// 商品の項目。1 つも推測できないときは None。
    pub product: Option<ProductSuggestion>,
    /// 焙煎度の候補。
    pub roast: Option<String>,
    /// 焙煎日の候補 (`YYYY-MM-DD`)。
    pub roast_date: Option<String>,
    /// 価格の候補 (整数)。通貨は推測しない (FR-19)。
    pub price_amount: Option<i64>,
    /// 重量の候補 (グラム)。
    pub weight_grams: Option<i64>,
}

impl PurchaseSuggestion {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        let product = optional_object(json, "product")?
            .map(|product| ProductSuggestion::from_json(&product))
            .transpose()?;
        Ok(Self {
            product,
            roast: optional_string(json, "roast")?,
            roast_date: optional_string(json, "roast_date")?,
            price_amount: optional_i64(json, "price_amount")?,
            weight_grams: optional_i64(json, "weight_grams")?,
        })
    }
}

/// 一覧の 1 ページ (カーソル方式。ADR-0002)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RecordPage<T> {
    /// このページの行。
    pub items: Vec<T>,
    /// 続きを引くカーソル。続きが無ければ None。
    pub next_cursor: Option<String>,
}

/// 写真のアップロード用 URL の発行の応答 (FR-10)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PhotoUploadTarget {
    /// 署名付きの PUT の URL (R2 の S3 互換エンドポイント)。
    pub url: String,
    /// 発行されたオブジェクトキー。完了の通知でそのまま返す。
    pub key: String,
}

impl PhotoUploadTarget {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        Ok(Self {
            url: string_field(json, "url")?,
            key: string_field(json, "key")?,
        })
    }
}

/// 一覧の応答から行の配列を読む。
pub fn items_field<T>(
    json: &Map<String, Value>,
    key: &str,
    parse: impl Fn(&Map<String, Value>) -> Result<T, RecordError>,
) -> Result<Vec<T>, RecordError> {
    let value = json
        .get(key)
        .ok_or_else(|| format_error(format!("the {key} field must be an array")))?;
    let Value::Array(values) = value else {
        return Err(format_error(format!("the {key} field must be an array")));
    };
    values
        .iter()
        .map(|value| match value {
            Value::Object(object) => parse(object),
            _ => Err(format_error(format!("an item of {key} must be an object"))),
        })
        .collect()
}

/// 一覧の応答から続きのカーソルを読む。文字列でも null でもない値は形式の違反にする。
pub fn next_cursor_field(json: &Map<String, Value>) -> Result<Option<String>, RecordError> {
    match json.get("next_cursor") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format_error(
            "the next_cursor field must be a string or null",
        )),
    }
}

/// 形式の違反のエラーを作る。
pub fn format_error(message: impl Into<String>) -> RecordError {
    RecordError::Format(message.into())
}

/// 文字列の項目を読む。
fn string_field(json: &Map<String, Value>, key: &str) -> Result<String, RecordError> {
    match json.get(key) {
        Some(Value::String(value)) => Ok(value.clone()),
        _ => Err(format_error(format!("the {key} field must be a string"))),
    }
}

/// 文字列の項目を読む。無い場合と `null` は None にする。
fn optional_string(json: &Map<String, Value>, key: &str) -> Result<Option<String>, RecordError> {
    match json.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format_error(format!(
            "the {key} field must be a string or null"
        ))),
    }
}

/// 整数の項目を読む。無い場合と `null` は None にする。
fn optional_i64(json: &Map<String, Value>, key: &str) -> Result<Option<i64>, RecordError> {
    match json.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| format_error(format!("the {key} field must be an integer"))),
        Some(_) => Err(format_error(format!(
            "the {key} field must be a number or null"
        ))),
    }
}

/// 小数の項目を読む。無い場合と `null` は None にする。
fn optional_f64(json: &Map<String, Value>, key: &str) -> Result<Option<f64>, RecordError> {
    match json.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => value
            .as_f64()
            .map(Some)
            .ok_or_else(|| format_error(format!("the {key} field must be a number"))),
        Some(_) => Err(format_error(format!(
            "the {key} field must be a number or null"
        ))),
    }
}

/// 評価 (1 から 5) の項目を読む。
fn optional_rating(json: &Map<String, Value>, key: &str) -> Result<Option<u8>, RecordError> {
    let Some(value) = optional_i64(json, key)? else {
        return Ok(None);
    };
    u8::try_from(value)
        .ok()
        .filter(|value| (1..=5).contains(value))
        .map(Some)
        .ok_or_else(|| format_error(format!("the {key} field must be between 1 and 5")))
}

/// 文字列の配列の項目を読む。無い場合は空の配列にする。
fn string_list(json: &Map<String, Value>, key: &str) -> Result<Vec<String>, RecordError> {
    match json.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| match value {
                Value::String(value) => Ok(value.clone()),
                _ => Err(format_error(format!("an item of {key} must be a string"))),
            })
            .collect(),
        Some(_) => Err(format_error(format!(
            "the {key} field must be an array or null"
        ))),
    }
}

/// ネストしたオブジェクトの項目を読む。
fn object_field(json: &Map<String, Value>, key: &str) -> Result<Map<String, Value>, RecordError> {
    match json.get(key) {
        Some(Value::Object(value)) => Ok(value.clone()),
        _ => Err(format_error(format!("the {key} field must be an object"))),
    }
}

/// ネストしたオブジェクトの項目を読む。無い場合と `null` は None にする。
fn optional_object(
    json: &Map<String, Value>,
    key: &str,
) -> Result<Option<Map<String, Value>>, RecordError> {
    match json.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format_error(format!(
            "the {key} field must be an object or null"
        ))),
    }
}
