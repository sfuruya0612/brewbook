//! 入力欄 (0039)。
//!
//! docs/design/components/Field のガイドライン。項目名を上に `label` (`ink-muted`) で置き、
//! 48 px の枠 (`paper-sunken` の地、`line-strong` の枠、`radius-sm`) に値を入れる。浮動ラベルは
//! 使わない。数値と日付は `mono` で組み、単位を `caption` で右端に添える。日付と時刻は
//! ブラウザ標準の date input と time input で選ぶ ([`TextFieldKind`])。

use dioxus::prelude::*;

use crate::i18n::{t, Key};
use crate::ui::Icon;

/// [`TextField`] が描く入力の種類。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TextFieldKind {
    /// 文字の入力 (`input { r#type: "text" }`)。
    #[default]
    Text,
    /// ブラウザ標準の日付の選択 (`input { r#type: "date" }`)。値は `YYYY-MM-DD`。
    Date,
    /// ブラウザ標準の時刻の選択 (`input { r#type: "time" }`)。値は `HH:MM`。
    Time,
}

/// 項目名と、枠と、理由 (誤りか助け) を縦に並べる。
#[component]
pub fn Field(
    /// 項目名 (ARB から取る)。
    label: String,
    /// 必須の項目か。「必須」を項目名の横に 400 で添える。
    #[props(default = false)]
    required: bool,
    /// 検証の誤り (ARB から取る)。あるときは枠を `signal` にし、理由を下に書く。
    #[props(default)]
    error: Option<String>,
    /// 入力の助け (ARB から取る)。誤りがあるときは出さない。
    #[props(default)]
    help: Option<String>,
    /// フォーカスを表すか。
    #[props(default = false)]
    focused: bool,
    /// 無効か。
    #[props(default = false)]
    disabled: bool,
    /// 枠の中身 ([`TextField`] など)。
    children: Element,
) -> Element {
    let mut classes = vec!["field".to_string()];
    if focused {
        classes.push("focus".to_string());
    }
    if error.is_some() {
        classes.push("error".to_string());
    }
    if disabled {
        classes.push("disabled".to_string());
    }
    let class = classes.join(" ");
    let message = error.clone().or_else(|| help.clone());
    rsx! {
        div { class,
            div { class: "lbl",
                "{label}"
                if required {
                    span { class: "req", "{t(Key::RequiredLabel)}" }
                }
            }
            {children}
            if let Some(message) = message {
                div { class: "help", "{message}" }
            }
        }
    }
}

/// [`Field`] の中に置く 1 行の入力欄。複数行 (area) にもできる。
#[component]
pub fn TextField(
    /// 値。
    value: String,
    /// 入力が変わったときの動き。
    #[props(default)]
    oninput: EventHandler<FormEvent>,
    /// 値が確定したときの動き (入力欄から離れたときなど)。住所の地図の更新に使う (FR-22)。
    #[props(default)]
    onchange: EventHandler<FormEvent>,
    /// 値を等幅で組むか (数値と日付は true)。
    #[props(default = false)]
    mono: bool,
    /// 入力の種類。日付と時刻はブラウザ標準のピッカーで選ぶ (既定は文字の入力)。
    #[props(default)]
    kind: TextFieldKind,
    /// 複数行の入力欄にするか (感想など)。
    #[props(default = false)]
    area: bool,
    /// プレースホルダー (ARB から取る)。
    #[props(default)]
    placeholder: Option<String>,
    /// 右端に添える単位 (g、℃、秒)。無いときは添えない。
    #[props(default)]
    unit: Option<String>,
    /// 右端に置くアイコン (カレンダー、時計)。
    #[props(default)]
    icon: Option<String>,
    /// 無効か。
    #[props(default = false)]
    disabled: bool,
) -> Element {
    let input_class = if mono { "in num" } else { "in" };
    let box_class = if area { "box area" } else { "box" };
    let input_type = match kind {
        TextFieldKind::Text => "text",
        TextFieldKind::Date => "date",
        TextFieldKind::Time => "time",
    };
    let placeholder = placeholder.unwrap_or_default();
    let on_input = move |event: FormEvent| oninput.call(event);
    let on_change = move |event: FormEvent| onchange.call(event);
    let unit = unit.map(|unit| rsx! { span { class: "unit", "{unit}" } });
    let icon = icon.map(|name| rsx! { Icon { name, muted: true } });
    rsx! {
        div { class: box_class,
            if area {
                textarea {
                    class: "{input_class}",
                    value: "{value}",
                    placeholder: "{placeholder}",
                    disabled,
                    oninput: on_input,
                    onchange: on_change,
                }
            } else {
                input {
                    class: "{input_class}",
                    r#type: input_type,
                    value: "{value}",
                    placeholder: "{placeholder}",
                    disabled,
                    oninput: on_input,
                    onchange: on_change,
                }
            }
            {unit}
            {icon}
        }
    }
}
