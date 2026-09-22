//! 登録用トークンによるパスキーの登録 (FR-1、ADR-0004)。
//!
//! 登録用トークンは管理者 (0018) が発行し、この経路では利用者を増やさない。
//! トークンは 32 バイトの乱数を base64url で符号化した文字列で、D1 には SHA-256 の
//! ハッシュだけを保存する。存在しないトークンは 404、使用済みは 409、期限切れは 410 を返す。
//! 登録が成功したら、トークンを使用済みにし、パスキーを利用者に紐づけ、セッションを発行する。

use coffee_log_core::auth::{self, validate_passkey_name};
use coffee_log_core::base64url;
use coffee_log_core::webauthn::{self, RegistrationInput};
use serde::Deserialize;
use worker::d1::{D1Database, D1Type};
use worker::{console_error, Env, Error, Request, Response, Result};

use super::session;
use super::{
    bad_request, conflict, consume_challenge, creation_options, database, delete_challenge,
    execute_batch, execute_changes, find_challenge, gone, issue_challenge, new_id, not_found,
    now_text, statement, ChallengeLookup, Config,
};
use crate::respond;

/// `POST /api/auth/register/begin` の入力。
#[derive(Debug, Deserialize)]
struct BeginInput {
    /// 管理者が発行した登録用トークン。
    token: String,
}

/// `POST /api/auth/register/complete` の入力。
#[derive(Debug, Deserialize)]
struct CompleteInput {
    /// 管理者が発行した登録用トークン。
    token: String,
    /// パスキーの名前。
    name: String,
    /// クライアントが作ったクレデンシャル。
    credential: RegistrationCredential,
}

/// 登録のクレデンシャル。`navigator.credentials.create` の JSON 表記。
/// クレデンシャル ID は authenticatorData から取り出すため (0004)、`id` は読まない。
#[derive(Debug, Deserialize)]
struct RegistrationCredential {
    response: RegistrationResponse,
}

/// 登録の応答。base64url の文字列で運ばれる。
#[derive(Debug, Deserialize)]
struct RegistrationResponse {
    #[serde(rename = "clientDataJSON")]
    client_data_json: String,
    #[serde(rename = "attestationObject")]
    attestation_object: String,
}

/// 登録のチャレンジを発行する。認証は不要。
pub async fn begin(req: &mut Request, env: &Env) -> Result<Response> {
    let input: BeginInput = match req.json().await {
        Ok(input) => input,
        Err(_) => return Ok(bad_request("the body must be JSON with a token")),
    };
    if input.token.trim().is_empty() {
        return Ok(bad_request("the token is missing"));
    }
    let config = Config::from_env(env)?;
    let d1 = database(env)?;
    let user_id = match find_token(&d1, &input.token).await? {
        TokenLookup::Valid { user_id, .. } => user_id,
        TokenLookup::Missing => return Ok(not_found("the registration token does not exist")),
        TokenLookup::Used => return Ok(conflict("the registration token has already been used")),
        TokenLookup::Expired => return Ok(gone("the registration token has expired")),
    };
    let challenge = issue_challenge(
        &d1,
        Some(&user_id),
        auth::KIND_REGISTRATION,
        config.challenge_ttl_seconds,
    )
    .await?;
    respond::json(&creation_options(&challenge, &user_id, &config)?)
}

/// 登録を検証し、パスキーを利用者に紐づけ、セッションを発行する。認証は不要。
pub async fn complete(req: &mut Request, env: &Env) -> Result<Response> {
    let input: CompleteInput = match req.json().await {
        Ok(input) => input,
        Err(_) => {
            return Ok(bad_request(
                "the body must be JSON with a token, a name and a credential",
            ))
        }
    };
    if input.token.trim().is_empty() {
        return Ok(bad_request("the token is missing"));
    }
    let name = match validate_passkey_name(&input.name) {
        Ok(name) => name,
        Err(error) => return Ok(bad_request(&error.to_string())),
    };
    let config = Config::from_env(env)?;
    let d1 = database(env)?;
    let (token_id, user_id) = match find_token(&d1, &input.token).await? {
        TokenLookup::Valid { id, user_id } => (id, user_id),
        TokenLookup::Missing => return Ok(not_found("the registration token does not exist")),
        TokenLookup::Used => return Ok(conflict("the registration token has already been used")),
        TokenLookup::Expired => return Ok(gone("the registration token has expired")),
    };

    // 登録の検証では、トークンの利用者と種別 (登録) とチャレンジ値の 3 つで行を引く。
    let challenge =
        match webauthn::client_data_challenge(&input.credential.response.client_data_json) {
            Ok(challenge) => challenge,
            Err(error) => {
                console_error!("the registration credential is not readable: {error}");
                return Ok(bad_request("the credential is not valid"));
            }
        };
    let challenge_id =
        match find_challenge(&d1, &challenge, auth::KIND_REGISTRATION, Some(&user_id)).await? {
            ChallengeLookup::Found { id } => id,
            ChallengeLookup::Missing => {
                return Ok(conflict(
                    "the challenge does not exist or has already been used",
                ))
            }
            ChallengeLookup::Expired { id } => {
                delete_challenge(&d1, &id)?.run().await?;
                return Ok(gone("the challenge has expired"));
            }
        };

    let registered = match webauthn::verify_registration(&RegistrationInput {
        rp_id: &config.rp_id,
        origin: &config.origin,
        expected_challenge: &challenge,
        client_data_json: &input.credential.response.client_data_json,
        attestation_object: &input.credential.response.attestation_object,
    }) {
        Ok(registered) => registered,
        Err(error) => {
            console_error!("the registration was rejected: {error}");
            return Ok(bad_request("the credential could not be verified"));
        }
    };

    // 検証に成功したチャレンジは使い切る。削除できたときだけ続ける (同時の登録で 2 回使われないようにする)。
    if !consume_challenge(&d1, &challenge_id).await? {
        return Ok(conflict(
            "the challenge does not exist or has already been used",
        ));
    }

    // トークンを使用済みにする。同じトークンの同時の登録を通さないよう、未使用のときだけ印を付ける。
    // 印を付けた後にパスキーの保存が失敗した場合、トークンは使用済みのまま残る (登録はやり直せない)。
    let now = now_text()?;
    let token_values = [D1Type::Text(&now), D1Type::Text(&token_id)];
    let changes = execute_changes(&d1, MARK_TOKEN_USED, &token_values).await?;
    if changes == 0 {
        return Ok(conflict("the registration token has already been used"));
    }

    // パスキーを利用者に紐づけ、セッションを発行する。1 つのまとまりで行う。
    let credential_id = base64url::encode(&registered.credential_id);
    let public_key = base64url::encode(&registered.cose_public_key);
    let sign_count = i32::try_from(registered.sign_count)
        .map_err(|_| Error::RustError("the signature counter does not fit in D1".to_owned()))?;
    let credential_row_id = new_id()?;
    let credential_values = [
        D1Type::Text(&credential_row_id),
        D1Type::Text(&user_id),
        D1Type::Text(&credential_id),
        D1Type::Text(&public_key),
        D1Type::Integer(sign_count),
        D1Type::Text(&name),
        D1Type::Text(&now),
    ];
    let insert_credential = statement(&d1, INSERT_CREDENTIAL, &credential_values)?;
    let (insert_session, cookie) = session::prepare(&d1, &user_id, config.session_ttl_seconds)?;
    execute_batch(&d1, vec![insert_credential, insert_session]).await?;

    // 応答は利用者と、作ったパスキーを返す。
    respond::json_with_cookie(
        &RegistrationResult {
            user_id,
            passkey: super::PasskeyResponse {
                id: credential_row_id,
                name,
                created_at: now,
                last_used_at: None,
            },
        },
        &cookie,
    )
}

/// 登録の検証の結果に使う行。利用者と、作ったパスキーの応答を返す。
#[derive(Debug, serde::Serialize)]
struct RegistrationResult {
    user_id: String,
    passkey: super::PasskeyResponse,
}

/// 登録用トークンを引いた結果。
enum TokenLookup {
    /// 未使用で有効期限内のトークン。
    Valid { id: String, user_id: String },
    /// 一致するハッシュの行が無い。
    Missing,
    /// 使用日時が記録済み。
    Used,
    /// 有効期限を過ぎている。
    Expired,
}

/// 登録用トークンのハッシュで行を引く。
async fn find_token(d1: &D1Database, token: &str) -> Result<TokenLookup> {
    let hash = auth::hash_secret(token);
    let values = [D1Type::Text(&hash)];
    let row: Option<TokenRow> = statement(d1, SELECT_TOKEN, &values)?.first(None).await?;
    let Some(row) = row else {
        return Ok(TokenLookup::Missing);
    };
    if row.used_at.is_some() {
        return Ok(TokenLookup::Used);
    }
    if auth::is_expired(&row.expires_at, &now_text()?) {
        return Ok(TokenLookup::Expired);
    }
    Ok(TokenLookup::Valid {
        id: row.id,
        user_id: row.user_id,
    })
}

/// 登録用トークンの行のうち検証に使う列。
#[derive(Debug, Deserialize)]
struct TokenRow {
    id: String,
    user_id: String,
    expires_at: String,
    used_at: Option<String>,
}

const SELECT_TOKEN: &str = "SELECT id, user_id, expires_at, used_at FROM registration_tokens \
                            WHERE token_hash = ?";
const MARK_TOKEN_USED: &str =
    "UPDATE registration_tokens SET used_at = ? WHERE id = ? AND used_at IS NULL";
const INSERT_CREDENTIAL: &str = "INSERT INTO passkey_credentials (id, user_id, credential_id, \
                                 public_key, sign_count, name, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)";
