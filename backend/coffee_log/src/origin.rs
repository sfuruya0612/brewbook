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
use worker::{Request, Response, Result, Url};

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
    let origin = req.headers().get("Origin")?;
    is_same_origin_value(origin.as_deref(), req.url()?.as_str())
}

/// `Origin` ヘッダとリクエストの URL から同一オリジンかを判定する。
///
/// どちらも URL として解釈してからオリジンを比べる。`Origin` はブラウザが必ずオリジンの形で
/// 送るが、既定のポート (`http` の 80、`https` の 443) は表記に含めないため、URL として
/// 解釈して正規化したうえで比較する。`Origin` が URL として読めない場合は一致しない扱いにする。
/// 境界値 (scheme、host、port の違い) は単体テストがこの関数を直接検査する。
/// `wrangler dev` は `Origin` の scheme をリクエストの scheme に合わせてから Worker に渡すため、
/// scheme の違いは `wrangler dev` の結合テストでは再現できない。
pub fn is_same_origin_value(origin: Option<&str>, request_url: &str) -> Result<bool> {
    let Some(origin) = origin else {
        return Ok(false);
    };
    let Ok(origin) = Url::parse(origin) else {
        return Ok(false);
    };
    Ok(origin.origin() == Url::parse(request_url)?.origin())
}

/// `Origin` の検証に失敗したリクエストの応答 (403)。
pub fn forbidden() -> Response {
    respond::error(
        ErrorCode::Forbidden,
        "the request origin is not the app origin",
    )
}
