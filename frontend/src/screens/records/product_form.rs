//! 商品の登録と編集の画面 (FR-7、FR-8)。
//!
//! 商品名は必須。Producer、Origin、Region、Process、Variety は自由記述で、入力中に過去の
//! 入力値の候補を出す (FR-13)。Flavor Notes はタグとして追加と削除ができ、更新では入力した
//! 配列で置き換える (FR-8)。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{t, Key};
use crate::records::{
    save_target, validate_product_form, RecordError, RecordServices, RecordsApi, SaveTarget,
    SuggestionTarget,
};
use crate::screens::ScreenAppBar;
use crate::ui::{Banner, Button, ButtonVariant, Chip, ChipVariant, Field, TextField};

use super::{clear_notice_after, mark_records_changed, retryable_banner, SuggestionField};

/// 商品の登録 (FR-7)。
#[component]
pub fn ProductFormScreen() -> Element {
    rsx! {
        ProductForm {}
    }
}

/// 商品の編集 (FR-7)。
#[component]
pub fn ProductEditScreen(id: String) -> Element {
    rsx! {
        ProductForm { id: Some(id) }
    }
}

/// 商品の登録と編集のフォーム。2 段組の右の面にも出せる。
#[component]
pub fn ProductForm(
    /// 編集する商品の ID。新規の登録のときは None。
    #[props(default)]
    id: Option<String>,

    /// 2 段組の右の面に出すか。
    #[props(default = false)]
    embedded: bool,

    /// 閉じる動き。無いときは前の画面へ戻る。
    #[props(default)]
    on_close: Option<EventHandler<()>>,

    /// 保存できたときの動き。無いときは前の画面へ戻る。
    #[props(default)]
    on_saved: Option<EventHandler<()>>,
) -> Element {
    let services = use_context::<RecordServices>();
    let mut revision = use_context::<Signal<u64>>();
    let mut notice = use_context::<Signal<Option<String>>>();
    let navigator = navigator();

    let mut name = use_signal(String::new);
    let mut producer = use_signal(String::new);
    let mut origin = use_signal(String::new);
    let mut region = use_signal(String::new);
    let mut process = use_signal(String::new);
    let mut variety = use_signal(String::new);
    let mut tags = use_signal(Vec::<String>::new);
    let mut tag_input = use_signal(String::new);
    let mut name_error = use_signal(|| None::<Key>);
    let mut save_error = use_signal(|| None::<RecordError>);
    let mut load_error = use_signal(|| None::<RecordError>);
    let mut loading = use_signal(|| id.is_some());
    let mut busy = use_signal(|| false);

    // 編集のために現在の値を読み込む。再試行でも同じ処理を呼ぶ。
    let reload_services = services.clone();
    let reload_id = id.clone();
    let reload = EventHandler::new(move |_| {
        let Some(id) = reload_id.clone() else {
            return;
        };
        let api = RecordsApi::new(reload_services.api.clone());
        spawn(async move {
            loading.set(true);
            load_error.set(None);
            match api.product(&id).await {
                Ok(product) => {
                    name.set(product.name.clone());
                    producer.set(product.producer.clone().unwrap_or_default());
                    origin.set(product.origin.clone().unwrap_or_default());
                    region.set(product.region.clone().unwrap_or_default());
                    process.set(product.process.clone().unwrap_or_default());
                    variety.set(product.variety.clone().unwrap_or_default());
                    tags.set(product.flavor_notes.clone());
                }
                Err(failure) => load_error.set(Some(failure)),
            }
            loading.set(false);
        });
    });
    use_effect(move || reload.call(()));

    let add_tag = EventHandler::new(move |_| {
        let value = tag_input().trim().to_string();
        if value.is_empty() {
            return;
        }
        let mut current = tags();
        if !current.contains(&value) {
            current.push(value);
            tags.set(current);
        }
        tag_input.set(String::new());
    });

    let save_services = services.clone();
    let save_id = id.clone();
    let save = EventHandler::new(move |_| {
        if busy() {
            return;
        }
        let input = match validate_product_form(
            &name(),
            &producer(),
            &origin(),
            &region(),
            &process(),
            &variety(),
            &tags(),
        ) {
            Ok(input) => input,
            Err(key) => {
                name_error.set(Some(key));
                save_error.set(None);
                return;
            }
        };
        name_error.set(None);
        save_error.set(None);
        busy.set(true);
        let api = RecordsApi::new(save_services.api.clone());
        let id = save_id.clone();
        spawn(async move {
            let result = match save_target(id.as_deref()) {
                SaveTarget::Update(id) => api.update_product(&id, &input).await,
                SaveTarget::Create => api.create_product(&input).await,
            };
            match result {
                Ok(_) => {
                    notice.set(Some(t(Key::SavedMessage).to_string()));
                    mark_records_changed(&mut revision);
                    match on_saved {
                        Some(handler) => handler.call(()),
                        None => navigator.go_back(),
                    }
                }
                Err(failure) => save_error.set(Some(failure)),
            }
            busy.set(false);
        });
    });

    let close = EventHandler::new(move |_| match on_close {
        Some(handler) => handler.call(()),
        None => navigator.go_back(),
    });
    let retry = EventHandler::new(move |_| reload.call(()));
    let retry_save = EventHandler::new(move |_| save.call(()));
    let failure = save_error();
    let load_failure = load_error();
    // 保存は新規でも常に出す (Flutter と同じ)。
    let actions = rsx! {
        Button {
            label: t(Key::SaveButton).to_string(),
            variant: ButtonVariant::Text,
            disabled: busy(),
            onclick: move |_| save.call(()),
        }
    };

    rsx! {
        div { class: "screen",
            ScreenAppBar {
                title: t(if id.is_some() { Key::ProductEditTitle } else { Key::ProductNewTitle }).to_string(),
                leading_icon: Some("close".to_string()),
                leading_label: Some(t(Key::CancelButton).to_string()),
                on_leading: move |_| close.call(()),
                actions,
            }
            div { class: "body",
                if loading() {
                    div { class: "empty", "{t(Key::Loading)}" }
                } else if let Some(failure) = load_failure {
                    div { class: "form",
                        {retryable_banner(&failure, retry)}
                    }
                } else {
                    div { class: "form",
                        if name_error().is_some() {
                            Banner { message: t(Key::ErrorValidation).to_string() }
                        }
                        Field {
                            label: t(Key::ProductNameLabel).to_string(),
                            required: true,
                            error: name_error().map(|key| t(key).to_string()),
                            disabled: busy(),
                            TextField {
                                value: name(),
                                disabled: busy(),
                                oninput: move |event: FormEvent| name.set(event.value()),
                            }
                        }
                        SuggestionField {
                            api: RecordsApi::new(services.api.clone()),
                            target: SuggestionTarget::Producer,
                            label: t(Key::Producer).to_string(),
                            value: producer,
                            disabled: busy(),
                        }
                        div { class: "grid2",
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Origin,
                                label: t(Key::Origin).to_string(),
                                value: origin,
                                disabled: busy(),
                            }
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Region,
                                label: t(Key::Region).to_string(),
                                value: region,
                                disabled: busy(),
                            }
                        }
                        div { class: "grid2",
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Process,
                                label: t(Key::Process).to_string(),
                                value: process,
                                disabled: busy(),
                            }
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Variety,
                                label: t(Key::Variety).to_string(),
                                value: variety,
                                disabled: busy(),
                            }
                        }
                        Field { label: t(Key::FlavorNotes).to_string(), disabled: busy(),
                            div { class: "chips",
                                for note in tags() {
                                    Chip {
                                        label: note.clone(),
                                        variant: ChipVariant::Tag,
                                        on_remove: Some(EventHandler::new(move |_| {
                                            let removed = note.clone();
                                            tags.set(
                                                tags().into_iter().filter(|tag| tag != &removed).collect(),
                                            );
                                        })),
                                    }
                                }
                            }
                            div { class: "tag-add",
                                TextField {
                                    value: tag_input(),
                                    placeholder: Some(t(Key::TagInputHint).to_string()),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| tag_input.set(event.value()),
                                }
                                Button {
                                    label: t(Key::AddButton).to_string(),
                                    variant: ButtonVariant::Secondary,
                                    size: crate::ui::ButtonSize::Sm,
                                    disabled: busy(),
                                    onclick: move |_| add_tag.call(()),
                                }
                            }
                        }
                        if let Some(failure) = failure {
                            {retryable_banner(&failure, retry_save)}
                        }
                    }
                }
            }
        }
        if let Some(message) = notice() {
            {clear_notice_after(notice)}
            div { class: "notice",
                crate::ui::Snackbar { message }
            }
        }
    }
}
