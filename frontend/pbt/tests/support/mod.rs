//! PBT が共有する補助。
//!
//! API の PBT は、送信の実装を偽の実装に差し替えて、要求と応答の JSON の往復を確かめる。

#![allow(dead_code)] // 補助は複数のテストクレートで共有するため、各クレートから見て未使用の項目がある

use std::cell::RefCell;
use std::future::Future;
use std::task::{Context, Poll, Waker};

use brew_book_frontend::api::{ApiRequest, ApiResponse, Transport, TransportFuture};

/// 未来を完了まで進める (テストが使う未来は IO を待たない)。
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

/// 要求の本文をそのまま応答の本文として返す偽の送信の実装。
pub struct EchoTransport {
    requests: RefCell<Vec<ApiRequest>>,
}

impl EchoTransport {
    /// 作る。
    pub fn new() -> Self {
        Self {
            requests: RefCell::new(Vec::new()),
        }
    }

    /// 記録した最後の要求。
    pub fn last_request(&self) -> ApiRequest {
        self.requests
            .borrow()
            .last()
            .cloned()
            .expect("a request must be sent")
    }
}

impl Transport for EchoTransport {
    fn send(&self, request: ApiRequest) -> TransportFuture {
        let response = Ok(ApiResponse {
            status: 200,
            body: request.body.clone(),
        });
        self.requests.borrow_mut().push(request);
        Box::pin(std::future::ready(response))
    }
}

/// 決まった応答を返す偽の送信の実装。
pub struct FixedTransport {
    response: ApiResponse,
}

impl FixedTransport {
    /// ステータスコードと本文を指定して作る。
    pub fn new(status: u16, body: String) -> Self {
        Self {
            response: ApiResponse {
                status,
                body: body.into_bytes(),
            },
        }
    }
}

impl Transport for FixedTransport {
    fn send(&self, _request: ApiRequest) -> TransportFuture {
        Box::pin(std::future::ready(Ok(self.response.clone())))
    }
}
