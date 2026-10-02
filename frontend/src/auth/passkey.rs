//! パスキー (WebAuthn) のクライアント (FR-1、FR-2、ADR-0004)。
//!
//! サーバーのオプションは JSON で受け渡し、サーバーが検証するクレデンシャルは JSON で返す
//! (Flutter の `PasskeyClient` と同じ)。JSON からブラウザの API が受け取る形への変換は
//! 実行環境に依存しない純粋な関数 ([`CreationOptions::from_json`] と
//! [`RequestOptions::from_json`]) として切り出し、native のテストで守る (0040)。
//! `navigator.credentials` の呼び出しは Web の実装 (`passkey_web`) だけが持つ。

use std::future::Future;
use std::pin::Pin;

use serde_json::{Map, Value};

use super::base64url;

/// パスキーの操作の失敗の種類 (Flutter の `PasskeyErrorKind` と同じ)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PasskeyErrorKind {
    /// 利用者が操作を取り消した (W3C WebAuthn Level 3 の `NotAllowedError` など)。
    Cancelled,

    /// この環境ではパスキーを使えない (ブラウザが対応していないなど)。
    Unsupported,

    /// その他の失敗。
    Failed,
}

/// パスキーの操作の失敗。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PasskeyError {
    /// 失敗の種類。画面の文言の分岐に使う。
    pub kind: PasskeyErrorKind,

    /// 失敗の原因 (JS の例外の説明など)。
    pub cause: String,
}

impl PasskeyError {
    /// 種類と原因から作る。
    pub fn new(kind: PasskeyErrorKind, cause: impl Into<String>) -> Self {
        Self {
            kind,
            cause: cause.into(),
        }
    }

    /// 利用者が取り消した失敗。
    pub fn cancelled(cause: impl Into<String>) -> Self {
        Self::new(PasskeyErrorKind::Cancelled, cause)
    }

    /// パスキーを使えない失敗。
    pub fn unsupported(cause: impl Into<String>) -> Self {
        Self::new(PasskeyErrorKind::Unsupported, cause)
    }

    /// その他の失敗。
    pub fn failed(cause: impl Into<String>) -> Self {
        Self::new(PasskeyErrorKind::Failed, cause)
    }

    /// JS の例外の名前から失敗の種類を決める (Flutter の `WebPasskeyClient.kindOf` と同じ)。
    ///
    /// 取り消しとタイムアウトは `NotAllowedError` で返る (W3C WebAuthn Level 3 の 5.1.4)。
    pub fn kind_of_error_name(name: Option<&str>) -> PasskeyErrorKind {
        match name {
            Some("NotAllowedError" | "AbortError") => PasskeyErrorKind::Cancelled,
            Some("NotSupportedError") => PasskeyErrorKind::Unsupported,
            _ => PasskeyErrorKind::Failed,
        }
    }
}

impl std::fmt::Display for PasskeyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "PasskeyError({:?}, cause: {})",
            self.kind, self.cause
        )
    }
}

impl std::error::Error for PasskeyError {}

/// パスキーの操作の未来。Web の実装もテストの偽の実装も同じ型で返す。
pub type PasskeyFuture<T> = Pin<Box<dyn Future<Output = Result<T, PasskeyError>>>>;

/// パスキー (WebAuthn) のクライアント。
///
/// 登録は `navigator.credentials.create`、ログインは `navigator.credentials.get` に対応する。
/// 返す値はサーバーの `register/complete` と `login/complete` の `credential` にそのまま渡す。
pub trait PasskeyClient {
    /// 登録の作成のオプションでパスキーを作る (FR-1)。
    fn create_credential(&self, options: CreationOptions) -> PasskeyFuture<Map<String, Value>>;

    /// ログインの要求のオプションでパスキーを使う (FR-2)。
    fn get_credential(&self, options: RequestOptions) -> PasskeyFuture<Map<String, Value>>;
}

/// `pubKeyCredParams` の `alg` が無いときの既定値 (ES256。ADR-0004)。
const DEFAULT_ALGORITHM: i32 = -7;

/// 登録の作成のオプション (サーバーの `POST /api/auth/register/begin` の応答)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CreationOptions {
    /// チャレンジ (復号済み)。
    pub challenge: Vec<u8>,

    /// Relying Party の ID。空ならブラウザの既定 (現在のオリジンのホスト) を使う。
    pub rp_id: String,

    /// Relying Party の表示名。
    pub rp_name: String,

    /// 利用者の ID (復号済み)。
    pub user_id: Vec<u8>,

    /// 利用者の名前。
    pub user_name: String,

    /// 利用者の表示名。無いときは名前を使う (Flutter と同じ)。
    pub user_display_name: String,

    /// 受け付ける公開鍵のアルゴリズム (COSE の alg)。
    pub algorithms: Vec<i32>,

    /// ミリ秒のタイムアウト。0 はタイムアウトを課さない。
    pub timeout: u32,

    /// attestation の伝達の希望 (既定は `none`)。
    pub attestation: String,

    /// resident key の希望 (既定は `preferred`)。
    pub resident_key: String,

    /// 利用者の確認の要求 (既定は `required`)。
    pub user_verification: String,
}

impl CreationOptions {
    /// サーバーの応答の JSON から読む。
    ///
    /// 無い項目は Flutter と同じ既定値で埋める (ADR-0004 の要求: attestation は `none`、
    /// residentKey は `preferred`、userVerification は `required`)。チャレンジと利用者の ID は
    /// base64url で運ばれ、読めなければ失敗にする。
    pub fn from_json(options: &Map<String, Value>) -> Result<Self, PasskeyError> {
        let rp = object_field(options, "rp");
        let user = object_field(options, "user")
            .ok_or_else(|| PasskeyError::failed("the user field is missing"))?;
        let user_name = string_field(user, "name").unwrap_or_default();
        let user_display_name =
            string_field(user, "displayName").unwrap_or_else(|| user_name.clone());
        Ok(Self {
            challenge: decode_field(options, "challenge")?,
            rp_id: rp.and_then(|rp| string_field(rp, "id")).unwrap_or_default(),
            rp_name: rp
                .and_then(|rp| string_field(rp, "name"))
                .unwrap_or_default(),
            user_id: decode_field(user, "id")?,
            user_name,
            user_display_name,
            algorithms: algorithms_of(options),
            timeout: timeout_of(options),
            attestation: string_field(options, "attestation").unwrap_or_else(|| "none".to_string()),
            resident_key: nested_string_field(options, "authenticatorSelection", "residentKey")
                .unwrap_or_else(|| "preferred".to_string()),
            user_verification: nested_string_field(
                options,
                "authenticatorSelection",
                "userVerification",
            )
            .unwrap_or_else(|| "required".to_string()),
        })
    }
}

/// ログインの要求のオプション (サーバーの `POST /api/auth/login/begin` の応答)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RequestOptions {
    /// チャレンジ (復号済み)。
    pub challenge: Vec<u8>,

    /// Relying Party の ID。空ならブラウザの既定を使う。
    pub rp_id: String,

    /// 利用者の確認の要求 (既定は `required`)。
    pub user_verification: String,

    /// 対象のクレデンシャル ID (復号済み)。空なら discoverable なパスキーを対象にする (FR-2)。
    pub allow_credentials: Vec<Vec<u8>>,

    /// ミリ秒のタイムアウト。0 はタイムアウトを課さない。
    pub timeout: u32,
}

impl RequestOptions {
    /// サーバーの応答の JSON から読む。
    ///
    /// 無い項目は Flutter と同じ既定値で埋める (`rpId` は空、`userVerification` は `required`)。
    /// チャレンジと `allowCredentials` の ID は base64url で運ばれ、読めなければ失敗にする。
    pub fn from_json(options: &Map<String, Value>) -> Result<Self, PasskeyError> {
        Ok(Self {
            challenge: decode_field(options, "challenge")?,
            rp_id: string_field(options, "rpId").unwrap_or_default(),
            user_verification: string_field(options, "userVerification")
                .unwrap_or_else(|| "required".to_string()),
            allow_credentials: allow_credentials_of(options)?,
            timeout: timeout_of(options),
        })
    }
}

/// オブジェクトの項目を読む。無い場合とオブジェクトでない場合は None。
fn object_field<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
    map.get(key).and_then(Value::as_object)
}

/// 文字列の項目を読む。無い場合と文字列でない場合は None。
fn string_field(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// 入れ子のオブジェクトの中の文字列の項目を読む。
fn nested_string_field(map: &Map<String, Value>, outer: &str, inner: &str) -> Option<String> {
    object_field(map, outer).and_then(|map| string_field(map, inner))
}

/// base64url の項目を復号して読む。
fn decode_field(map: &Map<String, Value>, key: &str) -> Result<Vec<u8>, PasskeyError> {
    let value = string_field(map, key)
        .ok_or_else(|| PasskeyError::failed(format!("the {key} field is missing")))?;
    base64url::decode(&value)
        .map_err(|error| PasskeyError::failed(format!("the {key} field is not base64url: {error}")))
}

/// `pubKeyCredParams` のアルゴリズムを読む。`alg` の無い項目は ES256 にする (Flutter と同じ)。
fn algorithms_of(options: &Map<String, Value>) -> Vec<i32> {
    options
        .get("pubKeyCredParams")
        .and_then(Value::as_array)
        .map(|parameters| {
            parameters
                .iter()
                .filter_map(Value::as_object)
                .map(|parameter| {
                    parameter
                        .get("alg")
                        .and_then(Value::as_i64)
                        .and_then(|alg| i32::try_from(alg).ok())
                        .unwrap_or(DEFAULT_ALGORITHM)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `allowCredentials` の ID を復号して読む。空の配列は discoverable なパスキーを対象にする。
fn allow_credentials_of(options: &Map<String, Value>) -> Result<Vec<Vec<u8>>, PasskeyError> {
    let Some(descriptors) = options.get("allowCredentials").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    descriptors
        .iter()
        .map(|descriptor| {
            let descriptor = descriptor.as_object().ok_or_else(|| {
                PasskeyError::failed("the allowCredentials entry is not an object")
            })?;
            decode_field(descriptor, "id")
        })
        .collect()
}

/// ミリ秒のタイムアウトを読む。無い場合は 0 (タイムアウトを課さない)。
fn timeout_of(options: &Map<String, Value>) -> u32 {
    options
        .get("timeout")
        .and_then(Value::as_u64)
        .map(|timeout| timeout.min(u64::from(u32::MAX)) as u32)
        .unwrap_or(0)
}
