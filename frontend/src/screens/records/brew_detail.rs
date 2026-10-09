//! 抽出の詳細の画面 (FR-11、UC-6)。
//!
//! 使った購入をたどり、その中の商品と店 (登録している場合) もたどれるようにする。幅 840 px
//! 以上の 2 段組では `embedded` を true にし、右の面に出す。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, Key};
use crate::records::display::{brew_reference_tiles, count_text, number_text, rating_text};
use crate::records::{
    record_error_key, record_error_not_found, RecordError, RecordServices, RecordsApi,
};
use crate::router::Route;
use crate::screens::ScreenAppBar;
use crate::ui::{IconButton, Ledger, LedgerRow, Rating, ReferenceChain, ReferenceTile};

use super::{clear_notice_after, mark_records_changed, retryable_banner, DeleteConfirm};

/// 抽出の詳細 (FR-11)。
#[component]
pub fn BrewDetailScreen(id: String) -> Element {
    rsx! {
        BrewDetail { id }
    }
}

/// 抽出の詳細の中身。2 段組の右の面にも出せる。
#[component]
pub fn BrewDetail(
    /// 表示する抽出の ID。
    id: String,

    /// 2 段組の右の面に出すか。戻るの代わりに閉じる操作を置かない。
    #[props(default = false)]
    embedded: bool,

    /// 編集を開く動き。無いときは編集の経路を上に積む。
    #[props(default)]
    on_edit: Option<EventHandler<()>>,

    /// 削除できたときの動き。無いときは前の画面へ戻る。
    #[props(default)]
    on_deleted: Option<EventHandler<()>>,
) -> Element {
    let services = use_context::<RecordServices>();
    let revision = use_context::<Signal<u64>>();
    let notice = use_context::<Signal<Option<String>>>();
    let navigator = navigator();
    let mut brew = use_signal(|| None::<crate::records::Brew>);
    let mut error = use_signal(|| None::<RecordError>);
    let mut loading = use_signal(|| true);
    // 削除の確認と、削除の失敗の再試行 (0056)。
    let mut delete_open = use_signal(|| false);
    let mut delete_error = use_signal(|| None::<RecordError>);

    let reload_id = id.clone();
    let reload_services = services.clone();
    let reload = EventHandler::new(move |_| {
        let api = RecordsApi::new(reload_services.api.clone());
        let id = reload_id.clone();
        spawn(async move {
            loading.set(true);
            error.set(None);
            match api.brew(&id).await {
                Ok(loaded) => brew.set(Some(loaded)),
                Err(failure) => error.set(Some(failure)),
            }
            loading.set(false);
        });
    });

    // マウント時と、記録が変わったときに読み直す。
    use_effect(move || {
        let _ = revision();
        reload.call(());
    });

    let language = current_language();
    let offset = services.clock.utc_offset_minutes();
    let current = brew();
    let failure = error();
    let delete_failure = delete_error();
    let favorited = current
        .as_ref()
        .is_some_and(|brew| brew.favorited_at.is_some());
    let favorite_services = services.clone();
    let favorite_id = id.clone();
    let mut favorite_revision = revision;
    let mut favorite_notice = notice;
    let favorite = EventHandler::new(move |_| {
        let api = RecordsApi::new(favorite_services.api.clone());
        let id = favorite_id.clone();
        spawn(async move {
            match api.set_brew_favorite(&id, !favorited).await {
                Ok(_) => mark_records_changed(&mut favorite_revision),
                Err(failure) => {
                    favorite_notice.set(Some(t(record_error_key(&failure)).to_string()))
                }
            }
        });
    });
    // 削除 (0056)。確認の後に削除し、失敗は再試行のバナーで知らせる。
    let delete_services = services.clone();
    let delete_id = id.clone();
    let mut delete_revision = revision;
    let mut delete_notice = notice;
    let close_after_delete = EventHandler::new(move |_| match on_deleted {
        Some(handler) => handler.call(()),
        None => {
            navigator.go_back();
        }
    });
    let delete_now = EventHandler::new(move |_| {
        let api = RecordsApi::new(delete_services.api.clone());
        let id = delete_id.clone();
        spawn(async move {
            match api.delete_brew(&id).await {
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
            close_after_delete.call(());
        });
    });
    let actions = current.as_ref().map(|_| {
        let favorite_name = if favorited { "star" } else { "star_border" }.to_string();
        let favorite_label = t(if favorited {
            Key::FavoriteRemoveLabel
        } else {
            Key::FavoriteAddLabel
        })
        .to_string();
        rsx! {
            IconButton {
                name: favorite_name,
                label: favorite_label,
                onclick: move |_| favorite.call(()),
            }
            IconButton {
                name: "edit".to_string(),
                label: t(Key::EditButton).to_string(),
                onclick: move |_| match on_edit {
                    Some(handler) => handler.call(()),
                    None => {
                        let _ = navigator.push(Route::BrewEdit { id: id.clone() });
                    }
                },
            }
            IconButton {
                name: "delete".to_string(),
                label: t(Key::DeleteButton).to_string(),
                onclick: move |_| delete_open.set(true),
            }
        }
    });

    rsx! {
        div { class: "screen",
            ScreenAppBar {
                title: t(Key::BrewDetailTitle).to_string(),
                menu: false,
                leading_icon: (!embedded).then_some("arrow_back_ios_new".to_string()),
                leading_label: Some(t(Key::CancelButton).to_string()),
                on_leading: move |_| {
                    navigator.go_back();
                },
                actions,
            }
            div { class: "body",
                if let Some(brew) = current {
                    div { class: "detail",
                        div { class: "detail-head",
                            div { class: "name", "{brew.purchase.product.name}" }
                            div { class: "t-value muted",
                                {crate::records::values::parse_utc_to_local(&brew.brewed_at, offset)
                                    .map(|datetime| crate::records::values::display_timestamp(datetime, language))
                                    .unwrap_or_else(|| brew.brewed_at.clone())}
                            }
                            if brew.rating.is_some() {
                                Rating { value: brew.rating, show_value: true }
                            }
                        }
                        Ledger {
                            LedgerRow {
                                label: t(Key::DoseLabel).to_string(),
                                value: number_text(brew.dose_grams),
                                unit: Some(t(Key::GramUnit).to_string()),
                            }
                            LedgerRow {
                                label: t(Key::WaterLabel).to_string(),
                                value: number_text(brew.water_grams),
                                unit: Some(t(Key::GramUnit).to_string()),
                            }
                            LedgerRow {
                                label: t(Key::WaterTempLabel).to_string(),
                                value: number_text(brew.water_temp_c),
                                unit: Some(t(Key::CelsiusUnit).to_string()),
                            }
                            LedgerRow {
                                label: t(Key::BrewTimeLabel).to_string(),
                                value: count_text(brew.brew_time_seconds),
                                unit: Some(t(Key::SecondUnit).to_string()),
                            }
                            LedgerRow {
                                label: t(Key::MethodLabel).to_string(),
                                value: brew.method.clone(),
                                mono: false,
                            }
                            LedgerRow {
                                label: t(Key::GrindSettingLabel).to_string(),
                                value: brew.grind_setting.clone(),
                                mono: false,
                            }
                            LedgerRow {
                                label: t(Key::RatingLabel).to_string(),
                                value: rating_text(brew.rating, language),
                            }
                        }
                        if let Some(notes) = brew.notes.clone().filter(|notes| !notes.is_empty()) {
                            SectionLabel { text: t(Key::NotesLabel).to_string() }
                            p { class: "t-body", "{notes}" }
                        }
                        SectionLabel { text: t(Key::UsedBeansLabel).to_string() }
                        ReferenceChain {
                            for (index, (kind, name)) in
                                brew_reference_tiles(&brew, language).into_iter().enumerate()
                            {
                                {
                                    let route = match kind {
                                        Key::PurchaseLabel => Some(Route::PurchaseDetail {
                                            id: brew.purchase.id.clone(),
                                        }),
                                        Key::ProductLabel => Some(Route::ProductEdit {
                                            id: brew.purchase.product.id.clone(),
                                        }),
                                        _ => brew.purchase.shop.as_ref().map(|shop| Route::ShopEdit {
                                            id: shop.id.clone(),
                                        }),
                                    };
                                    let _ = index;
                                    rsx! {
                                        ReferenceTile {
                                            kind: t(kind).to_string(),
                                            name,
                                            on_click: route.map(|route| EventHandler::new(move |_| {
                                                let _ = navigator.push(route.clone());
                                            })),
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if loading() {
                    div { class: "empty", "{t(Key::Loading)}" }
                }
                if let Some(failure) = failure {
                    {retryable_banner(&failure, reload)}
                }
                if let Some(failure) = delete_failure {
                    {retryable_banner(&failure, delete_now)}
                }
            }
        }
        if delete_open() {
            DeleteConfirm {
                open: delete_open,
                title: t(Key::DeleteBrewConfirmTitle).to_string(),
                message: t(Key::DeleteBrewConfirmMessage).to_string(),
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

/// 詳細の見出しの下に置く区画の見出し (0039 の Ledger に無いため、この画面で組む)。
#[component]
fn SectionLabel(text: String) -> Element {
    rsx! {
        div { class: "sec", "{text}" }
    }
}
