//! 店の一覧の画面 (FR-6)。
//!
//! 行から店の編集を開く。幅 840 px 以上では一覧とフォームを 2 段組にし、行を押すと右の面に
//! フォームを出す。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, Key};
use crate::records::display::shop_row_subtitle;
use crate::records::{record_error_key, RecordServices, RecordsApi};
use crate::router::Route;
use crate::screens::ScreenAppBar;
use crate::ui::{Fab, ListRow, NavigationRail, WideLayout};

use super::shop_form::ShopForm;
use super::{
    clear_notice_after, mark_records_changed, rail_items, use_wide_layout, RecordListView,
    RecordLoader, SortChoice,
};

/// 店の一覧の並び順の選択肢 (FR-20)。
const SORT_CHOICES: [SortChoice; 3] = [
    SortChoice {
        key: "created_at",
        label: Key::SortCreatedAt,
    },
    SortChoice {
        key: "name",
        label: Key::ShopNameLabel,
    },
    SortChoice {
        key: "updated_at",
        label: Key::SortUpdatedAt,
    },
];

/// 店の一覧 (FR-6)。
#[component]
pub fn ShopListScreen() -> Element {
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
        Box::pin(async move { api.shops(&options, cursor.as_deref()).await })
    });

    let favorite_services = services.clone();
    let row = Callback::new(move |shop: crate::records::Shop| {
        let subtitle = shop_row_subtitle(&shop, current_language());
        let id = shop.id.clone();
        let click_id = id.clone();
        let selected_now = selected() == Some(id);
        let favorited = shop.favorited_at.is_some();
        let favorite_id = shop.id.clone();
        let favorite_api = RecordsApi::new(favorite_services.api.clone());
        let mut favorite_revision = revision;
        let mut favorite_notice = notice;
        rsx! {
            ListRow {
                title: shop.name.clone(),
                subtitle: Some(rsx! { span { "{subtitle}" } }),
                favorited,
                on_favorite: Some(EventHandler::new(move |_| {
                    let api = favorite_api.clone();
                    let id = favorite_id.clone();
                    spawn(async move {
                        match api.set_shop_favorite(&id, !favorited).await {
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
                        let _ = navigator.push(Route::ShopEdit { id: click_id.clone() });
                    }
                })),
            }
        }
    });

    let detail = if wide() {
        if creating() {
            Some(rsx! {
                ShopForm {
                    embedded: true,
                    on_close: EventHandler::new(move |_| creating.set(false)),
                    on_saved: EventHandler::new(move |_| creating.set(false)),
                }
            })
        } else {
            selected().map(|id| {
                rsx! {
                    ShopForm {
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
            ScreenAppBar { title: t(Key::ShopsTitle).to_string(), brand: true }
            RecordListView::<crate::records::Shop> {
                load,
                row,
                empty_hint: Some(t(Key::ShopsEmptyHint).to_string()),
                show_toolbar: true,
                sort_options: SORT_CHOICES.to_vec(),
            }
            Fab {
                label: t(Key::NewShopButton).to_string(),
                icon: Some("add".to_string()),
                onclick: move |_| {
                    if wide() {
                        creating.set(true);
                        selected.set(None);
                    } else {
                        let _ = navigator.push(Route::ShopNew {});
                    }
                },
            }
        }
    };

    rsx! {
        WideLayout {
            rail: rsx! { NavigationRail { items: rail_items(navigator, 3) } },
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
