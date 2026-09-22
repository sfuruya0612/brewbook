//! `query` の単体テスト。組み立てた SQL が利用者 ID と `archived_at` の条件を必ず含むこと、
//! 値がプレースホルダで渡ることを確認する。

use coffee_log_core::cursor::CursorKey;
use coffee_log_core::query::{list, Archived, ListQuery, OrderKind, QueryError, Value};

const USER_ID: &str = "9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60";
const SHOP_ID: &str = "0d2b6f5e-3a4c-4a7b-9c8d-7e6f5a4b3c2d";

fn shop_query(archived: Archived, cursor: Option<CursorKey>, limit: u32) -> ListQuery<'static> {
    ListQuery {
        table: "shops",
        columns: "id, user_id, name, address, created_at, updated_at, archived_at",
        user_id: USER_ID,
        order_column: "created_at",
        order_kind: OrderKind::DateTime,
        archived,
        cursor,
        limit,
    }
}

#[test]
fn a_list_query_filters_by_the_user_and_excludes_archived_rows() {
    let statement = list(&shop_query(Archived::Exclude, None, 50)).unwrap();
    assert_eq!(
        statement.sql,
        "SELECT id, user_id, name, address, created_at, updated_at, archived_at FROM shops \
         WHERE user_id = ? AND archived_at IS NULL ORDER BY created_at DESC, id ASC LIMIT ?"
    );
    assert_eq!(
        statement.params,
        vec![Value::Text(USER_ID.to_owned()), Value::Integer(50)]
    );
    assert!(
        !statement.sql.contains(USER_ID),
        "the user id must not appear in the SQL: {}",
        statement.sql
    );
}

#[test]
fn a_list_query_with_include_archived_has_no_archived_filter() {
    let statement = list(&shop_query(Archived::Include, None, 50)).unwrap();
    assert!(
        !statement.sql.contains("archived_at IS NULL"),
        "include_archived must not add the archived filter: {}",
        statement.sql
    );
    assert!(statement.sql.contains("WHERE user_id = ?"));
}

#[test]
fn a_list_query_with_a_cursor_adds_the_keyset_condition() {
    let cursor = CursorKey::DateTime {
        at: "2026-09-21T12:34:56.789Z".to_owned(),
        id: SHOP_ID.to_owned(),
    };
    let statement = list(&shop_query(Archived::Exclude, Some(cursor), 10)).unwrap();
    assert_eq!(
        statement.sql,
        "SELECT id, user_id, name, address, created_at, updated_at, archived_at FROM shops \
         WHERE user_id = ? AND archived_at IS NULL \
         AND (created_at < ? OR (created_at = ? AND id > ?)) \
         ORDER BY created_at DESC, id ASC LIMIT ?"
    );
    assert_eq!(
        statement.params,
        vec![
            Value::Text(USER_ID.to_owned()),
            Value::Text("2026-09-21T12:34:56.789Z".to_owned()),
            Value::Text("2026-09-21T12:34:56.789Z".to_owned()),
            Value::Text(SHOP_ID.to_owned()),
            Value::Integer(10),
        ]
    );
}

#[test]
fn a_list_query_of_a_date_order_key_uses_the_date_column() {
    let cursor = CursorKey::Date {
        on: "2026-09-21".to_owned(),
        id: SHOP_ID.to_owned(),
    };
    let query = ListQuery {
        table: "purchases",
        columns: "id, user_id, purchased_on",
        user_id: USER_ID,
        order_column: "purchased_on",
        order_kind: OrderKind::Date,
        archived: Archived::Exclude,
        cursor: Some(cursor),
        limit: 200,
    };
    let statement = list(&query).unwrap();
    assert!(statement
        .sql
        .contains("AND (purchased_on < ? OR (purchased_on = ? AND id > ?))"));
    assert!(statement
        .sql
        .contains("ORDER BY purchased_on DESC, id ASC LIMIT ?"));
    assert_eq!(statement.params.len(), 5);
    assert_eq!(statement.params[4], Value::Integer(200));
}

#[test]
fn a_cursor_of_another_kind_is_rejected() {
    let date_cursor = CursorKey::Date {
        on: "2026-09-21".to_owned(),
        id: SHOP_ID.to_owned(),
    };
    assert_eq!(
        list(&shop_query(Archived::Exclude, Some(date_cursor), 50)),
        Err(QueryError::CursorKindMismatch)
    );
}

#[test]
fn every_list_query_keeps_the_user_and_archived_conditions() {
    for archived in [Archived::Exclude, Archived::Include] {
        for order_kind in [OrderKind::DateTime, OrderKind::Date] {
            for cursor in [None, Some(cursor_for(order_kind))] {
                let query = ListQuery {
                    table: "brews",
                    columns: "id, user_id, brewed_at",
                    user_id: USER_ID,
                    order_column: "brewed_at",
                    order_kind,
                    archived,
                    cursor,
                    limit: 50,
                };
                let statement = list(&query).unwrap();
                assert!(
                    statement.sql.contains("WHERE user_id = ?"),
                    "the user filter is missing: {}",
                    statement.sql
                );
                assert_eq!(statement.params[0], Value::Text(USER_ID.to_owned()));
                if archived == Archived::Exclude {
                    assert!(
                        statement.sql.contains("archived_at IS NULL"),
                        "the archived filter is missing: {}",
                        statement.sql
                    );
                }
                let placeholders = statement.sql.matches('?').count();
                assert_eq!(
                    placeholders,
                    statement.params.len(),
                    "every placeholder must have a value: {}",
                    statement.sql
                );
            }
        }
    }
}

fn cursor_for(order_kind: OrderKind) -> CursorKey {
    match order_kind {
        OrderKind::DateTime => CursorKey::DateTime {
            at: "2026-09-21T12:34:56.789Z".to_owned(),
            id: SHOP_ID.to_owned(),
        },
        OrderKind::Date => CursorKey::Date {
            on: "2026-09-21".to_owned(),
            id: SHOP_ID.to_owned(),
        },
    }
}
