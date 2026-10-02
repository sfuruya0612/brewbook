//! 記録の画面が使う依存 (0038)。

pub mod clock;
pub mod photo;
pub mod stats_period;
pub mod values;

use std::rc::Rc;

use crate::api::ApiClient;

pub use clock::Clock;
pub use photo::{
    ConvertedImage, ImageConverter, PhotoError, PhotoFuture, PhotoPicker, PickedPhoto,
    MAX_PHOTO_BYTES, MAX_PHOTO_LONG_SIDE,
};

/// 記録の画面が使う依存の束 (ADR-0007)。
///
/// API の呼び出しと、実行環境で実装が変わる写真の選択と変換と端末の時計を、この 1 つの型で
/// 画面へ配る (Flutter の `RecordServices` と同じ。0041 が使う)。実行環境に合う実装は
/// [`RecordServices::web`] が返し、テストは偽の実装を差し込む。
#[derive(Clone)]
pub struct RecordServices {
    /// 記録の API (FR-6 から FR-13)。
    pub api: ApiClient,

    /// 端末の時計とタイムゾーン (FR-9、FR-11、FR-18)。
    pub clock: Rc<dyn Clock>,

    /// 写真のファイルの選択 (FR-10)。
    pub photo_picker: Rc<dyn PhotoPicker>,

    /// 写真の JPEG への変換と縮小 (FR-10)。
    pub image_converter: Rc<dyn ImageConverter>,
}

impl PartialEq for RecordServices {
    /// 同じ依存を指しているときだけ等しいとみなす (Dioxus の prop の比較のため)。
    fn eq(&self, other: &Self) -> bool {
        self.api == other.api
            && Rc::ptr_eq(&self.clock, &other.clock)
            && Rc::ptr_eq(&self.photo_picker, &other.photo_picker)
            && Rc::ptr_eq(&self.image_converter, &other.image_converter)
    }
}

impl RecordServices {
    /// 依存を指定して束ねる。テストは偽の実装を渡す。
    pub fn new(
        api: ApiClient,
        clock: Rc<dyn Clock>,
        photo_picker: Rc<dyn PhotoPicker>,
        image_converter: Rc<dyn ImageConverter>,
    ) -> Self {
        Self {
            api,
            clock,
            photo_picker,
            image_converter,
        }
    }

    /// ブラウザの実装を束ねる (FR-10 の写真の選択と変換、端末の時計)。
    #[cfg(target_arch = "wasm32")]
    pub fn web(api: ApiClient) -> Self {
        Self::new(
            api,
            Rc::new(clock::DeviceClock),
            photo::create_photo_picker(),
            photo::create_image_converter(),
        )
    }
}
