//! 利用者の一覧と作成 (FR-17)。
//!
//! 一覧は表示名、作成日時、パスキーの数を HTML の表で返す。作成は表示名を検証し
//! (前後の空白を除いて 1 文字以上 50 文字以下)、範囲外は 400 を返す。
//! 成功したら一覧へ 303 で戻す (フォームの二重送信を避けるため)。

use worker::d1::D1Type;
use worker::{Env, Request, Response, Result};

use crate::queries;
use crate::{db, html, html_error, html_response, input};

/// `GET /` を処理する。利用者の一覧を HTML で返す。
pub async fn list(env: &Env) -> Result<Response> {
    let d1 = db::database(env)?;
    let result = db::statement(&d1, queries::SELECT_USERS, &[])?
        .all()
        .await?;
    let users: Vec<queries::UserRow> = result.results()?;
    html_response(html::users_page(&users))
}

/// `POST /users` を処理する。表示名を検証して利用者を作り、一覧へ 303 で戻す。
pub async fn create(mut req: Request, env: &Env) -> Result<Response> {
    let body = req.text().await?;
    let Some(raw) = input::form_field(&body, "display_name") else {
        return Ok(html_error(400, "the display name is missing"));
    };
    let display_name = match input::validate_display_name(&raw) {
        Ok(display_name) => display_name,
        Err(error) => return Ok(html_error(400, error.message())),
    };

    let d1 = db::database(env)?;
    let id = db::new_id()?;
    let created_at = db::now_text()?;
    let values = [
        D1Type::Text(&id),
        D1Type::Text(&display_name),
        D1Type::Text(&created_at),
    ];
    db::statement(&d1, queries::INSERT_USER, &values)?
        .run()
        .await?;

    let mut response = Response::empty()?.with_status(303);
    response.headers_mut().set("Location", "/")?;
    Ok(response)
}
