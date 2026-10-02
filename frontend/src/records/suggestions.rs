//! 自由記述の項目のサジェストの状態 (FR-13)。
//!
//! 入力中にサジェスト API を呼び、過去の入力値を候補として表示する。候補の選択は任意で、
//! 候補に無い値もそのまま入力できる。候補を引けなくても入力は妨げない。
//! Dioxus に依存しない純粋な型にして、native の単体テストと PBT で守る (ADR-0013)。
//! Flutter の `frontend/lib/widgets/suggestion_field.dart` と同じ動きにする。

/// 自由記述の項目の入力欄の状態。
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SuggestionState {
    /// 現在の入力の値。
    query: String,

    /// 表示中の候補。
    options: Vec<String>,

    /// 送った要求の世代。遅れて届いた応答を捨てるために使う。
    sequence: u64,

    /// 候補を表示しているか。
    open: bool,
}

impl SuggestionState {
    /// 空の状態を作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// 現在の入力の値。
    pub fn query(&self) -> &str {
        &self.query
    }

    /// 表示中の候補。
    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// 候補を表示しているか。
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// 入力が変わったことを記録し、要求の世代を進める。
    ///
    /// 前の候補は応答が届くまで残す (入力のたびに一覧が消えないようにする)。戻り値の世代を
    /// [`SuggestionState::apply_options`] に渡し、古い応答を捨てる。
    pub fn begin(&mut self, query: &str) -> u64 {
        self.query = query.to_string();
        self.sequence += 1;
        self.sequence
    }

    /// 応答の候補を反映する。古い世代の応答は捨てる (FR-13)。
    ///
    /// 反映したときは true を返す。候補が無いときは表示を閉じる (候補を引けなくても入力は続けられる)。
    pub fn apply_options(&mut self, sequence: u64, values: Vec<String>) -> bool {
        if sequence != self.sequence {
            return false;
        }
        self.options = values;
        self.open = !self.options.is_empty();
        true
    }

    /// 候補を選び、入力の値をその候補にする (FR-13)。表示を閉じる。
    pub fn select(&mut self, value: &str) -> String {
        self.query = value.to_string();
        self.options.clear();
        self.open = false;
        self.query.clone()
    }

    /// 表示を閉じる (フォーカスが外れたときなど)。
    pub fn close(&mut self) {
        self.open = false;
    }

    /// 候補を入力の値と比べ、一致した先頭部分と残りに分ける (FR-13)。
    ///
    /// 一致は前後の空白を除いた入力の小文字との前方一致で見る (Flutter と同じ)。
    /// 一致した先頭部分は `crema-ink` の 600 で示す (docs/design/components/Field)。
    pub fn highlight(&self, option: &str) -> (String, String) {
        highlight_parts(option, &self.query)
    }
}

/// 候補を入力の値と比べ、一致した先頭部分と残りに分ける。
pub fn highlight_parts(option: &str, query: &str) -> (String, String) {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return (String::new(), option.to_string());
    }
    let lower = option.to_lowercase();
    if !lower.starts_with(&query) {
        return (String::new(), option.to_string());
    }
    // 小文字化でバイト数が変わる文字 (ß など) は先頭部分を切り出せないため、全体を残りにする。
    if lower.len() != option.len() {
        return (String::new(), option.to_string());
    }
    let (prefix, rest) = option.split_at(query.len());
    (prefix.to_string(), rest.to_string())
}
