//! パスキーの追加と名前の変更と削除と一覧 (FR-3、ADR-0004)。
//!
//! 追加は登録と同じ検証を使い、チャレンジはセッションの利用者と種別 (登録) とチャレンジ値の 3 つで
//! 引く。名前は前後の空白を除いて 1 文字以上 50 文字以下とし、範囲外は 400 を返す。
//! 最後の 1 つは削除できず 409、他の利用者のパスキーの変更と削除は 404 を返す (ADR-0006)。

use brew_book_core::auth::{self, validate_passkey_name};
use brew_book_core::base64url;
use brew_book_core::webauthn::{self, RegistrationInput};
use serde::Deserialize;
use worker::d1::D1Type;
use worker::{console_error, Env, Error, Request, Response, Result};

use super::session::Session;
use super::{
    bad_request, conflict, consume_challenge, creation_options, database, delete_challenge,
    execute_batch, find_challenge, gone, issue_challenge, new_id, not_found, now_text, statement,
    ChallengeLookup, Config, PasskeyListResponse, PasskeyResponse, PasskeyRow,
};
use crate::respond;

/// `POST /api/passkeys/complete` の入力。
#[derive(Debug, Deserialize)]
struct AddInput {
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

/// `PATCH /api/passkeys/<ID>` の入力。
#[derive(Debug, Deserialize)]
struct RenameInput {
    /// 新しい名前。
    name: String,
}

/// パスキーの一覧を返す。認証が必要。
pub async fn list(env: &Env, session: &Session) -> Result<Response> {
    let d1 = database(env)?;
    let values = [D1Type::Text(&session.user_id)];
    let rows: Vec<PasskeyRow> = statement(&d1, SELECT_PASSKEYS, &values)?
        .all()
        .await?
        .results()?;
    respond::json(&PasskeyListResponse {
        passkeys: rows.into_iter().map(PasskeyResponse::from).collect(),
    })
}

/// パスキーの追加のチャレンジを発行する。認証が必要。
pub async fn begin(_req: &mut Request, env: &Env, session: &Session) -> Result<Response> {
    let config = Config::from_env(env)?;
    let d1 = database(env)?;
    let challenge = issue_challenge(
        &d1,
        Some(&session.user_id),
        auth::KIND_REGISTRATION,
        config.challenge_ttl_seconds,
    )
    .await?;
    respond::json(&creation_options(&challenge, &session.user_id, &config)?)
}

/// パスキーの追加を検証し、セッションの利用者に紐づける。認証が必要。
pub async fn complete(req: &mut Request, env: &Env, session: &Session) -> Result<Response> {
    let input: AddInput = match req.json().await {
        Ok(input) => input,
        Err(_) => {
            return Ok(bad_request(
                "the body must be JSON with a name and a credential",
            ))
        }
    };
    let name = match validate_passkey_name(&input.name) {
        Ok(name) => name,
        Err(error) => return Ok(bad_request(&error.to_string())),
    };
    let config = Config::from_env(env)?;
    let d1 = database(env)?;

    let challenge =
        match webauthn::client_data_challenge(&input.credential.response.client_data_json) {
            Ok(challenge) => challenge,
            Err(error) => {
                console_error!("the add credential is not readable: {error}");
                return Ok(bad_request("the credential is not valid"));
            }
        };
    let challenge_id = match find_challenge(
        &d1,
        &challenge,
        auth::KIND_REGISTRATION,
        Some(&session.user_id),
    )
    .await?
    {
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
            console_error!("the add was rejected: {error}");
            return Ok(bad_request("the credential could not be verified"));
        }
    };

    // 検証に成功したチャレンジは使い切る。削除できたときだけ続ける (同時の追加で 2 回使われないようにする)。
    if !consume_challenge(&d1, &challenge_id).await? {
        return Ok(conflict(
            "the challenge does not exist or has already been used",
        ));
    }

    let credential_id = base64url::encode(&registered.credential_id);
    let public_key = base64url::encode(&registered.cose_public_key);
    let sign_count = i32::try_from(registered.sign_count)
        .map_err(|_| Error::RustError("the signature counter does not fit in D1".to_owned()))?;
    let row_id = new_id()?;
    let now = now_text()?;
    let values = [
        D1Type::Text(&row_id),
        D1Type::Text(&session.user_id),
        D1Type::Text(&credential_id),
        D1Type::Text(&public_key),
        D1Type::Integer(sign_count),
        D1Type::Text(&name),
        D1Type::Text(&now),
    ];
    let insert = statement(&d1, INSERT_CREDENTIAL, &values)?;
    execute_batch(&d1, vec![insert]).await?;

    respond::json(&PasskeyResponse {
        id: row_id,
        name,
        created_at: now,
        last_used_at: None,
    })
}

/// パスキーの名前を変更する。認証が必要。
/// 他の利用者のパスキーと存在しない ID は区別せず 404 を返す (ADR-0006)。
pub async fn rename(
    req: &mut Request,
    env: &Env,
    session: &Session,
    id: Option<&str>,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the passkey does not exist"));
    };
    let input: RenameInput = match req.json().await {
        Ok(input) => input,
        Err(_) => return Ok(bad_request("the body must be JSON with a name")),
    };
    let name = match validate_passkey_name(&input.name) {
        Ok(name) => name,
        Err(error) => return Ok(bad_request(&error.to_string())),
    };
    let d1 = database(env)?;
    let values = [
        D1Type::Text(&name),
        D1Type::Text(id),
        D1Type::Text(&session.user_id),
    ];
    let changes = super::execute_changes(&d1, UPDATE_NAME, &values).await?;
    if changes == 0 {
        return Ok(not_found("the passkey does not exist"));
    }
    let values = [D1Type::Text(id), D1Type::Text(&session.user_id)];
    let row: Option<PasskeyRow> = statement(&d1, SELECT_PASSKEY, &values)?.first(None).await?;
    let Some(row) = row else {
        return Ok(not_found("the passkey does not exist"));
    };
    respond::json(&PasskeyResponse::from(row))
}

/// パスキーを削除する。認証が必要。最後の 1 つは削除できず 409 を返す (ADR-0004)。
pub async fn delete(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the passkey does not exist"));
    };
    let d1 = database(env)?;
    let values = [D1Type::Text(id), D1Type::Text(&session.user_id)];
    let row: Option<PasskeyRow> = statement(&d1, SELECT_PASSKEY, &values)?.first(None).await?;
    if row.is_none() {
        return Ok(not_found("the passkey does not exist"));
    }
    // 件数の確認と削除を 1 つの文で行い、同時の削除でも最後の 1 つを消せないようにする。
    let values = [
        D1Type::Text(id),
        D1Type::Text(&session.user_id),
        D1Type::Text(&session.user_id),
    ];
    let changes = super::execute_changes(&d1, DELETE_PASSKEY_IF_NOT_LAST, &values).await?;
    if changes == 0 {
        // 対象の行が同時に消えた場合は 404、最後の 1 つとして残った場合は 409 にする。
        let values = [D1Type::Text(id), D1Type::Text(&session.user_id)];
        let row: Option<PasskeyRow> = statement(&d1, SELECT_PASSKEY, &values)?.first(None).await?;
        if row.is_none() {
            return Ok(not_found("the passkey does not exist"));
        }
        return Ok(conflict("the last passkey cannot be deleted"));
    }
    respond::json(&serde_json::json!({ "id": id }))
}

pub(crate) const SELECT_PASSKEYS: &str =
    "SELECT id, name, created_at, last_used_at FROM passkey_credentials \
                               WHERE user_id = ? ORDER BY created_at, id";
pub(crate) const SELECT_PASSKEY: &str =
    "SELECT id, name, created_at, last_used_at FROM passkey_credentials \
                              WHERE id = ? AND user_id = ?";
pub(crate) const UPDATE_NAME: &str =
    "UPDATE passkey_credentials SET name = ? WHERE id = ? AND user_id = ?";
/// 利用者に他のパスキーが残っているときだけ削除する (最後の 1 つは削除できない)。
pub(crate) const DELETE_PASSKEY_IF_NOT_LAST: &str = "DELETE FROM passkey_credentials WHERE id = ? AND \
                                          user_id = ? AND (SELECT COUNT(*) FROM passkey_credentials \
                                          WHERE user_id = ?) > 1";
pub(crate) const INSERT_CREDENTIAL: &str = "INSERT INTO passkey_credentials (id, user_id, credential_id, \
                                 public_key, sign_count, name, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)";
