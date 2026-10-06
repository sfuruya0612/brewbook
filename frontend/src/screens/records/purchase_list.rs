//! 購入の一覧の画面 (FR-9)。
//!
//! 行から購入の詳細を開く。幅 840 px 以上では一覧と詳細を 2 段組にし、行を押すと右の面に
//! 詳細やフォームを出す。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, Key};
use crate::records::display::{purchase_price_text, purchase_row_subtitle, purchase_weight_text};
use crate::records::{record_error_key, RecordServices, RecordsApi};
use crate::router::Route;
use crate::screens::ScreenAppBar;
use crate::ui::{Fab, ListRow, ListThumb, NavigationRail, RowValue, WideLayout};

use super::{
    clear_notice_after, mark_records_changed, rail_items, use_wide_layout, RecordListView,
    RecordLoader, SortChoice,
};
use super::{purchase_detail::PurchaseDetail, purchase_form::PurchaseForm};

/// 購入の一覧の並び順の選択肢 (FR-20)。
const SORT_CHOICES: [SortChoice; 3] = [
    SortChoice {
        key: "purchased_on",
        label: Key::PurchasedOnLabel,
    },
    SortChoice {
        key: "price_amount",
        label: Key::PriceLabel,
    },
    SortChoice {
        key: "weight_grams",
        label: Key::WeightLabel,
    },
];

/// 購入の一覧 (FR-9)。
#[component]
pub fn PurchaseListScreen() -> Element {
    let services = use_context::<RecordServices>();
    let notice = use_context::<Signal<Option<String>>>();
    let revision = use_context::<Signal<u64>>();
    let navigator = navigator();
    let wide = use_wide_layout();
    let mut selected = use_signal(|| None::<String>);
    let mut creating = use_signal(|| false);
    let mut editing = use_signal(|| false);

    let load_services = services.clone();
    let load = RecordLoader::new(move |cursor, options| {
        let api = RecordsApi::new(load_services.api.clone());
        Box::pin(async move { api.purchases(&options, cursor.as_deref()).await })
    });

    let photo_services = services.clone();
    let favorite_services = services.clone();
    let row = Callback::new(move |purchase: crate::records::Purchase| {
        let subtitle = purchase_row_subtitle(&purchase, current_language());
        let photo = purchase.photo_key.as_ref().map(|_| {
            format!(
                "{}/purchases/{}/photo",
                photo_services.api.base_path(),
                purchase.id
            )
        });
        let weight =
            purchase_weight_text(&purchase).unwrap_or_else(|| t(Key::UnsetLabel).to_string());
        let price =
            purchase_price_text(&purchase).unwrap_or_else(|| t(Key::UnsetLabel).to_string());
        let id = purchase.id.clone();
        let click_id = id.clone();
        let selected_now = selected() == Some(id);
        let favorited = purchase.favorited_at.is_some();
        let favorite_id = purchase.id.clone();
        let favorite_api = RecordsApi::new(favorite_services.api.clone());
        let mut favorite_revision = revision;
        let mut favorite_notice = notice;
        rsx! {
            ListRow {
                leading: Some(rsx! {
                    ListThumb { src: photo, alt: t(Key::PhotoLabel).to_string() }
                }),
                title: purchase.product.name.clone(),
                subtitle: Some(rsx! { span { RowValue { text: subtitle } } }),
                trailing: Some(rsx! {
                    div { class: "trail",
                        div { class: "t-value", "{weight}" }
                        div { class: "t-caption muted", "{price}" }
                    }
                }),
                favorited,
                on_favorite: Some(EventHandler::new(move |_| {
                    let api = favorite_api.clone();
                    let id = favorite_id.clone();
                    spawn(async move {
                        match api.set_purchase_favorite(&id, !favorited).await {
                            Ok(_) => mark_records_changed(&mut favorite_revision),
                            Err(failure) => favorite_notice
                                .set(Some(t(record_error_key(&failure)).to_string())),
                        }
                    });
                })),
                selected: selected_now,
                on_click: Some(EventHandler::new(move |_| {
                    if wide() {
                        selected.set(Some(click_id.clone()));
                        creating.set(false);
                        editing.set(false);
                    } else {
                        let _ = navigator.push(Route::PurchaseDetail { id: click_id.clone() });
                    }
                })),
            }
        }
    });

    let detail = if wide() {
        if creating() {
            Some(rsx! {
                PurchaseForm {
                    embedded: true,
                    on_close: EventHandler::new(move |_| creating.set(false)),
                    on_saved: EventHandler::new(move |_| creating.set(false)),
                }
            })
        } else if let Some(id) = selected() {
            if editing() {
                Some(rsx! {
                    PurchaseForm {
                        id: Some(id),
                        embedded: true,
                        on_close: EventHandler::new(move |_| editing.set(false)),
                        on_saved: EventHandler::new(move |_| editing.set(false)),
                    }
                })
            } else {
                Some(rsx! {
                    PurchaseDetail {
                        id,
                        embedded: true,
                        on_edit: EventHandler::new(move |_| editing.set(true)),
                    }
                })
            }
        } else {
            None
        }
    } else {
        None
    };

    let list = rsx! {
        div { class: "screen",
            ScreenAppBar { title: t(Key::PurchasesTitle).to_string(), brand: true }
            RecordListView::<crate::records::Purchase> {
                load,
                row,
                empty_hint: Some(t(Key::PurchasesEmptyHint).to_string()),
                show_toolbar: true,
                sort_options: SORT_CHOICES.to_vec(),
            }
            Fab {
                label: t(Key::NewPurchaseButton).to_string(),
                icon: Some("add".to_string()),
                onclick: move |_| {
                    if wide() {
                        creating.set(true);
                        selected.set(None);
                        editing.set(false);
                    } else {
                        let _ = navigator.push(Route::PurchaseNew {});
                    }
                },
            }
        }
    };

    rsx! {
        WideLayout {
            rail: rsx! { NavigationRail { items: rail_items(navigator, 1) } },
            list,
            detail,
        }
        if let Some(message) = notice() {
            {clear_notice_after(notice)}
            div { class: "notice",
                crate::ui::Snackbar { message }
            }
        }
    }
}
