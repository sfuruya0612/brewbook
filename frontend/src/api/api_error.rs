//! API の応答のエラー (PRD の「エラー応答の規約」)。

use std::fmt;

use crate::i18n::Key;

/// API が返したエラーの応答 (PRD の「エラー応答の規約」)。
///
/// Backend のエラー応答は `{"error": {"code": "<snake_case>", "message": "<英語>"}}` の形を取る。
/// `code` と `message` はそのまま保持し、`status` で画面の分岐 (401 はログイン画面へ遷移させる) を
/// 行う。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ApiError {
    /// HTTP のステータスコード。
    pub status: u16,

    /// Backend が返したエラーの種別 (snake_case)。
    pub code: String,

    /// Backend が返した英語のメッセージ。画面には出さず、ログと原因の特定に使う。
    pub message: String,
}

impl ApiError {
    /// 認証が失われたか (401)。
    pub fn is_unauthorized(&self) -> bool {
        self.status == 401
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "ApiError(status: {}, code: {}, message: {})",
            self.status, self.code, self.message
        )
    }
}

/// 応答を取得できなかったこと (接続の失敗、応答の形式の違反) を表す。
///
/// 画面は再試行を促す表示にする (ADR-0007 のオンライン前提)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NetworkError {
    /// 失敗の原因 (接続の失敗、JSON の解析の失敗)。
    pub message: String,
}

impl NetworkError {
    /// 原因の説明から作る。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "NetworkError({})", self.message)
    }
}

/// API の呼び出しの失敗。
///
/// エラーの応答 ([`ApiCallError::Api`]) と、応答を取得できなかった失敗
/// ([`ApiCallError::Network`]) を分け、画面が扱いを変えられるようにする (再試行の案内と、
/// 401 のログイン画面への遷移)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ApiCallError {
    /// エラーの応答。
    Api(ApiError),

    /// 応答を取得できなかった失敗。画面は再試行を促す (ADR-0007)。
    Network(NetworkError),
}

impl ApiCallError {
    /// 認証が失われたか (401 の応答)。
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, Self::Api(error) if error.is_unauthorized())
    }

    /// 再試行を促す案内のキー (ADR-0007)。応答を取得できなかった失敗のときだけ返す。
    ///
    /// 画面はこの 2 つのキーでバナー (メッセージと「再試行」) を組む (0040 以降)。
    /// エラーの応答 (400、404 など) は入力の修正で直るため、再試行の案内は出さない。
    pub fn retry_keys(&self) -> Option<(Key, Key)> {
        match self {
            Self::Network(_) => Some((Key::ErrorNetwork, Key::RetryButton)),
            Self::Api(_) => None,
        }
    }
}

impl fmt::Display for ApiCallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Api(error) => write!(formatter, "{error}"),
            Self::Network(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ApiCallError {}
