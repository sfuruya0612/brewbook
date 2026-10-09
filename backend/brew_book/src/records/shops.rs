//! 店の API (FR-6)。
//!
//! 一覧はカーソル方式で、並び順は作成日時の降順と ID の昇順とする。応答には続きを引く
//! `next_cursor` を含める (ページが `limit` に満たないときは null)。
//! 更新は項目が無ければ変更せず、`null` で NULL にする (店名は必須のため `null` を拒否する)。
//! 存在しない ID と他の利用者の ID は区別せず 404 を返す (ADR-0006)。
//! `updated_at` は更新で現在時刻にする (ADR-0006)。

use brew_book_core::cursor::{CursorValue, SortKey};
use brew_book_core::query::{self, ShopValues};
use brew_book_core::records::{trim_optional, validate_name};
use serde::{Deserialize, Serialize};
use worker::d1::D1Database;
use worker::{Env, Request, Response, Result};

use super::{
    apply_favorite, internal_error, invalid_input, merge_name, not_found, query_error_response,
    read_input, ListParams,
};
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
    /// お気に入りにした日時。未設定のときは null (FR-21)。
    pub favorited_at: Option<String>,
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

/// 店の一覧を返す。認証が必要。`sort`、`order`、`favorite` を受け付ける (FR-20、FR-21)。
pub async fn list(req: &Request, env: &Env, session: &Session) -> Result<Response> {
    let params = match ListParams::from_request(req, SortKey::CreatedAt, query::SHOP_SORT_KEYS) {
        Ok(params) => params,
        Err(response) => return Ok(response),
    };
    let d1 = db::database(env)?;
    let statement = match query::shops_list(
        &session.user_id,
        params.sort,
        params.order,
        params.favorite,
        params.cursor.clone(),
        params.limit,
    ) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    let shops: Vec<ShopResponse> = db::prepared(&d1, &statement)?.all().await?.results()?;
    let next_cursor = shops
        .last()
        .and_then(|shop| params.next_cursor(sort_value(shop, params.sort), &shop.id, shops.len()));
    respond::json(&ShopListResponse { shops, next_cursor })
}

/// 並び順のキーの値を、応答の行から取り出す (FR-20)。
fn sort_value(shop: &ShopResponse, sort: SortKey) -> Option<CursorValue> {
    match sort {
        SortKey::CreatedAt => Some(CursorValue::Text(shop.created_at.clone())),
        SortKey::Name => Some(CursorValue::Text(shop.name.clone())),
        SortKey::UpdatedAt => Some(CursorValue::Text(shop.updated_at.clone())),
        _ => None,
    }
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
        favorited_at: None,
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
        favorited_at: shop.favorited_at,
    })
}

/// お気に入りを付ける (FR-21)。認証が必要。
pub async fn favorite_put(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    set_favorite(env, session, id, true).await
}

/// お気に入りを外す (FR-21)。認証が必要。
pub async fn favorite_delete(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    set_favorite(env, session, id, false).await
}

/// お気に入りを付け外しする (FR-21)。
///
/// 既に同じ状態のときは `favorited_at` と `updated_at` を変えず、現在の応答をそのまま返す
/// (繰り返し呼んでも状態と応答が同じになる)。
async fn set_favorite(
    env: &Env,
    session: &Session,
    id: Option<&str>,
    favorite: bool,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the shop does not exist"));
    };
    let d1 = db::database(env)?;
    let Some(shop) = find(&d1, &session.user_id, Some(id)).await? else {
        return Ok(not_found("the shop does not exist"));
    };
    let changed = apply_favorite(
        &d1,
        query::SHOPS_TABLE,
        id,
        &session.user_id,
        shop.favorited_at.as_deref(),
        favorite,
    )
    .await?;
    if changed.is_none() {
        return respond::json(&shop);
    }
    match find(&d1, &session.user_id, Some(id)).await? {
        Some(shop) => respond::json(&shop),
        None => Ok(internal_error()),
    }
}

/// 店を削除する (0056)。認証が必要。
///
/// 店を指定している購入の店の指定を外してから店の行を削除する。購入は残す。
/// 存在しない ID と他の利用者の ID は 404 にする (ADR-0006)。
pub async fn delete(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the shop does not exist"));
    };
    let d1 = db::database(env)?;
    if find(&d1, &session.user_id, Some(id)).await?.is_none() {
        return Ok(not_found("the shop does not exist"));
    }
    let now = db::now_text()?;
    let statements = vec![
        db::prepared(
            &d1,
            &query::shop_clear_purchases(&session.user_id, id, &now),
        )?,
        db::prepared(&d1, &query::shop_delete(&session.user_id, id))?,
    ];
    db::execute_batch(&d1, statements).await?;
    Ok(Response::empty()?.with_status(204))
}

/// 店の削除で店の指定が外れる購入の件数を返す (0056)。認証が必要。
pub async fn delete_impact(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the shop does not exist"));
    };
    let d1 = db::database(env)?;
    if find(&d1, &session.user_id, Some(id)).await?.is_none() {
        return Ok(not_found("the shop does not exist"));
    }
    let purchases =
        super::count_rows(&d1, &query::shop_delete_impact(&session.user_id, id)).await?;
    respond::json(&ShopDeleteImpactResponse { purchases })
}

/// 店の削除の影響の応答 (0056)。
#[derive(Debug, Serialize)]
struct ShopDeleteImpactResponse {
    /// 店の指定が外れる購入の件数。
    purchases: i64,
}

/// 店を 1 件引く。ID が無いときと行が無いときは None。
async fn find(d1: &D1Database, user_id: &str, id: Option<&str>) -> Result<Option<ShopResponse>> {
    let Some(id) = id else {
        return Ok(None);
    };
    let statement = query::shop_find(user_id, id);
    db::prepared(d1, &statement)?.first(None).await
}
