//! 通貨コードのマスタ (FR-9)。
//!
//! ISO 4217 はアプリが管理するデータではなく外部の規格のため、D1 のテーブルと配る API は
//! 持たず、固定リストとして持つ (Flutter の `frontend/lib/records/currencies.dart` と同じ)。
//! 追加の要望があれば [`CURRENCY_CODES`] と通貨名 (ARB の `currencyXxx`) を足す。

use crate::i18n::{text, Key, Language};

/// プルダウンに載せる主要な通貨 30 種。コードの昇順に並べる。
pub const CURRENCY_CODES: [&str; 30] = [
    "AED", "AUD", "BRL", "CAD", "CHF", "CNY", "CZK", "DKK", "EUR", "GBP", "HKD", "IDR", "INR",
    "JPY", "KRW", "MXN", "MYR", "NOK", "NZD", "PHP", "PLN", "SAR", "SEK", "SGD", "THB", "TRY",
    "TWD", "USD", "VND", "ZAR",
];

/// 通貨コードの既定値 (FR-9)。
pub const DEFAULT_CURRENCY: &str = "JPY";

/// 通貨コードの通貨名のキー。マスタに無いコードは None。
pub fn currency_key(code: &str) -> Option<Key> {
    let key = match code {
        "AED" => Key::CurrencyAed,
        "AUD" => Key::CurrencyAud,
        "BRL" => Key::CurrencyBrl,
        "CAD" => Key::CurrencyCad,
        "CHF" => Key::CurrencyChf,
        "CNY" => Key::CurrencyCny,
        "CZK" => Key::CurrencyCzk,
        "DKK" => Key::CurrencyDkk,
        "EUR" => Key::CurrencyEur,
        "GBP" => Key::CurrencyGbp,
        "HKD" => Key::CurrencyHkd,
        "IDR" => Key::CurrencyIdr,
        "INR" => Key::CurrencyInr,
        "JPY" => Key::CurrencyJpy,
        "KRW" => Key::CurrencyKrw,
        "MXN" => Key::CurrencyMxn,
        "MYR" => Key::CurrencyMyr,
        "NOK" => Key::CurrencyNok,
        "NZD" => Key::CurrencyNzd,
        "PHP" => Key::CurrencyPhp,
        "PLN" => Key::CurrencyPln,
        "SAR" => Key::CurrencySar,
        "SEK" => Key::CurrencySek,
        "SGD" => Key::CurrencySgd,
        "THB" => Key::CurrencyThb,
        "TRY" => Key::CurrencyTry,
        "TWD" => Key::CurrencyTwd,
        "USD" => Key::CurrencyUsd,
        "VND" => Key::CurrencyVnd,
        "ZAR" => Key::CurrencyZar,
        _ => return None,
    };
    Some(key)
}

/// 通貨コードの通貨名 (CLDR の表示名に合わせる)。マスタに無いコードはコードをそのまま返す。
pub fn currency_name(language: Language, code: &str) -> String {
    match currency_key(code) {
        Some(key) => text(language, key).to_string(),
        None => code.to_string(),
    }
}

/// プルダウンの選択肢の表示。マスタにあるコードは「コード 通貨名」(例: `JPY 日本円`)、
/// マスタに無いコードは名前を付けずコードだけにする。
pub fn currency_option_label(language: Language, code: &str) -> String {
    match currency_key(code) {
        Some(key) => format!("{code} {}", text(language, key)),
        None => code.to_string(),
    }
}

/// プルダウンの選択肢のコード。コードの昇順に並べる。
///
/// 現在値がマスタに無いときは、そのコードを足して選べるようにする (編集の互換)。
pub fn currency_options(current: Option<&str>) -> Vec<String> {
    let mut options: Vec<String> = CURRENCY_CODES.iter().map(|code| code.to_string()).collect();
    if let Some(current) = current {
        if !CURRENCY_CODES.contains(&current) {
            options.push(current.to_string());
            options.sort();
        }
    }
    options
}
