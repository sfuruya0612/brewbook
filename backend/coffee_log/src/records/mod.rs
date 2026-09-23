//! 店と商品と Flavor Notes のタグの API (FR-6、FR-7、FR-8、FR-12)。
//!
//! 入力の検証は `coffee_log_core::records`、SQL の組み立ては `coffee_log_core::query` が持つ。
//! ここは経路の処理 (入力の読み取り、D1 の実行、応答の組み立て) だけを行う。
//! 存在しない ID と他の利用者の ID は区別せず 404 を返す (ADR-0006)。

pub mod products;
pub mod shops;
pub mod tags;

use std::collections::HashMap;

use coffee_log_core::cursor::{parse_page_size, CursorKey};
use coffee_log_core::error::ErrorCode;
use coffee_log_core::query::{self, parse_include_archived, Archived, OrderKind, QueryError};
use coffee_log_core::records::validate_name;
use worker::d1::D1Database;
use worker::{console_error, Request, Response, Result};

use crate::db;
use crate::respond;

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
