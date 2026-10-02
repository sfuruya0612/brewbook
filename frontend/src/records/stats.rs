//! 統計と評価の推移の API (FR-18) の呼び出し。
//!
//! 集計は Backend の集計クエリが行い、このモジュールは期間と粒度と UTC オフセットを渡して
//! 集計結果を受け取るだけにする (ADR-0007)。Flutter の `frontend/lib/api/stats_api.dart` と
//! 同じ経路を呼び、応答の形の違反は [`RecordError::Format`] にする。

use serde_json::{Map, Value};

use crate::api::ApiClient;

use super::api::encode_query;
use super::models::{
    count_field, format_error, items_field, optional_f64, optional_i64, optional_rating,
    optional_string, string_field,
};
use super::stats_period::StatsGranularity;
use super::RecordError;

/// 抽出回数と豆の消費量の 1 区間 (FR-18)。
#[derive(Clone, PartialEq, Debug)]
pub struct BrewPeriod {
    /// 区間のキー (日別は `YYYY-MM-DD`、月別は `YYYY-MM`)。
    pub period: String,

    /// 区間内の抽出の件数。
    pub brew_count: u64,

    /// 区間内の豆の量の合計 (グラム)。
    pub dose_grams: f64,
}

impl BrewPeriod {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        Ok(Self {
            period: string_field(json, "period")?,
            brew_count: count_field(json, "brew_count")?,
            dose_grams: optional_f64(json, "dose_grams")?
                .ok_or_else(|| format_error("the dose_grams field must be a number".to_string()))?,
        })
    }
}

/// 購入金額と重量の 1 区間と通貨コードの組 (FR-18)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PurchasePeriod {
    /// 区間のキー (日別は `YYYY-MM-DD`、月別は `YYYY-MM`)。
    pub period: String,

    /// ISO 4217 の通貨コード。価格が無い購入は None。
    pub price_currency: Option<String>,

    /// 区間内の価格の合計 (通貨の最小単位)。
    pub price_amount: i64,

    /// 区間内の重量の合計 (グラム)。
    pub weight_grams: i64,

    /// 区間内の購入の件数。
    pub purchase_count: u64,
}

impl PurchasePeriod {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        Ok(Self {
            period: string_field(json, "period")?,
            price_currency: optional_string(json, "price_currency")?,
            price_amount: optional_i64(json, "price_amount")?.ok_or_else(|| {
                format_error("the price_amount field must be a number".to_string())
            })?,
            weight_grams: optional_i64(json, "weight_grams")?.ok_or_else(|| {
                format_error("the weight_grams field must be a number".to_string())
            })?,
            purchase_count: count_field(json, "purchase_count")?,
        })
    }
}

/// 抽出条件と評価の関係の 1 件の抽出 (FR-18)。
#[derive(Clone, PartialEq, Debug)]
pub struct BrewRating {
    /// 抽出の ID。
    pub id: String,

    /// 豆の量 (グラム)。条件が無ければ None。
    pub dose_grams: Option<f64>,

    /// 湯量 (グラム)。条件が無ければ None。
    pub water_grams: Option<f64>,

    /// 湯の温度 (摂氏)。条件が無ければ None。
    pub water_temp_c: Option<f64>,

    /// 時間 (秒)。条件が無ければ None。
    pub brew_time_seconds: Option<i64>,

    /// 評価 (1 から 5)。
    pub rating: u8,
}

impl BrewRating {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        Ok(Self {
            id: string_field(json, "id")?,
            dose_grams: optional_f64(json, "dose_grams")?,
            water_grams: optional_f64(json, "water_grams")?,
            water_temp_c: optional_f64(json, "water_temp_c")?,
            brew_time_seconds: optional_i64(json, "brew_time_seconds")?,
            rating: optional_rating(json, "rating")?.ok_or_else(|| {
                format_error("the rating field must be between 1 and 5".to_string())
            })?,
        })
    }
}

/// 購入ごとの評価の推移の 1 件の抽出 (FR-18)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RatingHistoryEntry {
    /// 抽出の ID。
    pub id: String,

    /// 抽出日時 (ISO 8601 の UTC)。
    pub brewed_at: String,

    /// 評価 (1 から 5)。
    pub rating: u8,
}

impl RatingHistoryEntry {
    /// JSON のオブジェクトから組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, RecordError> {
        Ok(Self {
            id: string_field(json, "id")?,
            brewed_at: string_field(json, "brewed_at")?,
            rating: optional_rating(json, "rating")?.ok_or_else(|| {
                format_error("the rating field must be between 1 and 5".to_string())
            })?,
        })
    }
}

/// 統計の API を [`ApiClient`] で呼ぶ (FR-18)。
#[derive(Clone)]
pub struct StatsApi {
    api: ApiClient,
}

impl PartialEq for StatsApi {
    /// 同じ API クライアントを指しているときだけ等しいとみなす。
    fn eq(&self, other: &Self) -> bool {
        self.api == other.api
    }
}

impl StatsApi {
    /// API クライアントから作る。
    pub fn new(api: ApiClient) -> Self {
        Self { api }
    }

    /// 元の API クライアント。
    pub fn client(&self) -> &ApiClient {
        &self.api
    }

    /// 抽出回数と豆の消費量を引く (FR-18)。
    ///
    /// `start` と `end` は端末のタイムゾーンでの日付。全期間では両方を省略する。
    /// `utc_offset_minutes` は端末のローカル時刻から UTC を引いた分数 (日本標準時は +540)。
    pub async fn brews(
        &self,
        start: Option<&str>,
        end: Option<&str>,
        granularity: StatsGranularity,
        utc_offset_minutes: i32,
    ) -> Result<Vec<BrewPeriod>, RecordError> {
        let json = self
            .api
            .get_json(&stats_path(
                "/stats/brews",
                start,
                end,
                Some(granularity),
                Some(utc_offset_minutes),
            ))
            .await?;
        items_field(&json, "brews", BrewPeriod::from_json)
    }

    /// 購入金額と重量を引く (FR-18)。
    ///
    /// 購入日はタイムゾーンを持たないため、UTC オフセットは渡さない。
    pub async fn purchases(
        &self,
        start: Option<&str>,
        end: Option<&str>,
        granularity: StatsGranularity,
    ) -> Result<Vec<PurchasePeriod>, RecordError> {
        let json = self
            .api
            .get_json(&stats_path(
                "/stats/purchases",
                start,
                end,
                Some(granularity),
                None,
            ))
            .await?;
        items_field(&json, "purchases", PurchasePeriod::from_json)
    }

    /// 抽出条件と評価の関係を引く (FR-18)。
    ///
    /// 散布図は粒度を持たないため、期間と UTC オフセットだけを渡す。
    pub async fn brew_ratings(
        &self,
        start: Option<&str>,
        end: Option<&str>,
        utc_offset_minutes: i32,
    ) -> Result<Vec<BrewRating>, RecordError> {
        let json = self
            .api
            .get_json(&stats_path(
                "/stats/brew-ratings",
                start,
                end,
                None,
                Some(utc_offset_minutes),
            ))
            .await?;
        items_field(&json, "brew_ratings", BrewRating::from_json)
    }

    /// 購入ごとの評価の推移を引く (FR-18)。期間で絞らず、アーカイブ済みの購入も指定できる。
    pub async fn rating_history(
        &self,
        purchase_id: &str,
    ) -> Result<Vec<RatingHistoryEntry>, RecordError> {
        let json = self
            .api
            .get_json(&format!("/purchases/{purchase_id}/rating-history"))
            .await?;
        items_field(&json, "ratings", RatingHistoryEntry::from_json)
    }
}

/// 統計の経路に、期間と粒度と UTC オフセットの指定を付ける (FR-18)。
///
/// 省略したパラメータは付けない (全期間は開始日と終了日の両方を省略する)。並びは Flutter の
/// `StatsApi._statsPath` と同じく、粒度、開始日、終了日、UTC オフセットの順にする。
pub fn stats_path(
    path: &str,
    start: Option<&str>,
    end: Option<&str>,
    granularity: Option<StatsGranularity>,
    utc_offset_minutes: Option<i32>,
) -> String {
    let mut query = String::new();
    let mut push = |name: &str, value: &str| {
        query.push(if query.is_empty() { '?' } else { '&' });
        query.push_str(name);
        query.push('=');
        query.push_str(&encode_query(value));
    };
    if let Some(granularity) = granularity {
        push("granularity", granularity.api_value());
    }
    if let Some(start) = start {
        push("start", start);
    }
    if let Some(end) = end {
        push("end", end);
    }
    if let Some(offset) = utc_offset_minutes {
        push("utc_offset_minutes", &offset.to_string());
    }
    format!("{path}{query}")
}
