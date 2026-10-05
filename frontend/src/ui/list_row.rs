//! 一覧の行 (0039)。
//!
//! docs/design/components/ListRow のガイドライン。1 行目は名前 (`heading`)、2 行目は日付
//! (`mono`) と参照先 (`caption`、`ink-muted`)、右端は数値か評価。高さは 64 px 以上、下端に
//! `line` の罫線を引く。行の地は `paper`。広い画面で選択中の行は `roast-soft` にする。

use dioxus::prelude::*;

use crate::i18n::{t, Key};
use crate::ui::{is_activation_key, Icon, IconButton};

/// 一覧の行。
#[component]
pub fn ListRow(
    /// 1 行目の名前。
    title: String,
    /// 行の先頭 (44 px の写真など)。無いときは置かない。
    #[props(default)]
    leading: Option<Element>,
    /// 2 行目。日付は [`RowValue`]、参照先は caption で組む。
    #[props(default)]
    subtitle: Option<Element>,
    /// 右端の数値か評価。
    #[props(default)]
    trailing: Option<Element>,
    /// 2 行目の下のタグ (商品の行だけ)。
    #[props(default)]
    tags: Option<Element>,
    /// お気に入りか。星の見た目を変える (FR-21)。
    #[props(default = false)]
    favorited: bool,
    /// 星を押したときの動き。無いときは星を置かない (選択のシートなど)。
    #[props(default)]
    on_favorite: Option<EventHandler<()>>,
    /// 広い画面で選択中か。地を `roast-soft` にする。
    #[props(default = false)]
    selected: bool,
    /// 押したときの動き。無いときは押せない。
    #[props(default)]
    on_click: Option<EventHandler<()>>,
) -> Element {
    let mut classes = vec!["row".to_string()];
    if selected {
        classes.push("selected".to_string());
    }
    let class = classes.join(" ");
    let interactive = on_click.is_some();
    let favorite = on_favorite.map(|handler| {
        let class = if favorited { "fav on" } else { "fav" };
        let name = if favorited { "star" } else { "star_border" }.to_string();
        let label = t(if favorited {
            Key::FavoriteRemoveLabel
        } else {
            Key::FavoriteAddLabel
        })
        .to_string();
        rsx! {
            div { class,
                IconButton {
                    name,
                    label,
                    stop_propagation: true,
                    onclick: move |_| handler.call(()),
                }
            }
        }
    });
    rsx! {
        div {
            class,
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
            {leading}
            div { class: "main",
                div { class: "name", "{title}" }
                if subtitle.is_some() {
                    div { class: "sub", {subtitle} }
                }
                if tags.is_some() {
                    div { class: "tags", {tags} }
                }
            }
            if trailing.is_some() {
                div { class: "side", {trailing} }
            }
            {favorite}
        }
    }
}

/// 一覧の行の 2 行目の日付など、等幅で組む値。
#[component]
pub fn RowValue(
    /// 値。
    text: String,
) -> Element {
    rsx! {
        span { class: "v", "{text}" }
    }
}

/// 44 px の写真の枠。写真が無いときはカメラの印を出す。
#[component]
pub fn ListThumb(
    /// 写真の URL。無いときはカメラの印にする。
    #[props(default)]
    src: Option<String>,
    /// 写真の説明 (読み上げ用。ARB から取る)。
    #[props(default)]
    alt: String,
) -> Element {
    let content = match src {
        Some(src) => rsx! { img { src, alt: "{alt}" } },
        None => rsx! { Icon { name: "photo_camera", muted: true } },
    };
    rsx! {
        div { class: "thumb", {content} }
    }
}
