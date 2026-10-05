//! 一覧のカーソルとページサイズ (ADR-0002、PRD の性能)。
//!
//! カーソルは並び順のキーの名前 (`sort` と同じ値)、方向 (`order` と同じ値)、キーの値
//! (NULL のときは null)、ID を JSON にして base64url で符号化した文字列とする。
//! キーの値の型はキーごとに決まり (日時と日付と名前は文字列、数値は数値)、
//! 復号できない値とキーの型に合わない値は 400 にする。
//! ページサイズは既定 50、最大 200 とし、200 を超える指定は 400 にする。

use serde::{Deserialize, Serialize};

use crate::datetime;
use crate::error::ErrorCode;

/// ページサイズの既定値。
pub const DEFAULT_PAGE_SIZE: u32 = 50;
/// ページサイズの上限。
pub const MAX_PAGE_SIZE: u32 = 200;

/// ページサイズの指定の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSizeError {
    /// 整数として解釈できない (小数、文字列、i64 に収まらない値)。
    NotAnInteger,
    /// 0 以下。
    NotPositive,
    /// 上限 (200) を超える。
    TooLarge,
}

impl PageSizeError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        match self {
            PageSizeError::NotAnInteger => "limit must be an integer",
            PageSizeError::NotPositive => "limit must be greater than 0",
            PageSizeError::TooLarge => "limit must be 200 or less",
        }
    }
}

/// `limit` のクエリパラメータを検証する。省略時は既定値 (50) を返す。
pub fn parse_page_size(limit: Option<&str>) -> Result<u32, PageSizeError> {
    let Some(text) = limit else {
        return Ok(DEFAULT_PAGE_SIZE);
    };
    let value: i64 = text.parse().map_err(|_| PageSizeError::NotAnInteger)?;
    if value <= 0 {
        return Err(PageSizeError::NotPositive);
    }
    if value > i64::from(MAX_PAGE_SIZE) {
        return Err(PageSizeError::TooLarge);
    }
    Ok(value as u32)
}

/// 一覧の並び順のキー (FR-20)。
///
/// 一覧ごとに受け付けるキーは [`crate::query`] の定数が定め、それ以外は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// 抽出日時 (ISO 8601 UTC)。
    BrewedAt,
    /// 評価 (1 から 5)。
    Rating,
    /// 豆の量 (グラム)。
    DoseGrams,
    /// 購入日 (`YYYY-MM-DD`)。
    PurchasedOn,
    /// 価格 (通貨の最小単位)。
    PriceAmount,
    /// 重量 (グラム)。
    WeightGrams,
    /// 作成日時 (ISO 8601 UTC)。
    CreatedAt,
    /// 名前 (大文字と小文字を区別しない)。
    Name,
    /// 更新日時 (ISO 8601 UTC)。
    UpdatedAt,
}

/// 並び順のキーの値の型。カーソルの復号の検証に使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortValueKind {
    /// 日時 (ISO 8601 UTC)。値は文字列。
    Timestamp,
    /// 日付 (`YYYY-MM-DD`)。値は文字列。
    Date,
    /// 名前。値は文字列。
    Text,
    /// 整数。値は整数の数値。
    Integer,
    /// 小数。値は数値。
    Real,
}

impl SortKey {
    /// クエリパラメータとカーソルで使う名前。
    pub fn as_str(self) -> &'static str {
        match self {
            SortKey::BrewedAt => "brewed_at",
            SortKey::Rating => "rating",
            SortKey::DoseGrams => "dose_grams",
            SortKey::PurchasedOn => "purchased_on",
            SortKey::PriceAmount => "price_amount",
            SortKey::WeightGrams => "weight_grams",
            SortKey::CreatedAt => "created_at",
            SortKey::Name => "name",
            SortKey::UpdatedAt => "updated_at",
        }
    }

    /// 並び順のキーの値の型。
    pub fn value_kind(self) -> SortValueKind {
        match self {
            SortKey::BrewedAt | SortKey::CreatedAt | SortKey::UpdatedAt => SortValueKind::Timestamp,
            SortKey::PurchasedOn => SortValueKind::Date,
            SortKey::Name => SortValueKind::Text,
            SortKey::Rating | SortKey::PriceAmount | SortKey::WeightGrams => SortValueKind::Integer,
            SortKey::DoseGrams => SortValueKind::Real,
        }
    }
}

/// 並び順のキーの名前の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKeyError {
    /// 一覧が受け付けるキーのどれでもない。
    Unknown,
}

impl SortKeyError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "invalid sort"
    }
}

/// 受け付けるキーの並びから `sort` の値を解釈する。それ以外は拒否する (FR-20)。
pub fn parse_sort_key(name: &str, allowed: &[SortKey]) -> Result<SortKey, SortKeyError> {
    allowed
        .iter()
        .copied()
        .find(|key| key.as_str() == name)
        .ok_or(SortKeyError::Unknown)
}

/// 並び順の方向 (FR-20)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// 降順 (既定)。
    #[default]
    Desc,
    /// 昇順。
    Asc,
}

impl SortOrder {
    /// クエリパラメータとカーソルで使う名前。
    pub fn as_str(self) -> &'static str {
        match self {
            SortOrder::Desc => "desc",
            SortOrder::Asc => "asc",
        }
    }

    /// 続きの条件で使う比較の向き。降順のときはキーより小さい行を引く。
    pub fn comparison(self) -> &'static str {
        match self {
            SortOrder::Desc => "<",
            SortOrder::Asc => ">",
        }
    }

    /// `asc` または `desc` を解釈する。それ以外は拒否する (FR-20)。
    pub fn parse(name: &str) -> Result<Self, SortOrderError> {
        match name {
            "asc" => Ok(SortOrder::Asc),
            "desc" => Ok(SortOrder::Desc),
            _ => Err(SortOrderError::Unknown),
        }
    }
}

/// 並び順の方向の指定の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrderError {
    /// `asc` でも `desc` でもない。
    Unknown,
}

impl SortOrderError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "invalid order"
    }
}

/// カーソルが運ぶ並び順のキーの値。
#[derive(Debug, Clone, PartialEq)]
pub enum CursorValue {
    /// 日時、日付、名前の文字列。
    Text(String),
    /// 整数のキーの値。
    Integer(i64),
    /// 小数のキーの値 (豆の量)。
    Real(f64),
}

/// カーソルが運ぶ並び順のキー。キーの名前、方向、キーの値 (NULL のときは None)、ID を持つ。
#[derive(Debug, Clone, PartialEq)]
pub struct CursorKey {
    /// 並び順のキー。`sort` と同じ値。
    pub sort: SortKey,
    /// 並び順の方向。`order` と同じ値。
    pub order: SortOrder,
    /// キーの値。NULL の行を指すときは None。
    pub value: Option<CursorValue>,
    /// キーと組になる ID。
    pub id: String,
}

impl CursorKey {
    /// 並び順のキーと組になる ID。
    pub fn id(&self) -> &str {
        &self.id
    }

    /// base64url (パディング無し) のカーソルに符号化する。
    pub fn encode(&self) -> String {
        let value = self.value.as_ref().map(|value| match value {
            CursorValue::Text(text) => serde_json::Value::String(text.clone()),
            CursorValue::Integer(number) => serde_json::Value::Number((*number).into()),
            CursorValue::Real(number) => serde_json::Number::from_f64(*number)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
        });
        let json = CursorJson {
            sort: self.sort.as_str().to_owned(),
            order: self.order.as_str().to_owned(),
            value,
            id: self.id.clone(),
        };
        let text =
            serde_json::to_string(&json).expect("a cursor of simple fields is always serializable");
        crate::base64url::encode(text.as_bytes())
    }

    /// base64url のカーソルを復号する。復号できない値と、キーと値の型が合わない値は拒否する。
    ///
    /// 復号は `crate::base64url` に任せる。非正準の符号化 (余ったビットが 0 でない値) は
    /// 復号できるが、その結果が JSON とキーの形にならなければ拒否される。
    pub fn decode(text: &str) -> Result<Self, CursorError> {
        let bytes = crate::base64url::decode(text).map_err(|_| CursorError::Invalid)?;
        let json: CursorJson = serde_json::from_slice(&bytes).map_err(|_| CursorError::Invalid)?;
        if json.id.is_empty() {
            return Err(CursorError::Invalid);
        }
        let sort = parse_sort_key(&json.sort, ALL_SORT_KEYS).map_err(|_| CursorError::Invalid)?;
        let order = SortOrder::parse(&json.order).map_err(|_| CursorError::Invalid)?;
        let value = decode_value(sort, json.value)?;
        Ok(CursorKey {
            sort,
            order,
            value,
            id: json.id,
        })
    }
}

/// カーソルが運ぶ全ての並び順のキー。復号は一覧ごとの制限を知らないため、全種類を受け付ける
/// (一覧が受け付けるキーかの判定はクエリの組み立てが `sort` と突き合わせて行う)。
const ALL_SORT_KEYS: &[SortKey] = &[
    SortKey::BrewedAt,
    SortKey::Rating,
    SortKey::DoseGrams,
    SortKey::PurchasedOn,
    SortKey::PriceAmount,
    SortKey::WeightGrams,
    SortKey::CreatedAt,
    SortKey::Name,
    SortKey::UpdatedAt,
];

/// カーソルのキーの値を、キーの型に合わせて検証して取り出す。
fn decode_value(
    sort: SortKey,
    value: Option<serde_json::Value>,
) -> Result<Option<CursorValue>, CursorError> {
    let Some(value) = value else {
        return Ok(None);
    };
    match (sort.value_kind(), value) {
        (SortValueKind::Timestamp, serde_json::Value::String(text)) => {
            if datetime::parse_epoch_millis(&text).is_ok() {
                Ok(Some(CursorValue::Text(text)))
            } else {
                Err(CursorError::Invalid)
            }
        }
        (SortValueKind::Date, serde_json::Value::String(text)) => {
            if datetime::is_valid_date(&text) {
                Ok(Some(CursorValue::Text(text)))
            } else {
                Err(CursorError::Invalid)
            }
        }
        (SortValueKind::Text, serde_json::Value::String(text)) => Ok(Some(CursorValue::Text(text))),
        (SortValueKind::Integer, serde_json::Value::Number(number)) => number
            .as_i64()
            .map(|value| Some(CursorValue::Integer(value)))
            .ok_or(CursorError::Invalid),
        (SortValueKind::Real, serde_json::Value::Number(number)) => number
            .as_f64()
            .map(|value| Some(CursorValue::Real(value)))
            .ok_or(CursorError::Invalid),
        _ => Err(CursorError::Invalid),
    }
}

/// カーソルの復号の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorError {
    /// base64url として復号できないか、JSON とキーの値が想定の形でない。
    Invalid,
}

impl CursorError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "invalid cursor"
    }
}

/// カーソルの JSON 表現。キーの名前、方向、キーの値 (null を取り得る)、ID を持つ。
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CursorJson {
    sort: String,
    order: String,
    value: Option<serde_json::Value>,
    id: String,
}
