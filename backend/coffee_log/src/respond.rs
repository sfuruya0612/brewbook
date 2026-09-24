//! API の応答の組み立て (PRD の「エラー応答の規約」)。
//!
//! エラー応答は `{"error": {"code": "<snake_case>", "message": "<英語>"}}` に統一する。
//! 成功の応答は JSON とし、セッションを発行した経路は `Set-Cookie` を付ける。

use coffee_log_core::error::{envelope, ErrorCode};
use serde::Serialize;
use worker::{console_error, Response, Result};

/// Worker が内部で失敗したときの応答のメッセージ。
const INTERNAL_ERROR_MESSAGE: &str = "internal error";

/// PRD の形式のエラー応答を組み立てる。応答の構築に失敗した場合は 500 の平文に落とす。
pub fn error(code: ErrorCode, message: &str) -> Response {
    match Response::from_json(&envelope(code, message)) {
        Ok(response) => response.with_status(code.status()),
        Err(error) => {
            console_error!("failed to build the error response: {error}");
            Response::error(INTERNAL_ERROR_MESSAGE, ErrorCode::Internal.status())
                .expect("a plain error response is always constructible")
        }
    }
}

/// JSON の応答を組み立てる。
pub fn json<T: Serialize>(value: &T) -> Result<Response> {
    Response::from_json(value)
}

/// ダウンロード用の JSON の応答を組み立てる。`Content-Disposition: attachment` を付ける (FR-14)。
pub fn json_attachment<T: Serialize>(value: &T, file_name: &str) -> Result<Response> {
    let mut response = Response::from_json(value)?;
    response.headers_mut().set(
        "Content-Disposition",
        &format!("attachment; filename=\"{file_name}\""),
    )?;
    Ok(response)
}

/// `Set-Cookie` を 1 つ付けた JSON の応答を組み立てる。
pub fn json_with_cookie<T: Serialize>(value: &T, cookie: &str) -> Result<Response> {
    set_cookie(Response::from_json(value)?, cookie)
}

/// `Set-Cookie` を 1 つ付けた応答にする。
pub fn set_cookie(mut response: Response, cookie: &str) -> Result<Response> {
    response.headers_mut().set("Set-Cookie", cookie)?;
    Ok(response)
}
