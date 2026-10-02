//! 参照先のタイル (0039)。
//!
//! docs/design/components/ReferenceTile のガイドライン。`roast-soft` の地に種類 (`caption`)
//! と名前 (`heading`) を置き、右端に `chevron_right` を付ける。押すとその記録の詳細へ移る
//! (UC-6)。フォームでは同じ形のピッカーを使う。

use dioxus::prelude::*;

use crate::ui::{is_activation_key, Icon};

/// 参照先 (購入、商品、店) をたどるタイル。
#[component]
pub fn ReferenceTile(
    /// 種類 (「購入」「商品」「店」。ARB から取る)。
    kind: String,
    /// たどる先の名前。
    name: String,
    /// 押したときの動き。無いときは押せない。
    #[props(default)]
    on_click: Option<EventHandler<()>>,
) -> Element {
    let interactive = on_click.is_some();
    rsx! {
        div {
            class: "tile",
            role: interactive.then_some("button"),
            tabindex: interactive.then_some("0"),
            onclick: move |_| {
                if let Some(handler) = on_click {
                    handler.call(());
                }
            },
            onkeydown: move |event| {
                if let Some(handler) = on_click {
                    if is_activation_key(&event) {
                        event.prevent_default();
                        handler.call(());
                    }
                }
            },
            div { class: "main",
                div { class: "k", "{kind}" }
                div { class: "n", "{name}" }
            }
            Icon { name: "chevron_right", muted: true }
        }
    }
}

/// 参照先のタイルを縦に並べる連鎖。タイルの間に 12 px の線を置く。
#[component]
pub fn ReferenceChain(
    /// たどる順のタイル。
    children: Element,
) -> Element {
    rsx! {
        div { class: "chain", {children} }
    }
}

/// フォームで参照先 (購入、商品、店) を選ぶピッカー。
#[component]
pub fn PickerTile(
    /// 種類 (「購入」「商品」「店」。ARB から取る)。
    kind: String,
    /// 選択中の名前。未選択のときは None。
    #[props(default)]
    name: Option<String>,
    /// 未選択のときの文言 (ARB から取る)。
    #[props(default)]
    placeholder: String,
    /// 押したときの動き。
    #[props(default)]
    on_click: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "picker",
            role: "button",
            tabindex: "0",
            onclick: move |_| on_click.call(()),
            onkeydown: move |event| {
                if is_activation_key(&event) {
                    event.prevent_default();
                    on_click.call(());
                }
            },
            div { class: "main",
                div { class: "k", "{kind}" }
                div { class: "n",
                    if let Some(name) = name {
                        "{name}"
                    } else {
                        span { class: "faint", "{placeholder}" }
                    }
                }
            }
            Icon { name: "chevron_right", muted: true }
        }
    }
}
