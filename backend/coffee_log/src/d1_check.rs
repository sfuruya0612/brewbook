//! 結合テストが D1 のバインディングを検証するための経路。
//!
//! PRD の API ではないため経路の台帳 (0001) に載せない。`D1_CHECK` の var が `true` のときだけ
//! 応答し、それ以外は台帳に無い経路と同じ 404 になる。結合テストは
//! `wrangler dev --var D1_CHECK:true` で有効にする (本番の vars には無い)。
//!
//! 利用者と店をプレースホルダ付きの INSERT で入れ、一覧の SQL を組み立てる共通部分
//! (`coffee_log_core::query`) の SELECT で引き直した結果を返す。

use coffee_log_core::datetime::format_epoch_millis;
use coffee_log_core::error::{envelope, ErrorCode};
use coffee_log_core::ids::uuid_v4_from_bytes;
use coffee_log_core::query::{self, Archived, ListQuery, OrderKind, Value};
use serde::{Deserialize, Serialize};
use worker::d1::D1Type;
use worker::{Date, Env, Error, Method, Request, Response, Result};

use crate::random;

/// この経路の経路名。ログの `route` に使う。
pub const ROUTE_NAME: &str = "d1_check";
/// この経路のパス。
pub const PATH: &str = "/api/__d1_check";
/// この経路を有効にする vars の名前。
pub const VAR_NAME: &str = "D1_CHECK";
/// この経路が有効な値。
const VAR_ENABLED: &str = "true";
/// この経路が入れる利用者の表示名。管理者が使う表示名とは別の、検証用の固定値。
const DISPLAY_NAME: &str = "d1 check";

const INSERT_USER: &str = "INSERT INTO users (id, display_name, created_at) VALUES (?, ?, ?)";
const INSERT_SHOP: &str = "INSERT INTO shops (id, user_id, name, address, created_at, updated_at, \
                            archived_at) VALUES (?, ?, ?, ?, ?, ?, NULL)";
const SHOP_COLUMNS: &str = "id, user_id, name, address, created_at, updated_at, archived_at";

/// 有効なら D1 の往復を実行する。無効な経路は None を返し、呼び出し側が 404 にする。
///
/// D1 の失敗は `Err` で返し、呼び出し側が 500 にする。
pub async fn run(req: &mut Request, env: &Env) -> Option<Result<Response>> {
    if req.path() != PATH || req.method() != Method::Post || !is_enabled(env) {
        return None;
    }
    Some(handle(req, env).await)
}

/// `D1_CHECK` の var が `true` のときだけ有効にする。
fn is_enabled(env: &Env) -> bool {
    matches!(env.var(VAR_NAME), Ok(var) if var.to_string() == VAR_ENABLED)
}

/// D1 の往復を実行して結果を JSON で返す。
async fn handle(req: &mut Request, env: &Env) -> Result<Response> {
    let input: CheckInput = match req.json().await {
        Ok(input) => input,
        Err(_) => return bad_request("the body must be JSON with a name and an optional address"),
    };

    let user_id = uuid_v4_from_bytes(random::bytes_16().map_err(Error::RustError)?);
    let shop_id = uuid_v4_from_bytes(random::bytes_16().map_err(Error::RustError)?);
    let created_at = format_epoch_millis(now_millis())
        .map_err(|error| Error::RustError(format!("failed to format the timestamp: {error:?}")))?;

    let d1 = env.d1("DB")?;
    let user_values = [
        D1Type::Text(&user_id),
        D1Type::Text(DISPLAY_NAME),
        D1Type::Text(&created_at),
    ];
    d1.prepare(INSERT_USER)
        .bind_refs(user_values.iter())?
        .run()
        .await?;

    let address = input.address.as_deref();
    let shop_values = [
        D1Type::Text(&shop_id),
        D1Type::Text(&user_id),
        D1Type::Text(&input.name),
        address.map_or(D1Type::Null, D1Type::Text),
        D1Type::Text(&created_at),
        D1Type::Text(&created_at),
    ];
    d1.prepare(INSERT_SHOP)
        .bind_refs(shop_values.iter())?
        .run()
        .await?;

    // 一覧の SQL は共通部分を通して組み立て、値はプレースホルダで渡す。
    let statement = query::list(&ListQuery {
        table: "shops",
        columns: SHOP_COLUMNS,
        user_id: &user_id,
        order_column: "created_at",
        order_kind: OrderKind::DateTime,
        archived: Archived::Exclude,
        cursor: None,
        limit: 50,
    })
    .map_err(|error| {
        Error::RustError(format!(
            "failed to build the list query: {}",
            error.message()
        ))
    })?;
    let params = statement
        .params
        .iter()
        .map(d1_value)
        .collect::<Result<Vec<D1Type>>>()?;
    let result = d1
        .prepare(statement.sql)
        .bind_refs(params.iter())?
        .all()
        .await?;

    let mut shops: Vec<ShopRow> = result.results()?;
    let shop = shops
        .drain(..)
        .next()
        .ok_or_else(|| Error::RustError("the inserted shop was not found".to_owned()))?;
    Response::from_json(&CheckOutput { user_id, shop })
}

/// 組み立てた値 (コアの型) を D1 に渡す値にする。整数は D1 の範囲に収まることを確認する。
fn d1_value(value: &Value) -> Result<D1Type<'_>> {
    Ok(match value {
        Value::Text(text) => D1Type::Text(text),
        Value::Integer(number) => {
            let number = i32::try_from(*number).map_err(|_| {
                Error::RustError(format!("the integer {number} does not fit in D1"))
            })?;
            D1Type::Integer(number)
        }
    })
}

fn now_millis() -> i64 {
    Date::now().as_millis() as i64
}

fn bad_request(message: &str) -> Result<Response> {
    Response::from_json(&envelope(ErrorCode::BadRequest, message))
        .map(|response| response.with_status(ErrorCode::BadRequest.status()))
}

/// この経路が受け取る入力。
#[derive(Debug, Deserialize)]
struct CheckInput {
    /// 店の名前。
    name: String,
    /// 店の住所 (NULL 許容)。
    address: Option<String>,
}

/// この経路が返す応答。
#[derive(Debug, Serialize)]
struct CheckOutput {
    /// 発行した利用者 ID。
    user_id: String,
    /// D1 から引き直した店の行。
    shop: ShopRow,
}

/// D1 から引き直した店の行。
#[derive(Debug, Serialize, Deserialize)]
struct ShopRow {
    id: String,
    user_id: String,
    name: String,
    address: Option<String>,
    created_at: String,
    updated_at: String,
    archived_at: Option<String>,
}
