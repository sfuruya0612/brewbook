//! 記録の画面が使う依存と型 (0038、0041)。
//!
//! API の呼び出し ([`api::RecordsApi`])、端末の時計、写真の選択と変換、写真のアップロードを
//! [`RecordServices`] が束ねて画面へ配る (Flutter の `RecordServices` と同じ)。値の変換は
//! [`values`]、フォームの検証と推測の適用は [`forms`]、一覧のページングは [`list`]、
//! サジェストの状態は [`suggestions`] が持ち、いずれも Dioxus に依存しない (ADR-0013)。

pub mod api;
pub mod chart;
pub mod clock;
pub mod currencies;
pub mod display;
pub mod forms;
pub mod inputs;
pub mod list;
pub mod maps;
pub mod models;
pub mod photo;
pub mod stats;
pub mod stats_period;
pub mod suggestions;
pub mod upload;
pub mod values;

use std::rc::Rc;

use crate::api::{ApiCallError, ApiClient};
use crate::i18n::Key;

pub use api::{ListOptions, RecordsApi, SortOrder, SuggestionTarget, PAGE_SIZE};
pub use clock::Clock;
pub use currencies::{currency_name, currency_option_label, currency_options, DEFAULT_CURRENCY};
pub use display::{
    brew_reference_tiles, brew_row_subtitle, count_text, number_text, product_row_subtitle,
    purchase_price_text, purchase_reference_tiles, purchase_row_subtitle, purchase_tile_name,
    purchase_weight_text, rating_text, shop_row_subtitle,
};
pub use forms::{
    apply_purchase_suggestion, optional_text, product_match, save_target, suggested_product_name,
    validate_brew_form, validate_product_form, validate_purchase_form, validate_shop_form,
    BrewFormErrors, BrewFormValues, ProductMatch, PurchaseFormErrors, PurchaseFormValues,
    SaveTarget,
};
pub use inputs::{BrewInput, ProductInput, PurchaseInput, ShopInput};
pub use list::{PageRequest, RecordList, LOAD_MORE_THRESHOLD};
pub use maps::map_embed_url;
pub use models::{
    Brew, DeleteImpact, PhotoUploadTarget, PlaceCandidate, Product, ProductSuggestion, Purchase,
    PurchaseSuggestion, RecordPage, Shop,
};
#[cfg(target_arch = "wasm32")]
pub use photo::{create_image_converter, create_photo_picker};
pub use photo::{
    ConvertedImage, ImageConverter, PhotoError, PhotoFuture, PhotoPicker, PickedPhoto,
    MAX_PHOTO_BYTES, MAX_PHOTO_LONG_SIDE,
};
pub use stats::{stats_path, BrewPeriod, BrewRating, PurchasePeriod, RatingHistoryEntry, StatsApi};
pub use suggestions::{highlight_parts, SuggestionState};
pub use upload::{PhotoUpload, PhotoUploader, UploadFuture, UploadRequest, UploadTransport};

/// 記録の操作の失敗。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RecordError {
    /// API の呼び出しの失敗。
    Api(ApiCallError),

    /// 応答の形式の違反。
    Format(String),

    /// 写真の選択と変換の失敗 (FR-10)。
    Photo(PhotoError),

    /// 署名付き URL へのアップロードの失敗 (FR-10)。
    Upload(String),

    /// 入力の検証の誤り。画面に出す文言のキーを持つ。
    Validation(Key),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Api(error) => write!(formatter, "{error}"),
            Self::Format(message) => write!(formatter, "FormatError({message})"),
            Self::Photo(error) => write!(formatter, "PhotoError({})", error.message),
            Self::Upload(message) => write!(formatter, "UploadError({message})"),
            Self::Validation(key) => write!(formatter, "ValidationError({key:?})"),
        }
    }
}

impl std::error::Error for RecordError {}

impl From<PhotoError> for RecordError {
    fn from(error: PhotoError) -> Self {
        Self::Photo(error)
    }
}

/// 例外を画面に出す文言のキーに変換する (FR-16。Flutter の `messageForError` と同じ)。
pub fn record_error_key(error: &RecordError) -> Key {
    match error {
        RecordError::Api(ApiCallError::Api(error)) if error.is_unauthorized() => {
            Key::ErrorUnauthorized
        }
        RecordError::Api(ApiCallError::Api(error)) => match error.status {
            400 => Key::ErrorValidation,
            404 => Key::ErrorNotFound,
            409 => Key::ErrorConflict,
            410 => Key::ErrorGone,
            _ => Key::ErrorUnexpected,
        },
        RecordError::Api(ApiCallError::Network(_)) => Key::ErrorNetwork,
        RecordError::Upload(_) => Key::ErrorNetwork,
        RecordError::Photo(_) | RecordError::Format(_) => Key::ErrorUnexpected,
        RecordError::Validation(key) => *key,
    }
}

/// 再試行を促す案内を出す失敗か (ADR-0007)。
///
/// 通信の失敗と、401 以外の API エラー (500 など) は再試行で回復し得る (デザインの Feedback の
/// 「通信エラーと 401 以外の API エラーは右端に「再試行」」)。401 はログイン画面へ戻すため、
/// 形式の違反と写真の失敗は再試行で直らないため、再試行は出さない (0041 のレビューの指摘)。
pub fn record_error_retry(error: &RecordError) -> bool {
    match error {
        RecordError::Api(ApiCallError::Network(_)) | RecordError::Upload(_) => true,
        RecordError::Api(ApiCallError::Api(error)) => !error.is_unauthorized(),
        RecordError::Photo(_) | RecordError::Format(_) | RecordError::Validation(_) => false,
    }
}

/// 記録が既に無い失敗 (404) か (0056)。
///
/// 削除の API と削除の影響の 404 は、記録が既に無いため (他の端末での削除を含む) 成功と
/// 同じ扱いにする。
pub fn record_error_not_found(error: &RecordError) -> bool {
    matches!(
        error,
        RecordError::Api(ApiCallError::Api(api_error)) if api_error.status == 404
    )
}

/// 記録の画面が使う依存の束 (ADR-0007)。
///
/// API の呼び出しと、実行環境で実装が変わる写真の選択と変換と端末の時計と写真のアップロードを、
/// この 1 つの型で画面へ配る (Flutter の `RecordServices` と同じ。0041 が使う)。実行環境に
/// 合う実装は [`RecordServices::web`] が返し、テストは偽の実装を差し込む。
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

    /// 写真のアップロード (FR-10、ADR-0003)。
    pub uploader: Rc<dyn PhotoUpload>,
}

impl PartialEq for RecordServices {
    /// 同じ依存を指しているときだけ等しいとみなす (Dioxus の prop の比較のため)。
    fn eq(&self, other: &Self) -> bool {
        self.api == other.api
            && Rc::ptr_eq(&self.clock, &other.clock)
            && Rc::ptr_eq(&self.photo_picker, &other.photo_picker)
            && Rc::ptr_eq(&self.image_converter, &other.image_converter)
            && Rc::ptr_eq(&self.uploader, &other.uploader)
    }
}

impl RecordServices {
    /// 依存を指定して束ねる。テストは偽の実装を渡す。
    pub fn new(
        api: ApiClient,
        clock: Rc<dyn Clock>,
        photo_picker: Rc<dyn PhotoPicker>,
        image_converter: Rc<dyn ImageConverter>,
        uploader: Rc<dyn PhotoUpload>,
    ) -> Self {
        Self {
            api,
            clock,
            photo_picker,
            image_converter,
            uploader,
        }
    }

    /// ブラウザの実装を束ねる (FR-10 の写真の選択と変換、端末の時計)。
    #[cfg(target_arch = "wasm32")]
    pub fn web(api: ApiClient) -> Self {
        let uploader = PhotoUploader::new(
            RecordsApi::new(api.clone()),
            upload::create_upload_transport(),
        );
        Self::new(
            api,
            Rc::new(clock::DeviceClock),
            photo::create_photo_picker(),
            photo::create_image_converter(),
            Rc::new(uploader),
        )
    }
}
