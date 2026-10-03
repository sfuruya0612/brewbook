//! 画面の帯 (0039、0047)。
//!
//! docs/design/components/AppBar のガイドライン。高さ 56 px、地は `paper`、下端に `line` の
//! 罫線を引き、影は付けない。先頭からハンバーガーボタン (`menu`。認証後の画面だけ)、
//! 既存の先頭の操作、アプリの印とアプリ名、その下の画面名、末尾の操作の順に置く。
//!
//! この部品は表示だけを担い、経路の操作は `menu` と `on_home` の prop で受け取る (0047)。

use dioxus::prelude::*;

use crate::i18n::{t, Key};
use crate::ui::{BrewbookMark, Icon};

/// 画面の帯。先頭の操作は戻るか閉じるの 1 つ、末尾の操作は最大 2 つ。
#[component]
pub fn AppBar(
    /// 画面の題 (ARB の `*Title` から取る)。アプリ名の下に出す。
    title: String,
    /// 先頭の操作のアイコン (戻るは `arrow_back_ios_new`、フォームは `close`)。
    #[props(default)]
    leading_icon: Option<String>,
    /// 先頭の操作の名前 (読み上げ用。ARB から取る)。
    #[props(default)]
    leading_label: Option<String>,
    /// 先頭の操作を押したときの動き。
    #[props(default)]
    on_leading: EventHandler<MouseEvent>,
    /// 末尾の操作 (アイコンと文字ボタンで最大 2 つ)。
    #[props(default)]
    actions: Option<Element>,
    /// ハンバーガーボタンとメニューの面 (認証後の画面だけ渡す。0047)。
    #[props(default)]
    menu: Option<Element>,
    /// アプリ名を押したときの動き。省略したときは何もしない。
    #[props(default)]
    on_home: Option<EventHandler<MouseEvent>>,
) -> Element {
    let leading = leading_icon.map(|name| {
        let label = leading_label.clone().unwrap_or_default();
        rsx! {
            button {
                class: "lead",
                r#type: "button",
                "aria-label": "{label}",
                onclick: move |event| on_leading.call(event),
                Icon { name }
            }
        }
    });
    rsx! {
        header { class: "appbar",
            {menu}
            {leading}
            div { class: "brand",
                button {
                    class: "app-name",
                    r#type: "button",
                    "aria-label": "{t(Key::AppTitle)}",
                    onclick: move |event| {
                        if let Some(handler) = on_home {
                            handler.call(event);
                        }
                    },
                    BrewbookMark { size: 32 }
                    span { class: "name", "{t(Key::AppTitle)}" }
                }
                div { class: "ttl", "{title}" }
            }
            {actions}
        }
    }
}
