//! 抽出の API (FR-11、FR-5)。
//!
//! 一覧はカーソル方式で、並び順は抽出日時の降順と ID の昇順とする (FR-11)。応答には続きを引く
//! `next_cursor` を含める (ページが `limit` に満たないときは null)。
//! 応答には購入 (`purchase`) をネストし、その中に商品 (`product`) と店 (`shop`) を含める (FR-11)。
//! 購入、商品、店の結合は 1 回の SQL で行う (ADR-0006)。店が無い購入では `shop` は null になる。
//! 購入を指定した登録と、参照先の購入を変更する更新は、存在しないか他の利用者の ID を 404 に
//! する (FR-11)。参照先を変えない更新は検査しない (0007 の設計判断)。
//! 存在しない ID と他の利用者の ID は区別せず 404 を返す (ADR-0006)。
//! `updated_at` は更新で現在時刻にする (ADR-0006)。

use brew_book_core::cursor::{CursorValue, SortKey};
use brew_book_core::query::{self, BrewValues};
use brew_book_core::records::{
    trim_optional, validate_count, validate_decimal, validate_rating, validate_timestamp,
};
use serde::{Deserialize, Serialize};
use worker::d1::D1Database;
use worker::{Env, Request, Response, Result};

use super::products::ProductResponse;
use super::purchases::{PurchaseJoinRow, PurchaseResponse};
use super::{
    apply_favorite, attach_flavor_notes, invalid_input, not_found, query_error_response,
    read_input, require_purchase, ListParams,
};
use crate::auth::session::Session;
use crate::db;
use crate::respond;

/// 抽出の応答。購入をネストし、その中に商品と店を含める (FR-11)。
#[derive(Debug, Serialize)]
pub struct BrewResponse {
    pub id: String,
    pub user_id: String,
    pub purchase_id: String,
    pub brewed_at: String,
    pub dose_grams: Option<f64>,
    pub water_grams: Option<f64>,
    pub water_temp_c: Option<f64>,
    pub brew_time_seconds: Option<i64>,
    pub method: Option<String>,
    pub grind_setting: Option<String>,
    pub rating: Option<i64>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// お気に入りにした日時。未設定のときは null (FR-21)。
    pub favorited_at: Option<String>,
    /// 購入。必須の参照のため常にある (FR-11)。
    pub purchase: PurchaseResponse,
}

/// 抽出の一覧の応答。
#[derive(Debug, Serialize)]
pub struct BrewListResponse {
    pub brews: Vec<BrewResponse>,
    /// 続きのページを引くカーソル。続きが無いときは null。
    pub next_cursor: Option<String>,
}

/// 抽出と、購入、商品、店を結合した行。項目名は `brew_book_core::query` が付ける列の別名と同じ。
#[derive(Debug, Deserialize)]
struct BrewJoinRow {
    b_id: String,
    b_user_id: String,
    b_purchase_id: String,
    b_brewed_at: String,
    b_dose_grams: Option<f64>,
    b_water_grams: Option<f64>,
    b_water_temp_c: Option<f64>,
    b_brew_time_seconds: Option<i64>,
    b_method: Option<String>,
    b_grind_setting: Option<String>,
    b_rating: Option<i64>,
    b_notes: Option<String>,
    b_created_at: String,
    b_updated_at: String,
    b_favorited_at: Option<String>,
    p_id: String,
    p_user_id: String,
    p_product_id: String,
    p_shop_id: Option<String>,
    p_purchased_on: String,
    p_roast: Option<String>,
    p_roast_date: Option<String>,
    p_price_amount: Option<i64>,
    p_price_currency: Option<String>,
    p_weight_grams: Option<i64>,
    p_photo_key: Option<String>,
    p_created_at: String,
    p_updated_at: String,
    p_favorited_at: Option<String>,
    pr_id: String,
    pr_user_id: String,
    pr_name: String,
    pr_producer: Option<String>,
    pr_origin: Option<String>,
    pr_region: Option<String>,
    pr_process: Option<String>,
    pr_variety: Option<String>,
    pr_created_at: String,
    pr_updated_at: String,
    pr_favorited_at: Option<String>,
    sh_id: Option<String>,
    sh_user_id: Option<String>,
    sh_name: Option<String>,
    sh_address: Option<String>,
    sh_created_at: Option<String>,
    sh_updated_at: Option<String>,
    sh_favorited_at: Option<String>,
}

impl BrewJoinRow {
    /// 応答に組み立てる。商品の Flavor Notes は別のクエリで引くため、ここでは空にする。
    ///
    /// 購入と商品と店の部分は、購入の行と同じ形にしてから組み立てる。
    fn into_response(self) -> BrewResponse {
        let purchase = PurchaseJoinRow {
            p_id: self.p_id,
            p_user_id: self.p_user_id,
            p_product_id: self.p_product_id,
            p_shop_id: self.p_shop_id,
            p_purchased_on: self.p_purchased_on,
            p_roast: self.p_roast,
            p_roast_date: self.p_roast_date,
            p_price_amount: self.p_price_amount,
            p_price_currency: self.p_price_currency,
            p_weight_grams: self.p_weight_grams,
            p_photo_key: self.p_photo_key,
            p_created_at: self.p_created_at,
            p_updated_at: self.p_updated_at,
            p_favorited_at: self.p_favorited_at,
            pr_id: self.pr_id,
            pr_user_id: self.pr_user_id,
            pr_name: self.pr_name,
            pr_producer: self.pr_producer,
            pr_origin: self.pr_origin,
            pr_region: self.pr_region,
            pr_process: self.pr_process,
            pr_variety: self.pr_variety,
            pr_created_at: self.pr_created_at,
            pr_updated_at: self.pr_updated_at,
            pr_favorited_at: self.pr_favorited_at,
            sh_id: self.sh_id,
            sh_user_id: self.sh_user_id,
            sh_name: self.sh_name,
            sh_address: self.sh_address,
            sh_created_at: self.sh_created_at,
            sh_updated_at: self.sh_updated_at,
            sh_favorited_at: self.sh_favorited_at,
        };
        BrewResponse {
            id: self.b_id,
            user_id: self.b_user_id,
            purchase_id: self.b_purchase_id,
            brewed_at: self.b_brewed_at,
            dose_grams: self.b_dose_grams,
            water_grams: self.b_water_grams,
            water_temp_c: self.b_water_temp_c,
            brew_time_seconds: self.b_brew_time_seconds,
            method: self.b_method,
            grind_setting: self.b_grind_setting,
            rating: self.b_rating,
            notes: self.b_notes,
            created_at: self.b_created_at,
            updated_at: self.b_updated_at,
            favorited_at: self.b_favorited_at,
            purchase: purchase.into_response(),
        }
    }
}

/// `POST /api/brews` の入力 (FR-11)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateInput {
    /// 購入の ID。必須。
    purchase_id: String,
    /// 抽出日時 (ISO 8601 の UTC)。必須。
    brewed_at: String,
    /// 豆の量 (グラム)。任意。0 以上で小数第 1 位まで。
    #[serde(default)]
    dose_grams: Option<serde_json::Number>,
    /// 湯量 (グラム)。任意。0 以上で小数第 1 位まで。
    #[serde(default)]
    water_grams: Option<serde_json::Number>,
    /// 湯の温度 (摂氏)。任意。0 以上で小数第 1 位まで。
    #[serde(default)]
    water_temp_c: Option<serde_json::Number>,
    /// 時間 (秒)。任意。0 以上の整数。
    #[serde(default)]
    brew_time_seconds: Option<i64>,
    /// 抽出方法。任意。
    #[serde(default)]
    method: Option<String>,
    /// 挽き目 (グラインダーの設定値)。任意。
    #[serde(default)]
    grind_setting: Option<String>,
    /// 評価。任意。1 から 5 の整数。
    #[serde(default)]
    rating: Option<i64>,
    /// 感想。任意。
    #[serde(default)]
    notes: Option<String>,
}

/// `PATCH /api/brews/<ID>` の入力。無い項目は変更しない。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateInput {
    /// 新しい購入。無いときは変更しない。`null` は拒否する (購入は必須)。
    #[serde(default, deserialize_with = "super::double_option")]
    purchase_id: Option<Option<String>>,
    /// 新しい抽出日時。無いときは変更しない。`null` は拒否する (抽出日時は必須)。
    #[serde(default, deserialize_with = "super::double_option")]
    brewed_at: Option<Option<String>>,
    /// 新しい豆の量。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    dose_grams: Option<Option<serde_json::Number>>,
    /// 新しい湯量。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    water_grams: Option<Option<serde_json::Number>>,
    /// 新しい湯の温度。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    water_temp_c: Option<Option<serde_json::Number>>,
    /// 新しい時間。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    brew_time_seconds: Option<Option<i64>>,
    /// 新しい抽出方法。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    method: Option<Option<String>>,
    /// 新しい挽き目。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    grind_setting: Option<Option<String>>,
    /// 新しい評価。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    rating: Option<Option<i64>>,
    /// 新しい感想。`null` のときは NULL にする。
    #[serde(default, deserialize_with = "super::double_option")]
    notes: Option<Option<String>>,
}

/// 抽出の一覧を返す。認証が必要。`sort`、`order`、`favorite` を受け付ける (FR-20、FR-21)。
pub async fn list(req: &Request, env: &Env, session: &Session) -> Result<Response> {
    let params = match ListParams::from_request(req, SortKey::BrewedAt, query::BREW_SORT_KEYS) {
        Ok(params) => params,
        Err(response) => return Ok(response),
    };
    let d1 = db::database(env)?;
    let statement = match query::brews_list(
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
    let rows: Vec<BrewJoinRow> = db::prepared(&d1, &statement)?.all().await?.results()?;
    let mut brews: Vec<BrewResponse> = rows.into_iter().map(BrewJoinRow::into_response).collect();
    attach_notes(&d1, &session.user_id, &mut brews).await?;
    let next_cursor = brews
        .last()
        .and_then(|brew| params.next_cursor(sort_value(brew, params.sort), &brew.id, brews.len()));
    respond::json(&BrewListResponse { brews, next_cursor })
}

/// 並び順のキーの値を、応答の行から取り出す (FR-20)。
fn sort_value(brew: &BrewResponse, sort: SortKey) -> Option<CursorValue> {
    match sort {
        SortKey::BrewedAt => Some(CursorValue::Text(brew.brewed_at.clone())),
        SortKey::Rating => brew.rating.map(CursorValue::Integer),
        SortKey::DoseGrams => brew.dose_grams.map(CursorValue::Real),
        _ => None,
    }
}

/// 抽出を登録する。認証が必要。
pub async fn create(req: &mut Request, env: &Env, session: &Session) -> Result<Response> {
    let Some(input) = read_input::<CreateInput>(req).await else {
        return Ok(invalid_input(
            "the body must be JSON with a purchase and a brewed timestamp",
        ));
    };
    let d1 = db::database(env)?;
    // 購入は、存在しないか他の利用者のものは 404 にする (FR-11)。
    if let Err(response) = require_purchase(&d1, &session.user_id, &input.purchase_id)
        .await?
        .or_return()
    {
        return Ok(response);
    }
    if let Err(error) = validate_timestamp(&input.brewed_at) {
        return Ok(invalid_input(error.message()));
    }
    let dose_grams = match merge_decimal(input.dose_grams) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let water_grams = match merge_decimal(input.water_grams) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let water_temp_c = match merge_decimal(input.water_temp_c) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let brew_time_seconds = match merge_count(input.brew_time_seconds) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let rating = match merge_rating(input.rating) {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };
    let method = trim_optional(input.method.as_deref());
    let grind_setting = trim_optional(input.grind_setting.as_deref());
    let notes = trim_optional(input.notes.as_deref());
    let id = db::new_id()?;
    let now = db::now_text()?;
    let values = BrewValues {
        purchase_id: &input.purchase_id,
        brewed_at: &input.brewed_at,
        dose_grams,
        water_grams,
        water_temp_c,
        brew_time_seconds,
        method: method.as_deref(),
        grind_setting: grind_setting.as_deref(),
        rating,
        notes: notes.as_deref(),
    };
    let statement = match query::brew_insert(&id, &session.user_id, &values, &now, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    respond_fetched(&d1, &session.user_id, &id).await
}

/// 抽出を 1 件返す。認証が必要。
pub async fn get(env: &Env, session: &Session, id: Option<&str>) -> Result<Response> {
    let d1 = db::database(env)?;
    let Some(id) = id else {
        return Ok(not_found("the brew does not exist"));
    };
    match find(&d1, &session.user_id, id).await? {
        Some(brew) => respond::json(&brew),
        None => Ok(not_found("the brew does not exist")),
    }
}

/// 抽出を更新する。認証が必要。
pub async fn update(
    req: &mut Request,
    env: &Env,
    session: &Session,
    id: Option<&str>,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the brew does not exist"));
    };
    let Some(input) = read_input::<UpdateInput>(req).await else {
        return Ok(invalid_input(
            "the body must be JSON with the fields to update",
        ));
    };
    let d1 = db::database(env)?;
    let Some(current) = find(&d1, &session.user_id, id).await? else {
        return Ok(not_found("the brew does not exist"));
    };
    let purchase_id = match input.purchase_id {
        Some(Some(purchase_id)) => {
            // 参照先を変更する更新では、新しい購入を登録と同じ条件で検査する (FR-11)。
            if purchase_id != current.purchase_id {
                if let Err(response) = require_purchase(&d1, &session.user_id, &purchase_id)
                    .await?
                    .or_return()
                {
                    return Ok(response);
                }
            }
            purchase_id
        }
        // 購入は必須のため `null` を拒否する。
        Some(None) => return Ok(invalid_input("the purchase must not be null")),
        None => current.purchase_id.clone(),
    };
    let brewed_at = match input.brewed_at {
        Some(Some(brewed_at)) => {
            if let Err(error) = validate_timestamp(&brewed_at) {
                return Ok(invalid_input(error.message()));
            }
            brewed_at
        }
        // 抽出日時は必須のため `null` を拒否する。
        Some(None) => return Ok(invalid_input("the brewed timestamp must not be null")),
        None => current.brewed_at.clone(),
    };
    let dose_grams = match input.dose_grams {
        Some(dose_grams) => match merge_decimal(dose_grams) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        },
        None => current.dose_grams,
    };
    let water_grams = match input.water_grams {
        Some(water_grams) => match merge_decimal(water_grams) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        },
        None => current.water_grams,
    };
    let water_temp_c = match input.water_temp_c {
        Some(water_temp_c) => match merge_decimal(water_temp_c) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        },
        None => current.water_temp_c,
    };
    let brew_time_seconds = match input.brew_time_seconds {
        Some(brew_time_seconds) => match merge_count(brew_time_seconds) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        },
        None => current.brew_time_seconds,
    };
    let rating = match input.rating {
        Some(rating) => match merge_rating(rating) {
            Ok(value) => value,
            Err(response) => return Ok(response),
        },
        None => current.rating,
    };
    let method = merge_text(input.method, current.method.clone());
    let grind_setting = merge_text(input.grind_setting, current.grind_setting.clone());
    let notes = merge_text(input.notes, current.notes.clone());
    let now = db::now_text()?;
    let values = BrewValues {
        purchase_id: &purchase_id,
        brewed_at: &brewed_at,
        dose_grams,
        water_grams,
        water_temp_c,
        brew_time_seconds,
        method: method.as_deref(),
        grind_setting: grind_setting.as_deref(),
        rating,
        notes: notes.as_deref(),
    };
    let statement = match query::brew_update(id, &session.user_id, &values, &now) {
        Ok(statement) => statement,
        Err(error) => return Ok(query_error_response(error)),
    };
    db::prepared(&d1, &statement)?.run().await?;
    respond_fetched(&d1, &session.user_id, id).await
}

/// 抽出を 1 件引く。購入、商品、店を結合する (FR-11)。
async fn find(d1: &D1Database, user_id: &str, id: &str) -> Result<Option<BrewResponse>> {
    let statement = query::brew_find(user_id, id);
    let row: Option<BrewJoinRow> = db::prepared(d1, &statement)?.first(None).await?;
    match row {
        Some(row) => {
            let mut brew = row.into_response();
            attach_notes(d1, user_id, std::slice::from_mut(&mut brew)).await?;
            Ok(Some(brew))
        }
        None => Ok(None),
    }
}

/// 登録または更新の後に、現在の内容を引いて返す。
async fn respond_fetched(d1: &D1Database, user_id: &str, id: &str) -> Result<Response> {
    match find(d1, user_id, id).await? {
        Some(brew) => respond::json(&brew),
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
        return Ok(not_found("the brew does not exist"));
    };
    let d1 = db::database(env)?;
    let Some(brew) = find(&d1, &session.user_id, id).await? else {
        return Ok(not_found("the brew does not exist"));
    };
    let changed = apply_favorite(
        &d1,
        query::BREWS_TABLE,
        id,
        &session.user_id,
        brew.favorited_at.as_deref(),
        favorite,
    )
    .await?;
    if changed.is_none() {
        return respond::json(&brew);
    }
    respond_fetched(&d1, &session.user_id, id).await
}

/// 応答の商品に Flavor Notes を付ける (FR-8)。
async fn attach_notes(d1: &D1Database, user_id: &str, brews: &mut [BrewResponse]) -> Result<()> {
    let mut products: Vec<&mut ProductResponse> = brews
        .iter_mut()
        .map(|brew| &mut brew.purchase.product)
        .collect();
    attach_flavor_notes(d1, user_id, &mut products).await
}

/// 小数の入力の値を検証する (0 以上で小数第 1 位まで)。
/// 検査は `serde_json` の数値の文字列表現に対して行う (0007 の設計判断)。
fn merge_decimal(input: Option<serde_json::Number>) -> std::result::Result<Option<f64>, Response> {
    match input {
        Some(number) => match validate_decimal(&number.to_string()) {
            Ok(value) => Ok(Some(value)),
            Err(error) => Err(invalid_input(error.message())),
        },
        None => Ok(None),
    }
}

/// 時間の入力の値を検証する (0 以上で D1 の整数に収まる値)。
fn merge_count(input: Option<i64>) -> std::result::Result<Option<i64>, Response> {
    match input {
        Some(value) => match validate_count(value) {
            Ok(value) => Ok(Some(value)),
            Err(error) => Err(invalid_input(error.message())),
        },
        None => Ok(None),
    }
}

/// 評価の入力の値を検証する (1 から 5)。
fn merge_rating(input: Option<i64>) -> std::result::Result<Option<i64>, Response> {
    match input {
        Some(value) => match validate_rating(value) {
            Ok(value) => Ok(Some(value)),
            Err(error) => Err(invalid_input(error.message())),
        },
        None => Ok(None),
    }
}

/// 更新の入力の任意の値を、現在の値と合わせる。
/// 入力が無ければ現在の値、`null` なら NULL、値があれば前後の空白を除いた値にする。
fn merge_text(input: Option<Option<String>>, current: Option<String>) -> Option<String> {
    match input {
        Some(value) => trim_optional(value.as_deref()),
        None => current,
    }
}
