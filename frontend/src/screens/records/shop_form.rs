//! 店の登録と編集の画面 (FR-6)。
//!
//! 店名は必須、住所は任意。編集では現在の値を読み込んでから上書きする。
//! 店名から住所の候補を検索でき、住所があるときは地図を出す (FR-22、ADR-0019)。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, Key};
use crate::records::{
    map_embed_url, record_error_key, save_target, validate_shop_form, PlaceCandidate, RecordError,
    RecordServices, RecordsApi, SaveTarget,
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
    // 住所の検索の状態 (FR-22)。
    let mut candidates = use_signal(Vec::<PlaceCandidate>::new);
    let mut searching = use_signal(|| false);
    let mut search_done = use_signal(|| false);
    let mut search_error = use_signal(|| None::<RecordError>);
    // 地図の状態 (FR-22)。キーは未設定のとき None のままにし、地図を出さない。
    let mut map_key = use_signal(|| None::<String>);
    let mut map_address = use_signal(String::new);

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
                    map_address.set(shop.address.clone().unwrap_or_default());
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

    // 地図の API キーを引く (FR-22)。引けなくてもフォームは動かす (地図を出さないだけ)。
    let config_services = services.clone();
    use_effect(move || {
        let api = RecordsApi::new(config_services.api.clone());
        spawn(async move {
            if let Ok(key) = api.maps_config().await {
                map_key.set(key);
            }
        });
    });

    // 店名から住所の候補を検索する (FR-22)。検索は操作でだけ行い、入力のたびには呼ばない。
    let search_services = services.clone();
    let search = EventHandler::new(move |_| {
        if searching() || busy() {
            return;
        }
        let query = name().trim().to_string();
        if query.is_empty() {
            return;
        }
        search_error.set(None);
        candidates.set(Vec::new());
        search_done.set(false);
        searching.set(true);
        let api = RecordsApi::new(search_services.api.clone());
        let lang = current_language().code();
        spawn(async move {
            match api.place_search(&query, lang).await {
                Ok(found) => {
                    candidates.set(found);
                    search_done.set(true);
                }
                Err(failure) => search_error.set(Some(failure)),
            }
            searching.set(false);
        });
    });
    let retry_search = EventHandler::new(move |_| search.call(()));

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
    let search_failure = search_error();
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
    // 住所の検索の候補 (FR-22)。選ぶと住所の欄に入り、店名の欄は変えない。
    let candidate_list = rsx! {
        div { class: "candidates",
            for candidate in candidates() {
                {
                    let candidate_address = candidate.address.clone();
                    rsx! {
                        button {
                            r#type: "button",
                            disabled: busy(),
                            onclick: move |_| {
                                address.set(candidate_address.clone());
                                map_address.set(candidate_address.clone());
                                candidates.set(Vec::new());
                                search_done.set(false);
                                dirty.set(true);
                            },
                            span { class: "n", "{candidate.name}" }
                            span { class: "s", "{candidate.address}" }
                        }
                    }
                }
            }
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
                        div { class: "field-actions",
                            Button {
                                label: t(Key::SearchAddressButton).to_string(),
                                variant: ButtonVariant::Secondary,
                                size: ButtonSize::Sm,
                                icon: Some("search".to_string()),
                                disabled: busy() || searching() || name().trim().is_empty(),
                                onclick: move |_| search.call(()),
                            }
                        }
                        if searching() {
                            div { class: "t-caption muted", "{t(Key::SearchingLabel)}" }
                        } else if !candidates().is_empty() {
                            {candidate_list}
                        } else if search_done() {
                            div { class: "t-caption muted", "{t(Key::AddressSearchEmpty)}" }
                        }
                        if let Some(failure) = search_failure {
                            {retryable_banner(&failure, retry_search)}
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
                                // 入力の確定 (入力欄から離れたとき) に地図を更新する (FR-22)。
                                onchange: move |event: FormEvent| {
                                    map_address.set(event.value());
                                },
                            }
                        }
                        if let Some(key) = map_key() {
                            if !map_address().trim().is_empty() {
                                div { class: "map",
                                    iframe {
                                        title: t(Key::MapTitle),
                                        src: map_embed_url(&key, &map_address(), current_language()),
                                        // `loading` は Dioxus の iframe の属性に無いため、
                                        // カスタム属性として渡す (遅延読み込み)。
                                        "loading": "lazy",
                                        referrerpolicy: "strict-origin-when-cross-origin",
                                    }
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
