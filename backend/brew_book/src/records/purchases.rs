//! 購入の API (FR-9、FR-5)。
//!
//! 一覧はカーソル方式で、並び順は購入日の降順と ID の昇順とする (FR-9)。応答には続きを引く
//! `next_cursor` を含める (ページが `limit` に満たないときは null)。
//! 商品の項目 (商品名、Producer、Origin、Region、Process、Variety、Flavor Notes) は購入が直接
//! 持つ (ADR-0022)。応答には店 (`shop`) をネストしたオブジェクトとして含み、結合は 1 回の
//! SQL で行う (ADR-0006)。店が無い購入では `shop` は null になる (FR-9)。
//! Flavor Notes はタグ名の配列 (`flavor_notes`) で、並びは名前の昇順にする (FR-8)。
//! 店を指定した登録と、参照先を変更する更新は、存在しないか他の利用者の ID を 404 に
//! する (FR-9)。参照先を変えない更新は検査しない (0007 の設計判断)。
//! 存在しない ID と他の利用者の ID は区別せず 404 を返す (ADR-0006)。
//! `updated_at` は更新で現在時刻にする (ADR-0006)。

use brew_book_core::cursor::{CursorValue, SortKey};
use brew_book_core::query::{self, PurchaseValues};
use brew_book_core::records::{
    trim_optional, validate_count, validate_currency, validate_day, validate_flavor_notes,
    validate_name,
};
use brew_book_core::taste::TasteProfile;
use serde::{Deserialize, Serialize};
use worker::d1::D1Database;
use worker::{Env, Request, Response, Result};

use super::shops::ShopResponse;
use super::{
    apply_favorite, attach_flavor_notes, invalid_input, merge_name, merge_text, not_found, photos,
    query_error_response, read_input, replace_flavor_notes_statements, require_shop, ListParams,
};
use crate::auth::session::Session;
use crate::db;
use crate::respond;

/// 購入の応答。商品の項目を直下に持ち、店をネストしたオブジェクトとして含む (FR-9、ADR-0022)。
#[derive(Debug, Serialize)]
pub struct PurchaseResponse {
    pub id: String,
    pub user_id: String,
    pub shop_id: Option<String>,
    pub purchased_on: String,
    pub roast: Option<String>,
    pub roast_date: Option<String>,
    pub price_amount: Option<i64>,
    pub price_currency: Option<String>,
    pub weight_grams: Option<i64>,
    /// 写真のオブジェクトキー。クライアントが写真の有無を知るために含める (操作は 0009)。
    pub photo_key: Option<String>,
    /// 商品名。必須 (FR-9)。
    pub name: String,
    pub producer: Option<String>,
    pub origin: Option<String>,
    pub region: Option<String>,
    pub process: Option<String>,
    pub variety: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// お気に入りにした日時。未設定のときは null (FR-21)。
    pub favorited_at: Option<String>,
    /// Flavor Notes のタグ名。並びは名前の昇順 (FR-8)。
    pub flavor_notes: Vec<String>,
    /// AI が推定した味と特徴。未生成のときは null (FR-24)。
    pub taste_profile: Option<TasteProfile>,
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

/// 購入と店を結合した行。項目名は `brew_book_core::query` が付ける列の別名と同じ。
/// 抽出の行も同じ列を持つため、項目は `records` の配下から読めるようにする。
#[derive(Debug, Deserialize)]
pub(super) struct PurchaseJoinRow {
    pub(super) p_id: String,
    pub(super) p_user_id: String,
    pub(super) p_shop_id: Option<String>,
    pub(super) p_purchased_on: String,
    pub(super) p_roast: Option<String>,
    pub(super) p_roast_date: Option<String>,
    pub(super) p_price_amount: Option<i64>,
    pub(super) p_price_currency: Option<String>,
    pub(super) p_weight_grams: Option<i64>,
    pub(super) p_photo_key: Option<String>,
    pub(super) p_name: String,
    pub(super) p_producer: Option<String>,
    pub(super) p_origin: Option<String>,
    pub(super) p_region: Option<String>,
    pub(super) p_process: Option<String>,
    pub(super) p_variety: Option<String>,
    pub(super) p_created_at: String,
    pub(super) p_updated_at: String,
    pub(super) p_favorited_at: Option<String>,
    /// AI が推定した味と特徴の JSON 文字列。未生成のときは None (FR-24)。
    pub(super) p_taste_profile: Option<String>,
    pub(super) sh_id: Option<String>,
    pub(super) sh_user_id: Option<String>,
    pub(super) sh_name: Option<String>,
    pub(super) sh_address: Option<String>,
    pub(super) sh_created_at: Option<String>,
    pub(super) sh_updated_at: Option<String>,
    pub(super) sh_favorited_at: Option<String>,
}

impl PurchaseJoinRow {
    /// 応答に組み立てる。Flavor Notes は別のクエリで引くため、ここでは空にする。
    pub(super) fn into_response(self) -> PurchaseResponse {
        let taste_profile = self
            .p_taste_profile
            .as_deref()
            .and_then(|text| serde_json::from_str::<TasteProfile>(text).ok());
        PurchaseResponse {
            id: self.p_id,
            user_id: self.p_user_id,
            shop_id: self.p_shop_id,
            purchased_on: self.p_purchased_on,
            roast: self.p_roast,
            roast_date: self.p_roast_date,
            price_amount: self.p_price_amount,
            price_currency: self.p_price_currency,
            weight_grams: self.p_weight_grams,
            photo_key: self.p_photo_key,
            name: self.p_name,
            producer: self.p_producer,
            origin: self.p_origin,
            region: self.p_region,
            process: self.p_process,
            variety: self.p_variety,
            created_at: self.p_created_at,
            updated_at: self.p_updated_at,
            favorited_at: self.p_favorited_at,
            flavor_notes: Vec::new(),
            taste_profile,
            // 店は LEFT JOIN のため、店が無いときは全ての列が NULL になる。
            shop: self.sh_id.map(|id| ShopResponse {
                id,
                user_id: self.sh_user_id.unwrap_or_default(),
                name: self.sh_name.unwrap_or_default(),
                address: self.sh_address,
                created_at: self.sh_created_at.unwrap_or_default(),
                updated_at: self.sh_updated_at.unwrap_or_default(),
                favorited_at: self.sh_favorited_at,
            }),
        }
    }
}

/// `POST /api/purchases` の入力 (FR-9)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateInput {
    /// 商品名。必須 (FR-9、ADR-0022)。
    name: String,
    /// 生産者。任意。
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
    /// Flavor Notes のタグ名の配列。任意 (省略は空の配列と同じ)。`null` は拒否する (FR-8)。
    #[serde(default, deserialize_with = "super::double_option")]
    flavor_notes: Option<Option<Vec<String>>>,
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
    /// 新しい Flavor Notes。無いときは変更しない。空の配列は全て外す。`null` は拒否する (FR-8)。
    #[serde(default, deserialize_with = "super::double_option")]
    flavor_notes: Option<Option<Vec<String>>>,
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

/// 購入の一覧を返す。認証が必要。`sort`、`order`、`favorite` を受け付ける (FR-20、FR-21)。
pub async fn list(req: &Request, env: &Env, session: &Session) -> Result<Response> {
    let params =
        match ListParams::from_request(req, SortKey::PurchasedOn, query::PURCHASE_SORT_KEYS) {
            Ok(params) => params,
            Err(response) => return Ok(response),
        };
    let d1 = db::database(env)?;
    let statement = match query::purchases_list(
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
    let rows: Vec<PurchaseJoinRow> = db::prepared(&d1, &statement)?.all().await?.results()?;
    let mut purchases = responses(rows);
    attach_notes(&d1, &session.user_id, &mut purchases).await?;
    let next_cursor = purchases.last().and_then(|purchase| {
        params.next_cursor(
            sort_value(purchase, params.sort),
            &purchase.id,
            purchases.len(),
        )
    });
    respond::json(&PurchaseListResponse {
        purchases,
        next_cursor,
    })
}

/// 並び順のキーの値を、応答の行から取り出す (FR-20)。
fn sort_value(purchase: &PurchaseResponse, sort: SortKey) -> Option<CursorValue> {
    match sort {
        SortKey::PurchasedOn => Some(CursorValue::Text(purchase.purchased_on.clone())),
        SortKey::PriceAmount => purchase.price_amount.map(CursorValue::Integer),
        SortKey::WeightGrams => purchase.weight_grams.map(CursorValue::Integer),
        _ => None,
    }
}

/// 購入を登録する。認証が必要。
pub async fn create(req: &mut Request, env: &Env, session: &Session) -> Result<Response> {
    let Some(input) = read_input::<CreateInput>(req).await else {
        return Ok(invalid_input(
            "the body must be JSON with a name and a purchased date",
        ));
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
    let d1 = db::database(env)?;
    // 店は、存在しないか他の利用者のものは 404 にする (FR-9)。
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
    let producer = trim_optional(input.producer.as_deref());
    let origin = trim_optional(input.origin.as_deref());
    let region = trim_optional(input.region.as_deref());
    let process = trim_optional(input.process.as_deref());
    let variety = trim_optional(input.variety.as_deref());
    let id = db::new_id()?;
    let now = db::now_text()?;
    let values = PurchaseValues {
        name: &name,
        producer: producer.as_deref(),
        origin: origin.as_deref(),
        region: region.as_deref(),
        process: process.as_deref(),
        variety: variety.as_deref(),
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
    // 購入の行とタグの対応を 1 つの batch (1 トランザクション) で書く (ADR-0002)。
    let mut statements = vec![db::prepared(&d1, &statement)?];
    if !flavor_notes.is_empty() {
        statements.extend(replace_flavor_notes_statements(
            &d1,
            &session.user_id,
            &id,
            &flavor_notes,
        )?);
    }
    db::execute_batch(&d1, statements).await?;
    respond_fetched(&d1, &session.user_id, &id).await
}

/// 購入を 1 件返す。認証が必要。
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
    let name = match merge_name(input.name, current.name.clone()) {
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
    let producer = merge_text(input.producer, current.producer.clone());
    let origin = merge_text(input.origin, current.origin.clone());
    let region = merge_text(input.region, current.region.clone());
    let process = merge_text(input.process, current.process.clone());
    let variety = merge_text(input.variety, current.variety.clone());
    let shop_id = match input.shop_id {
        Some(shop_id) => {
            if let Some(new_shop_id) = shop_id.as_deref() {
                if current.shop_id.as_deref() != Some(new_shop_id) {
                    // 参照先を変更する更新では、新しい参照先を登録と同じ条件で検査する (FR-9)。
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
        name: &name,
        producer: producer.as_deref(),
        origin: origin.as_deref(),
        region: region.as_deref(),
        process: process.as_deref(),
        variety: variety.as_deref(),
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
    // 購入の行とタグの対応を 1 つの batch (1 トランザクション) で書く (ADR-0002)。
    let mut statements = vec![db::prepared(&d1, &statement)?];
    if let Some(names) = notes.as_deref() {
        statements.extend(replace_flavor_notes_statements(
            &d1,
            &session.user_id,
            id,
            names,
        )?);
    }
    db::execute_batch(&d1, statements).await?;
    respond_fetched(&d1, &session.user_id, id).await
}

/// 購入を 1 件引く。店を結合し、Flavor Notes を付ける (FR-9)。
pub(super) async fn find(
    d1: &D1Database,
    user_id: &str,
    id: &str,
) -> Result<Option<PurchaseResponse>> {
    let statement = query::purchase_find(user_id, id);
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
/// 既に同じ状態のときは `favorited_at` と `updated_at` を変えず、現在の応答をそのまま返す。
async fn set_favorite(
    env: &Env,
    session: &Session,
    id: Option<&str>,
    favorite: bool,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the purchase does not exist"));
    };
    let d1 = db::database(env)?;
    let Some(purchase) = find(&d1, &session.user_id, id).await? else {
        return Ok(not_found("the purchase does not exist"));
    };
    let changed = apply_favorite(
        &d1,
        query::PURCHASES_TABLE,
        id,
        &session.user_id,
        purchase.favorited_at.as_deref(),
        favorite,
    )
    .await?;
    if changed.is_none() {
        return respond::json(&purchase);
    }
    respond_fetched(&d1, &session.user_id, id).await
}

/// 購入を削除する (0056)。認証が必要。
///
/// その購入の抽出、Flavor Notes の対応、購入の写真 (R2) も削除する。タグの行 (flavor_tags) は
/// 消さない (ADR-0006)。写真は D1 の前に消し、失敗した場合は記録が残ったまま再試行できる
/// (アカウント削除と同じ順序)。
pub async fn delete(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the purchase does not exist"));
    };
    let d1 = db::database(env)?;
    let Some(purchase) = find(&d1, &session.user_id, id).await? else {
        return Ok(not_found("the purchase does not exist"));
    };
    if let Some(key) = purchase.photo_key.as_deref() {
        photos::delete_objects(env, &[key.to_owned()]).await?;
    }
    // 外部キーの参照元から先に消す (ADR-0006)。1 つの batch で 1 トランザクションにする (ADR-0002)。
    let statements = vec![
        db::prepared(
            &d1,
            &query::delete_purchase_flavor_tags(&session.user_id, id),
        )?,
        db::prepared(&d1, &query::purchase_brews_delete(&session.user_id, id))?,
        db::prepared(&d1, &query::purchase_delete(&session.user_id, id))?,
    ];
    db::execute_batch(&d1, statements).await?;
    Ok(Response::empty()?.with_status(204))
}

/// 購入の削除で消える抽出の件数を返す (0056)。認証が必要。
pub async fn delete_impact(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the purchase does not exist"));
    };
    let d1 = db::database(env)?;
    if find(&d1, &session.user_id, id).await?.is_none() {
        return Ok(not_found("the purchase does not exist"));
    }
    let brews =
        super::count_rows(&d1, &query::purchase_delete_impact(&session.user_id, id)).await?;
    respond::json(&PurchaseDeleteImpactResponse { brews })
}

/// 購入の削除の影響の応答 (0056)。
#[derive(Debug, Serialize)]
struct PurchaseDeleteImpactResponse {
    /// 削除される抽出の件数。
    brews: i64,
}

/// 結合した行を応答の並びにする。
fn responses(rows: Vec<PurchaseJoinRow>) -> Vec<PurchaseResponse> {
    rows.into_iter()
        .map(PurchaseJoinRow::into_response)
        .collect()
}

/// 応答の購入に Flavor Notes を付ける (FR-8)。
async fn attach_notes(
    d1: &D1Database,
    user_id: &str,
    purchases: &mut [PurchaseResponse],
) -> Result<()> {
    let mut targets: Vec<&mut PurchaseResponse> = purchases.iter_mut().collect();
    attach_flavor_notes(d1, user_id, &mut targets).await
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
        currency = Some(brew_book_core::records::DEFAULT_CURRENCY.to_owned());
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
