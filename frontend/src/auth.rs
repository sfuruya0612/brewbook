//! ログインの状態と認証の操作 (FR-1、FR-2、FR-4)。
//!
//! セッションは HttpOnly の Cookie でブラウザが管理するため (ADR-0005)、このモジュールは
//! Cookie を扱わない。セッションの確認は専用の API を設けず、起動時に `GET /api/passkeys` を
//! 呼んで 401 かどうかで判定する (Flutter の `AuthController` と同じ)。
//!
//! 登録とログインは、サーバーのオプションを取る呼び出し、`navigator.credentials` の呼び出し、
//! サーバーに検証させる呼び出しの 3 段にする。`navigator.credentials` の呼び出しは
//! [`PasskeyClient`] の後ろに置き、native のテストは偽の実装に差し替える。

pub mod base64url;
mod passkey;

#[cfg(target_arch = "wasm32")]
pub mod passkey_web;

use std::rc::Rc;

use serde_json::{json, Map, Value};

use crate::api::{ApiCallError, ApiClient};
use crate::i18n::Key;

pub use passkey::{
    CreationOptions, PasskeyClient, PasskeyError, PasskeyErrorKind, PasskeyFuture, RequestOptions,
};

/// ログインの状態 (FR-1、FR-2、FR-4)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionStatus {
    /// 起動時の確認中。
    Checking,

    /// 起動時の確認が失敗し (401 以外の API エラー、ネットワークエラー)、ログイン状態が分からない。
    Unknown,

    /// セッションが無いか無効。
    SignedOut,

    /// セッションが有効。
    SignedIn,
}

/// 起動時に `GET /api/passkeys` を呼び、ログイン状態を判定する (FR-1、FR-2)。
///
/// セッションの確認は専用の API を設けず、401 かどうかで判定する (Flutter と同じ)。
/// 401 は [`ApiClient::set_on_unauthorized`] の callback がセッションが失われたことを記録し、
/// `AppShell` の遷移の判定がログイン画面へ戻す。401 以外の失敗とネットワークエラーはログイン
/// 状態が分からないため、再試行を促す表示にする (ADR-0007)。
pub async fn check_session(api: &ApiClient) -> SessionStatus {
    match api.get_json("/passkeys").await {
        Ok(_) => SessionStatus::SignedIn,
        Err(ApiCallError::Api(error)) if error.is_unauthorized() => SessionStatus::SignedOut,
        Err(_) => SessionStatus::Unknown,
    }
}

/// 認証の画面が使う依存の束 (0038 の `RecordServices` と同じ方針)。
#[derive(Clone)]
pub struct AuthServices {
    /// 認証の API (FR-1、FR-2、FR-4)。
    pub api: ApiClient,

    /// パスキーの操作。実行環境に合う実装を [`AuthServices::web`] が返し、テストは偽の実装を
    /// 差し込む。
    pub passkeys: Rc<dyn PasskeyClient>,
}

impl AuthServices {
    /// 依存を指定して束ねる。テストは偽の実装を渡す。
    pub fn new(api: ApiClient, passkeys: Rc<dyn PasskeyClient>) -> Self {
        Self { api, passkeys }
    }

    /// ブラウザの実装を束ねる (FR-1、FR-2)。
    #[cfg(target_arch = "wasm32")]
    pub fn web(api: ApiClient) -> Self {
        Self::new(api, Rc::new(passkey_web::WebPasskeyClient))
    }
}

/// 登録済みのパスキー (FR-3)。
///
/// 日時は API の形式 (ISO 8601 の UTC) の文字列のまま保持し、表示の直前に端末の
/// タイムゾーンへ変換する (Flutter の `Passkey` と同じ扱い)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Passkey {
    /// パスキーの ID。
    pub id: String,

    /// 利用者が付けた名前。
    pub name: String,

    /// 登録日時 (ISO 8601 の UTC)。
    pub created_at: String,

    /// 最終使用日時 (ISO 8601 の UTC)。まだ使われていなければ None。
    pub last_used_at: Option<String>,
}

impl Passkey {
    /// JSON のオブジェクト (`GET /api/passkeys` の 1 件) から組み立てる。
    pub fn from_json(json: &Map<String, Value>) -> Result<Self, AuthError> {
        Ok(Self {
            id: string_field(json, "id")?,
            name: string_field(json, "name")?,
            created_at: string_field(json, "created_at")?,
            last_used_at: optional_string_field(json, "last_used_at")?,
        })
    }
}

/// パスキーの一覧の応答から、パスキーの配列を読む (FR-3)。
pub fn passkeys_from_json(json: &Map<String, Value>) -> Result<Vec<Passkey>, AuthError> {
    let items = json
        .get("passkeys")
        .and_then(Value::as_array)
        .ok_or_else(|| format_error("the passkeys field must be an array"))?;
    items
        .iter()
        .map(|item| match item {
            Value::Object(object) => Passkey::from_json(object),
            _ => Err(format_error("a passkey must be a JSON object")),
        })
        .collect()
}

/// 文字列の項目を読む。無い場合と文字列でない場合は失敗にする。
fn string_field(json: &Map<String, Value>, key: &str) -> Result<String, AuthError> {
    json.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format_error(format!("the {key} field must be a string")))
}

/// 文字列の項目を読む。無い場合と `null` は None にする。
fn optional_string_field(
    json: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, AuthError> {
    match json.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format_error(format!("the {key} field must be a string"))),
    }
}

/// 応答の形が規約と違う失敗を作る。
fn format_error(message: impl Into<String>) -> AuthError {
    AuthError::Format(message.into())
}

/// 認証の操作の失敗。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AuthError {
    /// API の呼び出しの失敗。
    Api(ApiCallError),

    /// パスキーの操作の失敗。
    Passkey(PasskeyError),

    /// 応答の形が規約と違う失敗 (パスキーの一覧など)。
    Format(String),
}

impl From<ApiCallError> for AuthError {
    fn from(error: ApiCallError) -> Self {
        Self::Api(error)
    }
}

impl From<PasskeyError> for AuthError {
    fn from(error: PasskeyError) -> Self {
        Self::Passkey(error)
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Api(error) => write!(formatter, "{error}"),
            Self::Passkey(error) => write!(formatter, "{error}"),
            Self::Format(message) => write!(formatter, "FormatError({message})"),
        }
    }
}

impl std::error::Error for AuthError {}

/// パスキーでログインする (FR-2)。利用者名とパスワードは受け取らない。
///
/// 成功してもセッションの状態は変えない。画面が [`SessionStatus::SignedIn`] にする。
pub async fn login(services: &AuthServices) -> Result<(), AuthError> {
    let options = services
        .api
        .post_json("/auth/login/begin", &json!({}))
        .await?;
    let options = RequestOptions::from_json(&options)?;
    let credential = services.passkeys.get_credential(options).await?;
    services
        .api
        .post_json("/auth/login/complete", &json!({ "credential": credential }))
        .await?;
    Ok(())
}

/// 登録用トークンでパスキーを登録する (FR-1)。成功するとセッションが発行される。
///
/// 名前は [`passkey_name_for_request`] で検証した後の値を渡す。成功してもセッションの状態は
/// 変えない。画面が [`SessionStatus::SignedIn`] にする。
pub async fn register(services: &AuthServices, token: &str, name: &str) -> Result<(), AuthError> {
    let options = services
        .api
        .post_json("/auth/register/begin", &json!({ "token": token }))
        .await?;
    let options = CreationOptions::from_json(&options)?;
    let credential = services.passkeys.create_credential(options).await?;
    services
        .api
        .post_json(
            "/auth/register/complete",
            &json!({ "token": token, "name": name, "credential": credential }),
        )
        .await?;
    Ok(())
}

/// ログアウトする (FR-4)。成功してもセッションの状態は変えない。
///
/// 画面が [`SessionStatus::SignedOut`] にし、遷移の判定がログイン画面へ戻す。
pub async fn logout(services: &AuthServices) -> Result<(), AuthError> {
    services.api.post_json("/auth/logout", &json!({})).await?;
    Ok(())
}

/// 登録済みのパスキーの一覧を返す (FR-3)。
pub async fn passkeys(services: &AuthServices) -> Result<Vec<Passkey>, AuthError> {
    passkeys_from_json(&services.api.get_json("/passkeys").await?)
}

/// パスキーを追加する (FR-3)。追加したパスキーを返す。
///
/// 追加は登録 (FR-1) と同じ流れで行い、作成のオプションはセッションの利用者に対して発行される。
/// 名前は [`passkey_name_for_request`] で検証した後の値を渡す。
pub async fn add_passkey(services: &AuthServices, name: &str) -> Result<Passkey, AuthError> {
    let options = services
        .api
        .post_json("/passkeys/begin", &json!({}))
        .await?;
    let options = CreationOptions::from_json(&options)?;
    let credential = services.passkeys.create_credential(options).await?;
    let json = services
        .api
        .post_json(
            "/passkeys/complete",
            &json!({ "name": name, "credential": credential }),
        )
        .await?;
    Passkey::from_json(&json)
}

/// パスキーの名前を変更する (FR-3)。変更したパスキーを返す。
///
/// 名前は [`passkey_name_for_request`] で検証した後の値を渡す。
pub async fn rename_passkey(
    services: &AuthServices,
    id: &str,
    name: &str,
) -> Result<Passkey, AuthError> {
    let json = services
        .api
        .patch_json(&format!("/passkeys/{id}"), &json!({ "name": name }))
        .await?;
    Passkey::from_json(&json)
}

/// パスキーを削除する (FR-3)。最後の 1 つはサーバーが 409 を返す。
pub async fn delete_passkey(services: &AuthServices, id: &str) -> Result<(), AuthError> {
    services.api.delete_json(&format!("/passkeys/{id}")).await?;
    Ok(())
}

/// アカウントと全データを削除する (FR-15)。成功してもセッションの状態は変えない。
///
/// 画面が [`SessionStatus::SignedOut`] にし、遷移の判定がログイン画面へ戻す。
pub async fn delete_account(services: &AuthServices) -> Result<(), AuthError> {
    services.api.delete_json("/account").await?;
    Ok(())
}

/// パスキーの名前の上限 (ADR-0004)。
pub const PASSKEY_NAME_MAX_CHARS: usize = 50;

/// パスキーの名前を検証し、API に送る形 (前後の空白を除いた名前) を返す (FR-1、FR-3)。
///
/// 前後の空白を除いて 1 文字以上 50 文字以下を求める (ADR-0004)。文字数は Unicode のスカラー
/// 値の数で数える (Flutter の `String.runes` と同じ数え方。Backend の `validate_passkey_name`
/// に合わせる)。範囲外は画面に出す文言のキー ([`Key::PasskeyNameError`]) を返す。
pub fn passkey_name_for_request(value: &str) -> Result<String, Key> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > PASSKEY_NAME_MAX_CHARS {
        return Err(Key::PasskeyNameError);
    }
    Ok(name.to_owned())
}

/// 例外を画面に出す文言のキーに変換する (FR-16。Flutter の `messageForError` と同じ)。
pub fn message_key(error: &AuthError) -> Key {
    match error {
        AuthError::Api(ApiCallError::Api(error)) if error.is_unauthorized() => {
            Key::ErrorUnauthorized
        }
        AuthError::Api(ApiCallError::Api(error)) => match error.status {
            400 => Key::ErrorValidation,
            404 => Key::ErrorNotFound,
            409 => Key::ErrorConflict,
            410 => Key::ErrorGone,
            _ => Key::ErrorUnexpected,
        },
        AuthError::Api(ApiCallError::Network(_)) => Key::ErrorNetwork,
        AuthError::Passkey(error) => match error.kind {
            PasskeyErrorKind::Cancelled => Key::PasskeyCancelled,
            PasskeyErrorKind::Unsupported => Key::PasskeyUnsupported,
            PasskeyErrorKind::Failed => Key::ErrorUnexpected,
        },
        AuthError::Format(_) => Key::ErrorUnexpected,
    }
}

/// パスキーの削除の文言のキー (FR-3)。
///
/// 409 は最後の 1 つを消せないことを表すため、追加を促す文言にする (ADR-0004)。
pub fn delete_passkey_error_key(error: &AuthError) -> Key {
    match error {
        AuthError::Api(ApiCallError::Api(error)) if error.status == 409 => {
            Key::PasskeyLastDeleteError
        }
        _ => message_key(error),
    }
}

/// 登録の画面の文言のキー (FR-1。Flutter の `registerErrorMessage` と同じ)。
///
/// 登録の経路の 404、409、410 は登録用トークンの問題を表すため、専用の文言にする。
/// 400 は名前の不正とクレデンシャルの検証の失敗のどちらでも返るため、名前だけを指す文言に
/// せず、入力の確認を促す文言にする (名前は画面側でも検証する)。
pub fn register_error_key(error: &AuthError) -> Key {
    match error {
        AuthError::Api(ApiCallError::Api(api_error)) => match api_error.status {
            400 => Key::ErrorValidation,
            404 => Key::RegisterTokenNotFound,
            409 => Key::RegisterTokenUsed,
            410 => Key::RegisterTokenExpired,
            _ => message_key(error),
        },
        _ => message_key(error),
    }
}

/// ログインの画面の文言のキー (FR-2。Flutter の `loginErrorMessage` と同じ)。
///
/// 401 以外の API のエラー (チャレンジの検証の失敗、期限切れ、再利用) は、やり直しで回復する。
pub fn login_error_key(error: &AuthError) -> Key {
    match error {
        AuthError::Api(ApiCallError::Api(error)) if !error.is_unauthorized() => Key::LoginFailed,
        _ => message_key(error),
    }
}

/// 登録用のリンクが無効か (404、409、410)。無効のときは画面がフォームを出さない (FR-1)。
pub fn is_invalid_registration_token(error: &AuthError) -> bool {
    matches!(
        error,
        AuthError::Api(ApiCallError::Api(error)) if matches!(error.status, 404 | 409 | 410)
    )
}
