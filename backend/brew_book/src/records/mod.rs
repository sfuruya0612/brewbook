//! 店と商品と Flavor Notes のタグと、購入と抽出と写真の API (FR-6 から FR-12) と、
//! 過去の入力値のサジェストの API (FR-13)。
//!
//! 入力の検証は `brew_book_core::records`、SQL の組み立ては `brew_book_core::query` が持つ。
//! ここは経路の処理 (入力の読み取り、D1 の実行、応答の組み立て) だけを行う。
//! 存在しない ID と他の利用者の ID は区別せず 404 を返す (ADR-0006)。

pub mod brews;
pub mod photos;
pub mod products;
pub mod purchases;
pub mod shops;
pub mod stats;
pub mod suggestions;
pub mod tags;

use std::collections::HashMap;

use brew_book_core::cursor::{parse_page_size, CursorKey};
use brew_book_core::error::ErrorCode;
use brew_book_core::query::{self, parse_include_archived, Archived, OrderKind, QueryError};
use brew_book_core::records::validate_name;
use worker::d1::D1Database;
use worker::{console_error, Request, Response, Result};

use crate::db;
use crate::respond;

use self::products::ProductResponse;
use self::shops::ShopResponse;

/// 一覧のクエリパラメータ (limit、cursor、include_archived)。
pub struct ListParams {
    /// 取得件数。既定は 50、最大は 200 (PRD の性能)。
    pub limit: u32,
    /// 直前のページの最後の行を指すカーソル。
    pub cursor: Option<CursorKey>,
    /// アーカイブ済みの行の扱い。
    pub archived: Archived,
}

impl ListParams {
    /// リクエストのクエリ文字列から読む。誤りは 400 の応答にする。
    pub fn from_request(req: &Request) -> Result<Self, Response> {
        let url = match req.url() {
            Ok(url) => url,
            Err(error) => {
                console_error!("the request URL is not readable: {error}");
                return Err(internal_error());
            }
        };
        let mut limit = None;
        let mut cursor = None;
        let mut include_archived = None;
        for (name, value) in url.query_pairs() {
            match name.as_ref() {
                "limit" => limit = Some(value.into_owned()),
                "cursor" => cursor = Some(value.into_owned()),
                "include_archived" => include_archived = Some(value.into_owned()),
                _ => {}
            }
        }
        let limit =
            parse_page_size(limit.as_deref()).map_err(|error| invalid_input(error.message()))?;
        let cursor = match cursor {
            Some(text) => {
                Some(CursorKey::decode(&text).map_err(|error| invalid_input(error.message()))?)
            }
            None => None,
        };
        let archived = parse_include_archived(include_archived.as_deref())
            .map_err(|error| invalid_input(error.message()))?;
        Ok(Self {
            limit,
            cursor,
            archived,
        })
    }

    /// ページの続きのカーソル。最後の行の並び順のキーと ID を指す。
    /// 行数が `limit` に満たないときは None を返す (続きが無い可能性が高い)。
    /// カーソルの種類は並び順に合わせる (日付の並び順に日時のカーソルを返さない)。
    pub fn next_cursor(
        &self,
        order: OrderKind,
        last: Option<(&str, &str)>,
        count: usize,
    ) -> Option<String> {
        if count < self.limit as usize {
            return None;
        }
        last.map(|(key, id)| {
            let cursor = match order {
                OrderKind::DateTime => CursorKey::DateTime {
                    at: key.to_owned(),
                    id: id.to_owned(),
                },
                OrderKind::Date => CursorKey::Date {
                    on: key.to_owned(),
                    id: id.to_owned(),
                },
            };
            cursor.encode()
        })
    }
}

/// 400 の応答を組み立てる。
pub fn invalid_input(message: &str) -> Response {
    respond::error(ErrorCode::BadRequest, message)
}

/// 404 の応答を組み立てる。存在しない ID と他の利用者の ID は区別しない (ADR-0006)。
pub fn not_found(message: &str) -> Response {
    respond::error(ErrorCode::NotFound, message)
}

/// 409 の応答を組み立てる。アーカイブ済みの親を参照先に指定したときなどに使う (FR-9、FR-11)。
pub fn conflict(message: &str) -> Response {
    respond::error(ErrorCode::Conflict, message)
}

/// 参照先の検証の結果 (FR-9、FR-11)。拒否したときはそのまま返す応答を持つ。
pub enum Reference<T> {
    /// 参照できる。
    Found(T),
    /// 存在しないか他の利用者のもの (404)、またはアーカイブ済み (409)。
    Rejected(Response),
}

impl<T> Reference<T> {
    /// 拒否されたときは応答を返し、参照できるときは値を返す。
    pub fn or_return(self) -> std::result::Result<T, Response> {
        match self {
            Reference::Found(value) => Ok(value),
            Reference::Rejected(response) => Err(response),
        }
    }
}

/// 参照先の商品を検証する。存在しないか他の利用者のものは 404、アーカイブ済みは 409 (FR-9)。
pub async fn require_product(
    d1: &D1Database,
    user_id: &str,
    id: &str,
) -> Result<Reference<ProductResponse>> {
    let statement = query::product_find(user_id, id, Archived::Include);
    match db::prepared(d1, &statement)?
        .first::<ProductResponse>(None)
        .await?
    {
        None => Ok(Reference::Rejected(not_found("the product does not exist"))),
        Some(product) if product.archived_at.is_some() => {
            Ok(Reference::Rejected(conflict("the product is archived")))
        }
        Some(product) => Ok(Reference::Found(product)),
    }
}

/// 参照先の店を検証する。存在しないか他の利用者のものは 404、アーカイブ済みは 409 (FR-9)。
pub async fn require_shop(
    d1: &D1Database,
    user_id: &str,
    id: &str,
) -> Result<Reference<ShopResponse>> {
    let statement = query::shop_find(user_id, id, Archived::Include);
    match db::prepared(d1, &statement)?
        .first::<ShopResponse>(None)
        .await?
    {
        None => Ok(Reference::Rejected(not_found("the shop does not exist"))),
        Some(shop) if shop.archived_at.is_some() => {
            Ok(Reference::Rejected(conflict("the shop is archived")))
        }
        Some(shop) => Ok(Reference::Found(shop)),
    }
}

/// 参照先の購入を検証する。存在しないか他の利用者のものは 404、アーカイブ済みは 409 (FR-11)。
pub async fn require_purchase(d1: &D1Database, user_id: &str, id: &str) -> Result<Reference<()>> {
    let statement = query::purchase_find(user_id, id, Archived::Include);
    let row: Option<PurchaseReferenceRow> = db::prepared(d1, &statement)?.first(None).await?;
    match row {
        None => Ok(Reference::Rejected(not_found(
            "the purchase does not exist",
        ))),
        Some(row) if row.p_archived_at.is_some() => {
            Ok(Reference::Rejected(conflict("the purchase is archived")))
        }
        Some(_) => Ok(Reference::Found(())),
    }
}

/// 参照先の購入が存在するかを確かめる。存在しないか他の利用者のものは false にする。
///
/// アーカイブ済みでも存在として扱う (評価の推移は単件取得と同じくアーカイブ済みの購入を
/// 指定できる。FR-12)。
pub async fn purchase_exists(d1: &D1Database, user_id: &str, id: &str) -> Result<bool> {
    let statement = query::purchase_find(user_id, id, Archived::Include);
    let row: Option<PurchaseReferenceRow> = db::prepared(d1, &statement)?.first(None).await?;
    Ok(row.is_some())
}

/// 参照先の購入の検証に使う、結合した行のうち購入の状態だけの列。
#[derive(Debug, serde::Deserialize)]
struct PurchaseReferenceRow {
    p_archived_at: Option<String>,
}

/// 応答の商品に Flavor Notes を付ける (FR-8)。商品のタグは 1 つのクエリでまとめて引く。
pub async fn attach_flavor_notes(
    d1: &D1Database,
    user_id: &str,
    products: &mut [&mut ProductResponse],
) -> Result<()> {
    let ids: Vec<&str> = products.iter().map(|product| product.id.as_str()).collect();
    let notes = flavor_notes_for(d1, user_id, &ids).await?;
    for product in products {
        product.flavor_notes = notes.get(&product.id).cloned().unwrap_or_default();
    }
    Ok(())
}

/// クエリの組み立ての誤りを応答にする。
/// カーソルの種類の不一致は 400、コードの誤り (列と値の数の不一致など) は 500 にする。
pub fn query_error_response(error: QueryError) -> Response {
    respond::error(error.code(), error.message())
}

/// `null` を「NULL にする」、項目が無いことを「変更しない」として区別して読む
/// (`PATCH` の入力の `Option<Option<String>>` に使う)。
pub fn double_option<'de, D, T>(deserializer: D) -> std::result::Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    Ok(Some(<Option<T> as serde::Deserialize>::deserialize(
        deserializer,
    )?))
}

/// リクエストの本体を JSON として読み、入力の型にする。読めない場合は None を返す。
///
/// `worker::Request::json` は `serde_wasm_bindgen` で読むため `deny_unknown_fields` が効かない。
/// ここで `serde_json` に通し、受け取らない項目を無視せず 400 にする (0007 の購入と抽出と同じ扱い)。
pub async fn read_input<T: serde::de::DeserializeOwned>(req: &mut Request) -> Option<T> {
    let text = match req.text().await {
        Ok(text) => text,
        Err(error) => {
            // 本体の読み取りの失敗は内部の障害である。応答は 400 に揃えるが、原因をログに残す。
            console_error!("the request body is not readable: {error}");
            return None;
        }
    };
    serde_json::from_str(&text).ok()
}

/// 更新の入力の名前を、現在の名前と合わせる。
/// 入力が無ければ現在の名前を保ち、`null` と空白だけの名前は拒否する (店名と商品名は必須)。
pub fn merge_name(input: Option<Option<String>>, current: String) -> Result<String, Response> {
    match input {
        Some(Some(name)) => validate_name(&name).map_err(|error| invalid_input(error.message())),
        Some(None) => Err(invalid_input("the name must not be null")),
        None => Ok(current),
    }
}

/// 商品の Flavor Notes のタグを配列で置き換える (FR-8)。
///
/// タグの行は利用者ごとに名前で共有し、参照されなくなっても削除しない。消すのは商品とタグの
/// 対応だけとし、対応の削除と挿入は 1 つのまとまり (D1 の batch) で実行する。
pub async fn replace_flavor_notes(
    d1: &D1Database,
    user_id: &str,
    product_id: &str,
    names: &[String],
) -> Result<()> {
    let mut statements = vec![db::prepared(
        d1,
        &query::delete_product_flavor_tags(user_id, product_id),
    )?];
    for name in names {
        statements.push(db::prepared(
            d1,
            &query::insert_flavor_tag(&db::new_id()?, user_id, name),
        )?);
        statements.push(db::prepared(
            d1,
            &query::insert_product_flavor_tag(user_id, product_id, name),
        )?);
    }
    db::execute_batch(d1, statements).await
}

/// 商品 1 件のタグ名を引く (並びは名前の昇順)。
pub async fn flavor_notes(d1: &D1Database, user_id: &str, product_id: &str) -> Result<Vec<String>> {
    let mut notes = flavor_notes_for(d1, user_id, &[product_id]).await?;
    Ok(notes.remove(product_id).unwrap_or_default())
}

/// 複数の商品のタグ名をまとめて引く。商品 ID をキーにする。
///
/// 束縛する値が D1 の上限に収まるよう分けたクエリを、1 つのまとまり (batch) で実行する。
pub async fn flavor_notes_for(
    d1: &D1Database,
    user_id: &str,
    product_ids: &[&str],
) -> Result<HashMap<String, Vec<String>>> {
    let mut statements = Vec::new();
    for statement in query::product_flavor_notes(user_id, product_ids) {
        statements.push(db::prepared(d1, &statement)?);
    }
    if statements.is_empty() {
        return Ok(HashMap::new());
    }
    let mut notes: HashMap<String, Vec<String>> = HashMap::new();
    for result in d1.batch(statements).await? {
        if !result.success() {
            let error = result.error().unwrap_or_default();
            console_error!("a batched statement failed: {error}");
            return Err(worker::Error::RustError(
                "a batched statement failed".to_owned(),
            ));
        }
        let rows: Vec<FlavorNoteRow> = result.results()?;
        for row in rows {
            notes.entry(row.product_id).or_default().push(row.name);
        }
    }
    Ok(notes)
}

/// 内部エラーの応答を組み立てる。
pub fn internal_error() -> Response {
    respond::error(ErrorCode::Internal, "internal error")
}

/// 商品のタグの行のうち、応答の組み立てに使う列。
#[derive(Debug, serde::Deserialize)]
struct FlavorNoteRow {
    product_id: String,
    name: String,
}
