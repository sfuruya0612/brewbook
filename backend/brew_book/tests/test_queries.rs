//! 利用者向けの Worker が実行する SQL の単体テスト (0018、FR-17)。
//!
//! 利用者向けの SQL を列挙し、取得 (SELECT) に表示名 (`display_name`) が現れないことと、
//! `SELECT *` (列を明示しない取得) を使っていないことを検査する。列を明示しない取得では、
//! 取得する列が増えたときに `display_name` が応答に混ざっても列の検査をし抜けるため、
//! 両方を検査する。
//!
//! 列挙は、このクレートの SQL の定数 (`brew_book::queries::STATEMENTS`) と、記録と統計の
//! クエリを組み立てる `brew_book_core` の呼び出しを合わせて行う。`brew_book_core` が
//! 組み立てる SELECT は、この検査が知らない列を足せない (列の並びは定数だけが持つ)。

use brew_book::queries::{self, STATEMENTS};
use brew_book_core::cursor::{SortKey, SortOrder};
use brew_book_core::query::{self, SuggestionItem};
use brew_book_core::stats::{self, Granularity};

/// テスト用の利用者の ID。
const USER: &str = "00000000-0000-4000-8000-000000000001";

/// テスト用の記録の ID (パスや外部キーに使う)。
const ID: &str = "00000000-0000-4000-8000-000000000002";

/// 利用者向けの Worker が実行する SQL を列挙する。
fn statements() -> Vec<String> {
    let mut statements: Vec<String> = STATEMENTS.iter().map(|sql| (*sql).to_owned()).collect();

    // 記録の一覧と 1 件の取得 (brew_book_core::query)。列の並びは定数が持つ。
    statements.push(
        query::shops_list(USER, SortKey::CreatedAt, SortOrder::Desc, false, None, 20)
            .expect("the shops list must be built")
            .sql,
    );
    statements.push(
        query::products_list(
            USER,
            SortKey::CreatedAt,
            SortOrder::Desc,
            false,
            None,
            20,
            None,
        )
        .expect("the products list must be built")
        .sql,
    );
    // 名前の絞り込み (FR-19) を付けた商品の一覧も、同じ検査の対象にする。
    statements.push(
        query::products_list(
            USER,
            SortKey::Name,
            SortOrder::Asc,
            true,
            None,
            20,
            Some("名前"),
        )
        .expect("the products list with a name must be built")
        .sql,
    );
    statements.push(
        query::purchases_list(USER, SortKey::PurchasedOn, SortOrder::Desc, false, None, 20)
            .expect("the purchases list must be built")
            .sql,
    );
    statements.push(
        query::brews_list(USER, SortKey::BrewedAt, SortOrder::Desc, false, None, 20)
            .expect("the brews list must be built")
            .sql,
    );
    statements.push(query::shop_find(USER, ID).sql);
    statements.push(query::product_find(USER, ID).sql);
    statements.push(query::purchase_find(USER, ID).sql);
    statements.push(query::brew_find(USER, ID).sql);
    // お気に入りの付け外しの UPDATE (FR-21)。
    statements.push(
        query::set_favorited_at(
            "shops",
            ID,
            USER,
            Some("2026-09-21T00:00:00.000Z"),
            "2026-09-21T00:00:00.000Z",
        )
        .sql,
    );
    statements
        .push(query::set_favorited_at("shops", ID, USER, None, "2026-09-21T00:00:00.000Z").sql);
    statements.push(query::flavor_tags_list(USER).sql);
    statements.push(query::suggestions(USER, SuggestionItem::Producer, "q").sql);

    // エクスポート (FR-14) は 6 つのテーブルの全行を読む。
    for (table, columns) in [
        (query::SHOPS_TABLE, query::SHOP_COLUMNS),
        (query::PRODUCTS_TABLE, query::PRODUCT_COLUMNS),
        (query::FLAVOR_TAGS_TABLE, query::FLAVOR_TAG_COLUMNS),
        (
            query::PRODUCT_FLAVOR_TAGS_TABLE,
            query::PRODUCT_FLAVOR_TAG_COLUMNS,
        ),
        (query::PURCHASES_TABLE, query::PURCHASE_COLUMNS),
        (query::BREWS_TABLE, query::BREW_COLUMNS),
    ] {
        statements.push(query::export_rows(table, columns, "id ASC", USER).sql);
    }

    // アカウント削除 (FR-15) と Flavor Notes のタグ (FR-8)。
    statements.extend(
        query::account_delete(USER)
            .into_iter()
            .map(|statement| statement.sql),
    );
    statements.extend(
        query::product_flavor_notes(USER, &[ID])
            .into_iter()
            .map(|statement| statement.sql),
    );

    // 統計 (FR-18)。
    let period = stats::parse_period(Some("2026-09-01"), Some("2026-09-30"))
        .expect("the period must be parsed");
    statements.push(
        stats::brews_stats(USER, Granularity::Day, 0, &period)
            .expect("the brews stats must be built")
            .sql,
    );
    statements.push(
        stats::brew_ratings(USER, 0, &period)
            .expect("the brew ratings must be built")
            .sql,
    );
    statements.push(stats::purchases_stats(USER, Granularity::Day, &period).sql);
    statements.push(stats::rating_history(USER, ID).sql);

    statements
}

#[test]
fn the_ledger_is_not_empty() {
    assert!(!STATEMENTS.is_empty(), "the worker must have SQL");
    for sql in STATEMENTS {
        assert!(!sql.trim().is_empty(), "a statement must not be empty");
    }
    // 記録のクエリと統計のクエリも列挙に含まれる。
    assert!(
        statements().len() > STATEMENTS.len(),
        "the builders must add SQL"
    );
}

#[test]
fn the_selects_do_not_read_the_display_name() {
    for sql in statements() {
        if !queries::has_select(&sql) {
            continue;
        }
        assert!(
            !sql.to_ascii_lowercase().contains("display_name"),
            "a user-facing select must not read the display name: {sql}"
        );
    }
}

#[test]
fn no_statement_selects_every_column() {
    for sql in statements() {
        assert!(
            !queries::selects_every_column(&sql),
            "a user-facing statement must name the columns it reads: {sql}"
        );
    }
}

#[test]
fn the_ledger_covers_the_columns_of_every_exported_table() {
    // エクスポートが読む列の並びは、表示名を持たない (FR-17)。
    for columns in [
        query::SHOP_COLUMNS,
        query::PRODUCT_COLUMNS,
        query::PURCHASE_COLUMNS,
        query::BREW_COLUMNS,
        query::FLAVOR_TAG_COLUMNS,
        query::PRODUCT_FLAVOR_TAG_COLUMNS,
    ] {
        assert!(
            !columns.to_ascii_lowercase().contains("display_name"),
            "an exported column list must not have the display name: {columns}"
        );
        assert!(
            !columns.contains('*'),
            "an exported column list must not select every column: {columns}"
        );
    }
}

#[test]
fn has_select_reads_the_word() {
    assert!(queries::has_select("SELECT id FROM users"));
    assert!(queries::has_select(
        "INSERT INTO users (id) SELECT id FROM users"
    ));
    assert!(!queries::has_select("DELETE FROM users WHERE id = ?"));
}

#[test]
fn selects_every_column_reads_the_star_after_select() {
    assert!(queries::selects_every_column("SELECT * FROM users"));
    assert!(queries::selects_every_column("select\n  * from users"));
    // 集約は列を明示しているため含めない。
    assert!(!queries::selects_every_column(
        "SELECT COUNT(*) FROM passkey_credentials"
    ));
    assert!(!queries::selects_every_column("SELECT id FROM users"));
    // 語の一部を読まない。
    assert!(!queries::selects_every_column("SELECTED * FROM users"));
}
