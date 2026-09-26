//! 状態を変更するフォームの送信の `Origin` ヘッダの検証 (ADR-0005 と同じ規則)。
//!
//! 管理者 Worker は CORS を許可しないため、ブラウザは別オリジンからフォームを送れない。
//! それに加えて、状態を変更するリクエスト (POST、PUT、PATCH、DELETE) は `Origin` ヘッダが
//! リクエスト自身のオリジンと一致することを必須にする。一致しないリクエストと `Origin` の
//! 無いリクエストは 403 で拒否する (`Origin` が無い場合は検証できないため)。
//!
//! 利用者向けの Worker (`coffee_log::origin`) と同じ規則である。両者は別のクレートで、
//! 共有すると `coffee_log_core` が `worker` に依存してしまうため、同じ実装をここに持つ。

use coffee_log_core::routes::Method;
use worker::{Request, Response, Result, Url};

use crate::html_error;

/// 状態を変更するメソッドか (`Origin` の検証の対象か)。
pub fn changes_state(method: Method) -> bool {
    matches!(
        method,
        Method::Post | Method::Put | Method::Patch | Method::Delete
    )
}

/// 経路のメソッドに応じて `Origin` を検証する。検証の対象外のメソッドは常に通す。
pub fn check(method: Method, req: &Request) -> Result<bool> {
    if !changes_state(method) {
        return Ok(true);
    }
    is_same_origin(req)
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
    html_error(403, "the request origin is not the admin origin")
}
