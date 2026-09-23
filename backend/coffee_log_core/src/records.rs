//! 店、商品、Flavor Notes のタグの入力の検証 (FR-6、FR-7、FR-8)。
//!
//! 店名と商品名は前後の空白を除いて空なら拒否する。自由記述の値は保存時に前後の空白を除く
//! (0008 のサジェストの重複の除去と前方一致を同じ値の扱いにするため)。
//! タグ名も前後の空白を除いて空なら拒否し、同じ名前は 1 つにまとめる
//! (同じ利用者の同じタグ名は 1 つのタグとして共有する。FR-8)。

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
