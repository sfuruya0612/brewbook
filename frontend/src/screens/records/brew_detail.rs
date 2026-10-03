//! 抽出の詳細の画面 (FR-11、UC-6)。
//!
//! 使った購入をたどり、その中の商品と店 (登録している場合) もたどれるようにする。幅 840 px
//! 以上の 2 段組では `embedded` を true にし、右の面に出す。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, Key};
use crate::records::display::{brew_reference_tiles, count_text, number_text, rating_text};
use crate::records::{record_error_key, RecordError, RecordServices, RecordsApi};
use crate::router::Route;
use crate::screens::ScreenAppBar;
use crate::ui::{
    ArchivedBadge, IconButton, Ledger, LedgerRow, Rating, ReferenceChain, ReferenceTile,
};

use super::{archive_button, clear_notice_after, mark_records_changed, retryable_banner};

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
) -> Element {
    let services = use_context::<RecordServices>();
    let mut revision = use_context::<Signal<u64>>();
    let mut notice = use_context::<Signal<Option<String>>>();
    let navigator = navigator();
    let mut brew = use_signal(|| None::<crate::records::Brew>);
    let mut error = use_signal(|| None::<RecordError>);
    let mut loading = use_signal(|| true);

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

    let toggle_services = services.clone();
    let toggle_id = id.clone();
    let toggle = move |_| {
        let Some(current) = brew() else {
            return;
        };
        let api = RecordsApi::new(toggle_services.api.clone());
        let id = toggle_id.clone();
        let archived = !current.is_archived();
        spawn(async move {
            match api.set_brew_archived(&id, archived).await {
                Ok(updated) => {
                    let message = if updated.is_archived() {
                        Key::ArchivedMessage
                    } else {
                        Key::UnarchivedMessage
                    };
                    brew.set(Some(updated));
                    notice.set(Some(t(message).to_string()));
                    mark_records_changed(&mut revision);
                }
                Err(failure) => notice.set(Some(t(record_error_key(&failure)).to_string())),
            }
        });
    };

    let language = current_language();
    let offset = services.clock.utc_offset_minutes();
    let current = brew();
    let failure = error();
    let actions = current.as_ref().map(|brew| {
        let archived = brew.is_archived();
        rsx! {
            {archive_button(archived, false, EventHandler::new(toggle))}
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
        }
    });

    rsx! {
        div { class: "screen",
            ScreenAppBar {
                title: t(Key::BrewDetailTitle).to_string(),
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
                            div { class: "name",
                                "{brew.purchase.product.name}"
                                if brew.is_archived() {
                                    ArchivedBadge {}
                                }
                            }
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
