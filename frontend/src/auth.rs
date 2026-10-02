//! ログインの状態 (FR-1、FR-2、FR-4)。

use crate::api::{ApiCallError, ApiClient};

/// ログインの状態 (FR-1、FR-2、FR-4)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionStatus {
    /// 起動時の確認中。
    Checking,

    /// 起動時の確認が失敗し (401 以外の API エラー、ネットワークエラー)、ログイン状態が分からない。
    Unknown,

    /// セッションが無いか無効。
    SignedOut,

    /// セッションが有効。
    SignedIn,
}

/// 起動時に `GET /api/passkeys` を呼び、ログイン状態を判定する (FR-1、FR-2)。
///
/// セッションの確認は専用の API を設けず、401 かどうかで判定する (Flutter と同じ)。
/// 401 は [`ApiClient::set_on_unauthorized`] の callback がログイン画面へ遷移させ、401 以外の
/// 失敗とネットワークエラーはログイン状態が分からないため、再試行を促す表示にする (ADR-0007)。
pub async fn check_session(api: &ApiClient) -> SessionStatus {
    match api.get_json("/passkeys").await {
        Ok(_) => SessionStatus::SignedIn,
        Err(ApiCallError::Api(error)) if error.is_unauthorized() => SessionStatus::SignedOut,
        Err(_) => SessionStatus::Unknown,
    }
}
