//! ホーム (抽出の一覧。FR-11、FR-12)。
//!
//! 一覧から抽出の詳細を開き、末尾までスクロールすると次のページを読む。抽出の登録は
//! この画面から始め、保存完了までの最短経路を短く保つ (PRD の成功指標)。幅 840 px 以上では
//! 一覧と詳細を 2 段組にし、行を押すと右の面に詳細を出す。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::auth::{logout, message_key, AuthServices, SessionStatus};
use crate::i18n::{current_language, t, Key};
use crate::records::{brew_row_subtitle, RecordServices, RecordsApi};
use crate::router::Route;
use crate::screens::records::{
    clear_notice_after, rail_items, use_wide_layout, RecordListView, RecordLoader,
};
use crate::ui::{
    AppBar, Fab, IconButton, ListRow, NavigationRail, Rating, RowValue, Snackbar, WideLayout,
};

use super::{brew_detail::BrewDetail, brew_form::BrewForm};

/// ホーム (抽出の一覧。FR-11)。
#[component]
pub fn HomeScreen() -> Element {
    let services = use_context::<RecordServices>();
    let auth = use_context::<AuthServices>();
    let mut session = use_context::<Signal<SessionStatus>>();
    let mut notice = use_context::<Signal<Option<String>>>();
    let navigator = navigator();
    let wide = use_wide_layout();
    let mut menu_open = use_signal(|| false);
    let mut selected = use_signal(|| None::<String>);
    let mut creating = use_signal(|| false);

    let offset = services.clock.utc_offset_minutes();
    let load_services = services.clone();
    let load = RecordLoader::new(move |cursor, include_archived| {
        let api = RecordsApi::new(load_services.api.clone());
        Box::pin(async move { api.brews(cursor.as_deref(), include_archived).await })
    });

    let row = Callback::new(move |brew: crate::records::Brew| {
        let subtitle = brew_row_subtitle(&brew, current_language(), offset);
        let id = brew.id.clone();
        let click_id = id.clone();
        let selected_now = selected() == Some(id);
        rsx! {
            ListRow {
                title: brew.purchase.product.name.clone(),
                subtitle: Some(rsx! { span { RowValue { text: subtitle } } }),
                trailing: Some(rsx! { Rating { value: brew.rating } }),
                archived: brew.is_archived(),
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

    let menu = rsx! {
        if !wide() {
            IconButton {
                name: "more_vert".to_string(),
                label: t(Key::MenuTooltip).to_string(),
                onclick: move |_| menu_open.toggle(),
            }
            if menu_open() {
                div { class: "menu",
                    for (key, route) in [
                        (Key::PurchasesTitle, Route::Purchases {}),
                        (Key::ProductsTitle, Route::Products {}),
                        (Key::ShopsTitle, Route::Shops {}),
                        (Key::StatsTitle, Route::Stats {}),
                        (Key::SettingsTitle, Route::Settings {}),
                    ] {
                        div {
                            onclick: move |_| {
                                menu_open.set(false);
                                let _ = navigator.push(route.clone());
                            },
                            "{t(key)}"
                        }
                    }
                    div { class: "sep" }
                    div {
                        onclick: move |_| {
                            menu_open.set(false);
                            let auth = auth.clone();
                            spawn(async move {
                                match logout(&auth).await {
                                    Ok(()) => session.set(SessionStatus::SignedOut),
                                    Err(error) => notice.set(Some(t(message_key(&error)).to_string())),
                                }
                            });
                        },
                        "{t(Key::LogoutButton)}"
                    }
                }
            }
        }
    };

    let list = rsx! {
        div { class: "screen",
            AppBar { title: t(Key::HomeTitle).to_string(), wordmark: true, actions: menu }
            RecordListView::<crate::records::Brew> {
                load,
                row,
                empty_hint: Some(t(Key::HomeEmptyHint).to_string()),
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
