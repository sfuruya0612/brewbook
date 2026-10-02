//! ボタン (0039)。
//!
//! docs/design/components/Button のガイドライン。primary は 1 画面に 1 つ。高さは 48 px、
//! 小 (sm) は 36 px、文字は 15 px の 500。無効化は `paper-sunken` の地に `ink-faint` の文字。

use dioxus::prelude::*;

use crate::ui::Icon;

/// ボタンの種類。docs/design/components/Button の primary、secondary、text、danger、
/// danger-text、danger-outline に対応する。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonVariant {
    /// `roast` の塗り。フォームの「保存」、ログイン、登録、確認の肯定。
    #[default]
    Primary,
    /// 透明の地に `line-strong` の枠。画面内の補助操作。
    Secondary,
    /// `crema-ink` の文字だけ。AppBar の「保存」、ダイアログの「キャンセル」、「再試行」。
    Text,
    /// `signal` の塗り。確認ダイアログの「削除する」だけ。
    Danger,
    /// `signal` の文字だけの取り消せない操作。
    DangerText,
    /// `signal` の枠と文字。設定画面の入口。
    DangerOutline,
}

impl ButtonVariant {
    /// CSS のクラス名 (docs/design/components/bundle.css と同じ)。
    fn class(self) -> &'static str {
        match self {
            ButtonVariant::Primary => "primary",
            ButtonVariant::Secondary => "secondary",
            ButtonVariant::Text => "text",
            ButtonVariant::Danger => "danger",
            ButtonVariant::DangerText => "danger-text",
            ButtonVariant::DangerOutline => "danger-outline",
        }
    }
}

/// ボタンの大きさ。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonSize {
    /// 48 px (既定)。
    #[default]
    Md,
    /// 36 px (sm)。
    Sm,
}

/// ボタン。`label` は ARB から取る (FR-16)。
#[component]
pub fn Button(
    /// ボタンの文言 (ARB から取る)。
    label: String,
    /// 種類。既定は primary。
    #[props(default)]
    variant: ButtonVariant,
    /// 大きさ。既定は 48 px。
    #[props(default)]
    size: ButtonSize,
    /// 幅いっぱいに広げるか。
    #[props(default = false)]
    block: bool,
    /// 押せないか。処理中も文言を変えず無効化だけする。
    #[props(default = false)]
    disabled: bool,
    /// キーボードのフォーカスを表すか (プレビューの `focus` と同じ)。
    #[props(default = false)]
    focused: bool,
    /// 左に置くアイコン (Material Icons のリガチャ)。
    #[props(default)]
    icon: Option<String>,
    /// 押したときの動き。
    #[props(default)]
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let mut classes = vec!["btn".to_string(), variant.class().to_string()];
    if size == ButtonSize::Sm {
        classes.push("sm".to_string());
    }
    if block {
        classes.push("block".to_string());
    }
    if disabled {
        classes.push("disabled".to_string());
    }
    if focused {
        classes.push("focus".to_string());
    }
    let class = classes.join(" ");
    let aria_disabled = if disabled { "true" } else { "false" };
    let icon = icon.map(|name| rsx! { Icon { name } });
    rsx! {
        button {
            class,
            r#type: "button",
            disabled,
            "aria-disabled": aria_disabled,
            onclick: move |event| onclick.call(event),
            {icon}
            "{label}"
        }
    }
}

/// 拡張 FAB。一覧の右下に 1 つだけ置く (docs/design/components/Button)。
#[component]
pub fn Fab(
    /// ボタンの文言 (動詞。ARB から取る)。
    label: String,
    /// 左に置くアイコン。
    #[props(default)]
    icon: Option<String>,
    /// 押したときの動き。
    #[props(default)]
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let icon = icon.map(|name| rsx! { Icon { name } });
    rsx! {
        button {
            class: "fab",
            r#type: "button",
            onclick: move |event| onclick.call(event),
            {icon}
            "{label}"
        }
    }
}

/// アイコンだけのボタン (AppBar の末尾の操作など)。
#[component]
pub fn IconButton(
    /// アイコンの名前 (Material Icons のリガチャ)。
    name: String,
    /// ボタンの名前 (読み上げ用。ARB から取る)。
    label: String,
    /// 押したときの動き。
    #[props(default)]
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            class: "iconbtn",
            r#type: "button",
            "aria-label": "{label}",
            onclick: move |event| onclick.call(event),
            Icon { name }
        }
    }
}
