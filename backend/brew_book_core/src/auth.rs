//! 認証の共通部品 (ADR-0004、ADR-0006)。
//!
//! 登録用トークン、チャレンジ、セッションのトークンの作り方と保存の形、期限の照合、
//! セッションの Cookie の組み立てと取り出し、パスキーの名前の検証を、
//! バインディングに依存しない純粋な関数として持つ。
//!
//! 秘密値は 32 バイトの乱数を base64url で符号化した文字列とし、D1 には SHA-256 の
//! ハッシュだけを保存する。生の値は応答と Cookie にだけ載せる (ADR-0004)。
//! 期限は ISO 8601 UTC の固定長文字列で持つため、辞書順の比較が時刻の順と一致する (ADR-0002)。

use sha2::{Digest, Sha256};

use crate::base64url;
use crate::datetime::{format_epoch_millis, DateTimeError};

/// 秘密値のバイト長 (登録用トークン、チャレンジ、セッションのトークン)。ADR-0004。
pub const SECRET_LEN: usize = 32;

/// 登録用トークンの有効期限 (秒)。ADR-0004 は発行から 24 時間とする。
pub const REGISTRATION_TOKEN_TTL_SECONDS: i64 = 24 * 60 * 60;

/// チャレンジの有効期限 (秒)。ADR-0004 は 5 分とする。
pub const CHALLENGE_TTL_SECONDS: i64 = 5 * 60;

/// セッションの有効期限 (秒)。ADR-0004 は発行から 30 日とする。
pub const SESSION_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;

/// 登録のチャレンジの種別 (`webauthn_challenges.kind`)。ADR-0006。
pub const KIND_REGISTRATION: &str = "registration";
/// ログインのチャレンジの種別 (`webauthn_challenges.kind`)。ADR-0006。
pub const KIND_AUTHENTICATION: &str = "authentication";

/// セッションの Cookie の名前。ADR-0004。
pub const SESSION_COOKIE_NAME: &str = "session";

/// パスキーの名前の最大の文字数。ADR-0004 は 1 文字以上 50 文字以下とする。
pub const PASSKEY_NAME_MAX_CHARS: usize = 50;

/// 32 バイトの乱数を base64url の文字列にする。
pub fn encode_secret(bytes: [u8; SECRET_LEN]) -> String {
    base64url::encode(&bytes)
}

/// 秘密値の SHA-256 のハッシュ (小文字の 16 進 64 文字)。D1 にはこの値だけを保存する。
pub fn hash_secret(secret: &str) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(secret.as_bytes());
    let mut hash = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hash, "{byte:02x}");
    }
    hash
}

/// 現在時刻から有効期限の時刻を計算する。単位は秒で、現在時刻は epoch ミリ秒で受け取る。
pub fn expiry_from(now_millis: i64, ttl_seconds: i64) -> Result<String, DateTimeError> {
    let millis = now_millis.saturating_add(ttl_seconds.saturating_mul(1000));
    format_epoch_millis(millis)
}

/// 有効期限 (ISO 8601 UTC) が現在時刻 (同形式) を過ぎているかを、辞書順の比較で判定する。
/// 期限ちょうどの時刻は期限切れとして扱う (ADR-0002)。
pub fn is_expired(expires_at: &str, now: &str) -> bool {
    expires_at <= now
}

/// パスキーの名前の検証の失敗。呼び出し側は 400 を返す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    /// 前後の空白を除くと空になる (名前が無い)。
    Missing,
    /// 前後の空白を除いた文字数が上限を超える。
    TooLong,
}

impl std::fmt::Display for NameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameError::Missing => write!(formatter, "the name is missing"),
            NameError::TooLong => write!(
                formatter,
                "the name must be at most {PASSKEY_NAME_MAX_CHARS} characters"
            ),
        }
    }
}

impl std::error::Error for NameError {}

/// パスキーの名前を検証し、前後の空白を除いた名前を返す (ADR-0004)。
/// 1 文字以上 50 文字以下を求める。文字数は Unicode のスカラー値の数で数える。
pub fn validate_passkey_name(raw: &str) -> Result<String, NameError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(NameError::Missing);
    }
    if name.chars().count() > PASSKEY_NAME_MAX_CHARS {
        return Err(NameError::TooLong);
    }
    Ok(name.to_owned())
}

/// セッションの Cookie を組み立てる。
/// `HttpOnly`、`Secure`、`SameSite=Lax`、`Path=/` を付け、有効期限は Max-Age で表す (ADR-0004)。
pub fn session_cookie(token: &str, max_age_seconds: i64) -> String {
    format!(
        "{SESSION_COOKIE_NAME}={token}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={max_age_seconds}"
    )
}

/// ログアウトでセッションの Cookie を失効させる値 (Max-Age を 0 にする)。
pub fn expired_session_cookie() -> String {
    format!("{SESSION_COOKIE_NAME}=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0")
}

/// `Cookie` ヘッダからセッションのトークンを取り出す。
/// 同名の Cookie が複数ある場合は最初の空でない値を返す。
pub fn session_token(cookie_header: &str) -> Option<&str> {
    for cookie in cookie_header.split(';') {
        let Some((name, value)) = cookie.trim().split_once('=') else {
            continue;
        };
        if name.trim() == SESSION_COOKIE_NAME && !value.is_empty() {
            return Some(value.trim());
        }
    }
    None
}
