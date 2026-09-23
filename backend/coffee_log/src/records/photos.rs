//! 購入の写真 (FR-10、ADR-0003)。
//!
//! アップロード用 URL の発行、アップロード完了の通知、取得、削除の 4 つの経路を処理する。
//! 署名付き URL の生成は `coffee_log_core::photo`、R2 のオブジェクトの操作はバインディング
//! (`PHOTOS`) が行う。存在しない購入と他の利用者の購入は区別せず 404 を返す (FR-5)。
//!
//! クライアントは、アップロード用 URL の要求、PUT、完了通知の 3 回の呼び出しを行う (ADR-0003)。
//! 写真は 1 購入につき 1 枚とし、サーバー側の変換と縮小は行わない (PRD のやらないこと)。

use coffee_log_core::photo::{self, SigningConfig};
use coffee_log_core::query;
use serde::{Deserialize, Serialize};
use worker::{Date, Env, HttpMetadata, Request, Response, Result};

use super::purchases;
use super::{invalid_input, not_found, query_error_response, read_input};
use crate::auth::session::Session;
use crate::auth::var_or;
use crate::db;
use crate::respond;

/// R2 のバケットのバインディングの名前 (ADR-0003)。
pub const PHOTOS_BINDING: &str = "PHOTOS";
/// S3 互換エンドポイントの vars の名前。
pub const ENDPOINT_VAR: &str = "R2_ENDPOINT";
/// バケット名の vars の名前。
pub const BUCKET_VAR: &str = "R2_BUCKET";
/// バケット名の既定値 (ADR-0003)。
pub const DEFAULT_BUCKET: &str = "coffee-log-photos";
/// R2 の API トークンのアクセスキーを置く Secret の名前。
pub const ACCESS_KEY_ID_SECRET: &str = "R2_ACCESS_KEY_ID";
/// R2 の API トークンのシークレットを置く Secret の名前。
pub const SECRET_ACCESS_KEY_SECRET: &str = "R2_SECRET_ACCESS_KEY";
/// 署名付き URL の有効期限の秒数の vars の名前。テストが `--var` で短い値を注入する (FR-10)。
pub const URL_EXPIRES_SECONDS_VAR: &str = "PHOTO_URL_EXPIRES_SECONDS";

/// 購入が無いときの応答のメッセージ。存在しない購入と他の利用者の購入で共通にする (FR-5)。
const PURCHASE_NOT_FOUND: &str = "the purchase does not exist";
/// 写真が無いときの応答のメッセージ。
const PHOTO_NOT_FOUND: &str = "the photo does not exist";

/// 写真の設定。vars と Secret から読む。
/// ここで読む値は署名とオブジェクトの操作に使う (ADR-0003、PRD のセキュリティ)。
struct PhotoConfig {
    /// バケット名。vars の `R2_BUCKET`、無ければ既定値。
    bucket: String,
    /// S3 互換エンドポイント。vars の `R2_ENDPOINT`。
    endpoint: String,
    /// API トークンのアクセスキー。Secret の `R2_ACCESS_KEY_ID`。
    access_key_id: String,
    /// API トークンのシークレット。Secret の `R2_SECRET_ACCESS_KEY`。
    secret_access_key: String,
    /// 署名付き URL の有効期限 (秒)。
    url_expires_seconds: u64,
}

impl PhotoConfig {
    /// vars と Secret から設定を読む。
    ///
    /// エンドポイントと API トークンはデプロイで設定するため、無ければ内部エラーにする
    /// (リポジトリに実値を含めない。PRD のセキュリティ)。
    fn from_env(env: &Env) -> Result<Self> {
        Ok(Self {
            bucket: var_or(env, BUCKET_VAR, DEFAULT_BUCKET)?,
            endpoint: required_var(env, ENDPOINT_VAR)?,
            access_key_id: required_var(env, ACCESS_KEY_ID_SECRET)?,
            secret_access_key: required_var(env, SECRET_ACCESS_KEY_SECRET)?,
            url_expires_seconds: seconds_var_or(
                env,
                URL_EXPIRES_SECONDS_VAR,
                photo::DEFAULT_URL_EXPIRES_SECONDS,
            )?,
        })
    }

    /// 署名付き URL の生成の設定。
    fn signing(&self) -> SigningConfig<'_> {
        SigningConfig {
            endpoint: &self.endpoint,
            bucket: &self.bucket,
            access_key_id: &self.access_key_id,
            secret_access_key: &self.secret_access_key,
        }
    }
}

/// `POST /api/purchases/<購入 ID>/photo/upload-url` の入力 (FR-10)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UploadUrlInput {
    /// クライアントが変換した後の写真のサイズ (バイト)。
    size: i64,
}

/// `POST /api/purchases/<購入 ID>/photo` の入力 (FR-10)。
///
/// 発行時に申告したサイズを完了通知でもう一度受け取る。中間の申告を D1 に保存しないためで、
/// 保存先のオブジェクトのサイズが申告と一致することをこの値で確認する (0009 の実装判断)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompleteInput {
    /// 発行時に返したオブジェクトキー (`pending/<利用者 ID>/<UUID>.jpg`)。
    key: String,
    /// 発行時に申告したサイズ (バイト)。
    size: i64,
}

/// アップロード用 URL の発行の応答 (FR-10)。
#[derive(Debug, Serialize)]
struct UploadUrlResponse {
    /// 署名付き PUT URL。
    url: String,
    /// 発行したオブジェクトキー。完了通知でそのまま返す。
    key: String,
}

/// アップロード用 URL を発行する。認証が必要 (FR-10)。
///
/// 申告サイズが 5 MB を超える場合は 400 を返し、URL を発行しない (ADR-0003)。
pub async fn upload_url(
    req: &mut Request,
    env: &Env,
    session: &Session,
    id: Option<&str>,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found(PURCHASE_NOT_FOUND));
    };
    let d1 = db::database(env)?;
    if purchases::find(&d1, &session.user_id, id).await?.is_none() {
        return Ok(not_found(PURCHASE_NOT_FOUND));
    }
    let Some(input) = read_input::<UploadUrlInput>(req).await else {
        return Ok(invalid_input(
            "the body must be JSON with the declared size",
        ));
    };
    if let Err(error) = photo::validate_declared_size(input.size) {
        return Ok(invalid_input(error.message()));
    }
    let config = PhotoConfig::from_env(env)?;
    let uuid = db::new_id()?;
    let key = photo::pending_key(&session.user_id, &uuid);
    let now = photo::system_time_from_millis(Date::now().as_millis() as i64);
    let url = photo::presign_put_url(
        &config.signing(),
        &key,
        input.size,
        config.url_expires_seconds,
        now,
    )
    .map_err(|error| {
        worker::Error::RustError(format!("failed to presign the upload url: {error}"))
    })?;
    respond::json(&UploadUrlResponse { url, key })
}

/// アップロード完了を通知する。認証が必要 (FR-10)。
///
/// キーの形式を検査し、`pending/` のオブジェクトの存在、サイズ、Content-Type を確認してから
/// 購入に紐づける。確認に失敗した場合はオブジェクトを削除して 400 を返す (ADR-0003)。
pub async fn complete(
    req: &mut Request,
    env: &Env,
    session: &Session,
    id: Option<&str>,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found(PURCHASE_NOT_FOUND));
    };
    let d1 = db::database(env)?;
    let Some(purchase) = purchases::find(&d1, &session.user_id, id).await? else {
        return Ok(not_found(PURCHASE_NOT_FOUND));
    };
    let Some(input) = read_input::<CompleteInput>(req).await else {
        return Ok(invalid_input(
            "the body must be JSON with the object key and the declared size",
        ));
    };
    if let Err(error) = photo::validate_declared_size(input.size) {
        return Ok(invalid_input(error.message()));
    }
    // キーはクライアントから戻される信頼できない入力である。他の利用者の `pending/` の
    // オブジェクトを取り込まないよう、形式が一致しなければオブジェクトに触れず 400 にする (FR-10)。
    let Some(uuid) = photo::parse_pending_key(&session.user_id, &input.key) else {
        return Ok(invalid_input(
            "the object key is not a pending key of the caller",
        ));
    };
    let bucket = env.bucket(PHOTOS_BINDING)?;
    let Some(object) = bucket.head(&input.key).await? else {
        return Ok(invalid_input("the pending object does not exist"));
    };
    // 申告サイズと一致し、5 MB 以下で、Content-Type が `image/jpeg` であること (FR-10)。
    let matches = object.size() == input.size as u64
        && object.size() <= photo::MAX_BYTES as u64
        && object.http_metadata().content_type.as_deref() == Some(photo::CONTENT_TYPE);
    if !matches {
        bucket.delete(&input.key).await?;
        return Ok(invalid_input(
            "the pending object does not match the declared size and content type",
        ));
    }
    let Some(loaded) = bucket.get(&input.key).execute().await? else {
        return Ok(invalid_input("the pending object does not exist"));
    };
    let Some(body) = loaded.body() else {
        return Ok(invalid_input("the pending object has no body"));
    };
    // R2 のバインディングに複製と移動の操作が無いため、取得と保存の 2 操作で移す (ADR-0003)。
    // 写真は 5 MB 以下なので Worker のメモリに収まる。
    let bytes = body.bytes().await?;
    let new_key = photo::photo_key(&session.user_id, id, uuid);
    bucket
        .put(&new_key, bytes)
        .http_metadata(HttpMetadata {
            content_type: Some(photo::CONTENT_TYPE.to_owned()),
            ..Default::default()
        })
        .execute()
        .await?;
    bucket.delete(&input.key).await?;
    let now = db::now_text()?;
    let statement = match query::purchase_set_photo_key(id, &session.user_id, Some(&new_key), &now)
    {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    // 差し替えでは、紐づけが成功した後で古いオブジェクトを削除する (FR-10)。
    if let Some(old_key) = purchase.photo_key.as_deref() {
        if old_key != new_key && old_key.starts_with(photo::USERS_PREFIX) {
            bucket.delete(old_key).await?;
        }
    }
    purchases::respond_fetched(&d1, &session.user_id, id).await
}

/// 写真を返す。認証が必要 (FR-10)。
///
/// R2 バインディングで読み出し、`image/jpeg` で返す (ADR-0003)。写真が無い購入は 404 を返す。
/// 共用の端末でログアウト後にキャッシュから写真が見えないよう、`Cache-Control: private, no-store`
/// を付ける (UC-12)。
pub async fn get(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found(PURCHASE_NOT_FOUND));
    };
    let d1 = db::database(env)?;
    let Some(purchase) = purchases::find(&d1, &session.user_id, id).await? else {
        return Ok(not_found(PURCHASE_NOT_FOUND));
    };
    let Some(key) = purchase.photo_key else {
        return Ok(not_found(PHOTO_NOT_FOUND));
    };
    let bucket = env.bucket(PHOTOS_BINDING)?;
    let Some(object) = bucket.get(&key).execute().await? else {
        return Ok(not_found(PHOTO_NOT_FOUND));
    };
    let Some(body) = object.body() else {
        return Ok(not_found(PHOTO_NOT_FOUND));
    };
    let bytes = body.bytes().await?;
    let mut response = Response::from_bytes(bytes)?;
    let headers = response.headers_mut();
    headers.set("Content-Type", photo::CONTENT_TYPE)?;
    headers.set("Cache-Control", "private, no-store")?;
    Ok(response)
}

/// 写真を削除する。認証が必要 (FR-10)。
///
/// 購入の `photo_key` を NULL にしてからオブジェクトを削除する (FR-10)。
pub async fn delete(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found(PURCHASE_NOT_FOUND));
    };
    let d1 = db::database(env)?;
    let Some(purchase) = purchases::find(&d1, &session.user_id, id).await? else {
        return Ok(not_found(PURCHASE_NOT_FOUND));
    };
    let Some(key) = purchase.photo_key else {
        return Ok(not_found(PHOTO_NOT_FOUND));
    };
    let now = db::now_text()?;
    let statement = match query::purchase_set_photo_key(id, &session.user_id, None, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    let bucket = env.bucket(PHOTOS_BINDING)?;
    bucket.delete(&key).await?;
    purchases::respond_fetched(&d1, &session.user_id, id).await
}

/// 必須の vars または Secret を読む。無い場合と空の場合は内部エラーにする。
fn required_var(env: &Env, name: &str) -> Result<String> {
    let value = var_or(env, name, "")?;
    if value.is_empty() {
        return Err(worker::Error::RustError(format!(
            "the {name} binding is not configured"
        )));
    }
    Ok(value)
}

/// 秒数の vars を読み、無ければ既定値を返す。
fn seconds_var_or(env: &Env, name: &str, default: u64) -> Result<u64> {
    let text = var_or(env, name, "")?;
    if text.is_empty() {
        return Ok(default);
    }
    text.parse::<u64>().map_err(|_| {
        worker::Error::RustError(format!(
            "the {name} var must be an integer number of seconds but was {text}"
        ))
    })
}
