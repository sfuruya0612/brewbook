//! アカウントと全データの削除 (FR-15)。
//!
//! 利用者に属する全行 (ADR-0006 の 11 テーブルのうち、`user_id` がその利用者の行) を物理削除し、
//! R2 の `users/<利用者 ID>/` と `pending/<利用者 ID>/` の全オブジェクトを削除する。
//! ログイン用のチャレンジの行は `user_id` を持たないため対象外とし、有効期限で失効させる (ADR-0006)。
//!
//! 削除の順は、R2 のオブジェクトを先に消し、成功したら D1 の batch を実行する。
//! R2 の削除が失敗した場合はセッションが有効なうちに利用者が再試行でき、同じキーの削除は
//! 繰り返しても成功する。D1 の削除は batch で 1 トランザクションなので途中の状態は残らない
//! (ADR-0002)。応答は 204 とし、セッションの Cookie を失効させる。セッションの行は消えているため、
//! 同じ Cookie を使った以後の呼び出しは 401 になる。

use coffee_log_core::auth;
use coffee_log_core::query;
use worker::{Env, Response, Result};

use crate::auth::session::Session;
use crate::db;
use crate::records::photos;
use crate::respond;

/// `DELETE /api/account` を処理する。認証が必要 (FR-15)。
pub async fn delete(env: &Env, session: &Session) -> Result<Response> {
    let user_id = &session.user_id;
    // R2 の削除を先に行う (設計判断)。失敗した場合は利用者が同じセッションで再試行できる。
    photos::delete_user_objects(env, user_id).await?;
    // 利用者に属する全行の削除を 1 つの batch で実行する (ADR-0002、ADR-0006)。
    let d1 = db::database(env)?;
    let statements = query::account_delete(user_id)
        .into_iter()
        .map(|statement| db::prepared(&d1, &statement))
        .collect::<Result<Vec<_>>>()?;
    db::execute_batch(&d1, statements).await?;
    // 204 を返し、Cookie を失効させる。本体は持たない (FR-15)。
    let response = respond::set_cookie(Response::empty()?, &auth::expired_session_cookie())?;
    Ok(response.with_status(204))
}
