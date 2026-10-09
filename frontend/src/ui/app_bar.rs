//! 画面の帯 (0039、0047、0054)。
//!
//! docs/design/components/AppBar のガイドライン。高さ 56 px、地は `paper`、下端に `line` の
//! 罫線を引き、影は付けない。画面の種類で 3 つの形を使い分ける: 最上位の画面はハンバーガー
//! メニュー、印、画面名。詳細は戻る、画面名、末尾の操作。フォームは閉じる、画面名、末尾の
//! 操作 (詳細とフォームにはメニューと印を出さない)。
//!
//! この部品は表示だけを担い、経路の操作は `menu` と `on_home` の prop で受け取る (0047)。

use dioxus::prelude::*;

use crate::i18n::{t, Key};
use crate::ui::{BrewbookMark, Icon};

/// 画面の帯。先頭の操作は戻るか閉じるの 1 つ、末尾の操作は最大 3 つ (詳細の お気に入り、編集、削除)。
#[component]
pub fn AppBar(
    /// 画面の題 (ARB の `*Title` から取る)。
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
    /// 末尾の操作 (アイコンと文字ボタンで最大 3 つ)。
    #[props(default)]
    actions: Option<Element>,
    /// ハンバーガーボタンとメニューの面 (最上位の画面だけ渡す。0047)。
    #[props(default)]
    menu: Option<Element>,
    /// アプリの印を出すか (最上位の画面だけ true)。
    #[props(default = false)]
    brand: bool,
    /// 印を押したときの動き。省略したときは押せない印にする。
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
    let brandmark = brand.then(|| {
        let mark = rsx! { BrewbookMark { size: 22 } };
        match on_home {
            Some(handler) => rsx! {
                button {
                    class: "brandmark",
                    r#type: "button",
                    "aria-label": "{t(Key::AppTitle)}",
                    onclick: move |event| handler.call(event),
                    {mark}
                }
            },
            None => rsx! {
                span { class: "brandmark", {mark} }
            },
        }
    });
    let actions = actions.map(|actions| {
        rsx! {
            div { class: "acts", {actions} }
        }
    });
    rsx! {
        header { class: "appbar",
            {menu}
            {leading}
            {brandmark}
            div { class: "ttl", "{title}" }
            {actions}
        }
    }
}
