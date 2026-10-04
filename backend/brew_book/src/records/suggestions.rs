//! 過去の入力値のサジェストの API (FR-13)。
//!
//! 経路は `GET /api/suggestions/<項目>` とし、`q` (入力中の文字列) を省略可能なクエリパラメータで
//! 受け取る。項目は 8 つだけを受け付け、それ以外は 400 を返す。
//! `q` は前後の空白を除いた値の前方一致 (大文字と小文字を区別しない) で絞る。
//! 全ての記録の値も候補に含め、他の利用者の値は含めない (FR-13)。
//! API は候補を返すだけで、候補に無い値の入力を妨げない。

use brew_book_core::query::{self, parse_suggestion_field, SuggestionFieldError};
use brew_book_core::records::trim_text;
use serde::{Deserialize, Serialize};
use worker::{console_error, Env, Request, Response, Result};

use super::{internal_error, invalid_input};
use crate::auth::session::Session;
use crate::db;
use crate::respond;

/// サジェストの応答。候補の値の並び (`updated_at` の降順と、同じときの値の昇順)。
#[derive(Debug, Serialize)]
pub struct SuggestionResponse {
    /// 候補の値。最大 20 件。
    pub values: Vec<String>,
}

/// サジェストのクエリパラメータ。
struct SuggestionParams {
    /// 入力中の文字列。無いときは全値を候補にする。
    q: Option<String>,
}

impl SuggestionParams {
    /// リクエストのクエリ文字列から読む。1 つの文字列でない `q` は 400 の応答にする。
    fn from_request(req: &Request) -> Result<Self, Response> {
        let url = match req.url() {
            Ok(url) => url,
            Err(error) => {
                console_error!("the request URL is not readable: {error}");
                return Err(internal_error());
            }
        };
        let mut q = None;
        for (name, value) in url.query_pairs() {
            if name == "q" {
                // 複数回の指定は配列として扱い、受け取らない。
                if q.is_some() {
                    return Err(invalid_input("q must be a single string"));
                }
                q = Some(value.into_owned());
            }
        }
        Ok(Self { q })
    }
}

/// サジェストの候補を返す。認証が必要。
pub async fn list(
    req: &Request,
    env: &Env,
    session: &Session,
    field: Option<&str>,
) -> Result<Response> {
    // 項目名が無い (経路のパラメータが空の) 場合と 8 つの名前以外は、同じ 400 にする。
    let item = match field {
        Some(field) => match parse_suggestion_field(field) {
            Ok(item) => item,
            Err(error) => return Ok(invalid_input(error.message())),
        },
        None => return Ok(invalid_input(SuggestionFieldError::Unknown.message())),
    };
    let params = match SuggestionParams::from_request(req) {
        Ok(params) => params,
        Err(response) => return Ok(response),
    };
    let q = trim_text(params.q.as_deref().unwrap_or(""));
    let d1 = db::database(env)?;
    let statement = query::suggestions(&session.user_id, item, &q);
    let rows: Vec<SuggestionRow> = db::prepared(&d1, &statement)?.all().await?.results()?;
    respond::json(&SuggestionResponse {
        values: rows.into_iter().map(|row| row.value).collect(),
    })
}

/// D1 から引く候補の行。
#[derive(Debug, Deserialize)]
struct SuggestionRow {
    /// 候補の値。
    value: String,
}
