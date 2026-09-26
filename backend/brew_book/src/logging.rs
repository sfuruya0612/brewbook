//! リクエスト 1 件ごとのログ。
//!
//! PRD の運用のとおり、利用者の記録の内容 (商品名、感想など) をログに出さない。
//! そのため、ログに載せるのは経路のメタデータだけとし、リクエスト本文とクエリの値を持たない。

use serde::Serialize;
use worker::console_log;

/// ログ 1 行のイベント名。
pub const EVENT_REQUEST: &str = "request";

/// リクエスト 1 件のログに載せる経路のメタデータ。
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RequestLog<'a> {
    pub event: &'static str,
    pub route: &'a str,
    pub method: &'a str,
    pub status: u16,
    pub duration_ms: u64,
}

impl<'a> RequestLog<'a> {
    pub fn new(route: &'a str, method: &'a str, status: u16, duration_ms: u64) -> Self {
        Self {
            event: EVENT_REQUEST,
            route,
            method,
            status,
            duration_ms,
        }
    }
}

/// ログ 1 行の JSON を組み立てる。
pub fn line(log: &RequestLog<'_>) -> String {
    serde_json::to_string(log).expect("a request log with plain fields is always serializable")
}

/// wrangler dev の出力に 1 行の JSON としてログを出す。
pub fn log_request(log: &RequestLog<'_>) {
    console_log!("{}", line(log));
}
