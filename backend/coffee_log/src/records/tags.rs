//! Flavor Notes のタグの API (FR-8)。
//!
//! 利用者の全タグの一覧を返す。並び順は名前の昇順とする。
//! タグは商品の登録と更新が配列で置き換え、参照されなくなっても削除しない (0006 の商品の API)。

use coffee_log_core::query;
use serde::{Deserialize, Serialize};
use worker::{Env, Response, Result};

use crate::auth::session::Session;
use crate::db;
use crate::respond;

/// タグの応答。スキーマの列名をそのまま使う。
#[derive(Debug, Serialize, Deserialize)]
pub struct TagResponse {
    pub id: String,
    pub user_id: String,
    pub name: String,
}

/// タグの一覧の応答。
#[derive(Debug, Serialize)]
pub struct TagListResponse {
    pub flavor_tags: Vec<TagResponse>,
}

/// 利用者の Flavor Notes のタグの一覧を返す。認証が必要。
pub async fn list(env: &Env, session: &Session) -> Result<Response> {
    let d1 = db::database(env)?;
    let statement = query::flavor_tags_list(&session.user_id);
    let flavor_tags: Vec<TagResponse> = db::prepared(&d1, &statement)?.all().await?.results()?;
    respond::json(&TagListResponse { flavor_tags })
}
