//! パスキーによるログイン (FR-2、ADR-0004)。
//!
//! ログインのチャレンジ発行は入力を持たず、利用者名も受け取らない。`allowCredentials` は空にし、
//! discoverable なパスキーだけを対象にする。検証では `clientDataJSON` から取り出したチャレンジ値と
//! 種別 (ログイン) で行を引く (ADR-0006 の「チャレンジ値で引く」)。
//!
//! 署名カウンタの検査は 0004 の純粋な関数 [`webauthn::check_sign_count`] を使い、拒否なら 409 を
//! 返してログインを拒否し、受理なら保存値を更新する。ログインに成功したら、そのパスキーの
//! 最終使用日時を更新する。

use brew_book_core::auth;
use brew_book_core::base64url;
use brew_book_core::webauthn::{self, AuthenticationInput, SignCounter};
use serde::{Deserialize, Serialize};
use worker::d1::D1Type;
use worker::{console_error, Env, Error, Request, Response, Result};

use super::session;
use super::{
    bad_request, conflict, consume_challenge, database, delete_challenge, execute_changes,
    find_challenge, gone, issue_challenge, now_text, statement, ChallengeLookup, Config,
};
use crate::respond;

/// `POST /api/auth/login/begin` の応答。クライアントが `navigator.credentials.get` に渡す。
#[derive(Debug, Serialize)]
struct RequestOptions {
    challenge: String,
    #[serde(rename = "rpId")]
    rp_id: String,
    #[serde(rename = "userVerification")]
    user_verification: &'static str,
    /// 空にする。利用者名を受け取らないため、discoverable なパスキーだけが対象になる (FR-2)。
    #[serde(rename = "allowCredentials")]
    allow_credentials: Vec<String>,
    /// ミリ秒。チャレンジの有効期限に合わせる。
    timeout: u32,
}

/// `POST /api/auth/login/complete` の入力。
#[derive(Debug, Deserialize)]
struct CompleteInput {
    /// クライアントが作ったアサーション。
    credential: AuthenticationCredential,
}

/// ログインのクレデンシャル。`navigator.credentials.get` の JSON 表記。
#[derive(Debug, Deserialize)]
struct AuthenticationCredential {
    /// クレデンシャル ID (base64url)。`passkey_credentials.credential_id` と照合する。
    id: String,
    response: AuthenticationResponse,
}

/// ログインの応答。base64url の文字列で運ばれる。
#[derive(Debug, Deserialize)]
struct AuthenticationResponse {
    #[serde(rename = "clientDataJSON")]
    client_data_json: String,
    #[serde(rename = "authenticatorData")]
    authenticator_data: String,
    signature: String,
}

/// ログインの結果。発行したセッションの利用者を返す。
#[derive(Debug, Serialize)]
struct LoginResult {
    user_id: String,
}

/// ログインのチャレンジを発行する。入力を持たず、認証も不要。
pub async fn begin(_req: &mut Request, env: &Env) -> Result<Response> {
    let config = Config::from_env(env)?;
    let d1 = database(env)?;
    // 利用者を特定しないため、同じ種別の行が複数できる (ADR-0006)。
    let challenge = issue_challenge(
        &d1,
        None,
        auth::KIND_AUTHENTICATION,
        config.challenge_ttl_seconds,
    )
    .await?;
    respond::json(&RequestOptions {
        challenge,
        rp_id: config.rp_id.clone(),
        user_verification: "required",
        allow_credentials: Vec::new(),
        timeout: super::timeout_millis(config.challenge_ttl_seconds),
    })
}

/// ログインを検証し、セッションを発行する。認証は不要。
pub async fn complete(req: &mut Request, env: &Env) -> Result<Response> {
    let input: CompleteInput = match req.json().await {
        Ok(input) => input,
        Err(_) => return Ok(bad_request("the body must be JSON with a credential")),
    };
    let config = Config::from_env(env)?;
    let d1 = database(env)?;

    let challenge =
        match webauthn::client_data_challenge(&input.credential.response.client_data_json) {
            Ok(challenge) => challenge,
            Err(error) => {
                console_error!("the login credential is not readable: {error}");
                return Ok(bad_request("the credential is not valid"));
            }
        };
    let challenge_id =
        match find_challenge(&d1, &challenge, auth::KIND_AUTHENTICATION, None).await? {
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

    // クレデンシャル ID は全利用者で一意である (ADR-0006)。
    let values = [D1Type::Text(&input.credential.id)];
    let credential: Option<CredentialRow> = statement(&d1, SELECT_CREDENTIAL, &values)?
        .first(None)
        .await?;
    let Some(credential) = credential else {
        return Ok(bad_request("the credential is not registered"));
    };
    let public_key = base64url::decode(&credential.public_key).map_err(|error| {
        Error::RustError(format!(
            "the stored public key is not valid base64url: {error}"
        ))
    })?;

    let current = match webauthn::verify_authentication(&AuthenticationInput {
        rp_id: &config.rp_id,
        origin: &config.origin,
        expected_challenge: &challenge,
        client_data_json: &input.credential.response.client_data_json,
        authenticator_data: &input.credential.response.authenticator_data,
        signature: &input.credential.response.signature,
        cose_public_key: &public_key,
    }) {
        Ok(current) => current,
        Err(error) => {
            console_error!("the login was rejected: {error}");
            return Ok(bad_request("the credential could not be verified"));
        }
    };

    // 署名カウンタは両方 0 なら検査を省略し、今回の値が保存値以下なら拒否する (ADR-0004)。
    let stored = u32::try_from(credential.sign_count).unwrap_or(0);
    let counter = match webauthn::check_sign_count(stored, current) {
        Ok(counter) => counter,
        Err(_) => {
            return Ok(conflict("the signature counter did not increase"));
        }
    };
    let now = now_text()?;
    match counter {
        SignCounter::Skipped => {
            let values = [D1Type::Text(&now), D1Type::Text(&credential.id)];
            execute_changes(&d1, UPDATE_LAST_USED, &values).await?;
        }
        SignCounter::Updated(updated) => {
            let updated = i32::try_from(updated).map_err(|_| {
                Error::RustError("the signature counter does not fit in D1".to_owned())
            })?;
            // 保存値より大きいときだけ更新する。同時のログインで保存値が後退しないようにする。
            let values = [
                D1Type::Integer(updated),
                D1Type::Text(&now),
                D1Type::Text(&credential.id),
                D1Type::Integer(updated),
            ];
            let changes = execute_changes(&d1, UPDATE_SIGN_COUNT_IF_GREATER, &values).await?;
            if changes == 0 {
                return Ok(conflict("the signature counter did not increase"));
            }
        }
    }

    // 検証に成功したチャレンジは使い切る。削除できたときだけ続ける (同時の検証で 2 回使われないようにする)。
    if !consume_challenge(&d1, &challenge_id).await? {
        return Ok(conflict(
            "the challenge does not exist or has already been used",
        ));
    }

    let (insert_session, cookie) =
        session::prepare(&d1, &credential.user_id, config.session_ttl_seconds)?;
    insert_session.run().await?;

    respond::json_with_cookie(
        &LoginResult {
            user_id: credential.user_id,
        },
        &cookie,
    )
}

/// クレデンシャルの行のうち検証に使う列。
#[derive(Debug, Deserialize)]
struct CredentialRow {
    id: String,
    user_id: String,
    public_key: String,
    sign_count: i64,
}

pub(crate) const SELECT_CREDENTIAL: &str = "SELECT id, user_id, public_key, sign_count FROM \
                                 passkey_credentials WHERE credential_id = ?";
pub(crate) const UPDATE_LAST_USED: &str =
    "UPDATE passkey_credentials SET last_used_at = ? WHERE id = ?";
/// 保存値より大きいときだけ署名カウンタを更新する (同時のログインで保存値が後退しないようにする)。
pub(crate) const UPDATE_SIGN_COUNT_IF_GREATER: &str =
    "UPDATE passkey_credentials SET sign_count = ?, last_used_at = ? WHERE id = ? AND sign_count < ?";
