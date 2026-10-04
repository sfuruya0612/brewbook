//! 統計と評価の推移の API の入力の検証と SQL の組み立て (FR-18、ADR-0006)。
//!
//! 集計は brews と purchases に対するクエリで行い、日別と月別の区切りは SQLite の日時関数に
//! 任せる (ADR-0006)。抽出の区切りは `brewed_at` に UTC オフセット (分) を加えた値で行い、
//! 購入は日付だけの `purchased_on` をそのまま切る。
//! 日時関数の修飾子 (`+540 minutes`) は整数から組み立て、利用者の入力の文字列を SQL に連結しない。
//! 期間の端は端末のローカル時刻の日付として受け取り、抽出の API では開始日の 00:00 と終了日の
//! 翌日の 00:00 を UTC オフセットで UTC の瞬間に直して `brewed_at` を絞る (FR-18)。
//! 組み立てる文はどれも `user_id` の条件を持つ。

use crate::datetime;
use crate::error::ErrorCode;
use crate::query::{self, Statement, Value};

/// `utc_offset_minutes` の下限 (UTC-14:00)。
pub const MIN_OFFSET_MINUTES: i32 = -840;
/// `utc_offset_minutes` の上限 (UTC+14:00)。
pub const MAX_OFFSET_MINUTES: i32 = 840;

const MILLIS_PER_MINUTE: i64 = 60_000;
const MILLIS_PER_DAY: i64 = 86_400_000;

/// 区間の粒度。`granularity` の値。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    /// 日別。区間のキーは `YYYY-MM-DD`。
    Day,
    /// 月別。区間のキーは `YYYY-MM`。
    Month,
}

/// 統計の入力の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsError {
    /// `granularity` が無いか、`day` と `month` 以外。
    Granularity,
    /// `utc_offset_minutes` が無いか、-840 から 840 の整数でない。
    Offset,
    /// `start` か `end` が `YYYY-MM-DD` の実在する日付でない。
    /// 日付の端と大きなオフセットの組み合わせが、扱える日時の範囲の外になるときも含む。
    Date,
    /// `start` が `end` より後。
    ReversedPeriod,
}

impl StatsError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        match self {
            StatsError::Granularity => "granularity must be day or month",
            StatsError::Offset => "utc_offset_minutes must be an integer from -840 to 840",
            StatsError::Date => "the date must be an existing date in the format YYYY-MM-DD",
            StatsError::ReversedPeriod => "start must not be after end",
        }
    }
}

/// 検証済みの期間。`None` の端には条件を付けない (全期間)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Period {
    /// 開始日 (`YYYY-MM-DD`)。省略したときは None。
    pub start: Option<String>,
    /// 終了日 (`YYYY-MM-DD`)。省略したときは None。
    pub end: Option<String>,
}

/// `granularity` の値を検証する。必須で、`day` と `month` だけを受け付ける (FR-18)。
pub fn parse_granularity(text: Option<&str>) -> Result<Granularity, StatsError> {
    match text {
        Some("day") => Ok(Granularity::Day),
        Some("month") => Ok(Granularity::Month),
        _ => Err(StatsError::Granularity),
    }
}

/// `utc_offset_minutes` の値を検証する。必須で、-840 から 840 の整数だけを受け付ける (FR-18)。
///
/// 端末のローカル時刻から UTC を引いた分数 (日本標準時は +540) を表す。
pub fn parse_offset_minutes(text: Option<&str>) -> Result<i32, StatsError> {
    let Some(text) = text else {
        return Err(StatsError::Offset);
    };
    let value: i32 = text.parse().map_err(|_| StatsError::Offset)?;
    if !(MIN_OFFSET_MINUTES..=MAX_OFFSET_MINUTES).contains(&value) {
        return Err(StatsError::Offset);
    }
    Ok(value)
}

/// `start` と `end` の値を検証する。実在する `YYYY-MM-DD` だけを受け付け、
/// `start` が `end` より後なら拒否する (FR-18)。省略した端には条件を付けない。
pub fn parse_period(start: Option<&str>, end: Option<&str>) -> Result<Period, StatsError> {
    let start = parse_day(start)?;
    let end = parse_day(end)?;
    if let (Some(start), Some(end)) = (&start, &end) {
        // 検証済みの固定長の日付は、文字列の比較が日付の比較になる。
        if start > end {
            return Err(StatsError::ReversedPeriod);
        }
    }
    Ok(Period { start, end })
}

/// 日時関数の修飾子 (`+540 minutes`、`-300 minutes`、`+0 minutes`)。
///
/// オフセットの整数から組み立てる。利用者の入力の文字列は使わない (ADR-0006)。
pub fn offset_modifier(offset_minutes: i32) -> String {
    format!("{offset_minutes:+} minutes")
}

/// `GET /api/stats/brews` の SQL を組み立てる。
///
/// 区間のキー、区間内の抽出の件数、区間内の豆の量の合計を、区間のキーの昇順で返す。
/// 豆の量が NULL の抽出は件数に数え、合計には加えない (FR-18)。
pub fn brews_stats(
    user_id: &str,
    granularity: Granularity,
    offset_minutes: i32,
    period: &Period,
) -> Result<Statement, StatsError> {
    let key = period_key("b.brewed_at", granularity, Some(offset_minutes));
    let mut sql = String::new();
    let mut params = vec![Value::Text(user_id.to_owned())];
    sql.push_str("SELECT ");
    sql.push_str(&key);
    sql.push_str(" AS period, COUNT(*) AS brew_count, ");
    sql.push_str("COALESCE(ROUND(SUM(b.dose_grams), 1), 0) AS dose_grams FROM ");
    sql.push_str(query::BREWS_TABLE);
    sql.push_str(" AS b WHERE b.user_id = ?");
    push_brew_period(&mut sql, &mut params, period, offset_minutes)?;
    sql.push_str(" GROUP BY ");
    sql.push_str(&key);
    sql.push_str(" ORDER BY period ASC");
    Ok(Statement { sql, params })
}

/// `GET /api/stats/brew-ratings` の SQL を組み立てる。
///
/// 期間内の評価を持つ抽出の ID、豆の量、湯量、湯の温度、時間、評価を、抽出日時の昇順で返す。
/// 条件が NULL の項目は NULL のまま返す (FR-18)。
pub fn brew_ratings(
    user_id: &str,
    offset_minutes: i32,
    period: &Period,
) -> Result<Statement, StatsError> {
    let mut sql = String::new();
    let mut params = vec![Value::Text(user_id.to_owned())];
    sql.push_str(
        "SELECT b.id, b.dose_grams, b.water_grams, b.water_temp_c, b.brew_time_seconds, b.rating \
         FROM ",
    );
    sql.push_str(query::BREWS_TABLE);
    sql.push_str(" AS b WHERE b.user_id = ? AND b.rating IS NOT NULL");
    push_brew_period(&mut sql, &mut params, period, offset_minutes)?;
    sql.push_str(" ORDER BY b.brewed_at ASC, b.id ASC");
    Ok(Statement { sql, params })
}

/// `GET /api/stats/purchases` の SQL を組み立てる。
///
/// 区間と通貨コードの組ごとに、購入の件数と価格の合計と重量の合計を返す。区間のキーの昇順、
/// 同じ区間では通貨コードの昇順 (NULL が先頭) で返す。価格が NULL の購入は金額に、重量が
/// NULL の購入は重量に加えず、どちらも件数には数える (FR-18)。
pub fn purchases_stats(user_id: &str, granularity: Granularity, period: &Period) -> Statement {
    let key = period_key("p.purchased_on", granularity, None);
    let mut sql = String::new();
    sql.push_str("SELECT ");
    sql.push_str(&key);
    sql.push_str(
        " AS period, p.price_currency, COALESCE(SUM(p.price_amount), 0) AS price_amount, ",
    );
    sql.push_str(
        "COALESCE(SUM(p.weight_grams), 0) AS weight_grams, COUNT(*) AS purchase_count FROM ",
    );
    sql.push_str(query::PURCHASES_TABLE);
    sql.push_str(" AS p WHERE p.user_id = ?");
    // 購入日はタイムゾーンを持たない日付なので、オフセットを受け取らずそのまま絞る (FR-18)。
    let mut params = vec![Value::Text(user_id.to_owned())];
    if let Some(start) = &period.start {
        sql.push_str(" AND p.purchased_on >= ?");
        params.push(Value::Text(start.clone()));
    }
    if let Some(end) = &period.end {
        sql.push_str(" AND p.purchased_on <= ?");
        params.push(Value::Text(end.clone()));
    }
    sql.push_str(" GROUP BY ");
    sql.push_str(&key);
    sql.push_str(", p.price_currency ORDER BY period ASC, p.price_currency ASC");
    Statement { sql, params }
}

/// `GET /api/purchases/<ID>/rating-history` の SQL を組み立てる。
///
/// その購入に紐づく評価を持つ抽出の ID、抽出日時、評価を、抽出日時の昇順で返す。
/// 評価が NULL の抽出は含めず、期間で絞らない (FR-18)。
pub fn rating_history(user_id: &str, purchase_id: &str) -> Statement {
    let mut sql = String::new();
    sql.push_str("SELECT b.id, b.brewed_at, b.rating FROM ");
    sql.push_str(query::BREWS_TABLE);
    sql.push_str(
        " AS b WHERE b.user_id = ? AND b.purchase_id = ? AND \
         b.rating IS NOT NULL ORDER BY b.brewed_at ASC, b.id ASC",
    );
    Statement {
        sql,
        params: vec![
            Value::Text(user_id.to_owned()),
            Value::Text(purchase_id.to_owned()),
        ],
    }
}

/// 日付 1 つを検証し、そのままの値にする。
fn parse_day(text: Option<&str>) -> Result<Option<String>, StatsError> {
    match text {
        None => Ok(None),
        Some(text) if datetime::is_valid_date(text) => Ok(Some(text.to_owned())),
        Some(_) => Err(StatsError::Date),
    }
}

/// 抽出の文に、ローカル時刻の日付の期間の条件を足す。
///
/// 開始日はその日の 00:00、終了日は翌日の 00:00 を UTC の瞬間に直し、
/// `brewed_at` がその範囲に入る行だけを対象にする (FR-18)。
fn push_brew_period(
    sql: &mut String,
    params: &mut Vec<Value>,
    period: &Period,
    offset_minutes: i32,
) -> Result<(), StatsError> {
    if let Some(start) = &period.start {
        sql.push_str(" AND b.brewed_at >= ?");
        params.push(Value::Text(local_day_start(start, offset_minutes, 0)?));
    }
    if let Some(end) = &period.end {
        sql.push_str(" AND b.brewed_at < ?");
        params.push(Value::Text(local_day_start(end, offset_minutes, 1)?));
    }
    Ok(())
}

/// 端末のローカル時刻の日付の 00:00 を、UTC の瞬間 (ISO 8601 UTC の固定長) に直す。
///
/// `days` は 0 ならその日、1 なら翌日 (終了日の翌日の 00:00) を指す。
/// オフセットを引いて UTC にするため、扱える日時の範囲 (0000 年から 9999 年) の外になる
/// 組み合わせは日付の誤りにする。
fn local_day_start(date: &str, offset_minutes: i32, days: i64) -> Result<String, StatsError> {
    let local_midnight = datetime::parse_epoch_millis(&format!("{date}T00:00:00.000Z"))
        .map_err(|_| StatsError::Date)?;
    let instant =
        local_midnight - i64::from(offset_minutes) * MILLIS_PER_MINUTE + days * MILLIS_PER_DAY;
    datetime::format_epoch_millis(instant).map_err(|_| StatsError::Date)
}

/// 区間のキーの SQL 式。`offset_minutes` があるときは日時関数の修飾子を付ける。
fn period_key(column: &str, granularity: Granularity, offset_minutes: Option<i32>) -> String {
    let format = match granularity {
        Granularity::Day => "%Y-%m-%d",
        Granularity::Month => "%Y-%m",
    };
    match offset_minutes {
        Some(offset) => format!(
            "strftime('{format}', {column}, '{}')",
            offset_modifier(offset)
        ),
        None => format!("strftime('{format}', {column})"),
    }
}
