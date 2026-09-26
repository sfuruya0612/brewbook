//! 利用者向けの Worker が実行する SQL の台帳 (0018、FR-17)。
//!
//! 取得 (SELECT) に表示名 (`display_name`) が現れず、`SELECT *` (列を明示しない取得) を
//! 使っていないことを単体テスト (`tests/test_queries.rs`) が列挙して検査する。列を明示しない
//! 取得では、取得する列が増えたときに `display_name` が応答に混ざっても検査をし抜けるため、
//! 両方を検査する。
//!
//! 台帳はこのクレートの SQL の定数と、記録のクエリを組み立てる `coffee_log_core` の列の並びを
//! 持つ。`coffee_log_core` が組み立てる SELECT は、この列の並びからしか列を選ばない。

/// このクレート (`coffee_log`) が持つ SQL の定数。
pub const STATEMENTS: &[&str] = &[
    // チャレンジ (auth/mod.rs)。
    crate::auth::INSERT_CHALLENGE,
    crate::auth::DELETE_EXPIRED_CHALLENGES,
    crate::auth::DELETE_CHALLENGE,
    crate::auth::SELECT_CHALLENGE_WITH_USER,
    crate::auth::SELECT_CHALLENGE_WITHOUT_USER,
    // ログイン (auth/login.rs)。
    crate::auth::login::SELECT_CREDENTIAL,
    crate::auth::login::UPDATE_LAST_USED,
    crate::auth::login::UPDATE_SIGN_COUNT_IF_GREATER,
    // パスキー (auth/passkeys.rs)。
    crate::auth::passkeys::SELECT_PASSKEYS,
    crate::auth::passkeys::SELECT_PASSKEY,
    crate::auth::passkeys::UPDATE_NAME,
    crate::auth::passkeys::DELETE_PASSKEY_IF_NOT_LAST,
    crate::auth::passkeys::INSERT_CREDENTIAL,
    // 登録 (auth/register.rs)。
    crate::auth::register::SELECT_TOKEN,
    crate::auth::register::MARK_TOKEN_USED,
    crate::auth::register::INSERT_CREDENTIAL,
    // セッション (auth/session.rs)。
    crate::auth::session::SELECT_SESSION,
    crate::auth::session::INSERT_SESSION,
    crate::auth::session::DELETE_SESSION,
    // D1 のバインディングの確認 (d1_check.rs)。本番の vars には無い経路が使う。
    crate::d1_check::INSERT_USER,
];

/// SQL に SELECT 文が含まれるかを返す。
pub fn has_select(sql: &str) -> bool {
    words(sql).any(|word| word.eq_ignore_ascii_case("select"))
}

/// SQL に列を明示しない取得 (`SELECT *`) が含まれるかを返す。
///
/// `SELECT COUNT(*)` のような集約は列を明示しているため含めない。`SELECT` の後に空白だけを
/// 挟んで `*` が来る箇所を探す。
pub fn selects_every_column(sql: &str) -> bool {
    let bytes = sql.as_bytes();
    for word in words_with_positions(sql) {
        if !word.text.eq_ignore_ascii_case("select") {
            continue;
        }
        let mut next = word.end;
        while next < bytes.len() && bytes[next].is_ascii_whitespace() {
            next += 1;
        }
        if bytes.get(next) == Some(&b'*') {
            return true;
        }
    }
    false
}

/// SQL の語を順に返す (英数字と `_` の並びを 1 つの語とする)。
fn words(sql: &str) -> impl Iterator<Item = &str> {
    words_with_positions(sql).map(|word| word.text)
}

/// 語とその終わりの位置。
struct Word<'a> {
    text: &'a str,
    end: usize,
}

/// SQL の語と終わりの位置を順に返す。
fn words_with_positions(sql: &str) -> impl Iterator<Item = Word<'_>> {
    let mut start = None;
    let mut words = Vec::new();
    for (index, character) in sql.char_indices() {
        let is_word = character.is_ascii_alphanumeric() || character == '_';
        match (start, is_word) {
            (None, true) => start = Some(index),
            (Some(begin), false) => {
                words.push(Word {
                    text: &sql[begin..index],
                    end: index,
                });
                start = None;
            }
            _ => {}
        }
    }
    if let Some(begin) = start {
        words.push(Word {
            text: &sql[begin..],
            end: sql.len(),
        });
    }
    words.into_iter()
}
