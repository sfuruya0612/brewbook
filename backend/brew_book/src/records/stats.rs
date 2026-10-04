//! 統計と評価の推移の API (FR-18)。
//!
//! 集計は D1 の集計クエリで行う (ADR-0006)。区間のキーは日別なら `YYYY-MM-DD`、月別なら
//! `YYYY-MM` とし、記録の無い区間は返さない。区間と通貨コードの組は昇順で返す。
//! 期間の端は端末のローカル時刻の日付として受け取り、抽出の API だけが UTC オフセット (分) を
//! 受け取る (購入日はタイムゾーンを持たない)。
//! 購入ごとの評価の推移は期間で絞らず、購入を指定できる (単件取得と同じ扱い)。
//! 存在しない ID と他の利用者の ID は区別せず 404 を返す (ADR-0006)。

use brew_book_core::stats::{self, StatsError};
use serde::{Deserialize, Serialize};
use worker::{console_error, Env, Request, Response, Result};

use super::{internal_error, invalid_input, not_found, purchase_exists};
use crate::auth::session::Session;
use crate::db;
use crate::respond;

/// `GET /api/stats/brews` の応答の 1 区間。
#[derive(Debug, Serialize, Deserialize)]
pub struct BrewPeriod {
    /// 区間のキー (日別は `YYYY-MM-DD`、月別は `YYYY-MM`)。
    pub period: String,
    /// 区間内の抽出の件数。
    pub brew_count: i64,
    /// 区間内の豆の量の合計 (小数第 1 位まで)。
    pub dose_grams: f64,
}

/// 抽出回数と豆の消費量の応答。
#[derive(Debug, Serialize)]
pub struct BrewStatsResponse {
    pub brews: Vec<BrewPeriod>,
}

/// `GET /api/stats/purchases` の応答の 1 区間と通貨コードの組。
#[derive(Debug, Serialize, Deserialize)]
pub struct PurchasePeriod {
    /// 区間のキー (日別は `YYYY-MM-DD`、月別は `YYYY-MM`)。
    pub period: String,
    /// ISO 4217 の通貨コード。価格が無い購入は null。
    pub price_currency: Option<String>,
    /// 区間内の価格の合計 (通貨の最小単位)。
    pub price_amount: i64,
    /// 区間内の重量の合計 (グラム)。
    pub weight_grams: i64,
    /// 区間内の購入の件数。
    pub purchase_count: i64,
}

/// 購入金額と重量の応答。
#[derive(Debug, Serialize)]
pub struct PurchaseStatsResponse {
    pub purchases: Vec<PurchasePeriod>,
}

/// `GET /api/stats/brew-ratings` の応答の 1 件の抽出。
#[derive(Debug, Serialize, Deserialize)]
pub struct BrewRating {
    /// 抽出の ID。
    pub id: String,
    /// 豆の量 (グラム)。任意。
    pub dose_grams: Option<f64>,
    /// 湯量 (グラム)。任意。
    pub water_grams: Option<f64>,
    /// 湯の温度 (摂氏)。任意。
    pub water_temp_c: Option<f64>,
    /// 時間 (秒)。任意。
    pub brew_time_seconds: Option<i64>,
    /// 評価 (1 から 5)。
    pub rating: i64,
}

/// 抽出条件と評価の関係の応答。
#[derive(Debug, Serialize)]
pub struct BrewRatingResponse {
    pub brew_ratings: Vec<BrewRating>,
}

/// `GET /api/purchases/<ID>/rating-history` の応答の 1 件の抽出。
#[derive(Debug, Serialize, Deserialize)]
pub struct RatingHistoryEntry {
    /// 抽出の ID。
    pub id: String,
    /// 抽出日時 (ISO 8601 UTC)。
    pub brewed_at: String,
    /// 評価 (1 から 5)。
    pub rating: i64,
}

/// 購入ごとの評価の推移の応答。
#[derive(Debug, Serialize)]
pub struct RatingHistoryResponse {
    pub ratings: Vec<RatingHistoryEntry>,
}

/// 抽出回数と豆の消費量を返す。認証が必要。
pub async fn brews(req: &Request, env: &Env, session: &Session) -> Result<Response> {
    let values = match QueryValues::from_request(req) {
        Ok(values) => values,
        Err(response) => return Ok(response),
    };
    let granularity = match stats::parse_granularity(values.get("granularity")) {
        Ok(value) => value,
        Err(error) => return Ok(stats_error(error)),
    };
    let period = match stats::parse_period(values.get("start"), values.get("end")) {
        Ok(value) => value,
        Err(error) => return Ok(stats_error(error)),
    };
    let offset_minutes = match stats::parse_offset_minutes(values.get("utc_offset_minutes")) {
        Ok(value) => value,
        Err(error) => return Ok(stats_error(error)),
    };
    let d1 = db::database(env)?;
    let statement = match stats::brews_stats(&session.user_id, granularity, offset_minutes, &period)
    {
        Ok(statement) => statement,
        Err(error) => return Ok(stats_error(error)),
    };
    let brews: Vec<BrewPeriod> = db::prepared(&d1, &statement)?.all().await?.results()?;
    respond::json(&BrewStatsResponse { brews })
}

/// 購入金額と重量を返す。認証が必要。
pub async fn purchases(req: &Request, env: &Env, session: &Session) -> Result<Response> {
    let values = match QueryValues::from_request(req) {
        Ok(values) => values,
        Err(response) => return Ok(response),
    };
    let granularity = match stats::parse_granularity(values.get("granularity")) {
        Ok(value) => value,
        Err(error) => return Ok(stats_error(error)),
    };
    let period = match stats::parse_period(values.get("start"), values.get("end")) {
        Ok(value) => value,
        Err(error) => return Ok(stats_error(error)),
    };
    let d1 = db::database(env)?;
    let statement = stats::purchases_stats(&session.user_id, granularity, &period);
    let purchases: Vec<PurchasePeriod> = db::prepared(&d1, &statement)?.all().await?.results()?;
    respond::json(&PurchaseStatsResponse { purchases })
}

/// 抽出条件と評価の関係を返す。認証が必要。
pub async fn brew_ratings(req: &Request, env: &Env, session: &Session) -> Result<Response> {
    let values = match QueryValues::from_request(req) {
        Ok(values) => values,
        Err(response) => return Ok(response),
    };
    let period = match stats::parse_period(values.get("start"), values.get("end")) {
        Ok(value) => value,
        Err(error) => return Ok(stats_error(error)),
    };
    let offset_minutes = match stats::parse_offset_minutes(values.get("utc_offset_minutes")) {
        Ok(value) => value,
        Err(error) => return Ok(stats_error(error)),
    };
    let d1 = db::database(env)?;
    let statement = match stats::brew_ratings(&session.user_id, offset_minutes, &period) {
        Ok(statement) => statement,
        Err(error) => return Ok(stats_error(error)),
    };
    let brew_ratings: Vec<BrewRating> = db::prepared(&d1, &statement)?.all().await?.results()?;
    respond::json(&BrewRatingResponse { brew_ratings })
}

/// 購入ごとの評価の推移を返す。認証が必要。
///
/// 購入を指定できる (単件取得と同じ扱い。FR-18)。
pub async fn rating_history(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the purchase does not exist"));
    };
    let d1 = db::database(env)?;
    if !purchase_exists(&d1, &session.user_id, id).await? {
        return Ok(not_found("the purchase does not exist"));
    }
    let statement = stats::rating_history(&session.user_id, id);
    let ratings: Vec<RatingHistoryEntry> = db::prepared(&d1, &statement)?.all().await?.results()?;
    respond::json(&RatingHistoryResponse { ratings })
}

/// 統計の入力の誤りを 400 の応答にする。
fn stats_error(error: StatsError) -> Response {
    invalid_input(error.message())
}

/// リクエストのクエリ文字列の名前と値の組。
struct QueryValues {
    pairs: Vec<(String, String)>,
}

impl QueryValues {
    /// リクエストの URL から読む。URL が読めない場合は 500 の応答にする。
    fn from_request(req: &Request) -> Result<Self, Response> {
        let url = match req.url() {
            Ok(url) => url,
            Err(error) => {
                console_error!("the request URL is not readable: {error}");
                return Err(internal_error());
            }
        };
        Ok(Self {
            pairs: url
                .query_pairs()
                .map(|(name, value)| (name.into_owned(), value.into_owned()))
                .collect(),
        })
    }

    /// 名前の値。同じ名前が複数あるときは最後の値を使う (一覧の読み取りと同じ)。
    fn get(&self, name: &str) -> Option<&str> {
        self.pairs
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}
