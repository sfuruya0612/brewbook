//! API のエラー応答の組み立て。形式は PRD の「エラー応答の規約」に従う。
//!
//! 応答は `{"error": {"code": "<snake_case>", "message": "<英語>"}}` の JSON に統一する。

use serde::Serialize;

/// エラー応答の `code` と HTTP ステータスコードの対応。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    Gone,
    /// PRD の表には無い。ルーティング以外で Worker が失敗したときの応答に使う。
    Internal,
}

impl ErrorCode {
    /// 応答の `code` に載せる snake_case の文字列。
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::BadRequest => "bad_request",
            ErrorCode::Unauthorized => "unauthorized",
            ErrorCode::Forbidden => "forbidden",
            ErrorCode::NotFound => "not_found",
            ErrorCode::Conflict => "conflict",
            ErrorCode::Gone => "gone",
            ErrorCode::Internal => "internal_error",
        }
    }

    /// PRD の「エラー応答の規約」が定める HTTP ステータスコード。
    pub fn status(self) -> u16 {
        match self {
            ErrorCode::BadRequest => 400,
            ErrorCode::Unauthorized => 401,
            ErrorCode::Forbidden => 403,
            ErrorCode::NotFound => 404,
            ErrorCode::Conflict => 409,
            ErrorCode::Gone => 410,
            ErrorCode::Internal => 500,
        }
    }
}

/// `{"error": ...}` の内側のオブジェクト。
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ErrorBody<'a> {
    pub code: &'a str,
    pub message: &'a str,
}

/// エラー応答の全体。
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ErrorEnvelope<'a> {
    pub error: ErrorBody<'a>,
}

/// エラー応答を組み立てる。
pub fn envelope(code: ErrorCode, message: &str) -> ErrorEnvelope<'_> {
    ErrorEnvelope {
        error: ErrorBody {
            code: code.as_str(),
            message,
        },
    }
}

/// エラー応答を JSON の文字列にする。
pub fn body(code: ErrorCode, message: &str) -> String {
    serde_json::to_string(&envelope(code, message))
        .expect("an error envelope with string fields is always serializable")
}
