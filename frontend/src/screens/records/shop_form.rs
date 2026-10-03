//! 店の登録と編集の画面 (FR-6)。
//!
//! 店名は必須、住所は任意。編集では現在の値を読み込んでから上書きする。アーカイブと
//! アーカイブ解除はヘッダー (ScreenAppBar) の右端から行う (FR-12)。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{t, Key};
use crate::records::{
    record_error_key, save_target, validate_shop_form, RecordError, RecordServices, RecordsApi,
    SaveTarget,
};
use crate::screens::ScreenAppBar;
use crate::ui::{Button, ButtonVariant, Field, TextField};

use super::{archive_button, clear_notice_after, mark_records_changed, retryable_banner};

/// 店の登録 (FR-6)。
#[component]
pub fn ShopFormScreen() -> Element {
    rsx! {
        ShopForm {}
    }
}

/// 店の編集 (FR-6)。
#[component]
pub fn ShopEditScreen(id: String) -> Element {
    rsx! {
        ShopForm { id: Some(id) }
    }
}

/// 店の登録と編集のフォーム。2 段組の右の面にも出せる。
#[component]
pub fn ShopForm(
    /// 編集する店の ID。新規の登録のときは None。
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
    let mut address = use_signal(String::new);
    let mut name_error = use_signal(|| None::<Key>);
    let mut save_error = use_signal(|| None::<RecordError>);
    let mut load_error = use_signal(|| None::<RecordError>);
    let mut loading = use_signal(|| id.is_some());
    let mut busy = use_signal(|| false);
    let mut archived = use_signal(|| false);

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
            match api.shop(&id).await {
                Ok(shop) => {
                    name.set(shop.name.clone());
                    address.set(shop.address.clone().unwrap_or_default());
                    archived.set(shop.is_archived());
                }
                Err(failure) => load_error.set(Some(failure)),
            }
            loading.set(false);
        });
    });
    use_effect(move || reload.call(()));

    let archive_services = services.clone();
    let archive_id = id.clone();
    let toggle_archive = EventHandler::new(move |_| {
        let Some(id) = archive_id.clone() else {
            return;
        };
        let api = RecordsApi::new(archive_services.api.clone());
        let next = !archived();
        spawn(async move {
            match api.set_shop_archived(&id, next).await {
                Ok(updated) => {
                    archived.set(updated.is_archived());
                    notice.set(Some(
                        t(if updated.is_archived() {
                            Key::ArchivedMessage
                        } else {
                            Key::UnarchivedMessage
                        })
                        .to_string(),
                    ));
                    mark_records_changed(&mut revision);
                }
                Err(failure) => notice.set(Some(t(record_error_key(&failure)).to_string())),
            }
        });
    });

    let save_services = services.clone();
    let save_id = id.clone();
    let save = EventHandler::new(move |_| {
        if busy() {
            return;
        }
        let input = match validate_shop_form(&name(), &address()) {
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
                SaveTarget::Update(id) => api.update_shop(&id, &input).await,
                SaveTarget::Create => api.create_shop(&input).await,
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
    // 保存は新規でも常に出す。アーカイブは編集 (id がある) のときだけ出す
    // (0041 のレビューの指摘。Flutter と同じ)。
    let actions = rsx! {
        if id.is_some() {
            {archive_button(archived(), busy(), EventHandler::new(move |_| toggle_archive.call(())))}
        }
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
                title: t(if id.is_some() { Key::ShopEditTitle } else { Key::ShopNewTitle }).to_string(),
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
                        Field {
                            label: t(Key::ShopNameLabel).to_string(),
                            required: true,
                            error: name_error().map(|key| t(key).to_string()),
                            disabled: busy(),
                            TextField {
                                value: name(),
                                disabled: busy(),
                                oninput: move |event: FormEvent| name.set(event.value()),
                            }
                        }
                        Field {
                            label: t(Key::AddressLabel).to_string(),
                            disabled: busy(),
                            TextField {
                                value: address(),
                                disabled: busy(),
                                oninput: move |event: FormEvent| address.set(event.value()),
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
