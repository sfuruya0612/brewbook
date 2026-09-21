//! 管理者向けの Worker。
//!
//! 管理者画面と管理者 API は 0018 が実装する (ADR-0008)。
//! それまでの間は、全てのリクエストに PRD の形式の 404 を返す。

use coffee_log_core::error::{envelope, ErrorCode};
use worker::*;

#[event(fetch)]
pub async fn main(_req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let envelope = envelope(ErrorCode::NotFound, "route not found");
    Ok(Response::from_json(&envelope)?.with_status(ErrorCode::NotFound.status()))
}
