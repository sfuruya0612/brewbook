//! 管理者 Worker の SQL (src/queries.rs) の単体テスト (FR-17)。
//!
//! SQL を列挙し、現れるテーブル名が users、registration_tokens、passkey_credentials の
//! 3 つだけであることを照合する (ADR-0008)。それ以外のテーブルは読み書きしない。

use std::collections::BTreeSet;

use coffee_log_admin::queries::{self, STATEMENTS, TABLES};

/// SQL の列からテーブル名の集合を作る。
fn tables_in_statements() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for sql in STATEMENTS {
        for name in queries::table_names(sql) {
            names.insert(name);
        }
    }
    names
}

#[test]
fn the_statements_are_not_empty() {
    assert!(!STATEMENTS.is_empty(), "the worker must have SQL");
    for sql in STATEMENTS {
        assert!(!sql.trim().is_empty(), "a statement must not be empty");
    }
}

#[test]
fn sql_uses_the_three_tables_only() {
    let expected: BTreeSet<String> = TABLES.iter().map(|name| (*name).to_owned()).collect();
    assert_eq!(
        tables_in_statements(),
        expected,
        "the admin worker must touch only {expected:?}"
    );
}

#[test]
fn table_names_reads_the_keywords() {
    assert_eq!(
        queries::table_names("DELETE FROM registration_tokens WHERE id = ?"),
        vec!["registration_tokens"]
    );
    assert_eq!(
        queries::table_names("INSERT INTO users (id) VALUES (?)"),
        vec!["users"]
    );
    assert_eq!(
        queries::table_names("UPDATE users SET id = ? WHERE id = ?"),
        vec!["users"]
    );
    // 部分クエリの FROM も読む。
    assert_eq!(
        queries::table_names("SELECT (SELECT COUNT(*) FROM passkey_credentials) FROM users"),
        vec!["passkey_credentials", "users"]
    );
    // キーワードの無い文からは何も読まない。
    assert!(queries::table_names("SELECT 1").is_empty());
}
