//! パスキーの登録とログインとセッション (ADR-0004)。
//!
//! 認証が不要なのは、登録のチャレンジ発行と検証、ログインのチャレンジ発行と検証の 4 経路だけとする。
//! それ以外の利用者向けの経路はセッションを必須にし、セッションの解決は [`session::resolve`] に
//! 集約する (PRD のセキュリティ)。
//!
//! 秘密値 (登録用トークン、チャレンジ、セッションのトークン) は 32 バイトの乱数を base64url で
//! 符号化した文字列とし、D1 には SHA-256 のハッシュだけを保存する。生の値は応答と Cookie にだけ
//! 載せる。有効期限は ISO 8601 UTC の固定長文字列で持ち、辞書順の比較で判定する
//! (`coffee_log_core::auth`)。

pub mod login;
pub mod passkeys;
pub mod register;
pub mod session;

use coffee_log_core::auth;
use coffee_log_core::error::ErrorCode;
use serde::Serialize;
use worker::d1::{D1Database, D1PreparedStatement, D1Type};
use worker::{Date, Env, Error, Result};

use crate::random;
use crate::respond;

/// D1 の実行と現在時刻と UUID の補助は、記録の API (0006) と共有する (`crate::db`)。
pub use crate::db::{database, execute_batch, execute_changes, new_id, now_text, statement};

/// チャレンジの有効期限の vars の名前。テストが `--var` で短縮する (PRD の「制約と前提」)。
pub const CHALLENGE_TTL_VAR: &str = "CHALLENGE_TTL_SECONDS";
/// セッションの有効期限の vars の名前。テストが `--var` で短縮する。
pub const SESSION_TTL_VAR: &str = "SESSION_TTL_SECONDS";
/// Relying Party ID の vars の名前。
pub const RP_ID_VAR: &str = "RP_ID";
/// RP の Origin の vars の名前。
pub const ORIGIN_VAR: &str = "ORIGIN";
/// Relying Party ID の既定値。ローカルの `wrangler dev` の値とし、本番は 0017 が vars で設定する。
pub const DEFAULT_RP_ID: &str = "localhost";
/// Origin の既定値。ローカルの `wrangler dev` の値とし、本番は 0017 が vars で設定する。
pub const DEFAULT_ORIGIN: &str = "http://localhost:8787";

/// 認証の設定。vars から読む。
pub struct Config {
    /// Relying Party ID (例: `coffee-log.example.workers.dev`)。
    pub rp_id: String,
    /// RP の Origin (例: `https://coffee-log.example.workers.dev`)。
    pub origin: String,
    /// チャレンジの有効期限 (秒)。
    pub challenge_ttl_seconds: i64,
    /// セッションの有効期限 (秒)。
    pub session_ttl_seconds: i64,
}

impl Config {
    /// vars から設定を読む。有効期限の vars が無ければ ADR-0004 の値を使う。
    pub fn from_env(env: &Env) -> Result<Self> {
        Ok(Self {
            rp_id: var_or(env, RP_ID_VAR, DEFAULT_RP_ID)?,
            origin: var_or(env, ORIGIN_VAR, DEFAULT_ORIGIN)?,
            challenge_ttl_seconds: ttl_var_or(env, CHALLENGE_TTL_VAR, auth::CHALLENGE_TTL_SECONDS)?,
            session_ttl_seconds: ttl_var_or(env, SESSION_TTL_VAR, auth::SESSION_TTL_SECONDS)?,
        })
    }
}

/// 登録の作成のオプション (W3C WebAuthn Level 3 の 5.4)。
///
/// クライアントが `navigator.credentials.create` にほぼそのまま渡せる形で返す。
/// `attestation` は `none`、`residentKey` は `preferred`、`userVerification` は `required`、
/// 公開鍵のアルゴリズムは ES256 (alg -7) だけとする (ADR-0004)。
/// `user.id` は利用者の UUID の 16 バイト、`user.name` は利用者の UUID の文字列とし、
/// `user.displayName` は設定しない (表示名は管理者が利用者を識別するためだけのもの。FR-17)。
#[derive(Debug, Serialize)]
pub struct CreationOptions {
    pub challenge: String,
    pub rp: RelyingParty,
    pub user: UserEntity,
    #[serde(rename = "pubKeyCredParams")]
    pub pub_key_cred_params: Vec<PublicKeyCredentialParameter>,
    pub attestation: &'static str,
    #[serde(rename = "authenticatorSelection")]
    pub authenticator_selection: AuthenticatorSelection,
    /// ミリ秒。チャレンジの有効期限に合わせる。
    pub timeout: u32,
}

/// `rp` の内容。
#[derive(Debug, Serialize)]
pub struct RelyingParty {
    pub id: String,
    pub name: &'static str,
}

/// `user` の内容。`displayName` は持たない (ADR-0004)。
#[derive(Debug, Serialize)]
pub struct UserEntity {
    pub id: String,
    pub name: String,
}

/// `pubKeyCredParams` の要素。
#[derive(Debug, Serialize)]
pub struct PublicKeyCredentialParameter {
    #[serde(rename = "type")]
    pub type_: &'static str,
    pub alg: i32,
}

/// `authenticatorSelection` の内容。
#[derive(Debug, Serialize)]
pub struct AuthenticatorSelection {
    #[serde(rename = "residentKey")]
    pub resident_key: &'static str,
    #[serde(rename = "userVerification")]
    pub user_verification: &'static str,
}

/// 登録の作成のオプションを組み立てる。
pub fn creation_options(
    challenge: &str,
    user_id: &str,
    config: &Config,
) -> Result<CreationOptions> {
    Ok(CreationOptions {
        challenge: challenge.to_owned(),
        rp: RelyingParty {
            id: config.rp_id.clone(),
            name: "coffee-log",
        },
        user: UserEntity {
            id: user_id_bytes(user_id)?,
            name: user_id.to_owned(),
        },
        pub_key_cred_params: vec![PublicKeyCredentialParameter {
            type_: "public-key",
            alg: -7,
        }],
        attestation: "none",
        authenticator_selection: AuthenticatorSelection {
            resident_key: "preferred",
            user_verification: "required",
        },
        timeout: timeout_millis(config.challenge_ttl_seconds),
    })
}

/// チャレンジの有効期限 (秒) を、クライアントに渡すミリ秒にする。
fn timeout_millis(ttl_seconds: i64) -> u32 {
    let millis = ttl_seconds
        .saturating_mul(1000)
        .clamp(0, i64::from(u32::MAX));
    u32::try_from(millis).unwrap_or(u32::MAX)
}

/// vars を読み、無ければ既定値を返す。
pub fn var_or(env: &Env, name: &str, default: &str) -> Result<String> {
    match env.var(name) {
        Ok(var) => Ok(var.to_string()),
        Err(_) => Ok(default.to_owned()),
    }
}

/// 秒数の vars を読み、無ければ既定値を返す。
fn ttl_var_or(env: &Env, name: &str, default: i64) -> Result<i64> {
    let text = var_or(env, name, "")?;
    if text.is_empty() {
        return Ok(default);
    }
    text.parse::<i64>().map_err(|_| {
        Error::RustError(format!(
            "the {name} var must be an integer number of seconds but was {text}"
        ))
    })
}

/// 現在時刻と有効期限の秒数から、有効期限の時刻の文字列を作る。
pub fn expiry_text(ttl_seconds: i64) -> Result<String> {
    let now = Date::now().as_millis() as i64;
    auth::expiry_from(now, ttl_seconds)
        .map_err(|error| Error::RustError(format!("failed to format the expiry: {error:?}")))
}

/// 乱数の失敗を Worker のエラーにする。
pub fn random_bytes_32() -> Result<[u8; 32]> {
    random::bytes_32().map_err(Error::RustError)
}

/// 利用者 ID の 16 バイトを base64url にした文字列。WebAuthn の `user.id` に入れる (ADR-0004)。
pub fn user_id_bytes(user_id: &str) -> Result<String> {
    let bytes = coffee_log_core::ids::uuid_bytes(user_id)
        .ok_or_else(|| Error::RustError(format!("the user id is not a UUID: {user_id}")))?;
    Ok(coffee_log_core::base64url::encode(&bytes))
}

/// 期限切れのチャレンジの行を削除する。
/// 認証不要のログインのチャレンジ発行で行が増え続けないように、発行のたびに呼ぶ (ADR-0004)。
async fn delete_expired_challenges(d1: &D1Database, now: &str) -> Result<()> {
    let values = [D1Type::Text(now)];
    statement(d1, DELETE_EXPIRED_CHALLENGES, &values)?
        .run()
        .await?;
    Ok(())
}

/// 期限切れのチャレンジを削除し、新しいチャレンジの行を入れて、チャレンジの値を返す。
/// 登録のチャレンジは利用者を持ち、ログインのチャレンジは利用者を持たない (ADR-0006)。
pub async fn issue_challenge(
    d1: &D1Database,
    user_id: Option<&str>,
    kind: &str,
    ttl_seconds: i64,
) -> Result<String> {
    let now = now_text()?;
    delete_expired_challenges(d1, &now).await?;

    let challenge = auth::encode_secret(random_bytes_32()?);
    let id = new_id()?;
    let expires_at = expiry_text(ttl_seconds)?;
    let values = [
        D1Type::Text(&id),
        user_id.map_or(D1Type::Null, D1Type::Text),
        D1Type::Text(&challenge),
        D1Type::Text(kind),
        D1Type::Text(&expires_at),
    ];
    statement(d1, INSERT_CHALLENGE, &values)?.run().await?;
    Ok(challenge)
}

/// チャレンジの行を引いた結果。
pub enum ChallengeLookup {
    /// 有効期限内の行がある。
    Found { id: String },
    /// 一致する行が無い (使用済みの再利用か、存在しない値か、発行のたびの削除の後)。
    Missing,
    /// 行はあるが有効期限を過ぎている。呼び出し側が行を削除する。
    Expired { id: String },
}

/// チャレンジ値に一致する行を、チャレンジ値と種別で引く (ADR-0006 の「チャレンジ値で引く」)。
/// 登録の検証では、トークンの利用者と種別とチャレンジ値の 3 つで引く。
pub async fn find_challenge(
    d1: &D1Database,
    challenge: &str,
    kind: &str,
    user_id: Option<&str>,
) -> Result<ChallengeLookup> {
    let now = now_text()?;
    let row: Option<ChallengeRow> = match user_id {
        Some(user_id) => {
            let values = [
                D1Type::Text(challenge),
                D1Type::Text(kind),
                D1Type::Text(user_id),
            ];
            statement(d1, SELECT_CHALLENGE_WITH_USER, &values)?
                .first(None)
                .await?
        }
        None => {
            let values = [D1Type::Text(challenge), D1Type::Text(kind)];
            statement(d1, SELECT_CHALLENGE_WITHOUT_USER, &values)?
                .first(None)
                .await?
        }
    };
    let Some(row) = row else {
        return Ok(ChallengeLookup::Missing);
    };
    if auth::is_expired(&row.expires_at, &now) {
        return Ok(ChallengeLookup::Expired { id: row.id });
    }
    Ok(ChallengeLookup::Found { id: row.id })
}

/// チャレンジの行を削除する文。使用したチャレンジは 1 回で使い切る (ADR-0004)。
pub fn delete_challenge(d1: &D1Database, id: &str) -> Result<D1PreparedStatement> {
    let values = [D1Type::Text(id)];
    statement(d1, DELETE_CHALLENGE, &values)
}

/// チャレンジを使い切る。1 行だけ削除できたときだけ真を返す。
///
/// 検証に成功したチャレンジは 1 回で使い切る (ADR-0004)。削除の変更行数を見ることで、
/// 同じチャレンジを使った 2 本の検証が同時に来ても、続くのは 1 本だけになる。
pub async fn consume_challenge(d1: &D1Database, id: &str) -> Result<bool> {
    let changes = execute_changes(d1, DELETE_CHALLENGE, &[D1Type::Text(id)]).await?;
    Ok(changes == 1)
}

/// チャレンジの行のうち検証に使う列。
#[derive(serde::Deserialize)]
struct ChallengeRow {
    id: String,
    expires_at: String,
}

/// パスキーの一覧と 1 件の応答。名前、登録日時、最終使用日時を返す (FR-3)。
#[derive(Debug, Serialize)]
pub struct PasskeyResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// パスキーの行のうち応答に使う列。
#[derive(Debug, serde::Deserialize)]
pub struct PasskeyRow {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

impl From<PasskeyRow> for PasskeyResponse {
    fn from(row: PasskeyRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
        }
    }
}

/// パスキーの一覧の応答。
#[derive(Debug, Serialize)]
pub struct PasskeyListResponse {
    pub passkeys: Vec<PasskeyResponse>,
}

/// 認証不要の 4 経路以外のエラー応答。認証の要否は経路の台帳 (0001) が持つ。
pub fn unauthorized() -> worker::Response {
    respond::error(ErrorCode::Unauthorized, "the session is missing or expired")
}

/// 400 の応答を組み立てる。
pub fn bad_request(message: &str) -> worker::Response {
    respond::error(ErrorCode::BadRequest, message)
}

/// 404 の応答を組み立てる。存在しない ID と他の利用者の ID は区別しない (ADR-0006)。
pub fn not_found(message: &str) -> worker::Response {
    respond::error(ErrorCode::NotFound, message)
}

/// 409 の応答を組み立てる。
pub fn conflict(message: &str) -> worker::Response {
    respond::error(ErrorCode::Conflict, message)
}

/// 410 の応答を組み立てる。
pub fn gone(message: &str) -> worker::Response {
    respond::error(ErrorCode::Gone, message)
}

const INSERT_CHALLENGE: &str = "INSERT INTO webauthn_challenges (id, user_id, challenge, kind, \
                                expires_at) VALUES (?, ?, ?, ?, ?)";
const DELETE_EXPIRED_CHALLENGES: &str = "DELETE FROM webauthn_challenges WHERE expires_at <= ?";
const DELETE_CHALLENGE: &str = "DELETE FROM webauthn_challenges WHERE id = ?";
const SELECT_CHALLENGE_WITH_USER: &str = "SELECT id, expires_at FROM webauthn_challenges \
                                           WHERE challenge = ? AND kind = ? AND user_id = ?";
const SELECT_CHALLENGE_WITHOUT_USER: &str = "SELECT id, expires_at FROM webauthn_challenges \
                                              WHERE challenge = ? AND kind = ? AND user_id IS NULL";
