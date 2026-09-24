//! 結合テスト用の HTTP クライアント。
//!
//! セッションの Cookie は自動で保持せず、テストが応答の `Set-Cookie` から取り出して渡す。
//! `Secure` の Cookie を HTTP のクライアントの jar が保存するかどうかに依存しないためである。

use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::Method;
use serde_json::Value;

use super::DevServer;

/// API を呼ぶクライアント。セッションの Cookie を持てる。
pub struct ApiClient {
    base_url: String,
    client: Client,
    cookie: Option<String>,
}

impl ApiClient {
    /// API の基底 URL とセッションのトークンから作る。`None` ならセッションを持たない。
    pub fn new(base_url: &str, token: Option<&str>) -> Self {
        Self::with_timeout(base_url, token, Duration::from_secs(30))
    }

    /// 応答に時間のかかる呼び出し (想定規模の R2 の削除) のために、待ち時間を指定して作る。
    pub fn with_timeout(base_url: &str, token: Option<&str>, timeout: Duration) -> Self {
        Self {
            base_url: base_url.to_owned(),
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

    pub fn get(&self, path: &str) -> Response {
        self.send(Method::GET, path, None)
    }

    pub fn post(&self, path: &str) -> Response {
        self.send(Method::POST, path, None)
    }

    pub fn post_json(&self, path: &str, body: &Value) -> Response {
        self.send(Method::POST, path, Some(body))
    }

    pub fn patch_json(&self, path: &str, body: &Value) -> Response {
        self.send(Method::PATCH, path, Some(body))
    }

    pub fn delete(&self, path: &str) -> Response {
        self.send(Method::DELETE, path, None)
    }

    /// 1 件のリクエストを送る。
    fn send(&self, method: Method, path: &str, body: Option<&Value>) -> Response {
        let mut request = self
            .client
            .request(method, format!("{}{path}", self.base_url));
        if let Some(body) = body {
            request = request
                .header("content-type", "application/json")
                .body(serde_json::to_string(body).expect("the body must be serializable"));
        }
        if let Some(cookie) = &self.cookie {
            request = request.header("cookie", cookie);
        }
        request
            .send()
            .expect("the request must reach the dev server")
    }
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
