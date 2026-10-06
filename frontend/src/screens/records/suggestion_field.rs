//! 自由記述の項目の入力欄 (FR-13)。
//!
//! 入力中にサジェスト API を呼び、過去の入力値を候補として最大 20 件表示する。候補の選択は
//! 任意で、候補に無い値もそのまま入力できる。候補を引けなくても入力は妨げない。
//! 一致した先頭部分は `crema-ink` の 600 で示す (docs/design/components/Field)。

use dioxus::prelude::*;

use crate::records::{RecordsApi, SuggestionState, SuggestionTarget};
use crate::ui::{Field, TextField};

/// 自由記述の項目の入力欄。値は親の signal に書き、候補の状態はこの画面が持つ。
#[component]
pub fn SuggestionField(
    /// 記録の API (FR-13)。
    api: RecordsApi,

    /// サジェストの対象の項目名。
    target: SuggestionTarget,

    /// 項目名 (ARB から取る)。
    label: String,

    /// 入力の値。親の signal を共有する。
    value: Signal<String>,

    /// プレースホルダー (ARB から取る)。
    #[props(default)]
    hint: Option<String>,

    /// 値が変わったときの動き (フォームの破棄の確認に使う。0054)。
    #[props(default)]
    on_change: EventHandler<()>,

    /// 無効か。
    #[props(default = false)]
    disabled: bool,
) -> Element {
    let mut state = use_signal(SuggestionState::new);

    let on_input = move |event: FormEvent| {
        let text = event.value();
        let sequence = state.write().begin(&text);
        value.set(text.clone());
        on_change.call(());
        let api = api.clone();
        spawn(async move {
            // 候補は入力の補助であり、引けなくても入力は続けられる (FR-13)。
            let values = api.suggestions(target, &text).await.unwrap_or_default();
            state.write().apply_options(sequence, values);
        });
    };

    let query = state.read().query().to_string();
    let open = state.read().is_open();
    let options = state.read().options().to_vec();

    rsx! {
        Field { label, disabled,
            TextField {
                value: value(),
                placeholder: hint,
                disabled,
                oninput: on_input,
            }
            if open && !options.is_empty() {
                div { class: "suggest",
                    for option in options {
                        {
                            let (prefix, rest) = crate::records::highlight_parts(&option, &query);
                            rsx! {
                                div {
                                    onclick: move |_| {
                                        value.set(state.write().select(&option));
                                        on_change.call(());
                                    },
                                    if !prefix.is_empty() {
                                        b { "{prefix}" }
                                    }
                                    "{rest}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
