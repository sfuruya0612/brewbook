//! 画面の題を左に、操作を右に置く帯 (0039)。
//!
//! docs/design/components/AppBar のガイドライン。高さ 56 px、地は `paper`、下端に `line` の
//! 罫線を引き、影は付けない。ホームだけ題を `wordmark` (IBM Plex Serif、24 px) にする。

use dioxus::prelude::*;

use crate::ui::Icon;

/// 画面の帯。先頭の操作は戻るか閉じるの 1 つ、末尾の操作は最大 2 つ。
#[component]
pub fn AppBar(
    /// 画面の題 (ARB の `*Title` から取る)。
    title: String,
    /// ホームだけ true にし、題を `wordmark` で組む。
    #[props(default = false)]
    wordmark: bool,
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
) -> Element {
    let title_class = if wordmark { "ttl wordmark" } else { "ttl" };
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
            {leading}
            div { class: title_class, "{title}" }
            {actions}
        }
    }
}
