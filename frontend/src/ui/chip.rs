//! チップ (0039)。
//!
//! docs/design/components/Chip のガイドライン。高さ 32 px の丸い (`radius-full`) 選択肢。
//! 統計の期間の単一選択 (choice) と、フレーバーノートのタグ (tag)、追加の入口 (add) に使う。

use dioxus::prelude::*;

use crate::ui::Icon;

/// チップの種類。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ChipVariant {
    /// 期間などの単一選択。選択中だけ `roast` の塗り。
    #[default]
    Choice,
    /// フレーバーノートなどのタグ。`crema-soft` の地に削除アイコン。
    Tag,
    /// 末尾の破線の「タグを入力」。
    Add,
}

impl ChipVariant {
    /// CSS のクラス名 (docs/design/components/bundle.css と同じ)。
    fn class(self) -> &'static str {
        match self {
            ChipVariant::Choice => "",
            ChipVariant::Tag => "tag",
            ChipVariant::Add => "add",
        }
    }
}

/// チップ。文字は `label` (13 px の 500)。
#[component]
pub fn Chip(
    /// チップの文言 (ARB から取る)。
    label: String,
    /// 種類。既定は期間などの単一選択。
    #[props(default)]
    variant: ChipVariant,
    /// 選択中か。choice だけが使う。
    #[props(default = false)]
    selected: bool,
    /// 押したときの動き。
    #[props(default)]
    onclick: EventHandler<MouseEvent>,
    /// 削除の動き。tag だけが使う。無いときは削除アイコンを出さない。
    #[props(default)]
    on_remove: Option<EventHandler<MouseEvent>>,
) -> Element {
    let mut classes = vec!["chip".to_string()];
    if !variant.class().is_empty() {
        classes.push(variant.class().to_string());
    }
    if selected {
        classes.push("on".to_string());
    }
    let class = classes.join(" ");
    let remove = on_remove.map(|handler| {
        rsx! {
            button {
                class: "chip-remove",
                r#type: "button",
                "aria-label": "{label}",
                onclick: move |event| {
                    event.stop_propagation();
                    handler.call(event);
                },
                Icon { name: "close", muted: true }
            }
        }
    });
    if variant == ChipVariant::Tag {
        // タグは削除のボタンを内側に持つため、チップ自体はボタンにしない (button の入れ子を避ける)。
        rsx! {
            span { class,
                "{label}"
                {remove}
            }
        }
    } else {
        rsx! {
            button {
                class,
                r#type: "button",
                onclick: move |event| onclick.call(event),
                "{label}"
            }
        }
    }
}

/// 一覧と詳細の中の小さなタグ (高さ 22 px、文字 12 px)。削除アイコンは付けない。
#[component]
pub fn TagChip(
    /// タグの文言 (ARB から取る)。
    label: String,
) -> Element {
    rsx! {
        span { class: "chip tag mini", "{label}" }
    }
}
