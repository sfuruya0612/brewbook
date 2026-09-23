//! 店、商品、Flavor Notes のタグと、購入と抽出の入力の検証 (FR-6、FR-7、FR-8、FR-9、FR-11)。
//!
//! 店名と商品名は前後の空白を除いて空なら拒否する。自由記述の値は保存時に前後の空白を除く
//! (0008 のサジェストの重複の除去と前方一致を同じ値の扱いにするため)。
//! タグ名も前後の空白を除いて空なら拒否し、同じ名前は 1 つにまとめる
//! (同じ利用者の同じタグ名は 1 つのタグとして共有する。FR-8)。
//! 購入日は `YYYY-MM-DD` の実在する日付、抽出日時は ISO 8601 の UTC の固定長
//! (`YYYY-MM-DDTHH:MM:SS.mmmZ`) だけを受け付ける (ADR-0002)。
//! 数値の検査は JSON の数値の文字列表現に対して行い、浮動小数点の丸め誤差に依存しない (0007)。

use crate::datetime;
use crate::error::ErrorCode;

/// 名前 (店名と商品名) の検証の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    /// 前後の空白を除くと空になる。
    Empty,
}

impl NameError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "the name must not be empty"
    }
}

/// 名前を検証し、前後の空白を除いた値を返す。
pub fn validate_name(name: &str) -> Result<String, NameError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    Ok(name.to_owned())
}

/// 自由記述の値を、保存用に前後の空白を除いた値にする。
pub fn trim_text(text: &str) -> String {
    text.trim().to_owned()
}

/// 任意の自由記述の値を、保存用に前後の空白を除いた値にする。無い値は NULL のままにする。
pub fn trim_optional(text: Option<&str>) -> Option<String> {
    text.map(trim_text)
}

/// タグ名の検証の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagNameError {
    /// 前後の空白を除くと空になる。
    Empty,
}

impl TagNameError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "a flavor note must not be empty"
    }
}

/// 日付 (`YYYY-MM-DD`) の検証の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayError {
    /// 形式が違うか、実在しない日付 (うるう年を考慮する)。
    Invalid,
}

impl DayError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "the date must be an existing date in the format YYYY-MM-DD"
    }
}

/// 購入日と Roast Date の検証。タイムゾーンを持たない `YYYY-MM-DD` だけを受け付ける (ADR-0002)。
pub fn validate_day(text: &str) -> Result<(), DayError> {
    if datetime::is_valid_date(text) {
        Ok(())
    } else {
        Err(DayError::Invalid)
    }
}

/// 抽出日時の検証の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampError {
    /// 形式が違うか、実在しない日時。オフセット付きの日時も拒否する。
    Invalid,
}

impl TimestampError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "the timestamp must be an ISO 8601 UTC timestamp of the format YYYY-MM-DDTHH:MM:SS.sssZ"
    }
}

/// 抽出日時の検証。末尾が `Z` で小数秒がミリ秒 3 桁の ISO 8601 UTC だけを受け付ける (ADR-0002)。
pub fn validate_timestamp(text: &str) -> Result<(), TimestampError> {
    if datetime::parse_epoch_millis(text).is_ok() {
        Ok(())
    } else {
        Err(TimestampError::Invalid)
    }
}

/// 0 以上の整数の検証の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountError {
    /// 負の値。
    Negative,
    /// 保存できる整数の範囲 (D1 の整数は 32 ビット) の外。
    TooLarge,
}

impl CountError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        match self {
            CountError::Negative => "the value must not be negative",
            CountError::TooLarge => "the value is too large",
        }
    }
}

/// 価格、重量、時間の検証。0 以上で、D1 の整数 (32 ビット) に収まる値だけを受け付ける。
pub fn validate_count(value: i64) -> Result<i64, CountError> {
    if value < 0 {
        return Err(CountError::Negative);
    }
    if value > i64::from(i32::MAX) {
        return Err(CountError::TooLarge);
    }
    Ok(value)
}

/// 評価の検証の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatingError {
    /// 1 から 5 の整数でない。
    OutOfRange,
}

impl RatingError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "the rating must be an integer from 1 to 5"
    }
}

/// 評価の検証。1 から 5 の整数だけを受け付ける (FR-11)。
pub fn validate_rating(value: i64) -> Result<i64, RatingError> {
    if (1..=5).contains(&value) {
        Ok(value)
    } else {
        Err(RatingError::OutOfRange)
    }
}

/// 通貨コードの既定値 (FR-9)。
pub const DEFAULT_CURRENCY: &str = "JPY";

/// 通貨コードの検証の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrencyError {
    /// ISO 4217 の 3 文字の英大文字でない。
    NotThreeUppercaseLetters,
}

impl CurrencyError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "the price currency must be three uppercase ASCII letters"
    }
}

/// 通貨コードの検証。ISO 4217 の 3 文字の英大文字だけを受け付ける (FR-9)。
pub fn validate_currency(text: &str) -> Result<String, CurrencyError> {
    if text.len() == 3 && text.bytes().all(|byte| byte.is_ascii_uppercase()) {
        Ok(text.to_owned())
    } else {
        Err(CurrencyError::NotThreeUppercaseLetters)
    }
}

/// 小数 (豆の量、湯量、湯の温度) の検証の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimalError {
    /// 負の値。
    Negative,
    /// 小数第 2 位以下を持つか、指数表記で書かれている。
    NotOneDecimalPlace,
    /// 扱える値の範囲の外。
    OutOfRange,
}

impl DecimalError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        match self {
            DecimalError::Negative => "the value must not be negative",
            DecimalError::NotOneDecimalPlace => "the value must have at most one decimal place",
            DecimalError::OutOfRange => "the value is out of range",
        }
    }
}

/// 0 以上で小数第 1 位までの数を検証する (FR-11)。
///
/// 引数は `serde_json` の数値の文字列表現 ([`serde_json::Number::to_string`] の結果) とする。
/// 検査はその表記に対して行い、2 進の浮動小数点に変換してから桁数を数えない
/// (変換してから数えると丸め誤差に依存するため)。指数表記は受け付けない
/// (小数第 1 位までという条件を表記で判定できないため)。
pub fn validate_decimal(text: &str) -> Result<f64, DecimalError> {
    // 整数部と小数部の桁を、表記のまま見る。
    let (integer, fraction) = match text.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (text, None),
    };
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    if !digits(integer.trim_start_matches('-')) {
        return Err(DecimalError::NotOneDecimalPlace);
    }
    if let Some(fraction) = fraction {
        if fraction.len() != 1 || !digits(fraction) {
            return Err(DecimalError::NotOneDecimalPlace);
        }
    }
    let value: f64 = text.parse().map_err(|_| DecimalError::OutOfRange)?;
    if !value.is_finite() {
        return Err(DecimalError::OutOfRange);
    }
    if value < 0.0 {
        return Err(DecimalError::Negative);
    }
    Ok(value)
}

/// Flavor Notes のタグ名を検証し、保存用の並びにする。
///
/// 前後の空白を除き、空になる名前は拒否し、同じ名前は 1 つにまとめ、名前の昇順に並べる。
/// 並び順は SQLite の `ORDER BY name` (UTF-8 のバイト順) と同じにする。
pub fn validate_flavor_notes(names: &[String]) -> Result<Vec<String>, TagNameError> {
    let mut validated = Vec::with_capacity(names.len());
    for name in names {
        let name = name.trim();
        if name.is_empty() {
            return Err(TagNameError::Empty);
        }
        validated.push(name.to_owned());
    }
    validated.sort_unstable();
    validated.dedup();
    Ok(validated)
}
