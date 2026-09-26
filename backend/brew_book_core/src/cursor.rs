//! 一覧のカーソルとページサイズ (ADR-0002、PRD の性能)。
//!
//! カーソルは並び順のキー (日時または日付と ID) を JSON にして base64url で符号化した文字列
//! とする。復号できない値は 400 にする。ページサイズは既定 50、最大 200 とし、200 を超える
//! 指定は 400 にする。

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

/// カーソルが運ぶ並び順のキー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorKey {
    /// 日時 (ISO 8601 UTC) と ID。並び順は日時の降順と ID の昇順。
    DateTime { at: String, id: String },
    /// 日付 (`YYYY-MM-DD`) と ID。並び順は日付の降順と ID の昇順。
    Date { on: String, id: String },
}

impl CursorKey {
    /// 並び順のキーと組になる ID。
    pub fn id(&self) -> &str {
        match self {
            CursorKey::DateTime { id, .. } | CursorKey::Date { id, .. } => id,
        }
    }

    /// base64url (パディング無し) のカーソルに符号化する。
    pub fn encode(&self) -> String {
        let json = match self {
            CursorKey::DateTime { at, id } => CursorJson {
                at: Some(at.clone()),
                on: None,
                id: id.clone(),
            },
            CursorKey::Date { on, id } => CursorJson {
                at: None,
                on: Some(on.clone()),
                id: id.clone(),
            },
        };
        let text =
            serde_json::to_string(&json).expect("a cursor of string fields is always serializable");
        crate::base64url::encode(text.as_bytes())
    }

    /// base64url のカーソルを復号する。復号できない値と、キーの値が妥当でない値は拒否する。
    ///
    /// 復号は `crate::base64url` に任せる。非正準の符号化 (余ったビットが 0 でない値) は
    /// 復号できるが、その結果が JSON とキーの形にならなければ拒否される。
    pub fn decode(text: &str) -> Result<Self, CursorError> {
        let bytes = crate::base64url::decode(text).map_err(|_| CursorError::Invalid)?;
        let json: CursorJson = serde_json::from_slice(&bytes).map_err(|_| CursorError::Invalid)?;
        if json.id.is_empty() {
            return Err(CursorError::Invalid);
        }
        match (json.at, json.on) {
            (Some(at), None) if datetime::parse_epoch_millis(&at).is_ok() => {
                Ok(CursorKey::DateTime { at, id: json.id })
            }
            (None, Some(on)) if datetime::is_valid_date(&on) => {
                Ok(CursorKey::Date { on, id: json.id })
            }
            _ => Err(CursorError::Invalid),
        }
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

/// カーソルの JSON 表現。日時か日付のどちらか一方だけを持つ。
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CursorJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on: Option<String>,
    id: String,
}
