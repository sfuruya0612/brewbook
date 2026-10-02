//! 幅 840 px 以上の 2 段組のレイアウト (0039、0042)。
//!
//! docs/design/components/WideLayout のガイドライン。ナビゲーションレール (88 px) と一覧
//! (400 px) と詳細 (最大 720 px) を並べる。幅 840 px 未満ではレールと詳細を隠し、1 列に戻る
//! (判定は design.css のメディアクエリ。Flutter の `LayoutBuilder` に対応する)。
//!
//! 一覧と詳細に分かれない画面 (統計) は [`WidePage`] を使い、レールの右を内容が使う。

use dioxus::prelude::*;

use crate::ui::{BrewbookMark, Icon};

/// レールの項目のクラス。選択中は `on` を付ける。
fn item_class(selected: bool) -> &'static str {
    if selected {
        "item on"
    } else {
        "item"
    }
}

/// レールの 1 項目。
#[derive(Clone, PartialEq)]
pub struct RailItem {
    /// 項目の名前 (ARB から取る)。
    pub label: String,
    /// アイコンの名前 (Material Icons のリガチャ)。
    pub icon: String,
    /// 選択中か。`roast-soft` の地に `ink` になる。
    pub selected: bool,
    /// 押したときの動き。
    pub on_click: EventHandler<MouseEvent>,
}

/// ナビゲーションレール (88 px)。上に印、その下に抽出、購入、商品、店、統計、設定。
#[component]
pub fn NavigationRail(
    /// レールの項目。選択中は 1 つだけにする。
    items: Vec<RailItem>,
) -> Element {
    rsx! {
        nav { class: "rail",
            BrewbookMark {}
            for item in items {
                button {
                    class: item_class(item.selected),
                    r#type: "button",
                    onclick: move |event| item.on_click.call(event),
                    Icon { name: item.icon.clone(), muted: !item.selected }
                    span { "{item.label}" }
                }
            }
        }
    }
}

/// 幅 840 px 以上の配置。レール、一覧 (400 px)、詳細 (最大 720 px) の 2 段組にする。
#[component]
pub fn WideLayout(
    /// ナビゲーションレール ([`NavigationRail`])。
    rail: Element,
    /// 左の一覧の面 (幅 400 px)。
    list: Element,
    /// 右の詳細の面。無いときは空にする。
    #[props(default)]
    detail: Option<Element>,
) -> Element {
    rsx! {
        div { class: "wide-layout",
            {rail}
            div { class: "wide-list", {list} }
            div { class: "wide-detail",
                div { class: "wide-detail-inner", {detail} }
            }
        }
    }
}

/// 幅 840 px 以上の 1 面の配置。レールと、残りの幅いっぱいの内容を並べる。
///
/// 一覧と詳細に分かれない画面 (統計) に使う (Flutter の `BrewbookNavigationRail` と
/// `Expanded` の組み合わせと同じ)。幅 840 px 未満ではレールを隠す。
#[component]
pub fn WidePage(
    /// ナビゲーションレール ([`NavigationRail`])。
    rail: Element,
    /// 残りの幅いっぱいに置く内容。
    children: Element,
) -> Element {
    rsx! {
        div { class: "wide-page",
            {rail}
            div { class: "wide-page-main", {children} }
        }
    }
}
