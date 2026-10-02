//! 写真の選択と変換の依存 (FR-10、ADR-0003)。
//!
//! 実行環境で実装が変わる部分を trait に分け、記録の画面へ配る (ADR-0007)。Web の実装は
//! ブラウザの API を直接使う (ADR-0017)。テストは偽の実装を差し込む。

use std::future::Future;
use std::pin::Pin;

#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// 写真の操作の失敗。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PhotoError {
    /// 失敗の原因の説明。
    pub message: String,
}

impl PhotoError {
    /// 原因の説明から作る。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// 写真の操作の未来。
pub type PhotoFuture<T> = Pin<Box<dyn Future<Output = Result<T, PhotoError>>>>;

/// 端末から選んだ写真 (FR-10)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PickedPhoto {
    /// ファイルの名前。
    pub name: String,
    /// ファイルの内容 (JPEG、PNG、WebP など)。
    pub bytes: Vec<u8>,
}

/// 写真のファイルの選択 (FR-10、ADR-0003)。
///
/// Web の実装は `<input type="file">` のファイルの選択を開く。
pub trait PhotoPicker {
    /// 写真を選ばせる。取り消したときは None を返す。
    fn pick_photo(&self) -> PhotoFuture<Option<PickedPhoto>>;
}

/// 写真の長辺の上限 (px。FR-10)。
pub const MAX_PHOTO_LONG_SIDE: u32 = 2048;

/// 写真のサイズの上限 (バイト。FR-10 の申告サイズと同じ値)。
///
/// 変換の結果がこれを超えるときは、推測 (FR-19) の呼び出しを行わない。
pub const MAX_PHOTO_BYTES: usize = 5_000_000;

/// 送信する JPEG の品質。Flutter の実装と同じ値にする
/// (`frontend/lib/photo/image_converter_web.dart` の `jpegQuality`)。
#[cfg(target_arch = "wasm32")]
const JPEG_QUALITY: f64 = 0.92;

/// 変換した後の写真 (FR-10)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConvertedImage {
    /// JPEG の内容。
    pub bytes: Vec<u8>,
}

impl ConvertedImage {
    /// 変換後のサイズ (バイト)。アップロードの前にサーバーへ申告する (ADR-0003)。
    pub fn size(&self) -> usize {
        self.bytes.len()
    }
}

/// 写真の JPEG への変換と長辺の縮小 (FR-10、ADR-0003)。
///
/// 変換はクライアントで行い、サーバー側では行わない (PRD のやらないこと)。Web の実装は
/// Canvas API を使う (ADR-0017)。
pub trait ImageConverter {
    /// 端末が復号できる画像を JPEG に変換し、長辺を `max_long_side` px 以下に縮小する。
    fn convert_jpeg(&self, bytes: Vec<u8>, max_long_side: u32) -> PhotoFuture<ConvertedImage>;
}

/// Web の写真の選択を返す (FR-10)。
#[cfg(target_arch = "wasm32")]
pub fn create_photo_picker() -> Rc<dyn PhotoPicker> {
    Rc::new(BrowserPhotoPicker)
}

/// Web の写真の変換を返す (FR-10)。
#[cfg(target_arch = "wasm32")]
pub fn create_image_converter() -> Rc<dyn ImageConverter> {
    Rc::new(BrowserImageConverter)
}

/// 写真のファイルの選択が受け付ける形式 (FR-10)。
///
/// Web で復号できる画像はブラウザに依存する。
#[cfg(target_arch = "wasm32")]
const ACCEPTED_PHOTO_TYPES: &str = "image/jpeg,image/png,image/webp";

/// Web の写真の選択 (FR-10)。`<input type="file">` のファイルの選択を開く。
#[cfg(target_arch = "wasm32")]
struct BrowserPhotoPicker;

#[cfg(target_arch = "wasm32")]
impl PhotoPicker for BrowserPhotoPicker {
    fn pick_photo(&self) -> PhotoFuture<Option<PickedPhoto>> {
        Box::pin(async move {
            let window =
                web_sys::window().ok_or_else(|| PhotoError::new("window is not available"))?;
            let document = window
                .document()
                .ok_or_else(|| PhotoError::new("document is not available"))?;
            let input: web_sys::HtmlInputElement = document
                .create_element("input")
                .map_err(|error| PhotoError::new(js_message("create input", &error)))?
                .dyn_into()
                .map_err(|_| PhotoError::new("the created element is not an input"))?;
            input.set_type("file");
            input.set_accept(ACCEPTED_PHOTO_TYPES);
            // 選択のダイアログを開くには、input が文書に繋がっている必要があるブラウザがある。
            if let Some(body) = document.body() {
                let _ = body.append_child(&input);
            }

            // `change` か `cancel` のどちらかで完了する。`cancel` を送らないブラウザでは、
            // 選ばれるまで待ち続ける。
            let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                let resolve_change = resolve.clone();
                let on_change = wasm_bindgen::closure::Closure::once_into_js(move || {
                    let _ = resolve_change.call0(&wasm_bindgen::JsValue::NULL);
                });
                let _ = input.add_event_listener_with_callback(
                    "change",
                    on_change.unchecked_ref::<js_sys::Function>(),
                );
                let on_cancel = wasm_bindgen::closure::Closure::once_into_js(move || {
                    let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
                });
                let _ = input.add_event_listener_with_callback(
                    "cancel",
                    on_cancel.unchecked_ref::<js_sys::Function>(),
                );
            });
            input.click();
            wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(|error| PhotoError::new(js_message("select a photo", &error)))?;

            let photo = read_selected_photo(&input).await;
            input.remove();
            photo
        })
    }
}

/// 選択されたファイルを読む。選ばれていなければ None を返す。
#[cfg(target_arch = "wasm32")]
async fn read_selected_photo(
    input: &web_sys::HtmlInputElement,
) -> Result<Option<PickedPhoto>, PhotoError> {
    let Some(file) = input.files().and_then(|files| files.item(0)) else {
        return Ok(None);
    };
    let name = file.name();
    let promise = file.array_buffer();
    let buffer = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|error| PhotoError::new(js_message("read the photo", &error)))?;
    Ok(Some(PickedPhoto {
        name,
        bytes: js_sys::Uint8Array::new(&buffer).to_vec(),
    }))
}

/// Web の写真の変換 (FR-10)。Canvas API で JPEG に変換し、長辺を縮小する。
#[cfg(target_arch = "wasm32")]
struct BrowserImageConverter;

#[cfg(target_arch = "wasm32")]
impl ImageConverter for BrowserImageConverter {
    fn convert_jpeg(&self, bytes: Vec<u8>, max_long_side: u32) -> PhotoFuture<ConvertedImage> {
        Box::pin(async move {
            let window =
                web_sys::window().ok_or_else(|| PhotoError::new("window is not available"))?;
            let document = window
                .document()
                .ok_or_else(|| PhotoError::new("document is not available"))?;

            // 画像の内容を Blob にして、object URL から読み込む。
            let array = js_sys::Uint8Array::from(bytes.as_slice());
            let parts = js_sys::Array::new();
            parts.push(&array.buffer());
            let blob = web_sys::Blob::new_with_u8_array_sequence(&parts)
                .map_err(|error| PhotoError::new(js_message("create a blob", &error)))?;
            let url = web_sys::Url::create_object_url_with_blob(&blob)
                .map_err(|error| PhotoError::new(js_message("create an object url", &error)))?;

            let image: web_sys::HtmlImageElement = document
                .create_element("img")
                .map_err(|error| PhotoError::new(js_message("create an image", &error)))?
                .dyn_into()
                .map_err(|_| PhotoError::new("the created element is not an image"))?;
            let promise = js_sys::Promise::new(&mut |resolve, reject| {
                let on_load = wasm_bindgen::closure::Closure::once_into_js(move || {
                    let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
                });
                let _ = image.add_event_listener_with_callback(
                    "load",
                    on_load.unchecked_ref::<js_sys::Function>(),
                );
                let on_error = wasm_bindgen::closure::Closure::once_into_js(move || {
                    let _ = reject.call1(
                        &wasm_bindgen::JsValue::NULL,
                        &wasm_bindgen::JsValue::from_str("the image cannot be decoded"),
                    );
                });
                let _ = image.add_event_listener_with_callback(
                    "error",
                    on_error.unchecked_ref::<js_sys::Function>(),
                );
            });
            image.set_src(&url);
            let loaded = wasm_bindgen_futures::JsFuture::from(promise).await;
            let _ = web_sys::Url::revoke_object_url(&url);
            loaded.map_err(|error| PhotoError::new(js_message("decode the image", &error)))?;

            // 長辺を上限に収める。小さい画像は拡大しない。
            let width = image.natural_width();
            let height = image.natural_height();
            let long_side = width.max(height);
            let (width, height) = if long_side > max_long_side && long_side > 0 {
                let scale = f64::from(max_long_side) / f64::from(long_side);
                (
                    (f64::from(width) * scale).round() as u32,
                    (f64::from(height) * scale).round() as u32,
                )
            } else {
                (width, height)
            };

            let canvas: web_sys::HtmlCanvasElement = document
                .create_element("canvas")
                .map_err(|error| PhotoError::new(js_message("create a canvas", &error)))?
                .dyn_into()
                .map_err(|_| PhotoError::new("the created element is not a canvas"))?;
            canvas.set_width(width);
            canvas.set_height(height);
            let context = canvas
                .get_context("2d")
                .map_err(|error| PhotoError::new(js_message("get the canvas context", &error)))?
                .ok_or_else(|| PhotoError::new("the canvas context is not available"))?
                .dyn_into::<web_sys::CanvasRenderingContext2d>()
                .map_err(|_| PhotoError::new("the canvas context is not a 2d context"))?;
            context
                .draw_image_with_html_image_element_and_dw_and_dh(
                    &image,
                    0.0,
                    0.0,
                    f64::from(width),
                    f64::from(height),
                )
                .map_err(|error| PhotoError::new(js_message("draw the image", &error)))?;

            // Canvas の `toBlob` は callback を取るため、Promise に包んで待つ。JPEG の品質は
            // Flutter の実装と同じ 0.92 にする (`frontend/lib/photo/image_converter_web.dart`)。
            let promise = js_sys::Promise::new(&mut |resolve, reject| {
                let reject_error = reject.clone();
                let on_blob = wasm_bindgen::closure::Closure::once_into_js(
                    move |blob: Option<web_sys::Blob>| match blob {
                        Some(blob) => {
                            let _ = resolve.call1(&wasm_bindgen::JsValue::NULL, blob.as_ref());
                        }
                        None => {
                            let _ = reject_error.call1(
                                &wasm_bindgen::JsValue::NULL,
                                &wasm_bindgen::JsValue::from_str(
                                    "the canvas cannot be converted to JPEG",
                                ),
                            );
                        }
                    },
                );
                if let Err(error) = canvas.to_blob_with_type_and_encoder_options(
                    on_blob.unchecked_ref::<js_sys::Function>(),
                    "image/jpeg",
                    // 第 3 引数は `toBlob` の品質 (JPEG では 0 から 1 の数値)。
                    &wasm_bindgen::JsValue::from_f64(JPEG_QUALITY),
                ) {
                    let _ = reject.call1(&wasm_bindgen::JsValue::NULL, error.as_ref());
                }
            });
            let blob = wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(|error| PhotoError::new(js_message("convert to JPEG", &error)))?;
            let blob: web_sys::Blob = blob
                .dyn_into()
                .map_err(|_| PhotoError::new("the converted value is not a blob"))?;
            let promise = blob.array_buffer();
            let buffer = wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(|error| PhotoError::new(js_message("read the JPEG", &error)))?;
            Ok(ConvertedImage {
                bytes: js_sys::Uint8Array::new(&buffer).to_vec(),
            })
        })
    }
}

/// JS の例外をログと原因の特定に使える説明にする。
#[cfg(target_arch = "wasm32")]
fn js_message(context: &str, error: &wasm_bindgen::JsValue) -> String {
    match error.as_string() {
        Some(message) => format!("{context}: {message}"),
        None => format!("{context}: {error:?}"),
    }
}
