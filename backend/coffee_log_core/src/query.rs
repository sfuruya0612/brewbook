//! 一覧クエリの共通部分の組み立て (ADR-0006)。
//!
//! 全てのクエリに `user_id` の条件を必ず付け、一覧のクエリには `archived_at` の条件を付ける。
//! 値は必ずプレースホルダ (`?`) で渡し、SQL に値を連結しない。テーブル名と列名はコード内の
//! 定数だけを使い、利用者の入力は渡せない。個別のクエリは 0006 以降がこのモジュールに追加する。

use crate::cursor::CursorKey;
use crate::error::ErrorCode;

/// SQL に束縛する値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// 文字列。
    Text(String),
    /// 整数。
    Integer(i64),
}

/// アーカイブ済みの行の扱い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Archived {
    /// 既定。アーカイブ済みを除く (`archived_at IS NULL`)。
    Exclude,
    /// アーカイブ済みも返す (`include_archived=true` のときだけ指定する)。
    Include,
}

/// 並び順のキーの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderKind {
    /// 日時 (ISO 8601 UTC) の降順と ID の昇順。
    DateTime,
    /// 日付 (`YYYY-MM-DD`) の降順と ID の昇順。
    Date,
}

/// 一覧クエリの入力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListQuery<'a> {
    /// 対象のテーブル名。コード内の定数だけを渡す。
    pub table: &'static str,
    /// 選択する列の並び。コード内の定数だけを渡す。
    pub columns: &'static str,
    /// 絞り込む利用者の ID。
    pub user_id: &'a str,
    /// 並び順のキーの列名。コード内の定数だけを渡す。
    pub order_column: &'static str,
    /// 並び順のキーの種類。
    pub order_kind: OrderKind,
    /// アーカイブ済みの行の扱い。
    pub archived: Archived,
    /// 直前のページの最後の行を指すカーソル。先頭から引くときは None。
    pub cursor: Option<CursorKey>,
    /// 取得件数。
    pub limit: u32,
}

/// 組み立てた SQL と、プレースホルダに束縛する値 (SQL に現れる順)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub sql: String,
    pub params: Vec<Value>,
}

/// クエリの組み立ての誤り。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    /// カーソルが並び順のキーの種類と一致しない。
    CursorKindMismatch,
}

impl QueryError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        match self {
            QueryError::CursorKindMismatch => "invalid cursor",
        }
    }
}

/// 一覧の SQL を組み立てる。
///
/// `SELECT <columns> FROM <table> WHERE user_id = ? [AND archived_at IS NULL]
/// [AND (<order_column> < ? OR (<order_column> = ? AND id > ?))]
/// ORDER BY <order_column> DESC, id ASC LIMIT ?`
pub fn list(query: &ListQuery<'_>) -> Result<Statement, QueryError> {
    let mut sql = String::new();
    let mut params = Vec::new();

    sql.push_str("SELECT ");
    sql.push_str(query.columns);
    sql.push_str(" FROM ");
    sql.push_str(query.table);
    sql.push_str(" WHERE user_id = ?");
    params.push(Value::Text(query.user_id.to_owned()));

    if query.archived == Archived::Exclude {
        sql.push_str(" AND archived_at IS NULL");
    }

    if let Some(cursor) = &query.cursor {
        let key = match (cursor, query.order_kind) {
            (CursorKey::DateTime { at, .. }, OrderKind::DateTime) => at,
            (CursorKey::Date { on, .. }, OrderKind::Date) => on,
            _ => return Err(QueryError::CursorKindMismatch),
        };
        // 並び順のキーの降順と ID の昇順の続きを引く。
        sql.push_str(" AND (");
        sql.push_str(query.order_column);
        sql.push_str(" < ? OR (");
        sql.push_str(query.order_column);
        sql.push_str(" = ? AND id > ?))");
        params.push(Value::Text(key.clone()));
        params.push(Value::Text(key.clone()));
        params.push(Value::Text(cursor.id().to_owned()));
    }

    sql.push_str(" ORDER BY ");
    sql.push_str(query.order_column);
    sql.push_str(" DESC, id ASC LIMIT ?");
    params.push(Value::Integer(i64::from(query.limit)));

    Ok(Statement { sql, params })
}
