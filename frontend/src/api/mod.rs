//! API の呼び出し (0038)。
//!
//! 画面は同一オリジンの `/api` の下だけを呼ぶ (ADR-0005)。エラーの応答は共通の型
//! ([`ApiError`]) に変換し、応答を取得できない失敗は [`NetworkError`] にする。

mod api_client;
mod api_error;

#[cfg(target_arch = "wasm32")]
mod fetch;

pub use api_client::{
    ApiClient, ApiRequest, ApiResponse, Method, Transport, TransportError, TransportFuture,
};
pub use api_error::{ApiCallError, ApiError, NetworkError};

#[cfg(target_arch = "wasm32")]
pub use fetch::{create_client, FetchTransport};
