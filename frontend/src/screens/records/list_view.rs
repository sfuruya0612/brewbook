//! カーソル方式の一覧の共通の枠。
//!
//! 末尾までのスクロールでの追加読み込みと、並び順とお気に入りの絞り込みのツールバーを持つ
//! (FR-20、FR-21)。中身の表示は `row` が決める。選択のシート ([`super::RecordPickerSheet`])
//! ではツールバーを出さず、API の既定の並び順を使う。

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::i18n::{t, Key};
use crate::records::{ListOptions, PageRequest, RecordError, RecordList, RecordPage, SortOrder};
use crate::ui::{BrewbookMark, Chip, IconButton};

use super::{error_banner, retryable_banner};

/// 一覧の 1 ページを読む未来。
pub type LoadFuture<T> = Pin<Box<dyn Future<Output = Result<RecordPage<T>, RecordError>>>>;

/// 一覧の 1 ページを読む関数。画面ごとに API の呼び出しを閉じ込める。
pub struct RecordLoader<T>(Rc<dyn Fn(Option<String>, ListOptions) -> LoadFuture<T>>);

impl<T> Clone for RecordLoader<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for RecordLoader<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> RecordLoader<T> {
    /// ページを読む関数から作る。
    pub fn new(load: impl Fn(Option<String>, ListOptions) -> LoadFuture<T> + 'static) -> Self {
        Self(Rc::new(load))
    }

    /// 1 ページを読む。
    pub fn call(&self, cursor: Option<String>, options: ListOptions) -> LoadFuture<T> {
        (self.0)(cursor, options)
    }
}

/// ツールバーの並び順の選択肢 (FR-20)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SortChoice {
    /// `sort` に送るキー。
    pub key: &'static str,
    /// 選択肢の文言 (ARB から取る)。
    pub label: Key,
}

/// カーソル方式の一覧の枠。
///
/// マウント時と、記録の変更の通知 ([`super::mark_records_changed`]) のたびに先頭から読み直す。
/// 並び順、方向、お気に入りのみを変えたときもカーソルを捨てて先頭から読み直す (FR-20、FR-21)。
#[component]
pub fn RecordListView<T: Clone + PartialEq + 'static>(
    /// 一覧の 1 ページを読む。
    load: RecordLoader<T>,

    /// 行の組み立て。広い画面の選択中の地は親が決める。
    row: Callback<T, Element>,

    /// 記録が無いときに、次にすることを示す 1 文 (ARB から取る)。
    #[props(default)]
    empty_hint: Option<String>,

    /// 並び順とお気に入りのツールバーを出すか。選択のシートでは出さない。
    #[props(default = false)]
    show_toolbar: bool,

    /// 並び順の選択肢 (FR-20)。ツールバーを出す画面が渡す。
    #[props(default)]
    sort_options: Vec<SortChoice>,
) -> Element {
    let mut list = use_signal(RecordList::new);
    let items = use_signal(Vec::<T>::new);
    let revision = use_context::<Signal<u64>>();

    // ツールバーの選択 (FR-20、FR-21)。既定は降順で、お気に入りの絞り込みなし。
    let mut sort_index = use_signal(|| 0_usize);
    let mut order = use_signal(SortOrder::default);
    let mut favorite_only = use_signal(|| false);

    // マウント時と、記録が変わったときと、ツールバーの選択が変わったときに先頭から読み直す
    // (revision と選択の signal の読み取りで再実行される)。
    let effect_load = load.clone();
    let effect_choices = sort_options.clone();
    use_effect(move || {
        let _ = revision();
        let options = list_options(&effect_choices, sort_index(), order(), favorite_only());
        let request = list.write().reset();
        if let Some(request) = request {
            let load = effect_load.clone();
            spawn(async move {
                load_pages(load, list, items, options, Some(request)).await;
            });
        }
    });

    let state = list();
    let error = state.error().cloned();
    let loaded = state.is_loaded();
    let loading = state.is_loading();
    let has_more = state.next_cursor().is_some();
    drop(state);
    let rows: Vec<T> = items.read().clone();

    let retry_load = load.clone();
    let retry_choices = sort_options.clone();
    let scroll_load = load.clone();
    let scroll_choices = sort_options.clone();
    let current_sort_key = sort_options
        .get(sort_index())
        .map(|choice| choice.key)
        .unwrap_or_default();
    let change_choices = sort_options.clone();
    let order_icon = if order() == SortOrder::Asc {
        "arrow_upward"
    } else {
        "arrow_downward"
    }
    .to_string();
    let order_label = t(if order() == SortOrder::Asc {
        Key::SortAscending
    } else {
        Key::SortDescending
    })
    .to_string();

    rsx! {
        div { class: "list-view",
            if show_toolbar {
                div { class: "list-toolbar",
                    select {
                        class: "sort-select",
                        "aria-label": "{t(Key::SortLabel)}",
                        value: "{current_sort_key}",
                        onchange: move |event: FormEvent| {
                            let key = event.value();
                            if let Some(index) = change_choices.iter().position(|choice| choice.key == key) {
                                sort_index.set(index);
                            }
                        },
                        for choice in sort_options.iter() {
                            option { value: "{choice.key}", "{t(choice.label)}" }
                        }
                    }
                    IconButton {
                        name: order_icon,
                        label: order_label,
                        onclick: move |_| order.set(order().toggled()),
                    }
                    Chip {
                        label: t(Key::FavoritesOnlyLabel).to_string(),
                        selected: favorite_only(),
                        onclick: move |_| favorite_only.set(!favorite_only()),
                    }
                }
            }
            if let Some(error) = error {
                div { class: "list-error",
                    if crate::records::record_error_retry(&error) {
                        {retryable_banner(&error, EventHandler::new(move |_| {
                            let options = list_options(&retry_choices, sort_index(), order(), favorite_only());
                            let request = list.write().retry();
                            if let Some(request) = request {
                                let load = retry_load.clone();
                                spawn(async move {
                                    load_pages(load, list, items, options, Some(request)).await;
                                });
                            }
                        }))}
                    } else {
                        {error_banner(&error, None)}
                    }
                }
            }
            div {
                class: "list-scroll",
                onscroll: move |event: ScrollEvent| {
                    let remaining = f64::from(event.scroll_height())
                        - event.scroll_top()
                        - f64::from(event.client_height());
                    if list.read().should_load_more(remaining) {
                        let options = list_options(&scroll_choices, sort_index(), order(), favorite_only());
                        let request = list.write().load_more();
                        if let Some(request) = request {
                            let load = scroll_load.clone();
                            spawn(async move {
                                load_pages(load, list, items, options, Some(request)).await;
                            });
                        }
                    }
                },
                if rows.is_empty() {
                    if loaded {
                        div { class: "empty",
                            div { class: "mark faint",
                                BrewbookMark { size: 48 }
                            }
                            p { "{t(Key::NoRecords)}" }
                            if let Some(hint) = empty_hint.clone() {
                                p { class: "t-caption", "{hint}" }
                            }
                        }
                    } else {
                        div { class: "empty", "{t(Key::Loading)}" }
                    }
                } else {
                    div { class: "list",
                        for item in rows {
                            {row.call(item)}
                        }
                    }
                    if loading && has_more {
                        div { class: "empty t-caption", "{t(Key::Loading)}" }
                    }
                }
            }
        }
    }
}

/// ツールバーの選択から一覧の条件を組む (FR-20、FR-21)。
fn list_options(
    choices: &[SortChoice],
    index: usize,
    order: SortOrder,
    favorite_only: bool,
) -> ListOptions {
    ListOptions {
        sort: choices
            .get(index)
            .map(|choice| choice.key)
            .unwrap_or_default(),
        order,
        favorite_only,
    }
}

/// ページを続けて読む。読み込み中に届いた読み直しの要求があれば、その要求も送る。
async fn load_pages<T: Clone + 'static>(
    load: RecordLoader<T>,
    mut list: Signal<RecordList>,
    mut items: Signal<Vec<T>>,
    options: ListOptions,
    mut request: Option<PageRequest>,
) {
    while let Some(page_request) = request {
        let reset = page_request.cursor.is_none();
        if reset {
            // 読み直しは前の条件の行を消してから読む (失敗しても古い行を残さない。
            // 0041 のレビューの指摘)。
            items.write().clear();
        }
        let result = load.call(page_request.cursor.clone(), options).await;
        let outcome = match result {
            Ok(page) => {
                let count = page.items.len();
                let next_cursor = page.next_cursor;
                items.write().extend(page.items);
                Ok((count, next_cursor))
            }
            Err(error) => Err(error),
        };
        request = list.write().finish_load(outcome);
    }
}
