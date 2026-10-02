//! 設定の画面の操作 (FR-14)。
//!
//! エクスポートの取得は API クライアント、保存は実行環境で実装が変わる [`FileDownload`] が
//! 担う (ADR-0007)。Web の実装は Blob とオブジェクト URL とダウンロードのリンクを使う
//! (Flutter の `BrowserFileDownload` と同じ経路。ADR-0017)。

use std::rc::Rc;

use crate::api::ApiClient;
use crate::auth::AuthError;

/// ダウンロードするファイル (FR-14)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DownloadedFile {
    /// 保存するファイルの名前。
    pub file_name: String,

    /// ファイルの内容 (エクスポートの JSON など)。
    pub bytes: Vec<u8>,
}

/// 応答の内容をファイルとしてダウンロードする (FR-14、ADR-0007)。
///
/// Web の実装は応答を Blob にしてダウンロードのリンクを作る。テストは偽の実装を差し込む。
pub trait FileDownload {
    /// ファイルを保存する。
    fn save(&self, file: DownloadedFile);
}

/// 設定の画面が使う依存の束 (ADR-0007)。
#[derive(Clone)]
pub struct SettingsServices {
    /// エクスポートの取得 (FR-14)。
    pub api: ApiClient,

    /// ダウンロードの保存 (FR-14)。実行環境に合う実装を [`SettingsServices::web`] が返し、
    /// テストは偽の実装を差し込む。
    pub download: Rc<dyn FileDownload>,
}

impl SettingsServices {
    /// エクスポートのファイルの名前。Backend が付ける名前と同じにする (FR-14)。
    pub const EXPORT_FILE_NAME: &'static str = "brewbook-export.json";

    /// 依存を指定して束ねる。テストは偽の実装を渡す。
    pub fn new(api: ApiClient, download: Rc<dyn FileDownload>) -> Self {
        Self { api, download }
    }

    /// ブラウザの実装を束ねる (FR-14)。
    #[cfg(target_arch = "wasm32")]
    pub fn web(api: ApiClient) -> Self {
        Self::new(api, Rc::new(web::BrowserFileDownload))
    }
}

/// 全記録のエクスポートを取得し、JSON のファイルとしてダウンロードする (FR-14)。
pub async fn export_all(services: &SettingsServices) -> Result<(), AuthError> {
    let bytes = services.api.get_bytes("/export").await?;
    services.download.save(DownloadedFile {
        file_name: SettingsServices::EXPORT_FILE_NAME.to_string(),
        bytes,
    });
    Ok(())
}

/// Web のダウンロード (FR-14)。`web-sys` の Blob とオブジェクト URL とダウンロードのリンクを
/// 使う (Flutter の `BrowserFileDownload` と同じ経路)。
#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::JsCast;

    use super::{DownloadedFile, FileDownload};

    /// ダウンロードのファイルの MIME タイプ (FR-14 のエクスポートは JSON)。
    const DOWNLOAD_CONTENT_TYPE: &str = "application/json";

    /// Web の [`FileDownload`] (FR-14)。
    ///
    /// 応答のバイト列を Blob にし、そのオブジェクト URL を指すダウンロードのリンクを作って
    /// 押す。リンクは押した後に外し、オブジェクト URL は解放する (Flutter と同じ)。認証は
    /// 同じオリジンの Cookie で行われるため (ADR-0005)、ここではヘッダを扱わない。
    pub struct BrowserFileDownload;

    /// 保存できなかったことをコンソールに残す (握り潰さない。0043 のレビューの指摘)。
    ///
    /// 画面の通知は「エクスポートしました」のままにする (保存の失敗を利用者に伝える文言は
    /// 仕様に無い)。実際の保存は 0044 の E2E が確かめる。
    fn warn_save_failed(reason: &str) {
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&format!(
            "the export download could not be saved: {reason}"
        )));
    }

    impl FileDownload for BrowserFileDownload {
        fn save(&self, file: DownloadedFile) {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                warn_save_failed("the document is not available");
                return;
            };
            let array = js_sys::Uint8Array::from(file.bytes.as_slice());
            let parts = js_sys::Array::new();
            parts.push(&array.buffer());
            let options = web_sys::BlobPropertyBag::new();
            options.set_type(DOWNLOAD_CONTENT_TYPE);
            let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)
            else {
                warn_save_failed("the blob could not be created");
                return;
            };
            let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
                warn_save_failed("the object url could not be created");
                return;
            };
            if let Some(link) = download_link(&url, &file.file_name) {
                // リンクは文書に繋がっていないと押せないブラウザがある。
                if let Some(body) = document.body() {
                    let _ = body.append_child(&link);
                }
                link.click();
                link.remove();
            }
            let _ = web_sys::Url::revoke_object_url(&url);
        }
    }

    /// ダウンロードのリンクを組み立てる。ブラウザのテストが参照する。
    pub fn download_link(url: &str, file_name: &str) -> Option<web_sys::HtmlAnchorElement> {
        let document = web_sys::window()?.document()?;
        let link: web_sys::HtmlAnchorElement =
            document.create_element("a").ok()?.dyn_into().ok()?;
        link.set_href(url);
        link.set_download(file_name);
        Some(link)
    }
}

#[cfg(target_arch = "wasm32")]
pub use web::download_link;
