//! エラーのバナー、スナックバー、確認ダイアログ (0039)。
//!
//! docs/design/components/Feedback のガイドライン。色は `signal` と `ink` だけで、成功に
//! 色を足さない。文言は ARB から取ったものを渡す (FR-16)。

use dioxus::prelude::*;

use crate::i18n::{t, Key};
use crate::ui::{Button, ButtonVariant};

/// 画面に出すエラーのバナー。通信エラーと 401 以外の API エラーは「再試行」を右端に置く。
#[component]
pub fn Banner(
    /// 表示する文言 (ARB から取る)。
    message: String,
    /// 再試行の処理。渡さないときは再試行のボタンを出さない。
    #[props(default)]
    on_retry: Option<EventHandler<MouseEvent>>,
) -> Element {
    let retry = on_retry.map(|handler| {
        rsx! {
            button {
                class: "act",
                r#type: "button",
                onclick: move |event| handler.call(event),
                "{t(Key::RetryButton)}"
            }
        }
    });
    rsx! {
        div { class: "banner", role: "alert",
            div { class: "main", "{message}" }
            {retry}
        }
    }
}

/// 保存、アーカイブ、アーカイブ解除を知らせるスナックバー。取り消せる操作には「元に戻す」を
/// 右に付けてよい。画面に数えない (PRD の成功指標)。
#[component]
pub fn Snackbar(
    /// 表示する文言 (ARB から取る)。
    message: String,
    /// 取り消しなどの操作の文言 (ARB から取る)。無いときは出さない。
    #[props(default)]
    action_label: Option<String>,
    /// 操作の処理。
    #[props(default)]
    on_action: EventHandler<MouseEvent>,
) -> Element {
    let action = action_label.map(|label| {
        rsx! {
            button {
                class: "act",
                r#type: "button",
                onclick: move |event| on_action.call(event),
                "{label}"
            }
        }
    });
    rsx! {
        div { class: "snack", role: "status",
            span { "{message}" }
            {action}
        }
    }
}

/// 取り消せない操作の確認ダイアログ。確認のボタンだけを `signal` の塗りにし、キャンセルは
/// 文字ボタンにする (FR-15)。
#[component]
pub fn ConfirmDialog(
    /// 題 (ARB から取る)。
    title: String,
    /// 本文 (ARB から取る)。
    message: String,
    /// キャンセルの文言 (ARB から取る)。
    cancel_label: String,
    /// 肯定の文言 (ARB から取る)。
    confirm_label: String,
    /// 肯定が取り消せない操作か。true のとき `signal` の塗りにする。
    #[props(default = false)]
    danger: bool,
    /// キャンセルの処理。
    #[props(default)]
    on_cancel: EventHandler<MouseEvent>,
    /// 肯定の処理。
    #[props(default)]
    on_confirm: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "scrim",
            div { class: "dialog", role: "dialog", "aria-modal": "true", "aria-labelledby": "confirm-dialog-title",
                h2 { id: "confirm-dialog-title", "{title}" }
                p { "{message}" }
                div { class: "acts",
                    Button {
                        label: "{cancel_label}",
                        variant: ButtonVariant::Text,
                        onclick: move |event| on_cancel.call(event),
                    }
                    Button {
                        label: "{confirm_label}",
                        variant: if danger { ButtonVariant::Danger } else { ButtonVariant::Primary },
                        onclick: move |event| on_confirm.call(event),
                    }
                }
            }
        }
    }
}
