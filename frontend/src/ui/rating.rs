//! 評価 (0039)。
//!
//! docs/design/components/Rating のガイドライン。1 から 5 の評価を `crema` の丸で示す。
//! 星は使わない。一覧と詳細は 10 px の丸、入力は 28 px の丸を使う。評価は任意なので
//! 既定値を入れない (FR-11)。

use dioxus::prelude::Key as InputKey;
use dioxus::prelude::*;

use crate::i18n::{t, t_args, Key};

/// 丸の大きさ。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RatingSize {
    /// 一覧と詳細の 10 px の丸 (既定)。
    #[default]
    Small,
    /// 入力の 28 px の丸。
    Large,
}

/// 丸のクラス。点灯している丸に `on` を付ける。
fn dot_class(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        ""
    }
}

/// 読み上げ用の真偽の値。
fn bool_text(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

/// 点灯する丸の数。未評価 (None) は 0、5 を超える値は 5 に切り詰める。
pub fn lit_dots(value: Option<u8>) -> u8 {
    match value {
        Some(value) => value.min(5),
        None => 0,
    }
}

/// 評価の表示。`value` が無いときは 5 つとも `line` の丸にする。
#[component]
pub fn Rating(
    /// 評価 (1 から 5)。未評価のときは None。
    value: Option<u8>,
    /// 「4 / 5」を右に添えるか。
    #[props(default = false)]
    show_value: bool,
    /// 丸の大きさ。
    #[props(default)]
    size: RatingSize,
) -> Element {
    let lit = lit_dots(value);
    let class = if size == RatingSize::Large {
        "rating lg"
    } else {
        "rating"
    };
    let number = (show_value && lit > 0).then(|| {
        rsx! {
            span { class: "num", "{t_args(Key::RatingValue, &[(\"value\", &lit.to_string())])}" }
        }
    });
    rsx! {
        span { class,
            for index in 1..=5_u8 {
                i { class: dot_class(index <= lit) }
            }
            {number}
        }
    }
}

/// 評価の入力。押した丸までを塗り、右に値を示す。未評価のときは「未評価」を添える。
///
/// `role="radiogroup"` と `role="radio"` を付け、丸はフォーカスできてキーボードでも選べる
/// (Enter と Space で決定、左右の矢印で隣の値へ。WAI-ARIA の radio の要件。0039 のレビューの
/// 指摘)。`aria-checked` は選んだ値の丸だけ true にする (radio は 1 つだけ選ばれる)。
#[component]
pub fn RatingInput(
    /// 現在の評価 (1 から 5)。未評価のときは None。
    value: Option<u8>,
    /// 評価が変わったときの動き。同じ値を押しても選択を外さない。
    on_change: EventHandler<u8>,
    /// 押せるか。
    #[props(default = true)]
    enabled: bool,
) -> Element {
    let lit = lit_dots(value);
    let current = if lit > 0 {
        rsx! {
            span { class: "num", "{t_args(Key::RatingValue, &[(\"value\", &lit.to_string())])}" }
        }
    } else {
        rsx! {
            span { class: "num none", "{t(Key::RatingNone)}" }
        }
    };
    rsx! {
        span { class: "rating lg", role: "radiogroup",
            for index in 1..=5_u8 {
                i {
                    class: dot_class(index <= lit),
                    role: "radio",
                    tabindex: "0",
                    "aria-checked": bool_text(lit > 0 && index == lit),
                    "aria-label": "{t_args(Key::RatingValue, &[(\"value\", &index.to_string())])}",
                    onclick: move |_| {
                        if enabled {
                            on_change.call(index);
                        }
                    },
                    onkeydown: move |event| {
                        if !enabled {
                            return;
                        }
                        let next = match event.key() {
                            InputKey::Enter => Some(index),
                            InputKey::Character(ref value) if value == " " => Some(index),
                            InputKey::ArrowRight => Some(index.saturating_add(1).min(5)),
                            InputKey::ArrowLeft => Some(index.saturating_sub(1).max(1)),
                            _ => None,
                        };
                        if let Some(next) = next {
                            event.prevent_default();
                            on_change.call(next);
                        }
                    },
                }
            }
            {current}
        }
    }
}
