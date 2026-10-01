//! 商品の API (FR-7、FR-8、FR-12)。
//!
//! 一覧はカーソル方式で、並び順は作成日時の降順と ID の昇順とする。応答には続きを引く
//! `next_cursor` を含める (ページが `limit` に満たないときは null)。
//! 応答にはタグ名の配列 (`flavor_notes`) を含め、並びは名前の昇順にする (FR-8)。
//! Flavor Notes はタグ名の配列で置き換え、タグの行は利用者ごとに名前で共有する (FR-8)。
//! 単件取得はアーカイブ済みでも返し、更新もアーカイブ済みの商品にできる。
//! 更新は項目が無ければ変更せず、`null` で NULL にする (商品名は必須のため `null` を拒否する)。
//! アーカイブと解除は繰り返し呼んでも 200 を返す。
//! 存在しない ID と他の利用者の ID は区別せず 404 を返す (ADR-0006)。
//! `updated_at` は更新、アーカイブ、アーカイブ解除で現在時刻にする (ADR-0006)。

use brew_book_core::query::{self, Archived, OrderKind, ProductValues};
use brew_book_core::records::{trim_optional, validate_flavor_notes, validate_name};
use serde::{Deserialize, Serialize};
use worker::d1::D1Database;
use worker::{console_error, Env, Request, Response, Result};

use super::{
    flavor_notes, flavor_notes_for, internal_error, invalid_input, merge_name, not_found,
    query_error_response, read_input, replace_flavor_notes, ListParams,
};
use crate::auth::session::Session;
use crate::db;
use crate::respond;

/// 商品の応答。スキーマの列名に `flavor_notes` を足す (FR-8)。
/// D1 から引くときは `flavor_notes` の列が無いため、既定 (空) にする。
#[derive(Debug, Serialize, Deserialize)]
pub struct ProductResponse {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub producer: Option<String>,
    pub origin: Option<String>,
    pub region: Option<String>,
    pub process: Option<String>,
    pub variety: Option<String>,
    #[serde(default)]
    pub flavor_notes: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

/// 商品の一覧の応答。
#[derive(Debug, Serialize)]
pub struct ProductListResponse {
    pub products: Vec<ProductResponse>,
    /// 続きのページを引くカーソル。続きが無いときは null。
    pub next_cursor: Option<String>,
}

/// `POST /api/products` の入力。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateInput {
    /// 商品名。必須。
    name: String,
    /// 生産者。任意 (FR-7)。
    #[serde(default)]
    producer: Option<String>,
    /// 生産国。任意。
    #[serde(default)]
    origin: Option<String>,
    /// 地域。任意。
    #[serde(default)]
    region: Option<String>,
    /// 精製方法。任意。
    #[serde(default)]
    process: Option<String>,
    /// 品種。任意。
    #[serde(default)]
    variety: Option<String>,
    /// Flavor Notes のタグ名の配列。無いときは空にする (FR-8)。`null` は受け付けない。
    #[serde(default, deserialize_with = "super::double_option")]
    flavor_notes: Option<Option<Vec<String>>>,
}

/// `PATCH /api/products/<ID>` の入力。無い項目は変更しない。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateInput {
    /// 新しい商品名。無いときは変更しない。`null` は拒否する (商品名は必須)。
    #[serde(default, deserialize_with = "super::double_option")]
    name: Option<Option<String>>,
    /// 新しい生産者。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    producer: Option<Option<String>>,
    /// 新しい生産国。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    origin: Option<Option<String>>,
    /// 新しい地域。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    region: Option<Option<String>>,
    /// 新しい精製方法。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    process: Option<Option<String>>,
    /// 新しい品種。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    variety: Option<Option<String>>,
    /// 新しい Flavor Notes。無いときは変更しない。空の配列で全て外し、`null` は拒否する。
    #[serde(default, deserialize_with = "super::double_option")]
    flavor_notes: Option<Option<Vec<String>>>,
}

/// 商品の一覧だけが読む、名前の完全一致の絞り込み (FR-19)。
///
/// 推測した商品名で既存の商品を 1 リクエストで引くために追加する。共有の [`ListParams`] には
/// 足さず、商品の一覧だけが読むクエリとして扱う (他の一覧は名前の絞り込みを持たない)。
struct NameFilter {
    /// 前後の空白を除いた名前。絞り込まないときは None。
    name: Option<String>,
}

impl NameFilter {
    /// リクエストのクエリ文字列から読む。複数回の指定と、空白だけの値は 400 の応答にする。
    fn from_request(req: &Request) -> Result<Self, Response> {
        let url = match req.url() {
            Ok(url) => url,
            Err(error) => {
                console_error!("the request URL is not readable: {error}");
                return Err(internal_error());
            }
        };
        let mut name = None;
        for (key, value) in url.query_pairs() {
            if key == "name" {
                // 複数回の指定は配列として扱い、受け取らない (`q` と同じ扱い)。
                if name.is_some() {
                    return Err(invalid_input("name must be a single string"));
                }
                name = Some(value.into_owned());
            }
        }
        let name = match name {
            Some(name) => {
                let name = name.trim();
                if name.is_empty() {
                    return Err(invalid_input("the name filter must not be empty"));
                }
                Some(name.to_owned())
            }
            None => None,
        };
        Ok(Self { name })
    }
}

/// 商品の一覧を返す。認証が必要。
pub async fn list(req: &Request, env: &Env, session: &Session) -> Result<Response> {
    let params = match ListParams::from_request(req) {
        Ok(params) => params,
        Err(response) => return Ok(response),
    };
    // 名前の完全一致の絞り込み (FR-19)。照合は前後の空白を除き、大文字と小文字を区別しない。
    let name = match NameFilter::from_request(req) {
        Ok(filter) => filter.name,
        Err(response) => return Ok(response),
    };
    let d1 = db::database(env)?;
    let statement = match query::products_list(
        &session.user_id,
        params.archived,
        params.cursor.clone(),
        params.limit,
        name.as_deref(),
    ) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    let mut products: Vec<ProductResponse> =
        db::prepared(&d1, &statement)?.all().await?.results()?;
    let next_cursor = params.next_cursor(
        OrderKind::DateTime,
        products
            .last()
            .map(|product| (product.created_at.as_str(), product.id.as_str())),
        products.len(),
    );
    let ids: Vec<&str> = products.iter().map(|product| product.id.as_str()).collect();
    let mut notes = flavor_notes_for(&d1, &session.user_id, &ids).await?;
    for product in &mut products {
        product.flavor_notes = notes.remove(&product.id).unwrap_or_default();
    }
    respond::json(&ProductListResponse {
        products,
        next_cursor,
    })
}

/// 商品を登録する。認証が必要。
pub async fn create(req: &mut Request, env: &Env, session: &Session) -> Result<Response> {
    let Some(input) = read_input::<CreateInput>(req).await else {
        return Ok(invalid_input("the body must be JSON with a name"));
    };
    let name = match validate_name(&input.name) {
        Ok(name) => name,
        Err(error) => return Ok(invalid_input(error.message())),
    };
    let flavor_notes = match input.flavor_notes {
        Some(Some(names)) => match validate_flavor_notes(&names) {
            Ok(names) => names,
            Err(error) => return Ok(invalid_input(error.message())),
        },
        // `null` は受け付けない。タグを付けないときは項目を省くか空の配列を送る。
        Some(None) => return Ok(invalid_input("the flavor notes must be an array")),
        None => Vec::new(),
    };
    let producer = trim_optional(input.producer.as_deref());
    let origin = trim_optional(input.origin.as_deref());
    let region = trim_optional(input.region.as_deref());
    let process = trim_optional(input.process.as_deref());
    let variety = trim_optional(input.variety.as_deref());
    let d1 = db::database(env)?;
    let id = db::new_id()?;
    let now = db::now_text()?;
    let values = ProductValues {
        name: &name,
        producer: producer.as_deref(),
        origin: origin.as_deref(),
        region: region.as_deref(),
        process: process.as_deref(),
        variety: variety.as_deref(),
    };
    let statement = match query::product_insert(&id, &session.user_id, &values, &now, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    if !flavor_notes.is_empty() {
        replace_flavor_notes(&d1, &session.user_id, &id, &flavor_notes).await?;
    }
    respond::json(&ProductResponse {
        id,
        user_id: session.user_id.clone(),
        name,
        producer,
        origin,
        region,
        process,
        variety,
        flavor_notes,
        created_at: now.clone(),
        updated_at: now,
        archived_at: None,
    })
}

/// 商品を 1 件返す。認証が必要。アーカイブ済みでも返す (FR-12)。
pub async fn get(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let d1 = db::database(env)?;
    let Some(id) = id else {
        return Ok(not_found("the product does not exist"));
    };
    match find(&d1, &session.user_id, id).await? {
        Some(mut product) => {
            product.flavor_notes = flavor_notes(&d1, &session.user_id, id).await?;
            respond::json(&product)
        }
        None => Ok(not_found("the product does not exist")),
    }
}

/// 商品を更新する。認証が必要。
pub async fn update(
    req: &mut Request,
    env: &Env,
    session: &Session,
    id: Option<&str>,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the product does not exist"));
    };
    let Some(input) = read_input::<UpdateInput>(req).await else {
        return Ok(invalid_input(
            "the body must be JSON with the fields to update",
        ));
    };
    let d1 = db::database(env)?;
    let Some(product) = find(&d1, &session.user_id, id).await? else {
        return Ok(not_found("the product does not exist"));
    };
    let name = match merge_name(input.name, product.name) {
        Ok(name) => name,
        Err(response) => return Ok(response),
    };
    let notes = match input.flavor_notes {
        Some(Some(names)) => match validate_flavor_notes(&names) {
            Ok(names) => Some(names),
            Err(error) => return Ok(invalid_input(error.message())),
        },
        // `null` は受け付けない。全て外すときは空の配列を送る (項目が無いときは変更しない)。
        Some(None) => return Ok(invalid_input("the flavor notes must be an array")),
        None => None,
    };
    let producer = merge_text(input.producer, product.producer);
    let origin = merge_text(input.origin, product.origin);
    let region = merge_text(input.region, product.region);
    let process = merge_text(input.process, product.process);
    let variety = merge_text(input.variety, product.variety);
    let now = db::now_text()?;
    let values = ProductValues {
        name: &name,
        producer: producer.as_deref(),
        origin: origin.as_deref(),
        region: region.as_deref(),
        process: process.as_deref(),
        variety: variety.as_deref(),
    };
    let statement = match query::product_update(id, &session.user_id, &values, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    let flavor_notes = match notes {
        Some(names) => {
            replace_flavor_notes(&d1, &session.user_id, id, &names).await?;
            names
        }
        None => flavor_notes(&d1, &session.user_id, id).await?,
    };
    respond::json(&ProductResponse {
        id: product.id,
        user_id: product.user_id,
        name,
        producer,
        origin,
        region,
        process,
        variety,
        flavor_notes,
        created_at: product.created_at,
        updated_at: now,
        archived_at: product.archived_at,
    })
}

/// 商品をアーカイブする、またはアーカイブ解除する。認証が必要。
/// 同じ状態への遷移はエラーにしない (繰り返し呼んでも 200 を返す)。
pub async fn archive(
    env: &Env,
    session: &Session,
    id: Option<&str>,
    archived: bool,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the product does not exist"));
    };
    let d1 = db::database(env)?;
    let Some(mut product) = find(&d1, &session.user_id, id).await? else {
        return Ok(not_found("the product does not exist"));
    };
    let now = db::now_text()?;
    let archived_at = if archived { Some(now.as_str()) } else { None };
    let statement = match query::product_set_archived(id, &session.user_id, archived_at, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    product.flavor_notes = flavor_notes(&d1, &session.user_id, id).await?;
    respond::json(&ProductResponse {
        archived_at: archived_at.map(str::to_owned),
        updated_at: now,
        ..product
    })
}

/// 商品を 1 件引く。アーカイブ済みも返す (FR-12)。
async fn find(d1: &D1Database, user_id: &str, id: &str) -> Result<Option<ProductResponse>> {
    let statement = query::product_find(user_id, id, Archived::Include);
    db::prepared(d1, &statement)?.first(None).await
}

/// 更新の入力の任意の値を、現在の値と合わせる。
/// 入力が無ければ現在の値、`null` なら NULL、値があれば前後の空白を除いた値にする。
fn merge_text(input: Option<Option<String>>, current: Option<String>) -> Option<String> {
    match input {
        Some(value) => trim_optional(value.as_deref()),
        None => current,
    }
}
