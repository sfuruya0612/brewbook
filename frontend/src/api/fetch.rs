//! web-sys の fetch で要求を送る実装 (0038)。
//!
//! HTTP は gloo-net と web-sys の fetch を比較し、依存の少ない web-sys の fetch を選んだ。
//! web-sys、js-sys、wasm-bindgen-futures は dioxus の web のビルドが既に使うクレートで、
//! 追加のクレートを増やさない。

use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use super::api_client::{
    ApiClient, ApiRequest, ApiResponse, Transport, TransportError, TransportFuture,
};

/// web-sys の fetch で要求を送る。
pub struct FetchTransport;

impl Transport for FetchTransport {
    fn send(&self, request: ApiRequest) -> TransportFuture {
        Box::pin(async move { fetch(request).await })
    }
}

/// 同一オリジンの `/api` を fetch で呼ぶ API クライアントを作る (ADR-0005)。
pub fn create_client() -> ApiClient {
    ApiClient::new(Rc::new(FetchTransport))
}

/// 要求を送り、応答の本文を読み終えて返す。
async fn fetch(request: ApiRequest) -> Result<ApiResponse, TransportError> {
    let window = web_sys::window().ok_or_else(|| TransportError::new("window is not available"))?;

    let init = web_sys::RequestInit::new();
    init.set_method(request.method.as_str());
    // 同一オリジンの `/api` だけを呼ぶ (ADR-0005)。別オリジンへの送信はブラウザに任せない。
    init.set_mode(web_sys::RequestMode::SameOrigin);
    if !request.body.is_empty() {
        let body = js_sys::Uint8Array::from(request.body.as_slice());
        init.set_body(body.as_ref());
    }
    if let Some(content_type) = &request.content_type {
        let headers = web_sys::Headers::new()
            .map_err(|error| TransportError::new(js_message("Headers::new", &error)))?;
        headers
            .set("Content-Type", content_type)
            .map_err(|error| TransportError::new(js_message("Headers::set", &error)))?;
        init.set_headers_headers(&headers);
    }

    let web_request = web_sys::Request::new_with_str_and_init(&request.path, &init)
        .map_err(|error| TransportError::new(js_message("Request::new", &error)))?;
    let promise = window.fetch_with_request(&web_request);
    let value = JsFuture::from(promise)
        .await
        .map_err(|error| TransportError::new(js_message("fetch", &error)))?;
    let response: web_sys::Response = value
        .dyn_into()
        .map_err(|_| TransportError::new("the fetch result is not a Response"))?;

    let status = response.status();
    let promise = response
        .array_buffer()
        .map_err(|error| TransportError::new(js_message("Response::array_buffer", &error)))?;
    let buffer = JsFuture::from(promise)
        .await
        .map_err(|error| TransportError::new(js_message("Response::array_buffer", &error)))?;
    let body = js_sys::Uint8Array::new(&buffer).to_vec();
    Ok(ApiResponse { status, body })
}

/// JS の例外をログと原因の特定に使える説明にする。
pub(crate) fn js_message(context: &str, error: &wasm_bindgen::JsValue) -> String {
    match error.as_string() {
        Some(message) => format!("{context}: {message}"),
        None => format!("{context}: {error:?}"),
    }
}
