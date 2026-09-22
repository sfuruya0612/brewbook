//! マイグレーションの SQL と、ADR-0006 の列と本 issue の設計判断の一覧の照合。
//!
//! 初期スキーマ (`migrations/0001_initial_schema.sql`) を読み、次を確認する。
//!
//! - 11 テーブルの列が ADR-0006 の表と一致する (過不足のどちらも許さない)。
//! - ADR-0006 の参照 (外部キー) がある。
//! - 設計判断の UNIQUE 制約 (利用者ごとのタグ名、全利用者で一意の credential_id) がある。
//! - 設計判断の複合インデックス (利用者 ID、`archived_at`、並び順のキーの降順、ID) がある。
//!
//! 期待値は issue 本文と ADR の一覧を写したテストデータとして、SQL の解析結果と突き合わせる。

use std::collections::BTreeMap;
use std::path::Path;

/// 初期スキーマのマイグレーション。
const MIGRATION: &str = "migrations/0001_initial_schema.sql";

/// ADR-0006 の 11 テーブルと列。列の順は ADR-0006 の表に合わせる。
const EXPECTED_TABLES: &[(&str, &[&str])] = &[
    ("users", &["id", "display_name", "created_at"]),
    (
        "registration_tokens",
        &["id", "user_id", "token_hash", "expires_at", "used_at"],
    ),
    (
        "passkey_credentials",
        &[
            "id",
            "user_id",
            "credential_id",
            "public_key",
            "sign_count",
            "name",
            "created_at",
            "last_used_at",
        ],
    ),
    (
        "webauthn_challenges",
        &["id", "user_id", "challenge", "kind", "expires_at"],
    ),
    (
        "sessions",
        &["id", "user_id", "token_hash", "expires_at", "created_at"],
    ),
    (
        "shops",
        &[
            "id",
            "user_id",
            "name",
            "address",
            "created_at",
            "updated_at",
            "archived_at",
        ],
    ),
    (
        "products",
        &[
            "id",
            "user_id",
            "name",
            "producer",
            "origin",
            "region",
            "process",
            "variety",
            "created_at",
            "updated_at",
            "archived_at",
        ],
    ),
    ("flavor_tags", &["id", "user_id", "name"]),
    ("product_flavor_tags", &["user_id", "product_id", "tag_id"]),
    (
        "purchases",
        &[
            "id",
            "user_id",
            "product_id",
            "shop_id",
            "purchased_on",
            "roast",
            "roast_date",
            "price_amount",
            "price_currency",
            "weight_grams",
            "photo_key",
            "created_at",
            "updated_at",
            "archived_at",
        ],
    ),
    (
        "brews",
        &[
            "id",
            "user_id",
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
            "archived_at",
        ],
    ),
];

/// 本 issue の設計判断の UNIQUE 制約。
const EXPECTED_UNIQUE: &[(&str, &[&[&str]])] = &[
    // タグは利用者ごとに名前で一意にする (ADR-0006)。
    ("flavor_tags", &[&["user_id", "name"]]),
    // ログインが credential ID で引けるよう、全利用者で一意にする (本 issue の設計判断)。
    ("passkey_credentials", &[&["credential_id"]]),
];

/// 本 issue の設計判断の複合インデックス (利用者 ID、archived_at、並び順のキー)。
const EXPECTED_INDEXES: &[(&str, &str, &[&str])] = &[
    (
        "idx_shops_user_archived_created_at",
        "shops",
        &["user_id", "archived_at", "created_at DESC", "id"],
    ),
    (
        "idx_products_user_archived_created_at",
        "products",
        &["user_id", "archived_at", "created_at DESC", "id"],
    ),
    (
        "idx_purchases_user_archived_purchased_on",
        "purchases",
        &["user_id", "archived_at", "purchased_on DESC", "id"],
    ),
    (
        "idx_brews_user_archived_brewed_at",
        "brews",
        &["user_id", "archived_at", "brewed_at DESC", "id"],
    ),
];

#[test]
fn the_migration_has_the_eleven_tables_with_the_adr_columns() {
    let parsed = parse_migration(&read_migration());
    let expected: Vec<(&str, &[&str])> = EXPECTED_TABLES.to_vec();
    let actual: Vec<(&str, Vec<String>)> = parsed
        .tables
        .iter()
        .map(|table| (table.name.as_str(), table.columns.clone()))
        .collect();
    assert_eq!(
        actual.len(),
        expected.len(),
        "the migration must have exactly {} tables but has {}: {:?}",
        expected.len(),
        actual.len(),
        actual.iter().map(|(name, _)| name).collect::<Vec<_>>()
    );
    for (name, columns) in &expected {
        let table = parsed
            .tables
            .iter()
            .find(|table| table.name == *name)
            .unwrap_or_else(|| panic!("the migration must have the table {name}"));
        assert_eq!(
            table.columns,
            columns
                .iter()
                .map(|column| column.to_string())
                .collect::<Vec<_>>(),
            "the columns of {name} must match ADR-0006"
        );
    }
}

#[test]
fn the_migration_keeps_the_primary_keys() {
    let parsed = parse_migration(&read_migration());
    for table in &parsed.tables {
        // product_flavor_tags は id 列を持たないため、列の組を主キーにする。
        let expected: Vec<String> = if table.name == "product_flavor_tags" {
            vec!["product_id".to_owned(), "tag_id".to_owned()]
        } else {
            vec!["id".to_owned()]
        };
        assert_eq!(
            table.primary, expected,
            "the primary key of {} must be {expected:?}",
            table.name
        );
    }
}

/// ADR-0006 の参照 (外部キー)。(テーブル, 列, 参照先のテーブル, 参照先の列)。
const EXPECTED_FOREIGN_KEYS: &[(&str, &str, &str, &str)] = &[
    ("registration_tokens", "user_id", "users", "id"),
    ("passkey_credentials", "user_id", "users", "id"),
    ("webauthn_challenges", "user_id", "users", "id"),
    ("sessions", "user_id", "users", "id"),
    ("shops", "user_id", "users", "id"),
    ("products", "user_id", "users", "id"),
    ("flavor_tags", "user_id", "users", "id"),
    ("product_flavor_tags", "user_id", "users", "id"),
    ("product_flavor_tags", "product_id", "products", "id"),
    ("product_flavor_tags", "tag_id", "flavor_tags", "id"),
    ("purchases", "user_id", "users", "id"),
    ("purchases", "product_id", "products", "id"),
    ("purchases", "shop_id", "shops", "id"),
    ("brews", "user_id", "users", "id"),
    ("brews", "purchase_id", "purchases", "id"),
];

#[test]
fn the_migration_has_the_unique_constraints_of_the_design_decisions() {
    let parsed = parse_migration(&read_migration());
    for table in &parsed.tables {
        let expected: Vec<Vec<String>> = EXPECTED_UNIQUE
            .iter()
            .find(|(name, _)| *name == table.name)
            .map(|(_, constraints)| {
                constraints
                    .iter()
                    .map(|columns| columns.iter().map(|column| column.to_string()).collect())
                    .collect()
            })
            .unwrap_or_default();
        let mut actual = table.unique.clone();
        let mut expected = expected;
        actual.sort();
        expected.sort();
        assert_eq!(
            actual, expected,
            "the unique constraints of {} must match the design decisions",
            table.name
        );
    }
}

#[test]
fn the_migration_has_the_foreign_keys_of_the_adr() {
    let parsed = parse_migration(&read_migration());
    let actual: Vec<(String, String, String, String)> = parsed
        .tables
        .iter()
        .flat_map(|table| {
            table
                .foreign_keys
                .iter()
                .map(|(column, referenced_table, referenced_column)| {
                    (
                        table.name.clone(),
                        column.clone(),
                        referenced_table.clone(),
                        referenced_column.clone(),
                    )
                })
        })
        .collect();
    let expected: Vec<(String, String, String, String)> = EXPECTED_FOREIGN_KEYS
        .iter()
        .map(|(table, column, referenced_table, referenced_column)| {
            (
                (*table).to_owned(),
                (*column).to_owned(),
                (*referenced_table).to_owned(),
                (*referenced_column).to_owned(),
            )
        })
        .collect();
    // 本数の一致と、各行の存在を双方向で確認する。
    assert_eq!(
        actual.len(),
        expected.len(),
        "the foreign key count must match the ADR"
    );
    for key in &expected {
        assert!(
            actual.contains(key),
            "the foreign key {key:?} must exist in the migration"
        );
    }
}

#[test]
fn the_migration_has_the_composite_indexes_for_lists_and_suggestions_and_stats() {
    let parsed = parse_migration(&read_migration());
    let expected: Vec<(String, String, Vec<String>)> = EXPECTED_INDEXES
        .iter()
        .map(|(name, table, columns)| {
            (
                (*name).to_owned(),
                (*table).to_owned(),
                columns.iter().map(|column| (*column).to_owned()).collect(),
            )
        })
        .collect();
    let actual: Vec<(String, String, Vec<String>)> = parsed
        .indexes
        .iter()
        .map(|index| {
            (
                index.name.clone(),
                index.table.clone(),
                index.columns.clone(),
            )
        })
        .collect();
    assert_eq!(
        actual, expected,
        "the composite indexes must match the design decisions"
    );
}

fn read_migration() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(MIGRATION);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

/// マイグレーション 1 つを解析した結果。
#[derive(Debug, Default)]
struct ParsedMigration {
    tables: Vec<ParsedTable>,
    indexes: Vec<ParsedIndex>,
}

#[derive(Debug)]
struct ParsedTable {
    name: String,
    columns: Vec<String>,
    primary: Vec<String>,
    unique: Vec<Vec<String>>,
    /// (列, 参照先のテーブル, 参照先の列)。
    foreign_keys: Vec<(String, String, String)>,
}

#[derive(Debug)]
struct ParsedIndex {
    name: String,
    table: String,
    columns: Vec<String>,
}

/// `CREATE TABLE` と `CREATE INDEX` の文を解析する。SQL はこのテスト専用の単純な解析で扱う。
fn parse_migration(sql: &str) -> ParsedMigration {
    let mut parsed = ParsedMigration::default();
    for statement in statements(sql) {
        if let Some(rest) = strip_prefix_ignoring_case(&statement, "CREATE TABLE") {
            parsed.tables.push(parse_table(rest));
        } else if let Some(rest) = strip_prefix_ignoring_case(&statement, "CREATE UNIQUE INDEX") {
            parsed.indexes.push(parse_index(rest));
        } else if let Some(rest) = strip_prefix_ignoring_case(&statement, "CREATE INDEX") {
            parsed.indexes.push(parse_index(rest));
        }
    }
    parsed
}

fn parse_table(rest: &str) -> ParsedTable {
    let name = rest
        .trim_start()
        .split(|ch: char| ch == '(' || ch.is_whitespace())
        .next()
        .expect("a CREATE TABLE statement names the table")
        .to_owned();
    let body = paren_content(rest);
    let mut columns = Vec::new();
    let mut primary = Vec::new();
    let mut unique = Vec::new();
    let mut foreign_keys = Vec::new();
    for item in split_top_level(&body) {
        if let Some(rest) = strip_prefix_ignoring_case(&item, "UNIQUE") {
            unique.push(columns_of(&paren_content(rest)));
        } else if let Some(rest) = strip_prefix_ignoring_case(&item, "PRIMARY KEY") {
            primary = columns_of(&paren_content(rest));
        } else {
            let column = item
                .split_whitespace()
                .next()
                .expect("a column definition names the column")
                .to_owned();
            let rest = item[column.len()..].to_ascii_uppercase();
            // 列に付けた PRIMARY KEY と UNIQUE も制約として数える。
            if rest.contains("PRIMARY KEY") {
                primary = vec![column.clone()];
            }
            if rest.split_whitespace().any(|word| word == "UNIQUE") {
                unique.push(vec![column.clone()]);
            }
            // 列に付けた REFERENCES を外部キーとして数える。
            // 検索は大文字化した `rest` で行い、値は元の表記から取り出す。
            if let Some(position) = rest.find("REFERENCES") {
                let after = &item[column.len() + position + "REFERENCES".len()..];
                let referenced_table = after
                    .split_whitespace()
                    .next()
                    .expect("REFERENCES names the referenced table")
                    .to_owned();
                let referenced_column = columns_of(&paren_content(after))
                    .first()
                    .expect("REFERENCES names the referenced column")
                    .clone();
                foreign_keys.push((column.clone(), referenced_table, referenced_column));
            }
            columns.push(column);
        }
    }
    ParsedTable {
        name,
        columns,
        primary,
        unique,
        foreign_keys,
    }
}

fn parse_index(rest: &str) -> ParsedIndex {
    let mut words = rest.split_whitespace();
    let name = words
        .next()
        .expect("a CREATE INDEX statement names the index")
        .to_owned();
    let on = words.next().unwrap_or_default();
    assert!(
        on.eq_ignore_ascii_case("ON"),
        "the index {name} must be declared with ON but was {on}"
    );
    let table = words
        .next()
        .expect("a CREATE INDEX statement names the table")
        .to_owned();
    let columns = index_columns_of(&paren_content(rest));
    ParsedIndex {
        name,
        table,
        columns,
    }
}

/// 括弧の中身を返す。最初の `(` から対応する `)` までを取る。
fn paren_content(text: &str) -> String {
    let start = text.find('(').expect("the statement must have parentheses");
    let mut depth = 0_u32;
    for (offset, ch) in text[start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return text[start + 1..start + offset].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("the parentheses must be balanced in {text}");
}

/// 括弧の中の列の並びを返す。主キーと UNIQUE 制約では並び順の指定を付けない。
fn columns_of(body: &str) -> Vec<String> {
    body.split(',')
        .map(|column| column.trim().to_owned())
        .filter(|column| !column.is_empty())
        .collect()
}

/// 索引の列の並びを返す。並び順の指定は大文字にそろえる (省略時は付けない)。
fn index_columns_of(body: &str) -> Vec<String> {
    body.split(',')
        .map(|column| {
            let words: Vec<&str> = column.split_whitespace().collect();
            match words.as_slice() {
                [name] => (*name).to_owned(),
                [name, direction] => format!("{} {}", name, direction.to_ascii_uppercase()),
                _ => panic!("an index column is a name and an optional direction: {column}"),
            }
        })
        .filter(|column| !column.is_empty())
        .collect()
}

/// 括弧の外側の深さ 0 のコンマだけで分割する。
fn split_top_level(body: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0_u32;
    let mut current = String::new();
    for ch in body.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                items.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        items.push(current.trim().to_owned());
    }
    items
}

/// `--` の行コメントを除き、`;` で文に分ける。
fn statements(sql: &str) -> Vec<String> {
    sql.lines()
        .map(|line| line.split("--").next().unwrap_or_default())
        .collect::<Vec<&str>>()
        .join("\n")
        .split(';')
        .map(|statement| statement.trim().to_owned())
        .filter(|statement| !statement.is_empty())
        .collect()
}

fn strip_prefix_ignoring_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&text[prefix.len()..])
    } else {
        None
    }
}

/// 期待値のテーブル名の重複を検出する (テストデータの間違いを早く気付くため)。
#[test]
fn the_expected_test_data_has_no_duplicates() {
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    for (name, _) in EXPECTED_TABLES {
        *names.entry(name).or_default() += 1;
    }
    for (name, count) in names {
        assert_eq!(
            count, 1,
            "the expected table {name} is listed {count} times"
        );
    }
}
