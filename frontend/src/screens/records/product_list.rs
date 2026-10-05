//! 商品の一覧の画面 (FR-7、FR-8)。
//!
//! 行から商品の編集を開く。幅 840 px 以上では一覧とフォームを 2 段組にし、行を押すと右の面に
//! フォームを出す。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::t;
use crate::i18n::Key;
use crate::records::display::product_row_subtitle;
use crate::records::{record_error_key, RecordServices, RecordsApi};
use crate::router::Route;
use crate::screens::ScreenAppBar;
use crate::ui::{Fab, ListRow, NavigationRail, TagChip, WideLayout};

use super::product_form::ProductForm;
use super::{
    clear_notice_after, mark_records_changed, rail_items, use_wide_layout, RecordListView,
    RecordLoader, SortChoice,
};

/// 商品の一覧の並び順の選択肢 (FR-20)。
const SORT_CHOICES: [SortChoice; 3] = [
    SortChoice {
        key: "created_at",
        label: Key::SortCreatedAt,
    },
    SortChoice {
        key: "name",
        label: Key::ProductNameLabel,
    },
    SortChoice {
        key: "updated_at",
        label: Key::SortUpdatedAt,
    },
];

/// 商品の一覧 (FR-7)。
#[component]
pub fn ProductListScreen() -> Element {
    let services = use_context::<RecordServices>();
    let notice = use_context::<Signal<Option<String>>>();
    let revision = use_context::<Signal<u64>>();
    let navigator = navigator();
    let wide = use_wide_layout();
    let mut selected = use_signal(|| None::<String>);
    let mut creating = use_signal(|| false);

    let load_services = services.clone();
    let load = RecordLoader::new(move |cursor, options| {
        let api = RecordsApi::new(load_services.api.clone());
        Box::pin(async move { api.products(&options, cursor.as_deref(), None).await })
    });

    let favorite_services = services.clone();
    let row = Callback::new(move |product: crate::records::Product| {
        let subtitle = product_row_subtitle(&product);
        let notes: Vec<String> = product.flavor_notes.clone();
        let id = product.id.clone();
        let click_id = id.clone();
        let selected_now = selected() == Some(id);
        let favorited = product.favorited_at.is_some();
        let favorite_id = product.id.clone();
        let favorite_api = RecordsApi::new(favorite_services.api.clone());
        let mut favorite_revision = revision;
        let mut favorite_notice = notice;
        rsx! {
            ListRow {
                title: product.name.clone(),
                subtitle: subtitle.map(|text| rsx! { span { "{text}" } }),
                tags: Some(rsx! {
                    for note in notes {
                        TagChip { label: note }
                    }
                }),
                favorited,
                on_favorite: Some(EventHandler::new(move |_| {
                    let api = favorite_api.clone();
                    let id = favorite_id.clone();
                    spawn(async move {
                        match api.set_product_favorite(&id, !favorited).await {
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
                    } else {
                        let _ = navigator.push(Route::ProductEdit { id: click_id.clone() });
                    }
                })),
            }
        }
    });

    let detail = if wide() {
        if creating() {
            Some(rsx! {
                ProductForm {
                    embedded: true,
                    on_close: EventHandler::new(move |_| creating.set(false)),
                    on_saved: EventHandler::new(move |_| creating.set(false)),
                }
            })
        } else {
            selected().map(|id| {
                rsx! {
                    ProductForm {
                        id: Some(id),
                        embedded: true,
                        on_close: EventHandler::new(move |_| selected.set(None)),
                        on_saved: EventHandler::new(move |_| selected.set(None)),
                    }
                }
            })
        }
    } else {
        None
    };

    let list = rsx! {
        div { class: "screen",
            ScreenAppBar { title: t(Key::ProductsTitle).to_string() }
            RecordListView::<crate::records::Product> {
                load,
                row,
                empty_hint: Some(t(Key::ProductsEmptyHint).to_string()),
                show_toolbar: true,
                sort_options: SORT_CHOICES.to_vec(),
            }
            Fab {
                label: t(Key::NewProductButton).to_string(),
                icon: Some("add".to_string()),
                onclick: move |_| {
                    if wide() {
                        creating.set(true);
                        selected.set(None);
                    } else {
                        let _ = navigator.push(Route::ProductNew {});
                    }
                },
            }
        }
    };

    rsx! {
        WideLayout {
            rail: rsx! { NavigationRail { items: rail_items(navigator, 2) } },
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
