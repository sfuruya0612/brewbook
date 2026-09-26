//! 登録用トークンの発行 (FR-17)。
//!
//! 再発行では、その利用者の未使用の登録用トークンを削除してから新しいトークンを発行する
//! (紛失時の再発行で古いリンクが残らないようにする)。
//!
//! 有効期限は 24 時間とし、D1 には SHA-256 のハッシュだけを保存する (ADR-0004)。発行の応答は
//! 登録用リンクを表示する HTML を直接返し、303 のリダイレクトにはしない。生のトークンを
//! 保存しないため、一覧を含む他の応答では再表示できない (ADR-0008)。
//! 存在しない利用者の ID は 404 を返す。

use coffee_log_core::auth;
use worker::d1::D1Type;
use worker::{Env, Response, Result};

use crate::queries;
use crate::{db, html, html_error, html_response, input};

/// 利用者向けの Worker のオリジンの vars の名前 (ADR-0008)。
pub const APP_ORIGIN_VAR: &str = "APP_ORIGIN";

/// 利用者向けの Worker のオリジンの既定値。ローカルの `wrangler dev` の値 (ADR-0004)。
pub const DEFAULT_APP_ORIGIN: &str = "http://localhost:8787";

/// `POST /users/:id/tokens` を処理する。登録用トークンを発行し、リンクを HTML で返す。
pub async fn create(user_id: &str, env: &Env) -> Result<Response> {
    let d1 = db::database(env)?;

    // 存在しない利用者の ID は 404 を返す。
    let values = [D1Type::Text(user_id)];
    let user: Option<queries::UserIdRow> = db::statement(&d1, queries::SELECT_USER, &values)?
        .first(None)
        .await?;
    if user.is_none() {
        return Ok(html_error(404, "the user does not exist"));
    }

    // 生のトークンは応答にだけ載せ、D1 には SHA-256 のハッシュだけを保存する (ADR-0004)。
    let token = auth::encode_secret(db::random_bytes_32()?);
    let token_id = db::new_id()?;
    let token_hash = auth::hash_secret(&token);
    let expires_at = db::expiry_text(auth::REGISTRATION_TOKEN_TTL_SECONDS)?;

    // 未使用のトークンの削除と発行を 1 つのまとまりで行う (再発行。ADR-0008)。
    let delete = db::statement(&d1, queries::DELETE_UNUSED_TOKENS, &values)?;
    let insert_values = [
        D1Type::Text(&token_id),
        D1Type::Text(user_id),
        D1Type::Text(&token_hash),
        D1Type::Text(&expires_at),
    ];
    let insert = db::statement(&d1, queries::INSERT_TOKEN, &insert_values)?;
    db::execute_batch(&d1, vec![delete, insert]).await?;

    let link = input::registration_link(&app_origin(env)?, &token);
    html_response(html::token_page(&link))
}

/// 利用者向けの Worker のオリジンを vars から読む。無ければローカルの既定値を使う。
pub fn app_origin(env: &Env) -> Result<String> {
    match env.var(APP_ORIGIN_VAR) {
        Ok(origin) => Ok(origin.to_string()),
        Err(_) => Ok(DEFAULT_APP_ORIGIN.to_owned()),
    }
}
