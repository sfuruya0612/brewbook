//! 商品の登録と編集の画面 (FR-7、FR-8)。
//!
//! 商品名は必須。Producer、Origin、Region、Process、Variety は自由記述で、入力中に過去の
//! 入力値の候補を出す (FR-13)。Flavor Notes はタグとして追加と削除ができ、更新では入力した
//! 配列で置き換える (FR-8)。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{t, t_args, Key};
use crate::records::{
    record_error_key, record_error_not_found, save_target, validate_product_form, DeleteImpact,
    RecordError, RecordServices, RecordsApi, SaveTarget, SuggestionTarget,
};
use crate::screens::ScreenAppBar;
use crate::ui::{
    Banner, Button, ButtonSize, ButtonVariant, Chip, ChipVariant, Field, IconButton, TextField,
};

use super::{
    clear_notice_after, mark_records_changed, retryable_banner, DeleteConfirm, DiscardConfirm,
    SuggestionField,
};

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
    let mut favorited_at = use_signal(|| None::<String>);
    let mut loading = use_signal(|| id.is_some());
    let mut busy = use_signal(|| false);
    let mut dirty = use_signal(|| false);
    let mut discard_open = use_signal(|| false);
    // 削除の確認と、削除の影響と失敗の再試行 (0056)。
    let mut delete_open = use_signal(|| false);
    let mut delete_impact = use_signal(|| None::<DeleteImpact>);
    let mut impact_error = use_signal(|| None::<RecordError>);
    let mut delete_error = use_signal(|| None::<RecordError>);

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
                    favorited_at.set(product.favorited_at.clone());
                }
                Err(failure) => load_error.set(Some(failure)),
            }
            loading.set(false);
            // 読み込みで入った値は変更に数えない (0054)。
            dirty.set(false);
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
            dirty.set(true);
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
    let impact_failure = impact_error();
    let delete_failure = delete_error();
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
            match api.set_product_favorite(&favorite_id, !favorited).await {
                Ok(product) => {
                    favorited_at.set(product.favorited_at.clone());
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
    // 削除 (0056)。削除の影響の件数を引いて確認を出し、失敗は再試行のバナーで知らせる。
    let request_services = services.clone();
    let request_id = id.clone();
    let mut request_revision = revision;
    let mut request_notice = notice;
    let request_delete = EventHandler::new(move |_| {
        let Some(id) = request_id.clone() else {
            return;
        };
        let api = RecordsApi::new(request_services.api.clone());
        spawn(async move {
            match api.product_delete_impact(&id).await {
                Ok(impact) => {
                    delete_impact.set(Some(impact));
                    delete_open.set(true);
                }
                // 記録が既に無い場合は成功と同じ扱いにする (0056)。
                Err(failure) if record_error_not_found(&failure) => {
                    mark_records_changed(&mut request_revision);
                    request_notice.set(Some(t(Key::RecordDeletedMessage).to_string()));
                    close_now.call(());
                }
                Err(failure) => impact_error.set(Some(failure)),
            }
        });
    });
    let delete_services = services.clone();
    let delete_id = id.clone();
    let mut delete_revision = revision;
    let mut delete_notice = notice;
    let delete_now = EventHandler::new(move |_| {
        let Some(id) = delete_id.clone() else {
            return;
        };
        let api = RecordsApi::new(delete_services.api.clone());
        spawn(async move {
            match api.delete_product(&id).await {
                Ok(()) => {}
                // 記録が既に無い場合は成功と同じ扱いにする (0056)。
                Err(failure) if record_error_not_found(&failure) => {}
                Err(failure) => {
                    delete_error.set(Some(failure));
                    return;
                }
            }
            mark_records_changed(&mut delete_revision);
            delete_notice.set(Some(t(Key::RecordDeletedMessage).to_string()));
            // 削除の後は破棄の確認を経ずに閉じる (0056)。
            close_now.call(());
        });
    });
    let delete_message = delete_impact().map(|impact| match (impact.purchases, impact.brews) {
        (0, _) => t(Key::DeleteProductConfirmMessage).to_string(),
        (purchases, 0) => t_args(
            Key::DeleteProductConfirmMessageWithPurchases,
            &[("count", &purchases.to_string())],
        ),
        (purchases, brews) => t_args(
            Key::DeleteProductConfirmMessageWithCascades,
            &[
                ("purchases", &purchases.to_string()),
                ("brews", &brews.to_string()),
            ],
        ),
    });
    // 保存は新規でも常に出す (Flutter と同じ)。
    let actions = rsx! {
        if id.is_some() && !loading() && load_failure.is_none() {
            IconButton {
                name: favorite_name,
                label: favorite_label,
                onclick: move |_| toggle_favorite.call(()),
            }
            IconButton {
                name: "delete".to_string(),
                label: t(Key::DeleteButton).to_string(),
                onclick: move |_| request_delete.call(()),
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
                title: t(if id.is_some() { Key::ProductEditTitle } else { Key::ProductNewTitle }).to_string(),
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
                                oninput: move |event: FormEvent| {
                                    name.set(event.value());
                                    dirty.set(true);
                                },
                            }
                        }
                        SuggestionField {
                            api: RecordsApi::new(services.api.clone()),
                            target: SuggestionTarget::Producer,
                            label: t(Key::Producer).to_string(),
                            value: producer,
                            disabled: busy(),
                            on_change: move |_| dirty.set(true),
                        }
                        div { class: "grid2",
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Origin,
                                label: t(Key::Origin).to_string(),
                                value: origin,
                                disabled: busy(),
                                on_change: move |_| dirty.set(true),
                            }
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Region,
                                label: t(Key::Region).to_string(),
                                value: region,
                                disabled: busy(),
                                on_change: move |_| dirty.set(true),
                            }
                        }
                        div { class: "grid2",
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Process,
                                label: t(Key::Process).to_string(),
                                value: process,
                                disabled: busy(),
                                on_change: move |_| dirty.set(true),
                            }
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Variety,
                                label: t(Key::Variety).to_string(),
                                value: variety,
                                disabled: busy(),
                                on_change: move |_| dirty.set(true),
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
                                            dirty.set(true);
                                        })),
                                    }
                                }
                            }
                            div { class: "tag-add",
                                TextField {
                                    value: tag_input(),
                                    placeholder: Some(t(Key::TagInputHint).to_string()),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| {
                                        tag_input.set(event.value());
                                        dirty.set(true);
                                    },
                                }
                                Button {
                                    label: t(Key::AddButton).to_string(),
                                    variant: ButtonVariant::Secondary,
                                    size: ButtonSize::Sm,
                                    disabled: busy(),
                                    onclick: move |_| add_tag.call(()),
                                }
                            }
                        }
                        if let Some(failure) = failure {
                            {retryable_banner(&failure, retry_save)}
                        }
                        if let Some(failure) = impact_failure {
                            {retryable_banner(&failure, request_delete)}
                        }
                        if let Some(failure) = delete_failure {
                            {retryable_banner(&failure, delete_now)}
                        }
                    }
                }
            }
        }
        if discard_open() {
            DiscardConfirm { open: discard_open, on_discard: confirm_discard }
        }
        if delete_open() {
            DeleteConfirm {
                open: delete_open,
                title: t(Key::DeleteProductConfirmTitle).to_string(),
                message: delete_message.unwrap_or_default(),
                on_confirm: move |_| delete_now.call(()),
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
