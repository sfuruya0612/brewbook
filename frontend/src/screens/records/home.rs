//! ホーム (抽出の一覧。FR-11)。
//!
//! 一覧から抽出の詳細を開き、末尾までスクロールすると次のページを読む。抽出の登録は
//! この画面から始め、保存完了までの最短経路を短く保つ (PRD の成功指標)。幅 840 px 以上では
//! 一覧と詳細を 2 段組にし、行を押すと右の面に詳細を出す。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, Key};
use crate::records::{brew_row_subtitle, record_error_key, RecordServices, RecordsApi};
use crate::router::Route;
use crate::screens::records::{
    clear_notice_after, mark_records_changed, rail_items, use_wide_layout, RecordListView,
    RecordLoader, SortChoice,
};
use crate::screens::ScreenAppBar;
use crate::ui::{Fab, ListRow, NavigationRail, Rating, RowValue, Snackbar, WideLayout};

use super::{brew_detail::BrewDetail, brew_form::BrewForm};

/// 抽出の一覧の並び順の選択肢 (FR-20)。
const SORT_CHOICES: [SortChoice; 3] = [
    SortChoice {
        key: "brewed_at",
        label: Key::BrewedAtLabel,
    },
    SortChoice {
        key: "rating",
        label: Key::RatingLabel,
    },
    SortChoice {
        key: "dose_grams",
        label: Key::DoseLabel,
    },
];

/// ホーム (抽出の一覧。FR-11)。
#[component]
pub fn HomeScreen() -> Element {
    let services = use_context::<RecordServices>();
    let notice = use_context::<Signal<Option<String>>>();
    let revision = use_context::<Signal<u64>>();
    let navigator = navigator();
    let wide = use_wide_layout();
    let mut selected = use_signal(|| None::<String>);
    let mut creating = use_signal(|| false);

    let offset = services.clock.utc_offset_minutes();
    let load_services = services.clone();
    let load = RecordLoader::new(move |cursor, options| {
        let api = RecordsApi::new(load_services.api.clone());
        Box::pin(async move { api.brews(&options, cursor.as_deref()).await })
    });

    let favorite_services = services.clone();
    let row = Callback::new(move |brew: crate::records::Brew| {
        let subtitle = brew_row_subtitle(&brew, current_language(), offset);
        let id = brew.id.clone();
        let click_id = id.clone();
        let selected_now = selected() == Some(id);
        let favorited = brew.favorited_at.is_some();
        let favorite_id = brew.id.clone();
        let favorite_api = RecordsApi::new(favorite_services.api.clone());
        let mut favorite_revision = revision;
        let mut favorite_notice = notice;
        rsx! {
            ListRow {
                title: brew.purchase.product.name.clone(),
                subtitle: Some(rsx! { span { RowValue { text: subtitle } } }),
                trailing: Some(rsx! { Rating { value: brew.rating } }),
                favorited,
                on_favorite: Some(EventHandler::new(move |_| {
                    let api = favorite_api.clone();
                    let id = favorite_id.clone();
                    spawn(async move {
                        match api.set_brew_favorite(&id, !favorited).await {
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
                        let _ = navigator.push(Route::BrewDetail { id: click_id.clone() });
                    }
                })),
            }
        }
    });

    let detail = if wide() {
        if creating() {
            Some(rsx! {
                BrewForm {
                    embedded: true,
                    on_close: EventHandler::new(move |_| creating.set(false)),
                    on_saved: EventHandler::new(move |_| creating.set(false)),
                }
            })
        } else {
            selected().map(|id| {
                rsx! {
                    BrewDetail {
                        id,
                        embedded: true,
                        on_edit: EventHandler::new(move |_| creating.set(true)),
                    }
                }
            })
        }
    } else {
        None
    };

    let list = rsx! {
        div { class: "screen",
            ScreenAppBar { title: t(Key::BrewsLabel).to_string() }
            RecordListView::<crate::records::Brew> {
                load,
                row,
                empty_hint: Some(t(Key::HomeEmptyHint).to_string()),
                show_toolbar: true,
                sort_options: SORT_CHOICES.to_vec(),
            }
            Fab {
                label: t(Key::NewBrewButton).to_string(),
                icon: Some("add".to_string()),
                onclick: move |_| {
                    if wide() {
                        creating.set(true);
                        selected.set(None);
                    } else {
                        let _ = navigator.push(Route::BrewNew {});
                    }
                },
            }
        }
    };

    rsx! {
        WideLayout {
            rail: rsx! { NavigationRail { items: rail_items(navigator, 0) } },
            list,
            detail,
        }
        if let Some(message) = notice() {
            {clear_notice_after(notice)}
            div { class: "notice",
                Snackbar { message }
            }
        }
    }
}
