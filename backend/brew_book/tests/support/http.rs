//! 結合テスト用の HTTP クライアント。
//!
//! セッションの Cookie は自動で保持せず、テストが応答の `Set-Cookie` から取り出して渡す。
//! `Secure` の Cookie を HTTP のクライアントの jar が保存するかどうかに依存しないためである。
//!
//! 状態を変更するメソッド (POST、PUT、PATCH、DELETE) には、呼び出し先と同じオリジンの
//! `Origin` を自動で付ける (ADR-0005 の `Origin` の検証を通すため)。別オリジンと `Origin` の
//! 無い呼び出しは、[`ApiClient::with_origin`] と [`ApiClient::without_origin`] で作る。

use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::Method;
use serde_json::Value;

use super::DevServer;

/// API を呼ぶクライアント。セッションの Cookie を持てる。
pub struct ApiClient {
    base_url: String,
    /// 状態を変更するメソッドに付ける `Origin`。`None` なら付けない。
    origin: Option<String>,
    client: Client,
    cookie: Option<String>,
}

impl ApiClient {
    /// API の基底 URL とセッションのトークンから作る。`None` ならセッションを持たない。
    ///
    /// `Origin` は基底 URL のオリジン (基底 URL 自体) にする。
    pub fn new(base_url: &str, token: Option<&str>) -> Self {
        Self::with_timeout(base_url, token, Duration::from_secs(30))
    }

    /// 応答に時間のかかる呼び出し (想定規模の R2 の削除) のために、待ち時間を指定して作る。
    pub fn with_timeout(base_url: &str, token: Option<&str>, timeout: Duration) -> Self {
        Self {
            base_url: base_url.to_owned(),
            origin: Some(base_url.to_owned()),
            client: Client::builder()
                .timeout(timeout)
                .build()
                .expect("the HTTP client must build"),
            cookie: token.map(|token| format!("session={token}")),
        }
    }

    /// サーバーの基底 URL から作る。
    pub fn for_server(server: &DevServer, token: Option<&str>) -> Self {
        Self::new(&server.base_url(), token)
    }

    /// 状態を変更するメソッドに付ける `Origin` を差し替える (別オリジンからの呼び出しの検査に使う)。
    pub fn with_origin(mut self, origin: &str) -> Self {
        self.origin = Some(origin.to_owned());
        self
    }

    /// 状態を変更するメソッドに `Origin` を付けない (検証できない呼び出しの検査に使う)。
    pub fn without_origin(mut self) -> Self {
        self.origin = None;
        self
    }

    pub fn get(&self, path: &str) -> Response {
        self.send(Method::GET, path, None)
    }

    pub fn post(&self, path: &str) -> Response {
        self.send(Method::POST, path, None)
    }

    pub fn post_json(&self, path: &str, body: &Value) -> Response {
        self.send(Method::POST, path, Some(body))
    }

    /// 本体をバイト列として送る。Content-Type を指定する (写真からの推測の検査に使う。FR-19)。
    pub fn post_bytes(&self, path: &str, content_type: &str, bytes: &[u8]) -> Response {
        let mut request = self
            .client
            .request(Method::POST, format!("{}{path}", self.base_url))
            .header("content-type", content_type)
            .body(bytes.to_vec());
        if let Some(cookie) = &self.cookie {
            request = request.header("cookie", cookie);
        }
        if let Some(origin) = &self.origin {
            request = request.header("origin", origin);
        }
        request
            .send()
            .expect("the request must reach the dev server")
    }

    pub fn patch_json(&self, path: &str, body: &Value) -> Response {
        self.send(Method::PATCH, path, Some(body))
    }

    pub fn delete(&self, path: &str) -> Response {
        self.send(Method::DELETE, path, None)
    }

    /// プリフライト (OPTIONS) を送る。応答に CORS のヘッダが無いことの検査に使う。
    pub fn options(&self, path: &str) -> Response {
        self.send(Method::OPTIONS, path, None)
    }

    /// 1 件のリクエストを送る。GET と HEAD 以外には `Origin` を付ける (ブラウザの挙動に合わせる)。
    fn send(&self, method: Method, path: &str, body: Option<&Value>) -> Response {
        let mut request = self
            .client
            .request(method.clone(), format!("{}{path}", self.base_url));
        if let Some(body) = body {
            request = request
                .header("content-type", "application/json")
                .body(serde_json::to_string(body).expect("the body must be serializable"));
        }
        if let Some(cookie) = &self.cookie {
            request = request.header("cookie", cookie);
        }
        if origin_is_sent(&method) {
            if let Some(origin) = &self.origin {
                request = request.header("origin", origin);
            }
        }
        request
            .send()
            .expect("the request must reach the dev server")
    }
}

/// ブラウザが `Origin` を付けるメソッドか (GET と HEAD には付けない)。
fn origin_is_sent(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD)
}

/// 応答の状態コードと JSON の本体を返す。
pub fn read(response: Response) -> (u16, Value) {
    let status = response.status().as_u16();
    let text = response.text().expect("the response body must be readable");
    let body = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("the response must be JSON but was {text}: {error}"))
    };
    (status, body)
}

/// 応答の `Set-Cookie` からセッションのトークンを取り出す。
pub fn session_token(response: &Response) -> Option<String> {
    let value = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with("session="))?;
    let token = value.trim_start_matches("session=").split(';').next()?;
    if token.is_empty() {
        return None;
    }
    Some(token.to_owned())
}

/// エラー応答のコードを返す。
pub fn error_code(body: &Value) -> Option<&str> {
    body.get("error")?.get("code")?.as_str()
}
