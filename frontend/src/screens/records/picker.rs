//! 参照先 (購入、商品、店) を選ぶボトムシート (FR-9、FR-11)。
//!
//! 一覧はカーソル方式で、末尾までスクロールすると次のページを読む。シートは画面に数えない
//! (PRD の成功指標) ため、抽出の入力の購入の選択はこのシートで行う (docs/design/README.md)。
//! 行を押したときの処理は親が決め、シートは閉じる操作と参照を外す選択を持つ。

use dioxus::prelude::*;

use crate::i18n::{t, Key};
use crate::ui::{Button, ButtonVariant};

use super::{RecordListView, RecordLoader};

/// 参照先を選ぶボトムシート。
#[component]
pub fn RecordPickerSheet<T: Clone + PartialEq + 'static>(
    /// シートの題 (「購入を選ぶ」など。ARB から取る)。
    title: String,

    /// 一覧の 1 ページを読む。
    load: RecordLoader<T>,

    /// 行の組み立て。押したときの処理 (選択とシートを閉じる) は親が決める。
    row: Callback<T, Element>,

    /// 参照を外す選択肢の文言 (「店を指定しない」など)。無いときは置かない。
    #[props(default)]
    clear_label: Option<String>,

    /// 参照を外す選択の処理。
    #[props(default)]
    on_clear: EventHandler<MouseEvent>,

    /// 取り消しの処理。
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "sheet-scrim",
            onclick: move |_| on_close.call(()),
            div {
                class: "sheet",
                onclick: move |event| event.stop_propagation(),
                div { class: "ttl", "{title}" }
                RecordListView::<T> {
                    load,
                    row,
                    // 選択のシートは API の既定の並び順で、お気に入りの絞り込みと星を出さない
                    // (FR-20、FR-21)。
                    show_toolbar: false,
                }
                div { class: "acts",
                    if let Some(label) = clear_label {
                        Button {
                            label,
                            variant: ButtonVariant::Text,
                            onclick: move |event| on_clear.call(event),
                        }
                    }
                    Button {
                        label: t(Key::CancelButton).to_string(),
                        variant: ButtonVariant::Text,
                        onclick: move |_| on_close.call(()),
                    }
                }
            }
        }
    }
}
