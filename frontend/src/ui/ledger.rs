//! 詳細の項目名と値の表 (0039)。
//!
//! docs/design/components/Ledger のガイドライン。項目名は左に `label` (`ink-muted`)、値は
//! 右揃えの `mono`、行ごとに `line` の罫線を引く。未設定の項目は行を消さず「未設定」を
//! `ink-faint` で出す (ARB の `unsetLabel`)。

use dioxus::prelude::*;

use crate::i18n::{t, Key};

/// 項目名と値の表。行 ([`LedgerRow`]) を並べる。
#[component]
pub fn Ledger(
    /// 表の行。
    children: Element,
) -> Element {
    rsx! {
        div { class: "ledger", {children} }
    }
}

/// [`Ledger`] の 1 行。
#[component]
pub fn LedgerRow(
    /// 項目名 (ARB から取る)。
    label: String,
    /// 値。未設定のときは None。
    #[props(default)]
    value: Option<String>,
    /// 単位 (g、℃、秒、通貨コードなど)。caption の `ink-muted` で添える。
    #[props(default)]
    unit: Option<String>,
    /// 値を等幅で組むか。文字列の項目は false にする。
    #[props(default = true)]
    mono: bool,
    /// 値の代わりに置く操作 (写真の差し替えと削除など)。あるときは値を出さない。
    #[props(default)]
    trailing: Option<Element>,
) -> Element {
    let unset = value.as_deref().is_none_or(str::is_empty);
    let mut classes = vec!["v".to_string()];
    if !mono {
        classes.push("txt".to_string());
    }
    if unset {
        classes.push("unset".to_string());
    }
    let class = classes.join(" ");
    let value = value.filter(|value| !value.is_empty());
    let unit = unit.filter(|unit| !unit.is_empty());
    // `.k` と `.v` は `.ledger` の直接の子にする (原本のプレビューと同じ 2 列の grid に並べるため。
    // wrapper を挟むと grid の列に乗らない。0039 のレビューの指摘)。
    rsx! {
        div { class: "k", "{label}" }
        div { class,
            if trailing.is_some() {
                {trailing}
            } else if let Some(value) = value {
                "{value}"
                if let Some(unit) = unit {
                    span { class: "u", "{unit}" }
                }
            } else {
                "{t(Key::UnsetLabel)}"
            }
        }
    }
}
