//! 記録のクエリの組み立て (ADR-0006)。
//!
//! 全てのクエリに `user_id` の条件を必ず付け、一覧のクエリには `archived_at` の条件を付ける。
//! 値は必ずプレースホルダ (`?`) で渡し、SQL に値を連結しない。テーブル名と列名はコード内の
//! 定数だけを使い、利用者の入力は渡せない。
//!
//! 一覧と 1 件の取得は共通の関数 ([`list`] と [`find_one`]) を通して組み立て、店と商品と
//! Flavor Notes のタグの個別のクエリもこのモジュールが持つ。0007 と 0008 と 0010 と 0011 も
//! このモジュールを使う。

use crate::cursor::CursorKey;
use crate::error::ErrorCode;

/// SQL に束縛する値。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// NULL。
    Null,
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

/// `include_archived` のクエリパラメータの誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeArchivedError {
    /// `true` と `false` 以外の値。
    NotABoolean,
}

impl IncludeArchivedError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "include_archived must be true or false"
    }
}

/// `include_archived` のクエリパラメータを解釈する。省略時は既定 (アーカイブ済みを除く) を返す。
pub fn parse_include_archived(text: Option<&str>) -> Result<Archived, IncludeArchivedError> {
    match text {
        None | Some("false") => Ok(Archived::Exclude),
        Some("true") => Ok(Archived::Include),
        Some(_) => Err(IncludeArchivedError::NotABoolean),
    }
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

/// 1 件の取得のクエリの入力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindQuery<'a> {
    /// 対象のテーブル名。コード内の定数だけを渡す。
    pub table: &'static str,
    /// 選択する列の並び。コード内の定数だけを渡す。
    pub columns: &'static str,
    /// 絞り込む利用者の ID。
    pub user_id: &'a str,
    /// 取得する行の ID。
    pub id: &'a str,
    /// アーカイブ済みの行の扱い。アーカイブ済みも返すときは Include (FR-12)。
    pub archived: Archived,
}

/// 行の挿入のクエリの入力。
///
/// 主キーの `id` と `user_id` はこの関数が必ず先頭の列として置くため、`columns` と `values` に
/// 含めない。値は列と同じ数だけ渡す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertQuery<'a> {
    /// 対象のテーブル名。コード内の定数だけを渡す。
    pub table: &'static str,
    /// `id` と `user_id` を除く、挿入する列の並び。コード内の定数だけを渡す。
    pub columns: &'static [&'static str],
    /// 列と同じ数の値。
    pub values: Vec<Value>,
    /// 挿入する行の ID。
    pub id: &'a str,
    /// 行の利用者。
    pub user_id: &'a str,
}

/// 行の更新のクエリの入力。
///
/// 更新する行は `id` と `user_id` の両方で絞る。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateQuery<'a> {
    /// 対象のテーブル名。コード内の定数だけを渡す。
    pub table: &'static str,
    /// `SET` する列の並び。コード内の定数だけを渡す。
    pub columns: &'static [&'static str],
    /// 列と同じ数の値。
    pub values: Vec<Value>,
    /// 更新する行の ID。
    pub id: &'a str,
    /// 行の利用者。
    pub user_id: &'a str,
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
    /// 列の数と値の数が一致しない (コードの誤り)。
    ColumnCountMismatch,
}

impl QueryError {
    /// 応答のエラーの種別。
    pub fn code(self) -> ErrorCode {
        match self {
            QueryError::CursorKindMismatch => ErrorCode::BadRequest,
            QueryError::ColumnCountMismatch => ErrorCode::Internal,
        }
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        match self {
            QueryError::CursorKindMismatch => "invalid cursor",
            QueryError::ColumnCountMismatch => "the number of columns and values must match",
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

/// 1 件の取得の SQL を組み立てる。
///
/// `SELECT <columns> FROM <table> WHERE id = ? AND user_id = ? [AND archived_at IS NULL]`
pub fn find_one(query: &FindQuery<'_>) -> Statement {
    let mut sql = String::new();
    sql.push_str("SELECT ");
    sql.push_str(query.columns);
    sql.push_str(" FROM ");
    sql.push_str(query.table);
    sql.push_str(" WHERE id = ? AND user_id = ?");
    if query.archived == Archived::Exclude {
        sql.push_str(" AND archived_at IS NULL");
    }
    Statement {
        sql,
        params: vec![
            Value::Text(query.id.to_owned()),
            Value::Text(query.user_id.to_owned()),
        ],
    }
}

/// 挿入の SQL を組み立てる。
///
/// `INSERT INTO <table> (id, user_id, <columns>) VALUES (?, ?, ...)`
pub fn insert(query: &InsertQuery<'_>) -> Result<Statement, QueryError> {
    if query.columns.len() != query.values.len() {
        return Err(QueryError::ColumnCountMismatch);
    }
    let mut sql = String::new();
    sql.push_str("INSERT INTO ");
    sql.push_str(query.table);
    sql.push_str(" (id, user_id");
    for column in query.columns {
        sql.push_str(", ");
        sql.push_str(column);
    }
    sql.push_str(") VALUES (?, ?");
    for _ in query.columns {
        sql.push_str(", ?");
    }
    sql.push(')');

    let mut params = vec![
        Value::Text(query.id.to_owned()),
        Value::Text(query.user_id.to_owned()),
    ];
    params.extend(query.values.iter().cloned());
    Ok(Statement { sql, params })
}

/// 更新の SQL を組み立てる。更新する行は `id` と `user_id` の両方で絞る。
///
/// `UPDATE <table> SET <columns> = ?, ... WHERE id = ? AND user_id = ?`
pub fn update(query: &UpdateQuery<'_>) -> Result<Statement, QueryError> {
    if query.columns.len() != query.values.len() {
        return Err(QueryError::ColumnCountMismatch);
    }
    let mut sql = String::new();
    sql.push_str("UPDATE ");
    sql.push_str(query.table);
    sql.push_str(" SET ");
    for (index, column) in query.columns.iter().enumerate() {
        if index > 0 {
            sql.push_str(", ");
        }
        sql.push_str(column);
        sql.push_str(" = ?");
    }
    sql.push_str(" WHERE id = ? AND user_id = ?");

    let mut params = query.values.clone();
    params.push(Value::Text(query.id.to_owned()));
    params.push(Value::Text(query.user_id.to_owned()));
    Ok(Statement { sql, params })
}

/// 店のテーブル名。
pub const SHOPS_TABLE: &str = "shops";
/// 店の列の並び。応答の JSON の項目と同じ。
pub const SHOP_COLUMNS: &str = "id, user_id, name, address, created_at, updated_at, archived_at";
/// 商品のテーブル名。
pub const PRODUCTS_TABLE: &str = "products";
/// 商品の列の並び。応答の JSON の項目と同じ。
pub const PRODUCT_COLUMNS: &str = "id, user_id, name, producer, origin, region, process, variety, \
                                   created_at, updated_at, archived_at";
/// Flavor Notes のタグのテーブル名。
pub const FLAVOR_TAGS_TABLE: &str = "flavor_tags";
/// Flavor Notes のタグの列の並び。応答の JSON の項目と同じ。
pub const FLAVOR_TAG_COLUMNS: &str = "id, user_id, name";
/// 商品と Flavor Notes のタグの対応のテーブル名。
pub const PRODUCT_FLAVOR_TAGS_TABLE: &str = "product_flavor_tags";
/// 1 つのクエリに束縛できる値の数。D1 は 100 個までとする
/// (202 個を束縛したクエリを D1 が拒否することをローカルで確認した)。
pub const MAX_BOUND_VALUES: usize = 100;
/// タグ名を引くクエリは利用者 ID を 2 つ束縛するため、商品 ID は 98 件ずつにする。
const PRODUCT_IDS_PER_STATEMENT: usize = MAX_BOUND_VALUES - 2;

/// 店の `INSERT` の列 (`id` と `user_id` を除く)。
const SHOP_INSERT_COLUMNS: &[&str] = &["name", "address", "created_at", "updated_at"];
/// 店の `UPDATE` の列。`updated_at` は更新のたびに現在時刻にする (ADR-0006)。
const SHOP_UPDATE_COLUMNS: &[&str] = &["name", "address", "updated_at"];
/// 商品の `INSERT` の列 (`id` と `user_id` を除く)。
const PRODUCT_INSERT_COLUMNS: &[&str] = &[
    "name",
    "producer",
    "origin",
    "region",
    "process",
    "variety",
    "created_at",
    "updated_at",
];
/// 商品の `UPDATE` の列。
const PRODUCT_UPDATE_COLUMNS: &[&str] = &[
    "name",
    "producer",
    "origin",
    "region",
    "process",
    "variety",
    "updated_at",
];
/// アーカイブとアーカイブ解除の `UPDATE` の列 (ADR-0006)。
const ARCHIVED_COLUMNS: &[&str] = &["archived_at", "updated_at"];

/// 店の入力の値。NULL は None で表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShopValues<'a> {
    /// 店名。
    pub name: &'a str,
    /// 住所。任意。
    pub address: Option<&'a str>,
}

/// 商品の入力の値。NULL は None で表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductValues<'a> {
    /// 商品名。
    pub name: &'a str,
    /// 生産者。任意。
    pub producer: Option<&'a str>,
    /// 生産国。任意。
    pub origin: Option<&'a str>,
    /// 地域。任意。
    pub region: Option<&'a str>,
    /// 精製方法。任意。
    pub process: Option<&'a str>,
    /// 品種。任意。
    pub variety: Option<&'a str>,
}

/// 店の一覧を組み立てる。並び順は作成日時の降順と ID の昇順。
pub fn shops_list(
    user_id: &str,
    archived: Archived,
    cursor: Option<CursorKey>,
    limit: u32,
) -> Result<Statement, QueryError> {
    list(&ListQuery {
        table: SHOPS_TABLE,
        columns: SHOP_COLUMNS,
        user_id,
        order_column: "created_at",
        order_kind: OrderKind::DateTime,
        archived,
        cursor,
        limit,
    })
}

/// 店を 1 件取得する SQL を組み立てる。
pub fn shop_find(user_id: &str, id: &str, archived: Archived) -> Statement {
    find_one(&FindQuery {
        table: SHOPS_TABLE,
        columns: SHOP_COLUMNS,
        user_id,
        id,
        archived,
    })
}

/// 店を挿入する SQL を組み立てる。
pub fn shop_insert(
    id: &str,
    user_id: &str,
    values: &ShopValues<'_>,
    created_at: &str,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    insert(&InsertQuery {
        table: SHOPS_TABLE,
        columns: SHOP_INSERT_COLUMNS,
        values: vec![
            Value::Text(values.name.to_owned()),
            optional_text(values.address),
            Value::Text(created_at.to_owned()),
            Value::Text(updated_at.to_owned()),
        ],
        id,
        user_id,
    })
}

/// 店を更新する SQL を組み立てる。
pub fn shop_update(
    id: &str,
    user_id: &str,
    values: &ShopValues<'_>,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    update(&UpdateQuery {
        table: SHOPS_TABLE,
        columns: SHOP_UPDATE_COLUMNS,
        values: vec![
            Value::Text(values.name.to_owned()),
            optional_text(values.address),
            Value::Text(updated_at.to_owned()),
        ],
        id,
        user_id,
    })
}

/// 店のアーカイブとアーカイブ解除の SQL を組み立てる。
/// `archived_at` が None のときはアーカイブ解除になる。
pub fn shop_set_archived(
    id: &str,
    user_id: &str,
    archived_at: Option<&str>,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    set_archived(SHOPS_TABLE, id, user_id, archived_at, updated_at)
}

/// 商品の一覧を組み立てる。並び順は作成日時の降順と ID の昇順。
pub fn products_list(
    user_id: &str,
    archived: Archived,
    cursor: Option<CursorKey>,
    limit: u32,
) -> Result<Statement, QueryError> {
    list(&ListQuery {
        table: PRODUCTS_TABLE,
        columns: PRODUCT_COLUMNS,
        user_id,
        order_column: "created_at",
        order_kind: OrderKind::DateTime,
        archived,
        cursor,
        limit,
    })
}

/// 商品を 1 件取得する SQL を組み立てる。
pub fn product_find(user_id: &str, id: &str, archived: Archived) -> Statement {
    find_one(&FindQuery {
        table: PRODUCTS_TABLE,
        columns: PRODUCT_COLUMNS,
        user_id,
        id,
        archived,
    })
}

/// 商品を挿入する SQL を組み立てる。
pub fn product_insert(
    id: &str,
    user_id: &str,
    values: &ProductValues<'_>,
    created_at: &str,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    let mut product_values = product_value_list(values);
    product_values.push(Value::Text(created_at.to_owned()));
    product_values.push(Value::Text(updated_at.to_owned()));
    insert(&InsertQuery {
        table: PRODUCTS_TABLE,
        columns: PRODUCT_INSERT_COLUMNS,
        values: product_values,
        id,
        user_id,
    })
}

/// 商品を更新する SQL を組み立てる。
pub fn product_update(
    id: &str,
    user_id: &str,
    values: &ProductValues<'_>,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    let mut product_values = product_value_list(values);
    product_values.push(Value::Text(updated_at.to_owned()));
    update(&UpdateQuery {
        table: PRODUCTS_TABLE,
        columns: PRODUCT_UPDATE_COLUMNS,
        values: product_values,
        id,
        user_id,
    })
}

/// 商品のアーカイブとアーカイブ解除の SQL を組み立てる。
pub fn product_set_archived(
    id: &str,
    user_id: &str,
    archived_at: Option<&str>,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    set_archived(PRODUCTS_TABLE, id, user_id, archived_at, updated_at)
}

/// 利用者の Flavor Notes のタグの一覧を組み立てる。並び順は名前の昇順。
pub fn flavor_tags_list(user_id: &str) -> Statement {
    let mut sql = String::new();
    sql.push_str("SELECT ");
    sql.push_str(FLAVOR_TAG_COLUMNS);
    sql.push_str(" FROM ");
    sql.push_str(FLAVOR_TAGS_TABLE);
    sql.push_str(" WHERE user_id = ? ORDER BY name ASC");
    Statement {
        sql,
        params: vec![Value::Text(user_id.to_owned())],
    }
}

/// 商品の Flavor Notes のタグ名を引く SQL を、束縛する値の上限に収まるよう分けて組み立てる。
///
/// 返す文の並びは、商品 ID を [`PRODUCT_IDS_PER_STATEMENT`] 件ずつに分けたもの。
/// 空の並びに対しては空の並びを返す。同じ商品のタグ名は名前の昇順で返す。
pub fn product_flavor_notes(user_id: &str, product_ids: &[&str]) -> Vec<Statement> {
    product_ids
        .chunks(PRODUCT_IDS_PER_STATEMENT)
        .map(|chunk| {
            let mut sql = String::new();
            sql.push_str("SELECT pft.product_id, t.name FROM ");
            sql.push_str(PRODUCT_FLAVOR_TAGS_TABLE);
            sql.push_str(" AS pft INNER JOIN ");
            sql.push_str(FLAVOR_TAGS_TABLE);
            sql.push_str(
                " AS t ON t.id = pft.tag_id WHERE pft.user_id = ? AND t.user_id = ? AND \
                          pft.product_id IN (",
            );
            for index in 0..chunk.len() {
                if index > 0 {
                    sql.push_str(", ");
                }
                sql.push('?');
            }
            sql.push_str(") ORDER BY t.name ASC, pft.product_id ASC");

            let mut params = vec![
                Value::Text(user_id.to_owned()),
                Value::Text(user_id.to_owned()),
            ];
            params.extend(chunk.iter().map(|id| Value::Text((*id).to_owned())));
            Statement { sql, params }
        })
        .collect()
}

/// 商品の Flavor Notes のタグの対応を全て消す SQL を組み立てる (配列での置き換えの前段)。
pub fn delete_product_flavor_tags(user_id: &str, product_id: &str) -> Statement {
    let mut sql = String::new();
    sql.push_str("DELETE FROM ");
    sql.push_str(PRODUCT_FLAVOR_TAGS_TABLE);
    sql.push_str(" WHERE product_id = ? AND user_id = ?");
    Statement {
        sql,
        params: vec![
            Value::Text(product_id.to_owned()),
            Value::Text(user_id.to_owned()),
        ],
    }
}

/// タグの行を挿入する SQL を組み立てる。同じ利用者の同じ名前のタグがあれば何もしない (FR-8)。
pub fn insert_flavor_tag(id: &str, user_id: &str, name: &str) -> Statement {
    let mut sql = String::new();
    sql.push_str("INSERT INTO ");
    sql.push_str(FLAVOR_TAGS_TABLE);
    sql.push_str(" (id, user_id, name) VALUES (?, ?, ?) ON CONFLICT (user_id, name) DO NOTHING");
    Statement {
        sql,
        params: vec![
            Value::Text(id.to_owned()),
            Value::Text(user_id.to_owned()),
            Value::Text(name.to_owned()),
        ],
    }
}

/// 商品とタグの対応を、タグの名前で挿入する SQL を組み立てる。
///
/// タグの ID は利用者と名前で引く。タグの行は [`insert_flavor_tag`] が先に入れる。
pub fn insert_product_flavor_tag(user_id: &str, product_id: &str, name: &str) -> Statement {
    let mut sql = String::new();
    sql.push_str("INSERT INTO ");
    sql.push_str(PRODUCT_FLAVOR_TAGS_TABLE);
    sql.push_str(" (user_id, product_id, tag_id) SELECT ?, ?, id FROM ");
    sql.push_str(FLAVOR_TAGS_TABLE);
    sql.push_str(" WHERE user_id = ? AND name = ?");
    Statement {
        sql,
        params: vec![
            Value::Text(user_id.to_owned()),
            Value::Text(product_id.to_owned()),
            Value::Text(user_id.to_owned()),
            Value::Text(name.to_owned()),
        ],
    }
}

/// 商品の入力の値の並び (商品名、Producer、Origin、Region、Process、Variety)。
fn product_value_list(values: &ProductValues<'_>) -> Vec<Value> {
    vec![
        Value::Text(values.name.to_owned()),
        optional_text(values.producer),
        optional_text(values.origin),
        optional_text(values.region),
        optional_text(values.process),
        optional_text(values.variety),
    ]
}

/// 任意の文字列を値にする。None は NULL にする。
fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |text| Value::Text(text.to_owned()))
}

/// アーカイブとアーカイブ解除の SQL を組み立てる。
fn set_archived(
    table: &'static str,
    id: &str,
    user_id: &str,
    archived_at: Option<&str>,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    update(&UpdateQuery {
        table,
        columns: ARCHIVED_COLUMNS,
        values: vec![
            optional_text(archived_at),
            Value::Text(updated_at.to_owned()),
        ],
        id,
        user_id,
    })
}
