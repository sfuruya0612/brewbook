//! カーソル方式の一覧の共通の枠。
//!
//! 末尾までのスクロールでの追加読み込みを持つ。中身の表示は `row` が決める。Flutter の
//! `frontend/lib/widgets/record_list_view.dart` と同じ動きにする。

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::i18n::{t, Key};
use crate::records::{PageRequest, RecordError, RecordList, RecordPage};
use crate::ui::BrewbookMark;

use super::{error_banner, retryable_banner};

/// 一覧の 1 ページを読む未来。
pub type LoadFuture<T> = Pin<Box<dyn Future<Output = Result<RecordPage<T>, RecordError>>>>;

/// 一覧の 1 ページを読む関数。画面ごとに API の呼び出しを閉じ込める。
pub struct RecordLoader<T>(Rc<dyn Fn(Option<String>) -> LoadFuture<T>>);

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
    pub fn new(load: impl Fn(Option<String>) -> LoadFuture<T> + 'static) -> Self {
        Self(Rc::new(load))
    }

    /// 1 ページを読む。
    pub fn call(&self, cursor: Option<String>) -> LoadFuture<T> {
        (self.0)(cursor)
    }
}

/// カーソル方式の一覧の枠。
///
/// マウント時と、記録の変更の通知 ([`super::mark_records_changed`]) のたびに先頭から読み直す。
#[component]
pub fn RecordListView<T: Clone + PartialEq + 'static>(
    /// 一覧の 1 ページを読む。
    load: RecordLoader<T>,

    /// 行の組み立て。広い画面の選択中の地は親が決める。
    row: Callback<T, Element>,

    /// 記録が無いときに、次にすることを示す 1 文 (ARB から取る)。
    #[props(default)]
    empty_hint: Option<String>,
) -> Element {
    let mut list = use_signal(RecordList::new);
    let items = use_signal(Vec::<T>::new);
    let revision = use_context::<Signal<u64>>();

    // マウント時と、記録が変わったときに先頭から読み直す (revision の読み取りで再実行される)。
    let effect_load = load.clone();
    use_effect(move || {
        let _ = revision();
        let request = list.write().reset();
        if let Some(request) = request {
            let load = effect_load.clone();
            spawn(async move {
                load_pages(load, list, items, Some(request)).await;
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
    let scroll_load = load.clone();

    rsx! {
        div { class: "list-view",
            if let Some(error) = error {
                div { class: "list-error",
                    if crate::records::record_error_retry(&error) {
                        {retryable_banner(&error, EventHandler::new(move |_| {
                            let request = list.write().retry();
                            if let Some(request) = request {
                                let load = retry_load.clone();
                                spawn(async move {
                                    load_pages(load, list, items, Some(request)).await;
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
                        let request = list.write().load_more();
                        if let Some(request) = request {
                            let load = scroll_load.clone();
                            spawn(async move {
                                load_pages(load, list, items, Some(request)).await;
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

/// ページを続けて読む。読み込み中に届いた読み直しの要求があれば、その要求も送る。
async fn load_pages<T: Clone + 'static>(
    load: RecordLoader<T>,
    mut list: Signal<RecordList>,
    mut items: Signal<Vec<T>>,
    mut request: Option<PageRequest>,
) {
    while let Some(page_request) = request {
        let reset = page_request.cursor.is_none();
        if reset {
            // 読み直しは前の条件の行を消してから読む (失敗しても古い行を残さない。
            // 0041 のレビューの指摘)。
            items.write().clear();
        }
        let result = load.call(page_request.cursor.clone()).await;
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
