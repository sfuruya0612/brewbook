//! `query` の PBT。サジェスト (FR-13) の `LIKE` のパターンのエスケープの性質を検査する。
//!
//! 生成した任意の値をエスケープしたパターンから元の値に戻せることと、
//! エスケープされていないワイルドカードがパターンに残らないことを確認する
//! (値がワイルドカードとして扱われないことの根拠になる)。

use coffee_log_core::query::{self, SuggestionItem, Value};
use proptest::prelude::*;

/// 検査に使う利用者 ID。
const USER_ID: &str = "9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60";

/// アカウント削除 (FR-15) が対象にする 11 テーブルと、利用者を指す列の名前 (ADR-0006)。
/// 外部キーの参照元から先に消す順に並べる。`users` だけは主キーの `id` で引く。
const ACCOUNT_DELETE_TABLES: &[(&str, &str)] = &[
    ("product_flavor_tags", "user_id"),
    ("brews", "user_id"),
    ("purchases", "user_id"),
    ("flavor_tags", "user_id"),
    ("products", "user_id"),
    ("shops", "user_id"),
    ("sessions", "user_id"),
    ("passkey_credentials", "user_id"),
    ("webauthn_challenges", "user_id"),
    ("registration_tokens", "user_id"),
    ("users", "id"),
];

/// 利用者 ID と購入 ID に使う値。実際の ID (UUID) と同じ文字だけにする。
fn identifier() -> impl Strategy<Value = String> {
    "[0-9a-f-]{1,36}".prop_map(String::from)
}

/// 入力中の文字列の候補。ワイルドカードとエスケープ文字と日本語を含める。
fn query_text() -> impl Strategy<Value = String> {
    prop_oneof!["[a-zA-Z0-9%_\\\\]{0,10}", "[ぁ-ん一-龠]{0,6}", ".{0,20}",].prop_map(String::from)
}

/// エスケープしたパターンを元の値に戻す (テスト側の逆変換)。
fn unescape_like(pattern: &str) -> String {
    let mut text = String::with_capacity(pattern.len());
    let mut characters = pattern.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            text.push(
                characters
                    .next()
                    .expect("an escape character must be followed by a character"),
            );
        } else {
            text.push(character);
        }
    }
    text
}

/// エスケープされていないワイルドカード (`%` と `_`) があるか。
fn has_unescaped_wildcard(pattern: &str) -> bool {
    let mut characters = pattern.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            characters.next();
        } else if matches!(character, '%' | '_') {
            return true;
        }
    }
    false
}

/// サジェストの文に束縛されるパターンを取り出す。
fn bound_pattern(q: &str) -> String {
    let statement = query::suggestions(USER_ID, SuggestionItem::Producer, q);
    match statement.params.get(1) {
        Some(Value::Text(pattern)) => pattern.clone(),
        other => panic!("the pattern must be bound as text: {other:?}"),
    }
}

proptest! {
    /// 任意の値のエスケープは往復する (値の文字がそのままの文字として扱われる)。
    #[test]
    fn the_like_pattern_restores_the_given_value(q in query_text()) {
        let pattern = bound_pattern(&q);
        prop_assert_eq!(unescape_like(&pattern), q);
    }

    /// エスケープしたパターンには、エスケープされていないワイルドカードが残らない。
    /// (SQL がパターンに足す `%` はエスケープの対象の外にある。)
    #[test]
    fn the_like_pattern_has_no_unescaped_wildcard(q in query_text()) {
        let pattern = bound_pattern(&q);
        prop_assert!(!has_unescaped_wildcard(&pattern), "unexpected pattern {pattern:?}");
    }

    /// アカウント削除 (FR-15) の文は、利用者に属する行を持つ 11 テーブルを参照元から先の順に
    /// 覆い、どれも与えられた利用者 ID だけを束縛する。
    #[test]
    fn the_account_delete_statements_cover_every_table_in_dependency_order(user in identifier()) {
        let statements = query::account_delete(&user);
        prop_assert_eq!(statements.len(), ACCOUNT_DELETE_TABLES.len());
        for (statement, (table, column)) in statements.iter().zip(ACCOUNT_DELETE_TABLES) {
            let expected = format!("DELETE FROM {table} WHERE {column} = ?");
            prop_assert_eq!(statement.sql.as_str(), expected.as_str());
            prop_assert_eq!(statement.params.len(), 1);
            match &statement.params[0] {
                Value::Text(text) => prop_assert_eq!(text, &user),
                other => prop_assert!(false, "the user id must be bound as text: {other:?}"),
            }
        }
    }
}
