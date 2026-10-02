//! native のテストが共有する補助。
//!
//! API クライアントのテストは、送信の実装 ([`Transport`]) を偽の実装に差し替えて、送った要求と
//! 返す応答を制御する。非同期の呼び出しは [`block_on`] で待つ (Web 以外には非同期の実行時が
//! 無いため)。記録の依存のテストは、時計と写真の偽の実装を使う。

#![allow(dead_code)] // 補助は複数のテストクレートで共有するため、各クレートから見て未使用の項目がある

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use brew_book_frontend::api::{
    ApiClient, ApiRequest, ApiResponse, Transport, TransportError, TransportFuture,
};
use brew_book_frontend::auth::{
    CreationOptions, PasskeyClient, PasskeyError, PasskeyFuture, RequestOptions,
};
use brew_book_frontend::records::values::LocalDateTime;
use brew_book_frontend::records::{
    Clock, ConvertedImage, ImageConverter, PhotoFuture, PhotoPicker, PickedPhoto,
};
use serde_json::{Map, Value};

/// 未来を完了まで進める。
///
/// テストが使う未来は IO を待たないため、no-op の waker で繰り返し poll すれば完了する。
pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// 送った要求を記録し、用意した応答を順に返す偽の送信の実装。
pub struct FakeTransport {
    requests: RefCell<Vec<ApiRequest>>,
    responses: RefCell<VecDeque<Result<ApiResponse, TransportError>>>,
}

impl FakeTransport {
    /// 応答を順に返す偽の実装を作る。
    pub fn new(responses: Vec<Result<ApiResponse, TransportError>>) -> Self {
        Self {
            requests: RefCell::new(Vec::new()),
            responses: RefCell::new(responses.into()),
        }
    }

    /// ステータスコードと本文の応答。
    pub fn response(status: u16, body: &str) -> Result<ApiResponse, TransportError> {
        Ok(ApiResponse {
            status,
            body: body.as_bytes().to_vec(),
        })
    }

    /// 応答を取得できない失敗。
    pub fn failure(message: &str) -> Result<ApiResponse, TransportError> {
        Err(TransportError::new(message))
    }

    /// 記録した要求。
    pub fn requests(&self) -> Vec<ApiRequest> {
        self.requests.borrow().clone()
    }

    /// 記録した最後の要求。1 件も送られていなければ panic する。
    pub fn last_request(&self) -> ApiRequest {
        self.requests
            .borrow()
            .last()
            .cloned()
            .expect("a request must be sent")
    }
}

impl Transport for FakeTransport {
    fn send(&self, request: ApiRequest) -> TransportFuture {
        self.requests.borrow_mut().push(request);
        let response = self
            .responses
            .borrow_mut()
            .pop_front()
            .expect("a fake response must be queued");
        Box::pin(std::future::ready(response))
    }
}

/// 偽の送信の実装を持つ API クライアントを作る。
pub fn client(
    responses: Vec<Result<ApiResponse, TransportError>>,
) -> (ApiClient, Rc<FakeTransport>) {
    let transport = Rc::new(FakeTransport::new(responses));
    let client = ApiClient::new(transport.clone());
    (client, transport)
}

/// パスキーの偽の実装 (0040)。呼び出しのオプションを記録し、用意したクレデンシャルか失敗を返す。
pub struct FakePasskeyClient {
    /// `create_credential` と `get_credential` が返すクレデンシャル。
    pub credential: Map<String, Value>,

    /// 返す失敗。Some のときはクレデンシャルの代わりに失敗する。
    pub failure: Option<PasskeyError>,

    created: RefCell<Vec<CreationOptions>>,
    requested: RefCell<Vec<RequestOptions>>,
}

impl FakePasskeyClient {
    /// 固定のクレデンシャルを返す偽の実装を作る。
    pub fn new() -> Self {
        let mut credential = Map::new();
        credential.insert("id".to_string(), Value::String("ZmFrZQ".to_string()));
        credential.insert("type".to_string(), Value::String("public-key".to_string()));
        Self {
            credential,
            failure: None,
            created: RefCell::new(Vec::new()),
            requested: RefCell::new(Vec::new()),
        }
    }

    /// 登録で受け取ったオプション。
    pub fn created(&self) -> Vec<CreationOptions> {
        self.created.borrow().clone()
    }

    /// ログインで受け取ったオプション。
    pub fn requested(&self) -> Vec<RequestOptions> {
        self.requested.borrow().clone()
    }
}

impl Default for FakePasskeyClient {
    fn default() -> Self {
        Self::new()
    }
}

impl PasskeyClient for FakePasskeyClient {
    fn create_credential(&self, options: CreationOptions) -> PasskeyFuture<Map<String, Value>> {
        self.created.borrow_mut().push(options);
        let result = match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(self.credential.clone()),
        };
        Box::pin(std::future::ready(result))
    }

    fn get_credential(&self, options: RequestOptions) -> PasskeyFuture<Map<String, Value>> {
        self.requested.borrow_mut().push(options);
        let result = match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(self.credential.clone()),
        };
        Box::pin(std::future::ready(result))
    }
}

/// 端末の時計の偽の実装。
pub struct FakeClock {
    /// `now` が返す日時。
    pub now: LocalDateTime,
    /// `utc_offset_minutes` が返す分数。
    pub utc_offset_minutes: i32,
}

impl Clock for FakeClock {
    fn now(&self) -> LocalDateTime {
        self.now
    }

    fn utc_offset_minutes(&self) -> i32 {
        self.utc_offset_minutes
    }
}

/// 写真の選択の偽の実装。
pub struct FakePhotoPicker {
    /// `pick_photo` が返す写真。None なら取り消しとして None を返す。
    pub photo: Option<PickedPhoto>,
}

impl PhotoPicker for FakePhotoPicker {
    fn pick_photo(&self) -> PhotoFuture<Option<PickedPhoto>> {
        let photo = self.photo.clone();
        Box::pin(std::future::ready(Ok(photo)))
    }
}

/// 写真の変換の偽の実装。入力をそのまま返す。
pub struct FakeImageConverter;

impl ImageConverter for FakeImageConverter {
    fn convert_jpeg(&self, bytes: Vec<u8>, _max_long_side: u32) -> PhotoFuture<ConvertedImage> {
        Box::pin(std::future::ready(Ok(ConvertedImage { bytes })))
    }
}
