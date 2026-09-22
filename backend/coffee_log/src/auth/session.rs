//! セッション (ADR-0004)。
//!
//! セッションのトークンは 32 バイトの乱数を base64url で符号化した文字列とし、D1 には
//! SHA-256 のハッシュだけを保存する。クライアントには `session` という名前の Cookie で渡す。
//! 有効期限は発行から 30 日とし、照合は ISO 8601 UTC の文字列の辞書順の比較で行う。
//!
//! 認証が不要な経路はセッションを解決しない。認証が必要な経路の判定は経路の台帳 (0001) が持ち、
//! 実際の解決はここ 1 か所 (`resolve`) に置く。

use coffee_log_core::auth;
use worker::d1::{D1Database, D1PreparedStatement, D1Type};
use worker::{Env, Request, Response, Result};

use super::{database, new_id, now_text, random_bytes_32, statement};

/// 認証済みの利用者のセッション。
pub struct Session {
    /// `sessions` の行の ID。ログアウトで物理削除する行を指す。
    pub id: String,
    /// セッションの利用者。
    pub user_id: String,
}

/// `Cookie` ヘッダのセッションのトークンを照合し、有効なセッションを返す。
/// Cookie が無い、行が無い、有効期限を過ぎている場合は None を返し、呼び出し側が 401 にする。
pub async fn resolve(req: &Request, env: &Env) -> Result<Option<Session>> {
    let Some(header) = req.headers().get("Cookie")? else {
        return Ok(None);
    };
    let Some(token) = auth::session_token(&header) else {
        return Ok(None);
    };
    let hash = auth::hash_secret(token);
    let d1 = database(env)?;
    let values = [D1Type::Text(&hash)];
    let row: Option<SessionRow> = statement(&d1, SELECT_SESSION, &values)?.first(None).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if auth::is_expired(&row.expires_at, &now_text()?) {
        return Ok(None);
    }
    Ok(Some(Session {
        id: row.id,
        user_id: row.user_id,
    }))
}

/// セッションの行を組み立て、Cookie の値と一緒に返す。
///
/// 行の挿入は呼び出し側が行う。登録ではトークンの使用済みの印は排他のために別の文で付け、
/// パスキーの行とセッションの行を 1 つのまとまりとして入れる (ADR-0004)。
pub fn prepare(
    d1: &D1Database,
    user_id: &str,
    ttl_seconds: i64,
) -> Result<(D1PreparedStatement, String)> {
    let token = auth::encode_secret(random_bytes_32()?);
    let hash = auth::hash_secret(&token);
    let id = new_id()?;
    let now = now_text()?;
    let expires_at = super::expiry_text(ttl_seconds)?;
    let values = [
        D1Type::Text(&id),
        D1Type::Text(user_id),
        D1Type::Text(&hash),
        D1Type::Text(&expires_at),
        D1Type::Text(&now),
    ];
    let statement = statement(d1, INSERT_SESSION, &values)?;
    Ok((statement, auth::session_cookie(&token, ttl_seconds)))
}

/// ログアウト。セッションの行を物理削除し、Cookie を失効させる (FR-4、ADR-0004)。
pub async fn logout(env: &Env, session: &Session) -> Result<Response> {
    let d1 = database(env)?;
    let values = [D1Type::Text(&session.id)];
    statement(&d1, DELETE_SESSION, &values)?.run().await?;
    crate::respond::json_with_cookie(
        &serde_json::json!({ "logged_out": true }),
        &auth::expired_session_cookie(),
    )
}

/// セッションの行のうち検証に使う列。
#[derive(serde::Deserialize)]
struct SessionRow {
    id: String,
    user_id: String,
    expires_at: String,
}

const SELECT_SESSION: &str = "SELECT id, user_id, expires_at FROM sessions WHERE token_hash = ?";
const INSERT_SESSION: &str =
    "INSERT INTO sessions (id, user_id, token_hash, expires_at, created_at) VALUES (?, ?, ?, ?, ?)";
const DELETE_SESSION: &str = "DELETE FROM sessions WHERE id = ?";
