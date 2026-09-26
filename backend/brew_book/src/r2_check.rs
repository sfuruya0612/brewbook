//! 結合テストが R2 のバケットを下ごしらえし、残件数を数えるための経路。
//!
//! PRD の API ではないため経路の台帳 (0001) に載せない。`R2_CHECK` の var が `true` のときだけ
//! 応答し、それ以外は台帳に無い経路と同じ 404 になる。結合テストは
//! `wrangler dev --var R2_CHECK:true` で有効にする (本番の vars には無い)。
//!
//! アカウント削除 (0012) の想定規模のテストは 1 利用者あたり数千件のオブジェクトを必要とする。
//! `wrangler r2 object put` は 1 件につき 1 プロセスを要するため、バインディングでまとめて置き、
//! カーソルを繰り返して残件数を数える操作をここに置く。

use brew_book_core::error::{envelope, ErrorCode};
use brew_book_core::photo;
use serde::{Deserialize, Serialize};
use worker::{Env, Error, HttpMetadata, Method, Request, Response, Result};

use crate::records::photos::PHOTOS_BINDING;

/// この経路の経路名。ログの `route` に使う。
pub const ROUTE_NAME: &str = "r2_check";
/// この経路のパス。
pub const PATH: &str = "/api/__r2_check";
/// この経路を有効にする vars の名前。
pub const VAR_NAME: &str = "R2_CHECK";
/// この経路が有効な値。
const VAR_ENABLED: &str = "true";
/// 1 回の呼び出しで置ける件数の上限。テストは分けて呼ぶ。
pub const MAX_COUNT: i64 = 1_000;
/// 一覧を 1 回で引く件数の上限。
const LIST_PAGE_LIMIT: u32 = 1_000;
/// 置くオブジェクトの実体。中身は検査しない。
const BODY: &[u8] = b"\xff\xd8\xff\xe0r2-check";

/// 有効なら R2 の操作を実行する。無効な経路は None を返し、呼び出し側が 404 にする。
pub async fn run(req: &mut Request, env: &Env) -> Option<Result<Response>> {
    if req.path() != PATH || req.method() != Method::Post || !is_enabled(env) {
        return None;
    }
    Some(handle(req, env).await)
}

/// `R2_CHECK` の var が `true` のときだけ有効にする。
fn is_enabled(env: &Env) -> bool {
    matches!(env.var(VAR_NAME), Ok(var) if var.to_string() == VAR_ENABLED)
}

/// この経路が受け取る入力。
#[derive(Debug, Deserialize)]
struct CheckInput {
    /// `put` はオブジェクトを置き、`count` はプレフィックスごとの残件数を数える。
    action: String,
    /// 対象の利用者 ID。
    user_id: String,
    /// `users` は紐づけ済み、`pending` は紐づけ前のキーを対象にする。
    kind: String,
    /// `put` のときの添字の開始値。
    #[serde(default)]
    start: i64,
    /// `put` のときの件数。
    #[serde(default)]
    count: i64,
}

/// この経路が返す応答。
#[derive(Debug, Serialize)]
struct CheckOutput {
    /// `put` で置いた件数、`count` で数えた件数。
    count: usize,
    /// 一覧を引いた回数 (`count` のときだけ)。カーソルの繰り返しが起きたことの確認に使う。
    pages: usize,
}

/// 入力の操作を実行する。未知の操作と範囲外の件数は 400 にする。
async fn handle(req: &mut Request, env: &Env) -> Result<Response> {
    let input: CheckInput = match req.json().await {
        Ok(input) => input,
        Err(_) => return bad_request("the body must be JSON with action, user_id and kind"),
    };
    let bucket = env.bucket(PHOTOS_BINDING)?;
    match input.action.as_str() {
        "put" => {
            if input.start < 0 {
                return bad_request("the start must be 0 or greater");
            }
            if !(0..=MAX_COUNT).contains(&input.count) {
                return bad_request("the count must be between 0 and 1000");
            }
            for index in input.start..input.start + input.count {
                let key = match input.kind.as_str() {
                    "users" => object_key(&input.user_id, index),
                    "pending" => pending_object_key(&input.user_id, index),
                    _ => return bad_request("the kind must be users or pending"),
                };
                bucket
                    .put(&key, BODY.to_vec())
                    .http_metadata(HttpMetadata {
                        content_type: Some(photo::CONTENT_TYPE.to_owned()),
                        ..Default::default()
                    })
                    .execute()
                    .await?;
            }
            respond(&CheckOutput {
                count: input.count as usize,
                pages: 0,
            })
        }
        "count" => {
            let prefix = match input.kind.as_str() {
                "users" => photo::user_prefix(&input.user_id),
                "pending" => photo::pending_prefix(&input.user_id),
                _ => return bad_request("the kind must be users or pending"),
            };
            let mut count = 0;
            let mut pages = 0;
            let mut cursor: Option<String> = None;
            loop {
                let mut list = bucket.list().prefix(&prefix).limit(LIST_PAGE_LIMIT);
                if let Some(value) = &cursor {
                    list = list.cursor(value);
                }
                let page = list.execute().await?;
                count += page.objects().len();
                pages += 1;
                if !page.truncated() {
                    break;
                }
                cursor = Some(page.cursor().ok_or_else(|| {
                    Error::RustError("the object list is truncated without a cursor".to_owned())
                })?);
            }
            respond(&CheckOutput { count, pages })
        }
        _ => bad_request("the action must be put or count"),
    }
}

/// 置く鍵の添字から購入 ID を作る。写真の鍵の形を実物に合わせる (ADR-0003)。
/// テストは同じ ID の購入の行を下ごしらえし、`photo_key` に [`object_key`] の値を入れる。
pub fn photo_purchase_id(index: i64) -> String {
    format!("r2-check-purchase-{index:05}")
}

/// 置く鍵の添字から UUID を作る。写真の鍵の形を実物に合わせる (ADR-0003)。
pub fn object_uuid(index: i64) -> String {
    format!("00000000-0000-4000-8000-{index:012}")
}

/// 添字が指す紐づけ済みのオブジェクトキー (ADR-0003 のキーの形)。
pub fn object_key(user_id: &str, index: i64) -> String {
    photo::photo_key(user_id, &photo_purchase_id(index), &object_uuid(index))
}

/// 添字が指す紐づけ前のオブジェクトキー (ADR-0003 のキーの形)。
pub fn pending_object_key(user_id: &str, index: i64) -> String {
    photo::pending_key(user_id, &object_uuid(index))
}

fn respond<T: Serialize>(value: &T) -> Result<Response> {
    Response::from_json(value)
}

/// 400 の応答を PRD の形式で組み立てる。
fn bad_request(message: &str) -> Result<Response> {
    Response::from_json(&envelope(ErrorCode::BadRequest, message))
        .map(|response| response.with_status(ErrorCode::BadRequest.status()))
}
