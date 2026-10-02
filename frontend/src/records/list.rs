//! カーソル方式の一覧の状態 (FR-12、PRD の性能)。
//!
//! 一覧の画面と選択のシートが共有する、ページングと「アーカイブ済みを含める」の切り替えの
//! 状態機械。Dioxus に依存しない純粋な型にして、native の単体テストと PBT で守る (ADR-0013)。
//! Flutter の `frontend/lib/widgets/record_list_view.dart` と同じ動きにする。

use super::RecordError;

/// 末尾までの残りの高さがこの値 (px) を下回ったら、次のページを読む。
pub const LOAD_MORE_THRESHOLD: f64 = 200.0;

/// 一覧の 1 ページの要求。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PageRequest {
    /// 読むページのカーソル。先頭から読むときは None。
    pub cursor: Option<String>,

    /// アーカイブ済みの行を含めるか (FR-12)。
    pub include_archived: bool,
}

/// カーソル方式の一覧の状態。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RecordList {
    /// アーカイブ済みを含めるか。
    include_archived: bool,

    /// 1 ページ以上読めたか (読み込み中の表示と、記録が無い表示の区別に使う)。
    loaded: bool,

    /// 続きを引くカーソル。続きが無ければ None。
    next_cursor: Option<String>,

    /// 読み込み中か。
    loading: bool,

    /// 読み込み中に届いた読み直しの要求 (完了後に実行する)。
    pending_reset: bool,

    /// 読めた行の数。
    item_count: usize,

    /// 直近の読み込みの失敗。表示は画面が決める。
    error: Option<RecordError>,
}

impl Default for RecordList {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordList {
    /// 初期状態を作る。最初の読み込みは [`RecordList::reset`] で始める。
    pub fn new() -> Self {
        Self {
            include_archived: false,
            loaded: false,
            next_cursor: None,
            loading: false,
            pending_reset: false,
            item_count: 0,
            error: None,
        }
    }

    /// アーカイブ済みを含めるか (FR-12)。
    pub fn include_archived(&self) -> bool {
        self.include_archived
    }

    /// 1 ページ以上読めたか。
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// 読み込み中か。
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// 続きを引くカーソル。
    pub fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }

    /// 読めた行の数。
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    /// 直近の読み込みの失敗。
    pub fn error(&self) -> Option<&RecordError> {
        self.error.as_ref()
    }

    /// 先頭からの読み直しを始める。読み込み中なら完了後に実行する。
    pub fn reset(&mut self) -> Option<PageRequest> {
        self.begin_load(true)
    }

    /// アーカイブ済みを含める切り替えを反転し、先頭から読み直す (FR-12)。
    pub fn toggle_include_archived(&mut self) -> Option<PageRequest> {
        self.include_archived = !self.include_archived;
        self.begin_load(true)
    }

    /// 読み込みに失敗した後に、先頭から読み直す。
    pub fn retry(&mut self) -> Option<PageRequest> {
        self.begin_load(true)
    }

    /// 末尾まで来たときに次のページを読む。続きが無いか読み込み中なら何もしない。
    pub fn load_more(&mut self) -> Option<PageRequest> {
        self.next_cursor.as_ref()?;
        self.begin_load(false)
    }

    /// 末尾までの残りの高さから、次のページを読むべきか (Flutter の `loadMoreThreshold`)。
    pub fn should_load_more(&self, remaining_px: f64) -> bool {
        !self.loading && self.next_cursor.is_some() && remaining_px < LOAD_MORE_THRESHOLD
    }

    /// ページの読み込みを始める。[`PageRequest`] が返るときは、その要求を送る。
    ///
    /// 読み込み中に届いた読み直しの要求は、完了後に実行する (古い表示を残さない)。
    pub fn begin_load(&mut self, reset: bool) -> Option<PageRequest> {
        if self.loading {
            if reset {
                self.pending_reset = true;
            }
            return None;
        }
        self.loading = true;
        if reset {
            self.item_count = 0;
            self.next_cursor = None;
        }
        // 読み込みを始めるときは直近の失敗を消す (続きの読み込みの成功の後に前の失敗の
        // バナーが残らないようにする。0041 のレビューの指摘)。
        self.error = None;
        Some(PageRequest {
            cursor: if reset {
                None
            } else {
                self.next_cursor.clone()
            },
            include_archived: self.include_archived,
        })
    }

    /// ページの読み込みを終える。読み込み中に届いた読み直しがあれば、その要求を返す。
    pub fn finish_load(
        &mut self,
        result: Result<(usize, Option<String>), RecordError>,
    ) -> Option<PageRequest> {
        self.loading = false;
        match result {
            Ok((count, next_cursor)) => {
                self.item_count += count;
                self.next_cursor = next_cursor;
                self.loaded = true;
                self.error = None;
            }
            Err(error) => {
                self.error = Some(error);
            }
        }
        if self.pending_reset {
            self.pending_reset = false;
            return self.begin_load(true);
        }
        None
    }
}
