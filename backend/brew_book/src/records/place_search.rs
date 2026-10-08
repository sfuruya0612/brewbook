//! 店名からの住所の補完の API (FR-22、ADR-0019)。
//!
//! 経路は `GET /api/place-search` とし、`q` (店名) と `lang` (ja か en) を必須のクエリ
//! パラメータで受け取る。Places API (New) の Text Search を 1 回呼び、名前と住所の候補を
//! 最大 5 件返す。
//!
//! API キーは Secret の `GOOGLE_PLACES_API_KEY` から読み、無いときは 500 を返す。
//! 呼び出しの失敗は理由を区別せず 500 を返す。店名と応答はログに出さない (FR-19 と同じ扱い)。
//! 入力の検証と応答の解析は `brew_book_core::maps` が持ち、ここは経路の処理だけを行う。

use brew_book_core::maps::{self, SearchResponse};
use worker::{console_error, Env, Fetch, Headers, Method, Request, RequestInit, Response, Result};

use super::{internal_error, invalid_input};
use crate::auth::{self, session::Session};
use crate::respond;

/// API キーの Secret の名前 (ADR-0019)。
pub const API_KEY_SECRET: &str = "GOOGLE_PLACES_API_KEY";
/// Text Search (New) の経路 (ADR-0019)。
const SEARCH_URL: &str = "https://places.googleapis.com/v1/places:searchText";
/// 応答で返すフィールド。`displayName` と `formattedAddress` は Text Search Pro の SKU (ADR-0019)。
const FIELD_MASK: &str = "places.displayName,places.formattedAddress";

/// `GET /api/place-search` のクエリパラメータ。
struct SearchParams {
    /// 店名 (`q`)。
    query: String,
    /// 言語 (`lang`)。
    lang: String,
}

impl SearchParams {
    /// リクエストのクエリ文字列から読む。複数回の指定は 400 の応答にする。
    /// 無いパラメータは空の文字列にし、入力の検証で 400 にする。
    fn from_request(req: &Request) -> Result<Self, Response> {
        let url = match req.url() {
            Ok(url) => url,
            Err(error) => {
                console_error!("the request URL is not readable: {error}");
                return Err(internal_error());
            }
        };
        let mut query = None;
        let mut lang = None;
        for (name, value) in url.query_pairs() {
            match name.as_ref() {
                "q" => {
                    if query.is_some() {
                        return Err(invalid_input("q must be a single string"));
                    }
                    query = Some(value.into_owned());
                }
                "lang" => {
                    if lang.is_some() {
                        return Err(invalid_input("lang must be a single string"));
                    }
                    lang = Some(value.into_owned());
                }
                _ => {}
            }
        }
        Ok(Self {
            query: query.unwrap_or_default(),
            lang: lang.unwrap_or_default(),
        })
    }
}

/// 店名から住所の候補を返す。認証が必要 (FR-22)。
pub async fn search(req: &Request, env: &Env, _session: &Session) -> Result<Response> {
    let params = match SearchParams::from_request(req) {
        Ok(params) => params,
        Err(response) => return Ok(response),
    };
    let (query, lang) = match maps::validate_search_input(&params.query, &params.lang) {
        Ok(input) => input,
        Err(error) => return Ok(invalid_input(error.message())),
    };
    let api_key = match auth::var_or(env, API_KEY_SECRET, "")? {
        key if key.is_empty() => {
            // キーの未設定は設定の誤りである。店名はログに出さない (FR-22)。
            console_error!("the {API_KEY_SECRET} secret is not configured");
            return Ok(internal_error());
        }
        key => key,
    };
    let body = maps::search_request_body(&query, lang);
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(worker::wasm_bindgen::JsValue::from_str(&body)));
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("X-Goog-Api-Key", &api_key)?;
    headers.set("X-Goog-FieldMask", FIELD_MASK)?;
    init.with_headers(headers);
    let request = Request::new_with_init(SEARCH_URL, &init)?;
    let mut response = match Fetch::Request(request).send().await {
        Ok(response) => response,
        Err(error) => {
            console_error!("the place search failed: {error}");
            return Ok(internal_error());
        }
    };
    if response.status_code() != 200 {
        // 応答の本文 (店名に依存し得る) はログに出さない (FR-22)。状態コードだけ残す。
        console_error!(
            "the place search returned the status {}",
            response.status_code()
        );
        return Ok(internal_error());
    }
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            console_error!("the place search response is not readable: {error}");
            return Ok(internal_error());
        }
    };
    let candidates = match maps::parse_search_response(&text) {
        Ok(candidates) => candidates,
        Err(error) => {
            console_error!("the place search response is invalid: {}", error.message());
            return Ok(internal_error());
        }
    };
    respond::json(&SearchResponse { candidates })
}
