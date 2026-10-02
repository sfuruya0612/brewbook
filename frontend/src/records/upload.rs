//! 写真のアップロード (FR-10、ADR-0003)。
//!
//! 変換済みの写真について、アップロード用 URL の要求、R2 への PUT、完了の通知の 3 回の
//! 呼び出しを行う (ADR-0003)。PUT は R2 の S3 互換エンドポイント (別オリジン) に直接行い、
//! `Content-Type: image/jpeg` を付ける。CORS は R2 のバケットの設定が許可する (0009)。
//! 送信の下位の実装を [`UploadTransport`] に分けているため、native のテストは偽の実装で
//! 呼び出しの内容と応答を差し替えられる。

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use super::api::RecordsApi;
use super::models::Purchase;
use super::photo::ConvertedImage;
use super::RecordError;

/// 署名付き URL へ送る要求。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UploadRequest {
    /// 署名付きの PUT の URL。
    pub url: String,

    /// 本文の種類。常に `image/jpeg`。
    pub content_type: String,

    /// 変換済みの JPEG の内容。
    pub body: Vec<u8>,
}

/// 送信の未来。
pub type UploadFuture = Pin<Box<dyn Future<Output = Result<u16, String>>>>;

/// 署名付き URL へ PUT を送る下位の実装。
///
/// Web は web-sys の fetch、テストは偽の実装を使う。応答の状態コードを返し、接続できない
/// 失敗は説明の文字列にする。
pub trait UploadTransport {
    /// 要求を送る。
    fn put(&self, request: UploadRequest) -> UploadFuture;
}

/// 写真のアップロード (FR-10)。
pub trait PhotoUpload {
    /// 写真をアップロードし、紐づいた後の購入を返す。差し替えも同じ手順で行う (FR-10)。
    fn upload(
        &self,
        purchase_id: String,
        image: ConvertedImage,
    ) -> Pin<Box<dyn Future<Output = Result<Purchase, RecordError>>>>;
}

/// 記録の API と送信の実装を束ねたアップローダ。
pub struct PhotoUploader {
    records: RecordsApi,
    transport: Rc<dyn UploadTransport>,
}

impl PhotoUploader {
    /// 記録の API と送信の実装から作る。
    pub fn new(records: RecordsApi, transport: Rc<dyn UploadTransport>) -> Self {
        Self { records, transport }
    }
}

impl PartialEq for PhotoUploader {
    /// 同じ API と送信の実装を指しているときだけ等しいとみなす。
    fn eq(&self, other: &Self) -> bool {
        self.records == other.records && Rc::ptr_eq(&self.transport, &other.transport)
    }
}

impl PhotoUpload for PhotoUploader {
    fn upload(
        &self,
        purchase_id: String,
        image: ConvertedImage,
    ) -> Pin<Box<dyn Future<Output = Result<Purchase, RecordError>>>> {
        // 未来を 'static にするため、必要な依存を先に複製する (画面が spawn で待つ)。
        let records = self.records.clone();
        let transport = self.transport.clone();
        Box::pin(async move {
            // 変換後のサイズを申告して、署名付きの PUT の URL を受け取る (ADR-0003)。
            let size = image.size();
            let target = records.request_photo_upload_url(&purchase_id, size).await?;
            let status = transport
                .put(UploadRequest {
                    url: target.url,
                    content_type: "image/jpeg".to_string(),
                    body: image.bytes,
                })
                .await
                .map_err(RecordError::Upload)?;
            if status >= 400 {
                // R2 のエラーは XML で返るため、共通の型にせず再試行を促す表示にする (ADR-0007)。
                return Err(RecordError::Upload(format!(
                    "the upload failed with status {status}"
                )));
            }
            // アップロードの完了を通知し、購入に紐づける (FR-10)。
            records
                .complete_photo(&purchase_id, &target.key, size)
                .await
        })
    }
}

/// Web の送信の実装を返す (FR-10)。
#[cfg(target_arch = "wasm32")]
pub fn create_upload_transport() -> Rc<dyn UploadTransport> {
    Rc::new(fetch::FetchUploadTransport)
}

/// web-sys の fetch で署名付き URL へ PUT を送る実装 (FR-10)。
#[cfg(target_arch = "wasm32")]
mod fetch {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    use super::{UploadFuture, UploadRequest, UploadTransport};

    /// web-sys の fetch で PUT を送る。
    pub struct FetchUploadTransport;

    impl UploadTransport for FetchUploadTransport {
        fn put(&self, request: UploadRequest) -> UploadFuture {
            Box::pin(async move { put(request).await })
        }
    }

    /// 署名付き URL へ PUT を送り、応答の状態コードを返す。
    ///
    /// 送信先は R2 の S3 互換エンドポイント (別オリジン) のため、モードは既定の CORS にする。
    /// `Content-Type` は署名の条件に含まれるため必ず付ける (ADR-0003)。`Content-Length` は
    /// ブラウザが本文から自動で付ける。
    async fn put(request: UploadRequest) -> Result<u16, String> {
        let window = web_sys::window().ok_or_else(|| "window is not available".to_string())?;
        let init = web_sys::RequestInit::new();
        init.set_method("PUT");
        init.set_mode(web_sys::RequestMode::Cors);
        let body = js_sys::Uint8Array::from(request.body.as_slice());
        init.set_body(body.as_ref());
        let headers =
            web_sys::Headers::new().map_err(|error| js_message("Headers::new", &error))?;
        headers
            .set("Content-Type", &request.content_type)
            .map_err(|error| js_message("Headers::set", &error))?;
        init.set_headers_headers(&headers);

        let web_request = web_sys::Request::new_with_str_and_init(&request.url, &init)
            .map_err(|error| js_message("Request::new", &error))?;
        let promise = window.fetch_with_request(&web_request);
        let value = JsFuture::from(promise)
            .await
            .map_err(|error| js_message("fetch", &error))?;
        let response: web_sys::Response = value
            .dyn_into()
            .map_err(|_| "the fetch result is not a Response".to_string())?;
        Ok(response.status())
    }

    /// JS の例外をログと原因の特定に使える説明にする。
    fn js_message(context: &str, error: &wasm_bindgen::JsValue) -> String {
        match error.as_string() {
            Some(message) => format!("{context}: {message}"),
            None => format!("{context}: {error:?}"),
        }
    }
}
