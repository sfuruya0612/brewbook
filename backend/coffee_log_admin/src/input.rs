//! フォームの入力の解釈と検証 (ADR-0008)。
//!
//! 管理者画面は HTML のフォームだけを使うため、入力は
//! `application/x-www-form-urlencoded` の本文で届く。解釈と表示名の検証を純粋な関数として
//! 持ち、単体テストで検査する。

use worker::Url;

/// 表示名の最大の文字数 (FR-17)。
pub const DISPLAY_NAME_MAX_CHARS: usize = 50;

/// 表示名の検証の失敗。呼び出し側は 400 を返す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayNameError {
    /// 前後の空白を除くと空になる。
    Missing,
    /// 前後の空白を除いた文字数が上限を超える。
    TooLong,
}

impl DisplayNameError {
    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        match self {
            DisplayNameError::Missing => "the display name is missing",
            DisplayNameError::TooLong => "the display name must be at most 50 characters",
        }
    }
}

/// 表示名を検証し、前後の空白を除いた名前を返す (FR-17)。
/// 1 文字以上 50 文字以下を求める。文字数は Unicode のスカラー値の数で数える。
pub fn validate_display_name(raw: &str) -> Result<String, DisplayNameError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(DisplayNameError::Missing);
    }
    if name.chars().count() > DISPLAY_NAME_MAX_CHARS {
        return Err(DisplayNameError::TooLong);
    }
    Ok(name.to_owned())
}

/// `application/x-www-form-urlencoded` の本文から 1 つの項目を読む。
///
/// 本文を URL のクエリとして解釈し (`+` は空白、`%XX` は復号される)、最初に一致した項目の値を
/// 返す。項目が無い場合と、本文が解釈できない場合は `None` を返す。
pub fn form_field(body: &str, name: &str) -> Option<String> {
    let url = Url::parse(&format!("http://localhost/?{body}")).ok()?;
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

/// 発行した登録用リンクを組み立てる (FR-17)。
///
/// 利用者向けの Worker のオリジンと `/register?token=<トークン>` を連結する。
/// オリジンの末尾の `/` は 1 つだけに正規化する。
pub fn registration_link(app_origin: &str, token: &str) -> String {
    format!(
        "{}/register?token={token}",
        app_origin.trim_end_matches('/')
    )
}
