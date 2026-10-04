//! `records::list` の単体テスト (0041)。
//!
//! カーソル方式のページングを確かめる (完了条件 2)。

use brew_book_frontend::records::{PageRequest, RecordError, RecordList, LOAD_MORE_THRESHOLD};

/// 1 ページ読めたことにする。
fn page(list: &mut RecordList, count: usize, next: Option<&str>) {
    let request = list.finish_load(Ok((count, next.map(str::to_string))));
    assert!(request.is_none(), "no pending reset must be scheduled");
}

#[test]
fn the_first_load_requests_the_first_page() {
    let mut list = RecordList::new();
    let request = list.reset().expect("the first load must be requested");
    assert_eq!(request, PageRequest { cursor: None });
    assert!(list.is_loading());
    page(&mut list, 50, Some("cur"));
    assert!(list.is_loaded());
    assert_eq!(list.item_count(), 50);
    assert_eq!(list.next_cursor(), Some("cur"));
    assert!(!list.is_loading());
}

#[test]
fn the_next_page_uses_the_cursor_of_the_previous_page() {
    let mut list = RecordList::new();
    let _ = list.reset();
    page(&mut list, 50, Some("cur"));

    let request = list.load_more().expect("the next page must be requested");
    assert_eq!(
        request,
        PageRequest {
            cursor: Some("cur".to_string()),
        }
    );
    page(&mut list, 3, None);
    assert_eq!(list.item_count(), 53);
    assert_eq!(list.next_cursor(), None);
    // 続きが無ければ、これ以上読まない。
    assert!(list.load_more().is_none());
}

#[test]
fn the_next_page_is_loaded_only_near_the_end() {
    let mut list = RecordList::new();
    let _ = list.reset();
    page(&mut list, 50, Some("cur"));

    assert!(!list.should_load_more(LOAD_MORE_THRESHOLD));
    assert!(list.should_load_more(LOAD_MORE_THRESHOLD - 1.0));

    // 読み込み中は重ねて要求しない。
    let _ = list.load_more();
    assert!(!list.should_load_more(0.0));
}

#[test]
fn a_reset_while_loading_is_run_after_the_page_finishes() {
    let mut list = RecordList::new();
    let _ = list.reset();
    assert!(list.reset().is_none(), "the load is already running");

    let request = list
        .finish_load(Ok((50, Some("cur".to_string()))))
        .expect("the pending reset must be requested");
    assert_eq!(request, PageRequest { cursor: None });
    page(&mut list, 1, None);
    assert_eq!(list.item_count(), 1);
}

#[test]
fn a_failed_load_keeps_the_cursor_and_retries_from_the_first_page() {
    let mut list = RecordList::new();
    let _ = list.reset();
    let _ = list.finish_load(Err(RecordError::Format("broken".to_string())));
    assert!(list.error().is_some());
    assert!(!list.is_loaded());
    assert_eq!(list.item_count(), 0);

    let request = list.retry().expect("the retry must be requested");
    assert_eq!(request.cursor, None);
    assert!(list.error().is_none());
    page(&mut list, 1, None);
    assert!(list.is_loaded());
}

/// 続きの読み込みの失敗の後でも、次の読み込みの成功で失敗の表示が消える (0041 のレビューの指摘)。
#[test]
fn a_successful_load_clears_the_previous_error() {
    let mut list = RecordList::new();
    let _ = list.reset();
    page(&mut list, 1, Some("cur"));

    let _ = list.load_more().expect("the next page must be requested");
    let _ = list.finish_load(Err(RecordError::Format("broken".to_string())));
    assert!(list.error().is_some());

    let _ = list.load_more().expect("the next page must be requested");
    assert!(
        list.error().is_none(),
        "starting a load must clear the previous error"
    );
    page(&mut list, 1, None);
    assert!(list.error().is_none());
    assert!(list.is_loaded());
}

/// 読み直しの失敗でも、次の読み直しの要求が出せる (古い行を残さないための再読み込み)。
#[test]
fn a_failed_reset_can_be_retried() {
    let mut list = RecordList::new();
    let _ = list.reset();
    page(&mut list, 1, None);

    let request = list.reset().expect("the reset must be requested");
    assert_eq!(request.cursor, None);
    let _ = list.finish_load(Err(RecordError::Format("broken".to_string())));
    assert!(list.error().is_some());

    let request = list.retry().expect("the retry must be requested");
    assert_eq!(request.cursor, None);
    assert!(list.error().is_none());
}
