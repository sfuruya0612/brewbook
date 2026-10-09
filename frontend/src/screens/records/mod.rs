//! 記録の画面 (0041)。
//!
//! ホーム (抽出の一覧)、抽出の詳細と入力、購入の一覧と詳細と入力、商品の一覧と入力、店の一覧と
//! 入力の画面を持つ。画面の状態は Dioxus の signal で持ち、フォームの入力の検証と推測の適用は
//! [`crate::records`] の純粋なモジュールに置く (ADR-0013)。画面数の成功指標を守るため、抽出の
//! 入力は 1 画面に収め、参照先の選択はボトムシートで行う (docs/design/README.md)。

pub mod brew_detail;
pub mod brew_form;
pub mod home;
pub mod product_form;
pub mod product_list;
pub mod purchase_detail;
pub mod purchase_form;
pub mod purchase_list;
pub mod shop_form;
pub mod shop_list;

mod list_view;
mod picker;
mod suggestion_field;

use dioxus::prelude::*;
use dioxus_router::Navigator;

use crate::i18n::{t, Key};
use crate::records::{record_error_key, record_error_retry, RecordError};
use crate::router::Route;
use crate::ui::{Banner, ConfirmDialog, RailItem};

pub use list_view::{RecordListView, RecordLoader, SortChoice};
pub use picker::RecordPickerSheet;
pub use suggestion_field::SuggestionField;

/// ヘッダーのメニューとナビゲーションレールに並べる行き先 (0047)。
///
/// 抽出、購入、商品、店、統計、設定の 6 項目。並びの添字が [`rail_items`] の `active` に
/// 対応する。ホームのラベルは画面名ではなく「抽出」([`Key::BrewsLabel`]) にする。
pub const NAV_ENTRIES: [(Key, &str, Route); 6] = [
    (Key::BrewsLabel, "format_list_bulleted", Route::Home {}),
    (Key::PurchasesTitle, "shopping_bag", Route::Purchases {}),
    (Key::ProductsTitle, "spa", Route::Products {}),
    (Key::ShopsTitle, "storefront", Route::Shops {}),
    (Key::StatsTitle, "bar_chart", Route::Stats {}),
    (Key::SettingsTitle, "settings", Route::Settings {}),
];

/// 広い画面のナビゲーションレールの項目を組む (docs/design/components/WideLayout)。
///
/// `active` は選択中の項目の位置 (ホーム 0、購入 1、商品 2、店 3、統計 4、設定 5)。
pub fn rail_items(navigator: Navigator, active: usize) -> Vec<RailItem> {
    NAV_ENTRIES
        .into_iter()
        .enumerate()
        .map(|(index, (key, icon, route))| RailItem {
            label: t(key).to_string(),
            icon: icon.to_string(),
            selected: index == active,
            on_click: EventHandler::new(move |_| {
                let _ = navigator.push(route.clone());
            }),
        })
        .collect()
}

/// 記録の変更を一覧に知らせる (保存、写真の操作の後)。一覧は先頭から読み直す。
pub fn mark_records_changed(revision: &mut Signal<u64>) {
    revision.set(revision() + 1);
}

/// 失敗のバナーを組む。再試行できる失敗だけ「再試行」を付ける (ADR-0007)。
pub fn error_banner(error: &RecordError, on_retry: Option<EventHandler<MouseEvent>>) -> Element {
    rsx! {
        Banner { message: t(record_error_key(error)).to_string(), on_retry }
    }
}

/// 再試行できる失敗のバナーを組む。再試行の処理を受け取る。
pub fn retryable_banner(error: &RecordError, on_retry: EventHandler<()>) -> Element {
    let handler =
        record_error_retry(error).then_some(EventHandler::new(move |_| on_retry.call(())));
    rsx! {
        Banner { message: t(record_error_key(error)).to_string(), on_retry: handler }
    }
}

/// 保存せずに閉じる確認 (0054)。フォームに変更があるときだけ出す。
#[component]
pub fn DiscardConfirm(
    /// 開いているか。確認の後は false にする。
    open: Signal<bool>,
    /// 破棄して閉じるときの動き。
    on_discard: EventHandler<()>,
) -> Element {
    let mut open = open;
    rsx! {
        ConfirmDialog {
            title: t(Key::DiscardConfirmTitle).to_string(),
            message: t(Key::DiscardConfirmMessage).to_string(),
            cancel_label: t(Key::CancelButton).to_string(),
            confirm_label: t(Key::DiscardConfirmButton).to_string(),
            on_cancel: move |_| open.set(false),
            on_confirm: move |_| {
                open.set(false);
                on_discard.call(());
            },
        }
    }
}

/// 記録の削除の確認 (0056)。確認の後に削除の API を呼ぶ。
#[component]
pub fn DeleteConfirm(
    /// 開いているか。確認の後は false にする。
    open: Signal<bool>,
    /// 題 (ARB から取る)。
    title: String,
    /// 本文 (ARB から取る)。連鎖で消える記録の件数を含む。
    message: String,
    /// 削除するときの動き。
    on_confirm: EventHandler<()>,
) -> Element {
    let mut open = open;
    rsx! {
        ConfirmDialog {
            title,
            message,
            cancel_label: t(Key::CancelButton).to_string(),
            confirm_label: t(Key::DeleteConfirmButton).to_string(),
            danger: true,
            on_cancel: move |_| open.set(false),
            on_confirm: move |_| {
                open.set(false);
                on_confirm.call(());
            },
        }
    }
}

/// 写真を `data:` URL にするための base64 (プレビュー用。外部のクレートを増やさない)。
pub fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        encoded.push(char::from(ALPHABET[usize::from(first >> 2)]));
        encoded.push(char::from(
            ALPHABET[usize::from(((first & 0b11) << 4) | (second >> 4))],
        ));
        if chunk.len() > 1 {
            encoded.push(char::from(
                ALPHABET[usize::from(((second & 0b1111) << 2) | (third >> 6))],
            ));
        } else {
            encoded.push('=');
        }
        if chunk.len() > 2 {
            encoded.push(char::from(ALPHABET[usize::from(third & 0b111111)]));
        } else {
            encoded.push('=');
        }
    }
    encoded
}

/// 選択中の写真のプレビューの URL (変換済みの JPEG を `data:` URL にする)。
pub fn photo_preview_url(bytes: &[u8]) -> String {
    format!("data:image/jpeg;base64,{}", base64_encode(bytes))
}

/// 通知 (スナックバー) を数秒後に消す。Web 以外では何もしない。
pub fn clear_notice_after(mut notice: Signal<Option<String>>) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;

        let callback = wasm_bindgen::closure::Closure::once_into_js(move || notice.set(None));
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.unchecked_ref(),
                3000,
            );
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = &mut notice;
    }
}

/// 幅 840 px 以上のときに true になる signal (WideLayout の切り替え)。
///
/// 0039 の [`crate::ui::WideLayout`] は CSS のメディアクエリでも 2 段組を隠すが、行を押した
/// ときに詳細の面へ出すか、詳細の経路へ移るかの判定には幅を知る必要がある。
pub fn use_wide_layout() -> Signal<bool> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;

        let mut wide = use_signal(wide_media_query_matches);
        use_hook(move || {
            if let Some(query) = wide_media_query() {
                let listener_query = query.clone();
                let callback = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
                    wide.set(listener_query.matches());
                });
                let _ = query
                    .add_event_listener_with_callback("change", callback.as_ref().unchecked_ref());
                // リスナーはアプリの生存期間だけ必要なので、意図的に解放しない。
                callback.forget();
            }
        });
        wide
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use_signal(|| false)
    }
}

/// 幅 840 px 以上のメディアクエリ (WideLayout の切り替え)。
#[cfg(target_arch = "wasm32")]
fn wide_media_query() -> Option<web_sys::MediaQueryList> {
    web_sys::window()?
        .match_media("(min-width: 840px)")
        .ok()
        .flatten()
}

/// 幅 840 px 以上か。
#[cfg(target_arch = "wasm32")]
fn wide_media_query_matches() -> bool {
    wide_media_query().is_some_and(|query| query.matches())
}
