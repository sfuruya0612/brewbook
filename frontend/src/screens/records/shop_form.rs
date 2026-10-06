//! 店の登録と編集の画面 (FR-6)。
//!
//! 店名は必須、住所は任意。編集では現在の値を読み込んでから上書きする。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{t, Key};
use crate::records::{
    record_error_key, save_target, validate_shop_form, RecordError, RecordServices, RecordsApi,
    SaveTarget,
};
use crate::screens::ScreenAppBar;
use crate::ui::{Button, ButtonSize, ButtonVariant, Field, IconButton, TextField};

use super::{clear_notice_after, mark_records_changed, retryable_banner, DiscardConfirm};

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
    let mut favorited_at = use_signal(|| None::<String>);
    let mut loading = use_signal(|| id.is_some());
    let mut busy = use_signal(|| false);
    let mut dirty = use_signal(|| false);
    let mut discard_open = use_signal(|| false);

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
                    favorited_at.set(shop.favorited_at.clone());
                }
                Err(failure) => load_error.set(Some(failure)),
            }
            loading.set(false);
            // 読み込みで入った値は変更に数えない (0054)。
            dirty.set(false);
        });
    });
    use_effect(move || reload.call(()));

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

    // 変更があるときは、閉じる前に破棄の確認を出す (0054)。
    let close_now = EventHandler::new(move |_| match on_close {
        Some(handler) => handler.call(()),
        None => navigator.go_back(),
    });
    let request_close = EventHandler::new(move |_| {
        if dirty() {
            discard_open.set(true);
        } else {
            close_now.call(());
        }
    });
    let confirm_discard = EventHandler::new(move |_| close_now.call(()));
    let retry = EventHandler::new(move |_| reload.call(()));
    let retry_save = EventHandler::new(move |_| save.call(()));
    let failure = save_error();
    let load_failure = load_error();
    // 星は編集の画面 (id があるとき) にだけ置く (新規の登録では ID が無い。FR-21)。
    let favorite_services = services.clone();
    let favorite_id = id.clone();
    let mut favorite_revision = revision;
    let mut favorite_notice = notice;
    let toggle_favorite = EventHandler::new(move |_| {
        let Some(favorite_id) = favorite_id.clone() else {
            return;
        };
        let api = RecordsApi::new(favorite_services.api.clone());
        let favorited = favorited_at().is_some();
        spawn(async move {
            match api.set_shop_favorite(&favorite_id, !favorited).await {
                Ok(shop) => {
                    favorited_at.set(shop.favorited_at.clone());
                    mark_records_changed(&mut favorite_revision);
                }
                Err(failure) => {
                    favorite_notice.set(Some(t(record_error_key(&failure)).to_string()))
                }
            }
        });
    });
    let favorited = favorited_at().is_some();
    let favorite_name = if favorited { "star" } else { "star_border" }.to_string();
    let favorite_label = t(if favorited {
        Key::FavoriteRemoveLabel
    } else {
        Key::FavoriteAddLabel
    })
    .to_string();
    // 保存は新規でも常に出す (Flutter と同じ)。
    let actions = rsx! {
        if id.is_some() && !loading() && load_failure.is_none() {
            IconButton {
                name: favorite_name,
                label: favorite_label,
                onclick: move |_| toggle_favorite.call(()),
            }
        }
        Button {
            label: t(Key::SaveButton).to_string(),
            variant: ButtonVariant::Primary,
            size: ButtonSize::Sm,
            disabled: busy(),
            onclick: move |_| save.call(()),
        }
    };

    rsx! {
        div { class: "screen",
            ScreenAppBar {
                title: t(if id.is_some() { Key::ShopEditTitle } else { Key::ShopNewTitle }).to_string(),
                menu: false,
                leading_icon: Some("close".to_string()),
                leading_label: Some(t(Key::CancelButton).to_string()),
                on_leading: move |_| request_close.call(()),
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
                                oninput: move |event: FormEvent| {
                                    name.set(event.value());
                                    dirty.set(true);
                                },
                            }
                        }
                        Field {
                            label: t(Key::AddressLabel).to_string(),
                            disabled: busy(),
                            TextField {
                                value: address(),
                                disabled: busy(),
                                oninput: move |event: FormEvent| {
                                    address.set(event.value());
                                    dirty.set(true);
                                },
                            }
                        }
                        if let Some(failure) = failure {
                            {retryable_banner(&failure, retry_save)}
                        }
                    }
                }
            }
        }
        if discard_open() {
            DiscardConfirm { open: discard_open, on_discard: confirm_discard }
        }
        if let Some(message) = notice() {
            {clear_notice_after(notice)}
            div { class: "notice",
                crate::ui::Snackbar { message }
            }
        }
    }
}
