//! 記録のクエリの組み立て (ADR-0006)。
//!
//! 全てのクエリに `user_id` の条件を必ず付け、一覧のクエリには `archived_at` の条件を付ける
//! (サジェストはアーカイブ済みの行の値も候補に含めるため付けない。FR-13)。
//! 値は必ずプレースホルダ (`?`) で渡し、SQL に値を連結しない。テーブル名と列名はコード内の
//! 定数だけを使い、利用者の入力は渡せない。
//!
//! 一覧と 1 件の取得は共通の関数 ([`list`] と [`find_one`]) を通して組み立て、店と商品と
//! Flavor Notes のタグの個別のクエリもこのモジュールが持つ。0007 と 0008 と 0010 と 0011 も
//! このモジュールを使う。

use crate::cursor::CursorKey;
use crate::error::ErrorCode;

/// SQL に束縛する値。
///
/// 小数を持つため `Eq` は実装しない (値の比較は `PartialEq` で行う)。
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// NULL。
    Null,
    /// 文字列。
    Text(String),
    /// 整数。
    Integer(i64),
    /// 小数 (豆の量、湯量、湯の温度に使う)。
    Real(f64),
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

/// サジェスト (FR-13) の対象の項目。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionItem {
    /// 商品の Producer。
    Producer,
    /// 商品の Origin。
    Origin,
    /// 商品の Region。
    Region,
    /// 商品の Process。
    Process,
    /// 商品の Variety。
    Variety,
    /// 購入の Roast。
    Roast,
    /// 抽出の抽出方法。
    Method,
    /// 抽出の挽き目。
    GrindSetting,
}

impl SuggestionItem {
    /// 対象のテーブル名。コード内の定数だけを返す。
    pub fn table(self) -> &'static str {
        match self {
            SuggestionItem::Producer
            | SuggestionItem::Origin
            | SuggestionItem::Region
            | SuggestionItem::Process
            | SuggestionItem::Variety => PRODUCTS_TABLE,
            SuggestionItem::Roast => PURCHASES_TABLE,
            SuggestionItem::Method | SuggestionItem::GrindSetting => BREWS_TABLE,
        }
    }

    /// 対象の列名。コード内の定数だけを返す。
    pub fn column(self) -> &'static str {
        match self {
            SuggestionItem::Producer => "producer",
            SuggestionItem::Origin => "origin",
            SuggestionItem::Region => "region",
            SuggestionItem::Process => "process",
            SuggestionItem::Variety => "variety",
            SuggestionItem::Roast => "roast",
            SuggestionItem::Method => "method",
            SuggestionItem::GrindSetting => "grind_setting",
        }
    }
}

/// サジェストの項目名の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionFieldError {
    /// 8 つの項目名のどれでもない。
    Unknown,
}

impl SuggestionFieldError {
    /// 応答のエラーの種別 (400 Bad Request)。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        "the field must be one of producer, origin, region, process, variety, roast, method, grind_setting"
    }
}

/// サジェストの項目名を解釈する。8 つの名前だけを受け付け、それ以外は拒否する (FR-13)。
pub fn parse_suggestion_field(name: &str) -> Result<SuggestionItem, SuggestionFieldError> {
    match name {
        "producer" => Ok(SuggestionItem::Producer),
        "origin" => Ok(SuggestionItem::Origin),
        "region" => Ok(SuggestionItem::Region),
        "process" => Ok(SuggestionItem::Process),
        "variety" => Ok(SuggestionItem::Variety),
        "roast" => Ok(SuggestionItem::Roast),
        "method" => Ok(SuggestionItem::Method),
        "grind_setting" => Ok(SuggestionItem::GrindSetting),
        _ => Err(SuggestionFieldError::Unknown),
    }
}

/// サジェストの候補の上限 (FR-13)。
pub const SUGGESTION_LIMIT: u32 = 20;

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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
    list_qualified(
        &QualifiedList {
            from: query.table,
            columns: query.columns,
            alias: "",
            order_column: query.order_column,
            order_kind: query.order_kind,
            user_id: query.user_id,
            archived: query.archived,
        },
        query.cursor.clone(),
        query.limit,
    )
}

/// 1 件の取得の SQL を組み立てる。
///
/// `SELECT <columns> FROM <table> WHERE id = ? AND user_id = ? [AND archived_at IS NULL]`
pub fn find_one(query: &FindQuery<'_>) -> Statement {
    find_qualified(
        query.table,
        query.columns,
        "",
        query.user_id,
        query.id,
        query.archived,
    )
}

/// 別名を付けた一覧の入力。`from` にはテーブルと結合の並びを渡せる。
struct QualifiedList<'a> {
    /// `FROM` に置くテーブルと結合の並び。
    from: &'a str,
    /// 選択する列の並び。
    columns: &'a str,
    /// 列名を修飾する別名。空のときは修飾しない。
    alias: &'a str,
    /// 並び順のキーの列 (修飾前)。
    order_column: &'a str,
    /// 並び順のキーの種類。
    order_kind: OrderKind,
    /// 絞り込む利用者の ID。
    user_id: &'a str,
    /// アーカイブ済みの行の扱い。
    archived: Archived,
}

/// 別名を付けた一覧の SQL を組み立てる。
///
/// 列名は別名があるときだけ `<別名>.<列>` にする (`from` に複数のテーブルがあると `id` や
/// `user_id` が曖昧になるため)。
fn list_qualified(
    query: &QualifiedList<'_>,
    cursor: Option<CursorKey>,
    limit: u32,
) -> Result<Statement, QueryError> {
    let mut sql = String::new();
    let mut params = Vec::new();

    sql.push_str("SELECT ");
    sql.push_str(query.columns);
    sql.push_str(" FROM ");
    sql.push_str(query.from);
    sql.push_str(" WHERE ");
    sql.push_str(&qualified(query.alias, "user_id"));
    sql.push_str(" = ?");
    params.push(Value::Text(query.user_id.to_owned()));

    if query.archived == Archived::Exclude {
        sql.push_str(" AND ");
        sql.push_str(&qualified(query.alias, "archived_at"));
        sql.push_str(" IS NULL");
    }

    if let Some(cursor) = &cursor {
        let key = match (cursor, query.order_kind) {
            (CursorKey::DateTime { at, .. }, OrderKind::DateTime) => at,
            (CursorKey::Date { on, .. }, OrderKind::Date) => on,
            _ => return Err(QueryError::CursorKindMismatch),
        };
        // 並び順のキーの降順と ID の昇順の続きを引く。
        let order = qualified(query.alias, query.order_column);
        let id = qualified(query.alias, "id");
        sql.push_str(" AND (");
        sql.push_str(&order);
        sql.push_str(" < ? OR (");
        sql.push_str(&order);
        sql.push_str(" = ? AND ");
        sql.push_str(&id);
        sql.push_str(" > ?))");
        params.push(Value::Text(key.clone()));
        params.push(Value::Text(key.clone()));
        params.push(Value::Text(cursor.id().to_owned()));
    }

    sql.push_str(" ORDER BY ");
    sql.push_str(&qualified(query.alias, query.order_column));
    sql.push_str(" DESC, ");
    sql.push_str(&qualified(query.alias, "id"));
    sql.push_str(" ASC LIMIT ?");
    params.push(Value::Integer(i64::from(limit)));

    Ok(Statement { sql, params })
}

/// 別名を付けた 1 件の取得の SQL を組み立てる。`from` にはテーブルと結合の並びを渡せる。
fn find_qualified(
    from: &str,
    columns: &str,
    alias: &str,
    user_id: &str,
    id: &str,
    archived: Archived,
) -> Statement {
    let mut sql = String::new();
    sql.push_str("SELECT ");
    sql.push_str(columns);
    sql.push_str(" FROM ");
    sql.push_str(from);
    sql.push_str(" WHERE ");
    sql.push_str(&qualified(alias, "id"));
    sql.push_str(" = ? AND ");
    sql.push_str(&qualified(alias, "user_id"));
    sql.push_str(" = ?");
    if archived == Archived::Exclude {
        sql.push_str(" AND ");
        sql.push_str(&qualified(alias, "archived_at"));
        sql.push_str(" IS NULL");
    }
    Statement {
        sql,
        params: vec![Value::Text(id.to_owned()), Value::Text(user_id.to_owned())],
    }
}

/// 列名を別名で修飾する。別名が空のときはそのままの列名にする。
fn qualified(alias: &str, column: &str) -> String {
    if alias.is_empty() {
        column.to_owned()
    } else {
        format!("{alias}.{column}")
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
/// 購入のテーブル名。
pub const PURCHASES_TABLE: &str = "purchases";
/// 購入の列の並び。応答の JSON の項目と同じ。
pub const PURCHASE_COLUMNS: &str = "id, user_id, product_id, shop_id, purchased_on, roast, \
                                    roast_date, price_amount, price_currency, weight_grams, \
                                    photo_key, created_at, updated_at, archived_at";
/// 抽出のテーブル名。
pub const BREWS_TABLE: &str = "brews";
/// 抽出の列の並び。応答の JSON の項目と同じ。
pub const BREW_COLUMNS: &str = "id, user_id, purchase_id, brewed_at, dose_grams, water_grams, \
                                water_temp_c, brew_time_seconds, method, grind_setting, rating, \
                                notes, created_at, updated_at, archived_at";
/// Flavor Notes のタグのテーブル名。
pub const FLAVOR_TAGS_TABLE: &str = "flavor_tags";
/// Flavor Notes のタグの列の並び。応答の JSON の項目と同じ。
pub const FLAVOR_TAG_COLUMNS: &str = "id, user_id, name";
/// 商品と Flavor Notes のタグの対応のテーブル名。
pub const PRODUCT_FLAVOR_TAGS_TABLE: &str = "product_flavor_tags";
/// 商品と Flavor Notes のタグの対応の列の並び。応答の JSON の項目と同じ。
pub const PRODUCT_FLAVOR_TAG_COLUMNS: &str = "user_id, product_id, tag_id";
/// 1 つのクエリに束縛できる値の数。D1 は 100 個までとする
/// (202 個を束縛したクエリを D1 が拒否することをローカルで確認した)。
pub const MAX_BOUND_VALUES: usize = 100;
/// タグ名を引くクエリは利用者 ID を 2 つ束縛するため、商品 ID は 98 件ずつにする。
const PRODUCT_IDS_PER_STATEMENT: usize = MAX_BOUND_VALUES - 2;

/// 結合の SQL で使う購入の別名。
const PURCHASE_ALIAS: &str = "p";
/// 結合の SQL で使う抽出の別名。
const BREW_ALIAS: &str = "b";
/// 結合の SQL で使う商品の別名。
const PRODUCT_ALIAS: &str = "pr";
/// 結合の SQL で使う店の別名。
const SHOP_ALIAS: &str = "sh";

/// 購入と商品と店の結合。商品は INNER JOIN、店は LEFT JOIN で結合し、店が無い購入でも行が返る
/// (ADR-0006)。結合の条件にも利用者 ID を含める。
const PURCHASES_FROM: &str = "purchases AS p \
     INNER JOIN products AS pr ON pr.id = p.product_id AND pr.user_id = p.user_id \
     LEFT JOIN shops AS sh ON sh.id = p.shop_id AND sh.user_id = p.user_id";
/// 抽出と、購入、商品、店の結合。抽出は購入だけを参照し、商品と店は購入からたどる (ADR-0006)。
const BREWS_FROM: &str = "brews AS b \
     INNER JOIN purchases AS p ON p.id = b.purchase_id AND p.user_id = b.user_id \
     INNER JOIN products AS pr ON pr.id = p.product_id AND pr.user_id = p.user_id \
     LEFT JOIN shops AS sh ON sh.id = p.shop_id AND sh.user_id = p.user_id";

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
/// 購入の `INSERT` の列 (`id` と `user_id` を除く)。`photo_key` は 0009 が扱う。
const PURCHASE_INSERT_COLUMNS: &[&str] = &[
    "product_id",
    "shop_id",
    "purchased_on",
    "roast",
    "roast_date",
    "price_amount",
    "price_currency",
    "weight_grams",
    "created_at",
    "updated_at",
];
/// 購入の `UPDATE` の列。
const PURCHASE_UPDATE_COLUMNS: &[&str] = &[
    "product_id",
    "shop_id",
    "purchased_on",
    "roast",
    "roast_date",
    "price_amount",
    "price_currency",
    "weight_grams",
    "updated_at",
];
/// 抽出の `INSERT` の列 (`id` と `user_id` を除く)。
const BREW_INSERT_COLUMNS: &[&str] = &[
    "purchase_id",
    "brewed_at",
    "dose_grams",
    "water_grams",
    "water_temp_c",
    "brew_time_seconds",
    "method",
    "grind_setting",
    "rating",
    "notes",
    "created_at",
    "updated_at",
];
/// 抽出の `UPDATE` の列。
const BREW_UPDATE_COLUMNS: &[&str] = &[
    "purchase_id",
    "brewed_at",
    "dose_grams",
    "water_grams",
    "water_temp_c",
    "brew_time_seconds",
    "method",
    "grind_setting",
    "rating",
    "notes",
    "updated_at",
];
/// アーカイブとアーカイブ解除の `UPDATE` の列 (ADR-0006)。
const ARCHIVED_COLUMNS: &[&str] = &["archived_at", "updated_at"];
/// 購入の写真の `UPDATE` の列 (0009)。`photo_key` は付け外しの両方がある。
const PHOTO_KEY_COLUMNS: &[&str] = &["photo_key", "updated_at"];

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

/// 購入の入力の値。NULL は None で表す。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PurchaseValues<'a> {
    /// 商品の ID。必須 (FR-9)。
    pub product_id: &'a str,
    /// 店の ID。任意 (店が無い購入がある。ADR-0006)。
    pub shop_id: Option<&'a str>,
    /// 購入日 (`YYYY-MM-DD`、タイムゾーンを持たない。ADR-0002)。
    pub purchased_on: &'a str,
    /// Roast。任意。
    pub roast: Option<&'a str>,
    /// Roast Date (`YYYY-MM-DD`)。任意。
    pub roast_date: Option<&'a str>,
    /// 価格 (通貨の最小単位)。任意。
    pub price_amount: Option<i64>,
    /// ISO 4217 の通貨コード。価格が無いときは NULL にする (0007 の設計判断)。
    pub price_currency: Option<&'a str>,
    /// 重量 (グラム)。任意。
    pub weight_grams: Option<i64>,
}

/// 抽出の入力の値。NULL は None で表す。購入と抽出日時以外は任意 (FR-11)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrewValues<'a> {
    /// 購入の ID。必須 (FR-11)。
    pub purchase_id: &'a str,
    /// 抽出日時 (ISO 8601 の UTC。ADR-0002)。
    pub brewed_at: &'a str,
    /// 豆の量 (グラム)。任意。
    pub dose_grams: Option<f64>,
    /// 湯量 (グラム)。任意。
    pub water_grams: Option<f64>,
    /// 湯の温度 (摂氏)。任意。
    pub water_temp_c: Option<f64>,
    /// 時間 (秒)。任意。
    pub brew_time_seconds: Option<i64>,
    /// 抽出方法。任意。
    pub method: Option<&'a str>,
    /// 挽き目 (グラインダーの設定値)。任意。
    pub grind_setting: Option<&'a str>,
    /// 評価 (1 から 5)。任意。
    pub rating: Option<i64>,
    /// 感想。任意。
    pub notes: Option<&'a str>,
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

/// 購入と商品と店を結合した一覧を組み立てる。並び順は購入日の降順と ID の昇順 (FR-9)。
pub fn purchases_list(
    user_id: &str,
    archived: Archived,
    cursor: Option<CursorKey>,
    limit: u32,
) -> Result<Statement, QueryError> {
    list_qualified(
        &QualifiedList {
            from: PURCHASES_FROM,
            columns: &purchases_columns(),
            alias: PURCHASE_ALIAS,
            order_column: "purchased_on",
            order_kind: OrderKind::Date,
            user_id,
            archived,
        },
        cursor,
        limit,
    )
}

/// 購入と商品と店を結合した 1 件の取得の SQL を組み立てる。
pub fn purchase_find(user_id: &str, id: &str, archived: Archived) -> Statement {
    find_qualified(
        PURCHASES_FROM,
        &purchases_columns(),
        PURCHASE_ALIAS,
        user_id,
        id,
        archived,
    )
}

/// 購入を挿入する SQL を組み立てる。`photo_key` は 0009 が扱うため NULL のままにする。
pub fn purchase_insert(
    id: &str,
    user_id: &str,
    values: &PurchaseValues<'_>,
    created_at: &str,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    let mut purchase_values = purchase_value_list(values);
    purchase_values.push(Value::Text(created_at.to_owned()));
    purchase_values.push(Value::Text(updated_at.to_owned()));
    insert(&InsertQuery {
        table: PURCHASES_TABLE,
        columns: PURCHASE_INSERT_COLUMNS,
        values: purchase_values,
        id,
        user_id,
    })
}

/// 購入を更新する SQL を組み立てる。
pub fn purchase_update(
    id: &str,
    user_id: &str,
    values: &PurchaseValues<'_>,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    let mut purchase_values = purchase_value_list(values);
    purchase_values.push(Value::Text(updated_at.to_owned()));
    update(&UpdateQuery {
        table: PURCHASES_TABLE,
        columns: PURCHASE_UPDATE_COLUMNS,
        values: purchase_values,
        id,
        user_id,
    })
}

/// 購入のアーカイブとアーカイブ解除の SQL を組み立てる。
pub fn purchase_set_archived(
    id: &str,
    user_id: &str,
    archived_at: Option<&str>,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    set_archived(PURCHASES_TABLE, id, user_id, archived_at, updated_at)
}

/// 購入の写真の参照を付け外しする SQL を組み立てる (0009)。`None` のときは NULL にする。
pub fn purchase_set_photo_key(
    id: &str,
    user_id: &str,
    photo_key: Option<&str>,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    update(&UpdateQuery {
        table: PURCHASES_TABLE,
        columns: PHOTO_KEY_COLUMNS,
        values: vec![optional_text(photo_key), Value::Text(updated_at.to_owned())],
        id,
        user_id,
    })
}

/// 抽出と、購入、商品、店を結合した一覧を組み立てる。並び順は抽出日時の降順と ID の昇順 (FR-11)。
pub fn brews_list(
    user_id: &str,
    archived: Archived,
    cursor: Option<CursorKey>,
    limit: u32,
) -> Result<Statement, QueryError> {
    list_qualified(
        &QualifiedList {
            from: BREWS_FROM,
            columns: &brews_columns(),
            alias: BREW_ALIAS,
            order_column: "brewed_at",
            order_kind: OrderKind::DateTime,
            user_id,
            archived,
        },
        cursor,
        limit,
    )
}

/// 抽出と、購入、商品、店を結合した 1 件の取得の SQL を組み立てる。
pub fn brew_find(user_id: &str, id: &str, archived: Archived) -> Statement {
    find_qualified(
        BREWS_FROM,
        &brews_columns(),
        BREW_ALIAS,
        user_id,
        id,
        archived,
    )
}

/// 抽出を挿入する SQL を組み立てる。
pub fn brew_insert(
    id: &str,
    user_id: &str,
    values: &BrewValues<'_>,
    created_at: &str,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    let mut brew_values = brew_value_list(values);
    brew_values.push(Value::Text(created_at.to_owned()));
    brew_values.push(Value::Text(updated_at.to_owned()));
    insert(&InsertQuery {
        table: BREWS_TABLE,
        columns: BREW_INSERT_COLUMNS,
        values: brew_values,
        id,
        user_id,
    })
}

/// 抽出を更新する SQL を組み立てる。
pub fn brew_update(
    id: &str,
    user_id: &str,
    values: &BrewValues<'_>,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    let mut brew_values = brew_value_list(values);
    brew_values.push(Value::Text(updated_at.to_owned()));
    update(&UpdateQuery {
        table: BREWS_TABLE,
        columns: BREW_UPDATE_COLUMNS,
        values: brew_values,
        id,
        user_id,
    })
}

/// 抽出のアーカイブとアーカイブ解除の SQL を組み立てる。
pub fn brew_set_archived(
    id: &str,
    user_id: &str,
    archived_at: Option<&str>,
    updated_at: &str,
) -> Result<Statement, QueryError> {
    set_archived(BREWS_TABLE, id, user_id, archived_at, updated_at)
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

/// サジェストの候補を引く SQL を組み立てる (FR-13)。
///
/// 値ごとに `updated_at` の最大を取り、その降順、同じときは値の昇順 (Unicode コードポイント) で
/// 並べ、先頭の [`SUGGESTION_LIMIT`] 件にする。
/// アーカイブ済みの行の値も候補に含めるため `archived_at` の条件は付けず、利用者 ID の条件は付ける。
/// `q` には前後の空白を除いた値を渡す。比較は `lower(列) LIKE lower(?) || '%' ESCAPE '\'` で行い、
/// 大文字と小文字を区別しない。`q` の中の `%` と `_` と `\` は文字として扱う。
///
/// `SELECT <column> AS value FROM <table> WHERE user_id = ? \
///  AND lower(<column>) LIKE lower(?) || '%' ESCAPE '\' GROUP BY <column> \
///  ORDER BY MAX(updated_at) DESC, value ASC LIMIT ?`
pub fn suggestions(user_id: &str, item: SuggestionItem, query: &str) -> Statement {
    let column = item.column();
    let mut sql = String::new();
    sql.push_str("SELECT ");
    sql.push_str(column);
    sql.push_str(" AS value FROM ");
    sql.push_str(item.table());
    sql.push_str(" WHERE user_id = ? AND lower(");
    sql.push_str(column);
    sql.push_str(") LIKE lower(?) || '%' ESCAPE '\\' GROUP BY ");
    sql.push_str(column);
    sql.push_str(" ORDER BY MAX(updated_at) DESC, value ASC LIMIT ?");
    Statement {
        sql,
        params: vec![
            Value::Text(user_id.to_owned()),
            Value::Text(escape_like(query)),
            Value::Integer(i64::from(SUGGESTION_LIMIT)),
        ],
    }
}

/// LIKE のパターンにする値をエスケープする。
///
/// `%` と `_` を文字として扱い、エスケープ文字 (`\`) そのものも 2 つにして文字にする。
fn escape_like(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// エクスポート (FR-14) で、利用者の 1 つのテーブルの全行を引く SQL を組み立てる。
///
/// `SELECT <columns> FROM <table> WHERE user_id = ? ORDER BY <order_by>`
///
/// アーカイブ済みの行も含める (FR-14。既定の一覧の `archived_at IS NULL` を付けない)。
/// 件数の上限も付けず、並び順は `order_by` の昇順にする。
pub fn export_rows(
    table: &'static str,
    columns: &'static str,
    order_by: &'static str,
    user_id: &str,
) -> Statement {
    let mut sql = String::new();
    sql.push_str("SELECT ");
    sql.push_str(columns);
    sql.push_str(" FROM ");
    sql.push_str(table);
    sql.push_str(" WHERE user_id = ? ORDER BY ");
    sql.push_str(order_by);
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

/// 購入の入力の値の並び (商品、店、購入日、Roast、Roast Date、価格、通貨コード、重量)。
fn purchase_value_list(values: &PurchaseValues<'_>) -> Vec<Value> {
    vec![
        Value::Text(values.product_id.to_owned()),
        optional_text(values.shop_id),
        Value::Text(values.purchased_on.to_owned()),
        optional_text(values.roast),
        optional_text(values.roast_date),
        optional_integer(values.price_amount),
        optional_text(values.price_currency),
        optional_integer(values.weight_grams),
    ]
}

/// 抽出の入力の値の並び (購入、抽出日時、豆の量、湯量、湯の温度、時間、抽出方法、挽き目、評価、感想)。
fn brew_value_list(values: &BrewValues<'_>) -> Vec<Value> {
    vec![
        Value::Text(values.purchase_id.to_owned()),
        Value::Text(values.brewed_at.to_owned()),
        optional_real(values.dose_grams),
        optional_real(values.water_grams),
        optional_real(values.water_temp_c),
        optional_integer(values.brew_time_seconds),
        optional_text(values.method),
        optional_text(values.grind_setting),
        optional_integer(values.rating),
        optional_text(values.notes),
    ]
}

/// 購入と商品と店の結合で選択する列の並び。別名は Worker が結果を読むときの項目名になる。
fn purchases_columns() -> String {
    join_columns(&[
        (PURCHASE_ALIAS, PURCHASE_COLUMNS),
        (PRODUCT_ALIAS, PRODUCT_COLUMNS),
        (SHOP_ALIAS, SHOP_COLUMNS),
    ])
}

/// 抽出と、購入、商品、店の結合で選択する列の並び。
fn brews_columns() -> String {
    join_columns(&[
        (BREW_ALIAS, BREW_COLUMNS),
        (PURCHASE_ALIAS, PURCHASE_COLUMNS),
        (PRODUCT_ALIAS, PRODUCT_COLUMNS),
        (SHOP_ALIAS, SHOP_COLUMNS),
    ])
}

/// 複数のテーブルの列を、接頭辞を付けた選択の並びにする。
fn join_columns(tables: &[(&str, &str)]) -> String {
    tables
        .iter()
        .map(|(alias, columns)| prefixed_columns(alias, columns))
        .collect::<Vec<_>>()
        .join(", ")
}

/// 列の並びを `<別名>.<列> AS <別名>_<列>` の並びにする。
///
/// 結合の結果は 1 つの平坦な行になるため、列ごとに一意の別名を付ける。
fn prefixed_columns(alias: &str, columns: &str) -> String {
    columns
        .split(',')
        .map(str::trim)
        .filter(|column| !column.is_empty())
        .map(|column| format!("{alias}.{column} AS {alias}_{column}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// 任意の整数を値にする。None は NULL にする。
fn optional_integer(value: Option<i64>) -> Value {
    value.map_or(Value::Null, Value::Integer)
}

/// 任意の小数を値にする。None は NULL にする。
fn optional_real(value: Option<f64>) -> Value {
    value.map_or(Value::Null, Value::Real)
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
