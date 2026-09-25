//! 状態を変更する API の `Origin` ヘッダの検証 (ADR-0005、PRD のセキュリティ)。
//!
//! Backend は CORS を許可しないため、ブラウザは別オリジンから API を呼べない。
//! それに加えて、状態を変更するリクエスト (POST、PUT、PATCH、DELETE) は `Origin` ヘッダが
//! リクエスト自身のオリジンと一致することを必須にする。一致しないリクエストと `Origin` の
//! 無いリクエストは 403 で拒否する (`Origin` が無い場合は検証できないため)。
//!
//! iOS ネイティブから API を使うときは `Origin` が付かない。この扱いは iOS 対応時に
//! 署名の検証と合わせて決める (issue 0017 が提起した論点)。
//! 写真の R2 の S3 互換エンドポイントへの PUT は Worker を経由しないため、この検証の対象外で、
//! R2 バケットの CORS でアプリのオリジンからの PUT だけを許可する (ADR-0003)。

use coffee_log_core::error::ErrorCode;
use coffee_log_core::routes::Method;
use worker::{Request, Response, Result};

use crate::respond;

/// 状態を変更するメソッドか (`Origin` の検証の対象か)。
pub fn changes_state(method: Method) -> bool {
    matches!(
        method,
        Method::Post | Method::Put | Method::Patch | Method::Delete
    )
}

/// `Origin` ヘッダがリクエスト自身のオリジンと一致するか。
///
/// オリジンは scheme、host、port の組で比較する。既定のポート (http の 80、https の 443) は
/// ブラウザも `Url::origin` も表記に含めないため、表記のまま比較できる。
/// `Origin` が無いリクエストは、同じオリジンからのものかを検証できないため一致しない扱いにする。
pub fn is_same_origin(req: &Request) -> Result<bool> {
    let Some(origin) = req.headers().get("Origin")? else {
        return Ok(false);
    };
    let expected = req.url()?.origin().ascii_serialization();
    Ok(origin == expected)
}

/// `Origin` の検証に失敗したリクエストの応答 (403)。
pub fn forbidden() -> Response {
    respond::error(
        ErrorCode::Forbidden,
        "the request origin is not the app origin",
    )
}
