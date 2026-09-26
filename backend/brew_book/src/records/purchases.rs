//! 購入の API (FR-9、FR-12、FR-5)。
//!
//! 一覧はカーソル方式で、並び順は購入日の降順と ID の昇順とする (FR-9)。応答には続きを引く
//! `next_cursor` を含める (ページが `limit` に満たないときは null)。
//! 応答には商品 (`product`) と店 (`shop`) をネストしたオブジェクトとして含み、結合は 1 回の
//! SQL で行う (ADR-0006)。店が無い購入では `shop` は null になる (FR-9)。
//! 商品と店を指定した登録と、参照先を変更する更新は、存在しないか他の利用者の ID を 404、
//! アーカイブ済みの親を 409 にする (FR-9)。参照先を変えない更新は検査しない
//! (親をアーカイブしても購入を編集できるようにするため。0007 の設計判断)。
//! 単件取得はアーカイブ済みでも返し、更新もアーカイブ済みの購入にできる。
//! アーカイブと解除は繰り返し呼んでも 200 を返す。
//! 存在しない ID と他の利用者の ID は区別せず 404 を返す (ADR-0006)。
//! `updated_at` は更新、アーカイブ、アーカイブ解除で現在時刻にする (ADR-0006)。

use coffee_log_core::query::{self, Archived, OrderKind, PurchaseValues};
use coffee_log_core::records::{trim_optional, validate_count, validate_currency, validate_day};
use serde::{Deserialize, Serialize};
use worker::d1::D1Database;
use worker::{Env, Request, Response, Result};

use super::products::ProductResponse;
use super::shops::ShopResponse;
use super::{
    attach_flavor_notes, invalid_input, not_found, query_error_response, read_input,
    require_product, require_shop, ListParams,
};
use crate::auth::session::Session;
use crate::db;
use crate::respond;

/// 購入の応答。商品と店をネストしたオブジェクトとして含む (FR-9)。
#[derive(Debug, Serialize)]
pub struct PurchaseResponse {
    pub id: String,
    pub user_id: String,
    pub product_id: String,
    pub shop_id: Option<String>,
    pub purchased_on: String,
    pub roast: Option<String>,
    pub roast_date: Option<String>,
    pub price_amount: Option<i64>,
    pub price_currency: Option<String>,
    pub weight_grams: Option<i64>,
    /// 写真のオブジェクトキー。クライアントが写真の有無を知るために含める (操作は 0009)。
    pub photo_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
    /// 商品。必須の参照のため常にある (FR-9)。
    pub product: ProductResponse,
    /// 店。店が無い購入では null になる (FR-9)。
    pub shop: Option<ShopResponse>,
}

/// 購入の一覧の応答。
#[derive(Debug, Serialize)]
pub struct PurchaseListResponse {
    pub purchases: Vec<PurchaseResponse>,
    /// 続きのページを引くカーソル。続きが無いときは null。
    pub next_cursor: Option<String>,
}

/// 購入と商品と店を結合した行。項目名は `coffee_log_core::query` が付ける列の別名と同じ。
/// 抽出の行も同じ列を持つため、項目は `records` の配下から読めるようにする。
#[derive(Debug, Deserialize)]
pub(super) struct PurchaseJoinRow {
    pub(super) p_id: String,
    pub(super) p_user_id: String,
    pub(super) p_product_id: String,
    pub(super) p_shop_id: Option<String>,
    pub(super) p_purchased_on: String,
    pub(super) p_roast: Option<String>,
    pub(super) p_roast_date: Option<String>,
    pub(super) p_price_amount: Option<i64>,
    pub(super) p_price_currency: Option<String>,
    pub(super) p_weight_grams: Option<i64>,
    pub(super) p_photo_key: Option<String>,
    pub(super) p_created_at: String,
    pub(super) p_updated_at: String,
    pub(super) p_archived_at: Option<String>,
    pub(super) pr_id: String,
    pub(super) pr_user_id: String,
    pub(super) pr_name: String,
    pub(super) pr_producer: Option<String>,
    pub(super) pr_origin: Option<String>,
    pub(super) pr_region: Option<String>,
    pub(super) pr_process: Option<String>,
    pub(super) pr_variety: Option<String>,
    pub(super) pr_created_at: String,
    pub(super) pr_updated_at: String,
    pub(super) pr_archived_at: Option<String>,
    pub(super) sh_id: Option<String>,
    pub(super) sh_user_id: Option<String>,
    pub(super) sh_name: Option<String>,
    pub(super) sh_address: Option<String>,
    pub(super) sh_created_at: Option<String>,
    pub(super) sh_updated_at: Option<String>,
    pub(super) sh_archived_at: Option<String>,
}

impl PurchaseJoinRow {
    /// 応答に組み立てる。商品の Flavor Notes は別のクエリで引くため、ここでは空にする。
    pub(super) fn into_response(self) -> PurchaseResponse {
        PurchaseResponse {
            id: self.p_id,
            user_id: self.p_user_id,
            product_id: self.p_product_id,
            shop_id: self.p_shop_id,
            purchased_on: self.p_purchased_on,
            roast: self.p_roast,
            roast_date: self.p_roast_date,
            price_amount: self.p_price_amount,
            price_currency: self.p_price_currency,
            weight_grams: self.p_weight_grams,
            photo_key: self.p_photo_key,
            created_at: self.p_created_at,
            updated_at: self.p_updated_at,
            archived_at: self.p_archived_at,
            product: ProductResponse {
                id: self.pr_id,
                user_id: self.pr_user_id,
                name: self.pr_name,
                producer: self.pr_producer,
                origin: self.pr_origin,
                region: self.pr_region,
                process: self.pr_process,
                variety: self.pr_variety,
                flavor_notes: Vec::new(),
                created_at: self.pr_created_at,
                updated_at: self.pr_updated_at,
                archived_at: self.pr_archived_at,
            },
            // 店は LEFT JOIN のため、店が無いときは全ての列が NULL になる。
            shop: self.sh_id.map(|id| ShopResponse {
                id,
                user_id: self.sh_user_id.unwrap_or_default(),
                name: self.sh_name.unwrap_or_default(),
                address: self.sh_address,
                created_at: self.sh_created_at.unwrap_or_default(),
                updated_at: self.sh_updated_at.unwrap_or_default(),
                archived_at: self.sh_archived_at,
            }),
        }
    }
}

/// `POST /api/purchases` の入力 (FR-9)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateInput {
    /// 商品の ID。必須。
    product_id: String,
    /// 店の ID。任意 (店が無い購入がある。ADR-0006)。
    #[serde(default)]
    shop_id: Option<String>,
    /// 購入日 (`YYYY-MM-DD`)。必須。
    purchased_on: String,
    /// Roast。任意。
    #[serde(default)]
    roast: Option<String>,
    /// Roast Date (`YYYY-MM-DD`)。任意。
    #[serde(default)]
    roast_date: Option<String>,
    /// 価格 (通貨の最小単位)。任意。
    #[serde(default)]
    price_amount: Option<i64>,
    /// ISO 4217 の通貨コード。省略したときは JPY を保存する (FR-9)。
    #[serde(default)]
    price_currency: Option<String>,
    /// 重量 (グラム)。任意。
    #[serde(default)]
    weight_grams: Option<i64>,
}

/// `PATCH /api/purchases/<ID>` の入力。無い項目は変更しない。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateInput {
    /// 新しい商品。無いときは変更しない。`null` は拒否する (商品は必須)。
    #[serde(default, deserialize_with = "super::double_option")]
    product_id: Option<Option<String>>,
    /// 新しい店。`null` のときは店を外す。
    #[serde(default, deserialize_with = "super::double_option")]
    shop_id: Option<Option<String>>,
    /// 新しい購入日。無いときは変更しない。`null` は拒否する (購入日は必須)。
    #[serde(default, deserialize_with = "super::double_option")]
    purchased_on: Option<Option<String>>,
    /// 新しい Roast。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    roast: Option<Option<String>>,
    /// 新しい Roast Date。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    roast_date: Option<Option<String>>,
    /// 新しい価格。`null` のときは通貨コードも null にする。
    #[serde(default, deserialize_with = "super::double_option")]
    price_amount: Option<Option<i64>>,
    /// 新しい通貨コード。`null` は拒否する (価格と組で扱うため)。
    #[serde(default, deserialize_with = "super::double_option")]
    price_currency: Option<Option<String>>,
    /// 新しい重量。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    weight_grams: Option<Option<i64>>,
}

/// 購入の一覧を返す。認証が必要。アーカイブ済みは既定では返さない (FR-12)。
pub async fn list(req: &Request, env: &Env, session: &Session) -> Result<Response> {
    let params = match ListParams::from_request(req) {
        Ok(params) => params,
        Err(response) => return Ok(response),
    };
    let d1 = db::database(env)?;
    let statement = match query::purchases_list(
        &session.user_id,
        params.archived,
        params.cursor.clone(),
        params.limit,
    ) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    let rows: Vec<PurchaseJoinRow> = db::prepared(&d1, &statement)?.all().await?.results()?;
    let mut purchases = responses(rows);
    attach_notes(&d1, &session.user_id, &mut purchases).await?;
    let next_cursor = params.next_cursor(
        OrderKind::Date,
        purchases
            .last()
            .map(|purchase| (purchase.purchased_on.as_str(), purchase.id.as_str())),
        purchases.len(),
    );
    respond::json(&PurchaseListResponse {
        purchases,
        next_cursor,
    })
}

/// 購入を登録する。認証が必要。
pub async fn create(req: &mut Request, env: &Env, session: &Session) -> Result<Response> {
    let Some(input) = read_input::<CreateInput>(req).await else {
        return Ok(invalid_input(
            "the body must be JSON with a product and a purchased date",
        ));
    };
    let d1 = db::database(env)?;
    // 商品と店は、存在しないか他の利用者のものは 404、アーカイブ済みは 409 にする (FR-9)。
    if let Err(response) = require_product(&d1, &session.user_id, &input.product_id)
        .await?
        .or_return()
    {
        return Ok(response);
    }
    if let Some(shop_id) = input.shop_id.as_deref() {
        if let Err(response) = require_shop(&d1, &session.user_id, shop_id)
            .await?
            .or_return()
        {
            return Ok(response);
        }
    }
    if let Err(error) = validate_day(&input.purchased_on) {
        return Ok(invalid_input(error.message()));
    }
    if let Some(roast_date) = input.roast_date.as_deref() {
        if let Err(error) = validate_day(roast_date) {
            return Ok(invalid_input(error.message()));
        }
    }
    let (price_amount, price_currency) = match merge_price(
        input.price_amount.map(Some),
        input.price_currency.map(Some),
        None,
        None,
    ) {
        Ok(pair) => pair,
        Err(response) => return Ok(response),
    };
    let weight_grams = match merge_count(input.weight_grams) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let roast = trim_optional(input.roast.as_deref());
    let id = db::new_id()?;
    let now = db::now_text()?;
    let values = PurchaseValues {
        product_id: &input.product_id,
        shop_id: input.shop_id.as_deref(),
        purchased_on: &input.purchased_on,
        roast: roast.as_deref(),
        roast_date: input.roast_date.as_deref(),
        price_amount,
        price_currency: price_currency.as_deref(),
        weight_grams,
    };
    let statement = match query::purchase_insert(&id, &session.user_id, &values, &now, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    respond_fetched(&d1, &session.user_id, &id).await
}

/// 購入を 1 件返す。認証が必要。アーカイブ済みでも返す (FR-12)。
pub async fn get(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let d1 = db::database(env)?;
    let Some(id) = id else {
        return Ok(not_found("the purchase does not exist"));
    };
    match find(&d1, &session.user_id, id).await? {
        Some(purchase) => respond::json(&purchase),
        None => Ok(not_found("the purchase does not exist")),
    }
}

/// 購入を更新する。認証が必要。
pub async fn update(
    req: &mut Request,
    env: &Env,
    session: &Session,
    id: Option<&str>,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the purchase does not exist"));
    };
    let Some(input) = read_input::<UpdateInput>(req).await else {
        return Ok(invalid_input(
            "the body must be JSON with the fields to update",
        ));
    };
    let d1 = db::database(env)?;
    let Some(current) = find(&d1, &session.user_id, id).await? else {
        return Ok(not_found("the purchase does not exist"));
    };
    let product_id = match input.product_id {
        Some(Some(product_id)) => {
            // 参照先を変更する更新では、新しい参照先を登録と同じ条件で検査する (FR-9)。
            if product_id != current.product_id {
                if let Err(response) = require_product(&d1, &session.user_id, &product_id)
                    .await?
                    .or_return()
                {
                    return Ok(response);
                }
            }
            product_id
        }
        // 商品は必須のため `null` を拒否する。
        Some(None) => return Ok(invalid_input("the product must not be null")),
        None => current.product_id.clone(),
    };
    let shop_id = match input.shop_id {
        Some(shop_id) => {
            if let Some(new_shop_id) = shop_id.as_deref() {
                if current.shop_id.as_deref() != Some(new_shop_id) {
                    if let Err(response) = require_shop(&d1, &session.user_id, new_shop_id)
                        .await?
                        .or_return()
                    {
                        return Ok(response);
                    }
                }
            }
            shop_id
        }
        None => current.shop_id.clone(),
    };
    let purchased_on = match input.purchased_on {
        Some(Some(purchased_on)) => {
            if let Err(error) = validate_day(&purchased_on) {
                return Ok(invalid_input(error.message()));
            }
            purchased_on
        }
        // 購入日は必須のため `null` を拒否する。
        Some(None) => return Ok(invalid_input("the purchased date must not be null")),
        None => current.purchased_on.clone(),
    };
    let roast_date = match input.roast_date {
        Some(roast_date) => {
            if let Some(roast_date) = roast_date.as_deref() {
                if let Err(error) = validate_day(roast_date) {
                    return Ok(invalid_input(error.message()));
                }
            }
            roast_date
        }
        None => current.roast_date.clone(),
    };
    let (price_amount, price_currency) = match merge_price(
        input.price_amount,
        input.price_currency,
        current.price_amount,
        current.price_currency.clone(),
    ) {
        Ok(pair) => pair,
        Err(response) => return Ok(response),
    };
    let weight_grams = match input.weight_grams {
        Some(weight_grams) => match merge_count(weight_grams) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        },
        None => current.weight_grams,
    };
    let roast = match input.roast {
        Some(roast) => trim_optional(roast.as_deref()),
        None => current.roast.clone(),
    };
    let now = db::now_text()?;
    let values = PurchaseValues {
        product_id: &product_id,
        shop_id: shop_id.as_deref(),
        purchased_on: &purchased_on,
        roast: roast.as_deref(),
        roast_date: roast_date.as_deref(),
        price_amount,
        price_currency: price_currency.as_deref(),
        weight_grams,
    };
    let statement = match query::purchase_update(id, &session.user_id, &values, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    respond_fetched(&d1, &session.user_id, id).await
}

/// 購入をアーカイブする、またはアーカイブ解除する。認証が必要。
/// 同じ状態への遷移はエラーにしない (繰り返し呼んでも 200 を返す)。
pub async fn archive(
    env: &Env,
    session: &Session,
    id: Option<&str>,
    archived: bool,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the purchase does not exist"));
    };
    let d1 = db::database(env)?;
    let Some(mut purchase) = find(&d1, &session.user_id, id).await? else {
        return Ok(not_found("the purchase does not exist"));
    };
    let now = db::now_text()?;
    let archived_at = if archived { Some(now.as_str()) } else { None };
    let statement = match query::purchase_set_archived(id, &session.user_id, archived_at, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    purchase.archived_at = archived_at.map(str::to_owned);
    purchase.updated_at = now;
    respond::json(&purchase)
}

/// 購入を 1 件引く。商品と店を結合し、アーカイブ済みも返す (FR-9、FR-12)。
pub(super) async fn find(
    d1: &D1Database,
    user_id: &str,
    id: &str,
) -> Result<Option<PurchaseResponse>> {
    let statement = query::purchase_find(user_id, id, Archived::Include);
    let row: Option<PurchaseJoinRow> = db::prepared(d1, &statement)?.first(None).await?;
    match row {
        Some(row) => {
            let mut purchase = row.into_response();
            attach_notes(d1, user_id, std::slice::from_mut(&mut purchase)).await?;
            Ok(Some(purchase))
        }
        None => Ok(None),
    }
}

/// 登録または更新の後に、現在の内容を引いて返す。
pub(super) async fn respond_fetched(d1: &D1Database, user_id: &str, id: &str) -> Result<Response> {
    match find(d1, user_id, id).await? {
        Some(purchase) => respond::json(&purchase),
        None => Ok(super::internal_error()),
    }
}

/// 結合した行を応答の並びにする。
fn responses(rows: Vec<PurchaseJoinRow>) -> Vec<PurchaseResponse> {
    rows.into_iter()
        .map(PurchaseJoinRow::into_response)
        .collect()
}

/// 応答の商品に Flavor Notes を付ける (FR-8)。
async fn attach_notes(
    d1: &D1Database,
    user_id: &str,
    purchases: &mut [PurchaseResponse],
) -> Result<()> {
    let mut products: Vec<&mut ProductResponse> = purchases
        .iter_mut()
        .map(|purchase| &mut purchase.product)
        .collect();
    attach_flavor_notes(d1, user_id, &mut products).await
}

/// 価格と通貨コードの組を決める (0007 の設計判断)。
///
/// 価格が無いときは通貨コードも null にする。価格だけを指定して通貨コードを省略した場合は
/// JPY を保存し、更新では現在の通貨コードを保つ。価格があるのに通貨コードを null にする
/// ことはできない (組は片方だけを持たない)。
fn merge_price(
    amount: Option<Option<i64>>,
    currency: Option<Option<String>>,
    current_amount: Option<i64>,
    current_currency: Option<String>,
) -> std::result::Result<(Option<i64>, Option<String>), Response> {
    let nulled_currency = matches!(currency, Some(None));
    // 価格が無いのに通貨コードだけを指定することはできない (組は片方だけを持たない)。
    let currency_without_amount = matches!(currency, Some(Some(_)));
    let mut currency = match currency {
        Some(Some(text)) => Some(match validate_currency(&text) {
            Ok(currency) => currency,
            Err(error) => return Err(invalid_input(error.message())),
        }),
        Some(None) => None,
        None => current_currency,
    };
    // 価格は 0 以上で D1 の整数に収まる値だけを受け付ける (FR-9)。
    let amount = match amount {
        Some(Some(value)) => Some(match validate_count(value) {
            Ok(value) => value,
            Err(error) => return Err(invalid_input(error.message())),
        }),
        Some(None) => None,
        None => current_amount,
    };
    if amount.is_none() {
        if currency_without_amount {
            return Err(invalid_input(
                "the price currency cannot be set without the price",
            ));
        }
        currency = None;
    } else if nulled_currency {
        return Err(invalid_input(
            "the price currency cannot be null when the price is set",
        ));
    } else if currency.is_none() {
        currency = Some(coffee_log_core::records::DEFAULT_CURRENCY.to_owned());
    }
    Ok((amount, currency))
}

/// 価格、重量、時間の検証 (0 以上で D1 の整数に収まる値)。
fn merge_count(value: Option<i64>) -> std::result::Result<Option<i64>, Response> {
    match value {
        Some(value) => match validate_count(value) {
            Ok(value) => Ok(Some(value)),
            Err(error) => Err(invalid_input(error.message())),
        },
        None => Ok(None),
    }
}
