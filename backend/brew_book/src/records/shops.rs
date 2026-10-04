//! 店の API (FR-6)。
//!
//! 一覧はカーソル方式で、並び順は作成日時の降順と ID の昇順とする。応答には続きを引く
//! `next_cursor` を含める (ページが `limit` に満たないときは null)。
//! 更新は項目が無ければ変更せず、`null` で NULL にする (店名は必須のため `null` を拒否する)。
//! 存在しない ID と他の利用者の ID は区別せず 404 を返す (ADR-0006)。
//! `updated_at` は更新で現在時刻にする (ADR-0006)。

use brew_book_core::query::{self, OrderKind, ShopValues};
use brew_book_core::records::{trim_optional, validate_name};
use serde::{Deserialize, Serialize};
use worker::d1::D1Database;
use worker::{Env, Request, Response, Result};

use super::{invalid_input, merge_name, not_found, query_error_response, read_input, ListParams};
use crate::auth::session::Session;
use crate::db;
use crate::respond;

/// 店の応答。スキーマの列名をそのまま使う。
#[derive(Debug, Serialize, Deserialize)]
pub struct ShopResponse {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub address: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 店の一覧の応答。
#[derive(Debug, Serialize)]
pub struct ShopListResponse {
    pub shops: Vec<ShopResponse>,
    /// 続きのページを引くカーソル。続きが無いときは null。
    pub next_cursor: Option<String>,
}

/// `POST /api/shops` の入力。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateInput {
    /// 店名。必須。
    name: String,
    /// 住所。任意 (FR-6)。
    #[serde(default)]
    address: Option<String>,
}

/// `PATCH /api/shops/<ID>` の入力。無い項目は変更しない。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateInput {
    /// 新しい店名。無いときは変更しない。`null` は拒否する (店名は必須)。
    #[serde(default, deserialize_with = "super::double_option")]
    name: Option<Option<String>>,
    /// 新しい住所。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    address: Option<Option<String>>,
}

/// 店の一覧を返す。認証が必要。
pub async fn list(req: &Request, env: &Env, session: &Session) -> Result<Response> {
    let params = match ListParams::from_request(req) {
        Ok(params) => params,
        Err(response) => return Ok(response),
    };
    let d1 = db::database(env)?;
    let statement = match query::shops_list(&session.user_id, params.cursor.clone(), params.limit) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    let shops: Vec<ShopResponse> = db::prepared(&d1, &statement)?.all().await?.results()?;
    let next_cursor = params.next_cursor(
        OrderKind::DateTime,
        shops
            .last()
            .map(|shop| (shop.created_at.as_str(), shop.id.as_str())),
        shops.len(),
    );
    respond::json(&ShopListResponse { shops, next_cursor })
}

/// 店を登録する。認証が必要。
pub async fn create(req: &mut Request, env: &Env, session: &Session) -> Result<Response> {
    let Some(input) = read_input::<CreateInput>(req).await else {
        return Ok(invalid_input("the body must be JSON with a name"));
    };
    let name = match validate_name(&input.name) {
        Ok(name) => name,
        Err(error) => return Ok(invalid_input(error.message())),
    };
    let address = trim_optional(input.address.as_deref());
    let d1 = db::database(env)?;
    let id = db::new_id()?;
    let now = db::now_text()?;
    let values = ShopValues {
        name: &name,
        address: address.as_deref(),
    };
    let statement = match query::shop_insert(&id, &session.user_id, &values, &now, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    respond::json(&ShopResponse {
        id,
        user_id: session.user_id.clone(),
        name,
        address,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 店を 1 件返す。認証が必要。
pub async fn get(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let d1 = db::database(env)?;
    match find(&d1, &session.user_id, id).await? {
        Some(shop) => respond::json(&shop),
        None => Ok(not_found("the shop does not exist")),
    }
}

/// 店を更新する。認証が必要。
pub async fn update(
    req: &mut Request,
    env: &Env,
    session: &Session,
    id: Option<&str>,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the shop does not exist"));
    };
    let Some(input) = read_input::<UpdateInput>(req).await else {
        return Ok(invalid_input(
            "the body must be JSON with the fields to update",
        ));
    };
    let d1 = db::database(env)?;
    let Some(shop) = find(&d1, &session.user_id, Some(id)).await? else {
        return Ok(not_found("the shop does not exist"));
    };
    let name = match merge_name(input.name, shop.name) {
        Ok(name) => name,
        Err(response) => return Ok(response),
    };
    let address = match input.address {
        Some(address) => trim_optional(address.as_deref()),
        None => shop.address,
    };
    let now = db::now_text()?;
    let values = ShopValues {
        name: &name,
        address: address.as_deref(),
    };
    let statement = match query::shop_update(id, &session.user_id, &values, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    respond::json(&ShopResponse {
        id: shop.id,
        user_id: shop.user_id,
        name,
        address,
        created_at: shop.created_at,
        updated_at: now,
    })
}

/// 店を 1 件引く。ID が無いときと行が無いときは None。
async fn find(d1: &D1Database, user_id: &str, id: Option<&str>) -> Result<Option<ShopResponse>> {
    let Some(id) = id else {
        return Ok(None);
    };
    let statement = query::shop_find(user_id, id);
    db::prepared(d1, &statement)?.first(None).await
}
