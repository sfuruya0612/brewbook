//! 写真の変換のブラウザテスト (0041、FR-10)。
//!
//! 長辺 4,000 px の PNG と JPEG から、JPEG かつ長辺 2048 px 以下の出力が得られることを
//! 確かめる (FR-10 の受け入れ基準)。ブラウザで動かすため `dioxus:test-web`
//! (wasm-bindgen-test) で実行する。

#![cfg(target_arch = "wasm32")]

use brew_book_frontend::records::{create_image_converter, MAX_PHOTO_LONG_SIDE};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// ブラウザの document。
fn document() -> web_sys::Document {
    web_sys::window()
        .expect("the test must run in a browser")
        .document()
        .expect("the page must have a document")
}

/// 幅 4,000 px、高さ 3,000 px の絵を描いた canvas を作る。
fn source_canvas() -> web_sys::HtmlCanvasElement {
    let canvas: web_sys::HtmlCanvasElement = document()
        .create_element("canvas")
        .expect("a canvas must be creatable")
        .dyn_into()
        .expect("the element must be a canvas");
    canvas.set_width(4000);
    canvas.set_height(3000);
    let context: web_sys::CanvasRenderingContext2d = canvas
        .get_context("2d")
        .expect("the context must be readable")
        .expect("the context must exist")
        .dyn_into()
        .expect("the context must be a 2d context");
    context.set_fill_style_str("#4a2f1c");
    context.fill_rect(0.0, 0.0, 4000.0, 3000.0);
    // 一様な絵では JPEG の大きさが小さくなりすぎるため、縞を足す。
    context.set_fill_style_str("#e6d5b8");
    for x in (0..4000).step_by(80) {
        context.fill_rect(f64::from(x), 0.0, 40.0, 3000.0);
    }
    canvas
}

/// canvas の内容を指定の形式の Blob にして、そのバイト列を返す。
async fn encode(canvas: &web_sys::HtmlCanvasElement, mime: &str) -> Vec<u8> {
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let on_blob =
            wasm_bindgen::closure::Closure::once_into_js(move |blob: Option<web_sys::Blob>| {
                match blob {
                    Some(blob) => {
                        let _ = resolve.call1(&wasm_bindgen::JsValue::NULL, blob.as_ref());
                    }
                    None => {
                        let _ = reject.call1(
                            &wasm_bindgen::JsValue::NULL,
                            &wasm_bindgen::JsValue::from_str("the canvas cannot encode the image"),
                        );
                    }
                }
            });
        let _ = canvas.to_blob_with_type(on_blob.unchecked_ref::<js_sys::Function>(), mime);
    });
    let blob = JsFuture::from(promise)
        .await
        .expect("the canvas must encode the image");
    let blob: web_sys::Blob = blob.dyn_into().expect("the value must be a blob");
    let buffer = JsFuture::from(blob.array_buffer())
        .await
        .expect("the blob must be readable");
    js_sys::Uint8Array::new(&buffer).to_vec()
}

/// 画像のバイト列を復号し、(幅、高さ) を返す。
async fn decode_size(bytes: &[u8]) -> (u32, u32) {
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array.buffer());
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts).expect("a blob must be created");
    let url =
        web_sys::Url::create_object_url_with_blob(&blob).expect("an object url must be created");
    let image: web_sys::HtmlImageElement = document()
        .create_element("img")
        .expect("an image must be creatable")
        .dyn_into()
        .expect("the element must be an image");
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let on_load = wasm_bindgen::closure::Closure::once_into_js(move || {
            let _ = resolve.call0(&wasm_bindgen::JsValue::NULL);
        });
        let _ = image
            .add_event_listener_with_callback("load", on_load.unchecked_ref::<js_sys::Function>());
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
    let loaded = JsFuture::from(promise).await;
    let _ = web_sys::Url::revoke_object_url(&url);
    loaded.expect("the converted image must be decodable");
    (image.natural_width(), image.natural_height())
}

/// PNG の入力を JPEG に変換し、長辺を 2048 px 以下にする (FR-10)。
#[wasm_bindgen_test]
async fn a_large_png_becomes_a_jpeg_with_a_long_side_of_2048() {
    let canvas = source_canvas();
    let source = encode(&canvas, "image/png").await;
    assert_eq!(
        &source[..4],
        &[0x89, 0x50, 0x4E, 0x47],
        "the source must be a PNG"
    );

    let converter = create_image_converter();
    let converted = converter
        .convert_jpeg(source, MAX_PHOTO_LONG_SIDE)
        .await
        .expect("the conversion must succeed");

    // JPEG の先頭の目印 (SOI とマーカー)。
    assert_eq!(
        &converted.bytes[..3],
        &[0xFF, 0xD8, 0xFF],
        "the output must be a JPEG"
    );
    let (width, height) = decode_size(&converted.bytes).await;
    assert!(
        width.max(height) <= MAX_PHOTO_LONG_SIDE,
        "the long side must be at most 2048 px but was {width}x{height}"
    );
    assert_eq!(
        (width, height),
        (2048, 1536),
        "the aspect ratio must be kept"
    );
}

/// JPEG の入力も同じ条件で変換する (FR-10)。
#[wasm_bindgen_test]
async fn a_large_jpeg_becomes_a_jpeg_with_a_long_side_of_2048() {
    let canvas = source_canvas();
    let source = encode(&canvas, "image/jpeg").await;
    assert_eq!(
        &source[..3],
        &[0xFF, 0xD8, 0xFF],
        "the source must be a JPEG"
    );

    let converter = create_image_converter();
    let converted = converter
        .convert_jpeg(source, MAX_PHOTO_LONG_SIDE)
        .await
        .expect("the conversion must succeed");

    assert_eq!(
        &converted.bytes[..3],
        &[0xFF, 0xD8, 0xFF],
        "the output must be a JPEG"
    );
    let (width, height) = decode_size(&converted.bytes).await;
    assert!(width.max(height) <= MAX_PHOTO_LONG_SIDE);
    assert_eq!((width, height), (2048, 1536));
}
