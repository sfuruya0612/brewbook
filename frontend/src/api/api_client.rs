//! `/api` の下を呼ぶ API クライアント (ADR-0007)。
//!
//! セッションはブラウザの Cookie が管理するため (ADR-0005)、このクライアントは Cookie を
//! 扱わない。HTTP は web-sys の fetch を使う ([`crate::api::Transport`] の Web の実装)。
//! 送信の下位の実装を [`Transport`] に分けているため、テストは偽の実装で呼び出しの内容と
//! 応答を差し替えられる。

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use serde_json::{Map, Value};

use super::api_error::{ApiCallError, ApiError, NetworkError};

/// HTTP のメソッド。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Method {
    /// `GET`。
    Get,
    /// `POST`。
    Post,
    /// `PATCH`。
    Patch,
    /// `PUT`。
    Put,
    /// `DELETE`。
    Delete,
}

impl Method {
    /// fetch に渡すメソッドの名前。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Patch => "PATCH",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
        }
    }
}

/// 送る要求。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ApiRequest {
    /// HTTP のメソッド。
    pub method: Method,

    /// `/api` から始まる同一オリジンの経路 (ADR-0005)。
    pub path: String,

    /// 本文の種類。本文が無いときは None。
    pub content_type: Option<String>,

    /// 本文。無いときは空。
    pub body: Vec<u8>,
}

/// 受け取った応答。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ApiResponse {
    /// HTTP のステータスコード。
    pub status: u16,

    /// 応答の本文。
    pub body: Vec<u8>,
}

/// 要求を送れなかった失敗 (接続の失敗)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TransportError {
    /// 失敗の原因の説明。
    pub message: String,
}

impl TransportError {
    /// 原因の説明から作る。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// 送信の未来。Web の fetch もテストの偽の実装も、同じ型で返す。
pub type TransportFuture = Pin<Box<dyn Future<Output = Result<ApiResponse, TransportError>>>>;

/// 401 の応答を受け取ったときに呼ぶ callback。セッションが失われたことを記録するために使う
/// (遷移は `AppShell` の遷移の判定が行う。ADR-0007)。
pub type UnauthorizedCallback = Rc<dyn Fn()>;

/// 設定を差し替えられる callback の置き場。
type SharedUnauthorizedCallback = Rc<RefCell<Option<UnauthorizedCallback>>>;

/// 要求を送る下位の実装。
///
/// Web は web-sys の fetch、テストは偽の実装を使う。接続できない失敗は [`TransportError`] に
/// する。エラーの応答 (4xx、5xx) は失敗にせず、[`ApiResponse`] として返す。
pub trait Transport {
    /// 要求を送る。
    fn send(&self, request: ApiRequest) -> TransportFuture;
}

/// `/api` の下を呼ぶ API クライアント (ADR-0007)。
///
/// エラーの応答は共通の型 ([`ApiError`]) に、応答を取得できない失敗は [`NetworkError`] に
/// 変換する。401 の応答を受け取ったときは [`ApiClient::set_on_unauthorized`] の callback を
/// 呼び、セッションが失われたことを記録する (遷移は `AppShell` の遷移の判定が行う。ADR-0007)。
#[derive(Clone)]
pub struct ApiClient {
    transport: Rc<dyn Transport>,
    base_path: String,
    on_unauthorized: SharedUnauthorizedCallback,
}

impl PartialEq for ApiClient {
    /// 同じ送信の実装と同じ基準の経路なら同じものとみなす (Dioxus の prop の比較のため)。
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.transport, &other.transport) && self.base_path == other.base_path
    }
}

impl ApiClient {
    /// 同一オリジンの `/api` を呼ぶ (ADR-0005)。
    pub const DEFAULT_BASE_PATH: &'static str = "/api";

    /// 送信の実装を指定して作る。
    pub fn new(transport: Rc<dyn Transport>) -> Self {
        Self::with_base_path(transport, Self::DEFAULT_BASE_PATH)
    }

    /// 送信の実装と基準の経路を指定して作る。
    pub fn with_base_path(transport: Rc<dyn Transport>, base_path: impl Into<String>) -> Self {
        Self {
            transport,
            base_path: base_path.into(),
            on_unauthorized: Rc::new(RefCell::new(None)),
        }
    }

    /// `/api` の手前までの経路。
    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// 401 の応答を受け取ったときに呼ぶ callback を設定する。セッションが失われたことを
    /// 記録するために使う (ADR-0007)。起動時にセッションの監視を設定する。
    pub fn set_on_unauthorized(&self, callback: UnauthorizedCallback) {
        *self.on_unauthorized.borrow_mut() = Some(callback);
    }

    /// `GET` を呼び、JSON のオブジェクトを返す。
    pub async fn get_json(&self, path: &str) -> Result<Map<String, Value>, ApiCallError> {
        let response = self.send(Method::Get, path, None, Vec::new()).await?;
        decode_json(&response)
    }

    /// `GET` を呼び、応答の本文をバイト列として返す (FR-14 のダウンロード)。
    ///
    /// 認証は同じオリジンの Cookie で行う (ADR-0005)。
    pub async fn get_bytes(&self, path: &str) -> Result<Vec<u8>, ApiCallError> {
        let response = self.send(Method::Get, path, None, Vec::new()).await?;
        Ok(response.body)
    }

    /// `POST` を呼び、JSON のオブジェクトを返す。
    pub async fn post_json(
        &self,
        path: &str,
        body: &Value,
    ) -> Result<Map<String, Value>, ApiCallError> {
        let body = encode_json(body)?;
        let response = self
            .send(Method::Post, path, Some("application/json"), body)
            .await?;
        decode_json(&response)
    }

    /// `POST` を呼び、バイト列を本文として送り、JSON のオブジェクトを返す (FR-19 の写真の送信)。
    pub async fn post_bytes(
        &self,
        path: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<Map<String, Value>, ApiCallError> {
        let response = self
            .send(Method::Post, path, Some(content_type), bytes)
            .await?;
        decode_json(&response)
    }

    /// `PATCH` を呼び、JSON のオブジェクトを返す。
    pub async fn patch_json(
        &self,
        path: &str,
        body: &Value,
    ) -> Result<Map<String, Value>, ApiCallError> {
        let body = encode_json(body)?;
        let response = self
            .send(Method::Patch, path, Some("application/json"), body)
            .await?;
        decode_json(&response)
    }

    /// `PUT` を呼び、JSON のオブジェクトを返す。本文は送らない (FR-21 のお気に入り)。
    pub async fn put_json(&self, path: &str) -> Result<Map<String, Value>, ApiCallError> {
        let response = self.send(Method::Put, path, None, Vec::new()).await?;
        decode_json(&response)
    }

    /// `DELETE` を呼び、JSON のオブジェクトを返す。
    pub async fn delete_json(&self, path: &str) -> Result<Map<String, Value>, ApiCallError> {
        let response = self.send(Method::Delete, path, None, Vec::new()).await?;
        decode_json(&response)
    }

    /// 要求を送り、エラーの応答を [`ApiError`] に、応答を取得できない失敗を
    /// [`NetworkError`] にする。
    async fn send(
        &self,
        method: Method,
        path: &str,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Result<ApiResponse, ApiCallError> {
        let request = ApiRequest {
            method,
            path: format!("{}{path}", self.base_path),
            content_type: content_type.map(str::to_string),
            body,
        };
        let response = self
            .transport
            .send(request)
            .await
            .map_err(|error| ApiCallError::Network(NetworkError::new(error.message)))?;
        if response.status >= 400 {
            let error = error_from(&response);
            if error.is_unauthorized() {
                // 借用を解いてから呼ぶ (callback が callback を設定し直しても壊れないように)。
                let callback = self.on_unauthorized.borrow().clone();
                if let Some(callback) = callback {
                    callback();
                }
            }
            return Err(ApiCallError::Api(error));
        }
        Ok(response)
    }
}

/// エラーの応答を [`ApiError`] にする。規約の形でない応答はステータスコードだけを運ぶ。
fn error_from(response: &ApiResponse) -> ApiError {
    if let Ok(Value::Object(body)) = serde_json::from_slice::<Value>(&response.body) {
        if let Some(Value::Object(error)) = body.get("error") {
            if let (Some(Value::String(code)), Some(Value::String(message))) =
                (error.get("code"), error.get("message"))
            {
                return ApiError {
                    status: response.status,
                    code: code.clone(),
                    message: message.clone(),
                };
            }
        }
    }
    ApiError {
        status: response.status,
        code: "unknown".to_string(),
        message: String::from_utf8_lossy(&response.body).into_owned(),
    }
}

/// 成功の応答の本文を JSON のオブジェクトにする。空の本文は空のオブジェクトとする。
fn decode_json(response: &ApiResponse) -> Result<Map<String, Value>, ApiCallError> {
    if response.body.is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_slice::<Value>(&response.body) {
        Ok(Value::Object(body)) => Ok(body),
        _ => Err(ApiCallError::Network(NetworkError::new(format!(
            "the response body is not a JSON object: {}",
            String::from_utf8_lossy(&response.body)
        )))),
    }
}

/// JSON の値を本文にする。
fn encode_json(body: &Value) -> Result<Vec<u8>, ApiCallError> {
    serde_json::to_vec(body).map_err(|error| {
        ApiCallError::Network(NetworkError::new(format!(
            "the JSON body cannot be encoded: {error}"
        )))
    })
}
