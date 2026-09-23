//! `query` の単体テスト。組み立てた SQL が利用者 ID と `archived_at` の条件を必ず含むこと、
//! 値がプレースホルダで渡ることを確認する。
//!
//! 組み立てる関数ごとの SQL は [`record_queries`]、条件の付け忘れの検出は [`conditions`] が検査する。

use coffee_log_core::cursor::CursorKey;
use coffee_log_core::query::{
    parse_include_archived, Archived, IncludeArchivedError, QueryError, Statement, Value,
};

const USER_ID: &str = "9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60";
const SHOP_ID: &str = "0d2b6f5e-3a4c-4a7b-9c8d-7e6f5a4b3c2d";
const AT: &str = "2026-09-21T12:34:56.789Z";

mod list_builders {
    use super::*;
    use coffee_log_core::query::{list, ListQuery, OrderKind};

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
                at: AT.to_owned(),
                id: SHOP_ID.to_owned(),
            },
            OrderKind::Date => CursorKey::Date {
                on: "2026-09-21".to_owned(),
                id: SHOP_ID.to_owned(),
            },
        }
    }
}

mod record_queries {
    use super::*;
    use coffee_log_core::query::{
        self, insert, update, InsertQuery, ProductValues, ShopValues, UpdateQuery, SHOPS_TABLE,
    };

    fn shop_values<'a>(name: &'a str, address: Option<&'a str>) -> ShopValues<'a> {
        ShopValues { name, address }
    }

    fn product_values<'a>(name: &'a str) -> ProductValues<'a> {
        ProductValues {
            name,
            producer: None,
            origin: None,
            region: None,
            process: None,
            variety: None,
        }
    }

    #[test]
    fn a_shop_find_query_filters_by_the_id_and_the_user() {
        let statement = query::shop_find(USER_ID, SHOP_ID, Archived::Include);
        assert_eq!(
            statement.sql,
            "SELECT id, user_id, name, address, created_at, updated_at, archived_at FROM shops \
             WHERE id = ? AND user_id = ?"
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
            ]
        );
    }

    #[test]
    fn a_find_query_can_restrict_to_active_rows() {
        // アーカイブ済みの親を参照先に指定したときの 409 の判定 (0007) に使う。
        let statement = query::product_find(USER_ID, SHOP_ID, Archived::Exclude);
        assert!(
            statement.sql.contains("AND archived_at IS NULL"),
            "the active rows must be selectable: {}",
            statement.sql
        );
    }

    #[test]
    fn a_shop_insert_places_the_id_and_the_user_id_first() {
        let statement = insert(&InsertQuery {
            table: SHOPS_TABLE,
            columns: &["name", "address", "created_at", "updated_at"],
            values: vec![
                Value::Text("店".to_owned()),
                Value::Null,
                Value::Text(AT.to_owned()),
                Value::Text(AT.to_owned()),
            ],
            id: SHOP_ID,
            user_id: USER_ID,
        })
        .unwrap();
        assert_eq!(
            statement.sql,
            "INSERT INTO shops (id, user_id, name, address, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)"
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
                Value::Text("店".to_owned()),
                Value::Null,
                Value::Text(AT.to_owned()),
                Value::Text(AT.to_owned()),
            ]
        );
    }

    #[test]
    fn a_shop_insert_of_a_blank_address_keeps_the_null() {
        let statement =
            query::shop_insert(SHOP_ID, USER_ID, &shop_values("店", None), AT, AT).unwrap();
        assert_eq!(statement.params[3], Value::Null);
        let statement =
            query::shop_insert(SHOP_ID, USER_ID, &shop_values("店", Some("住所")), AT, AT).unwrap();
        assert_eq!(statement.params[3], Value::Text("住所".to_owned()));
    }

    #[test]
    fn a_product_insert_and_update_carry_every_field() {
        let insert =
            query::product_insert(SHOP_ID, USER_ID, &product_values("豆"), AT, AT).unwrap();
        assert_eq!(
            insert.sql,
            "INSERT INTO products (id, user_id, name, producer, origin, region, process, variety, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        );
        assert_eq!(insert.params.len(), 10);
        assert_eq!(
            insert.sql.matches('?').count(),
            insert.params.len(),
            "every placeholder must have a value: {}",
            insert.sql
        );
        let update = query::product_update(SHOP_ID, USER_ID, &product_values("豆"), AT).unwrap();
        assert_eq!(
            update.sql,
            "UPDATE products SET name = ?, producer = ?, origin = ?, region = ?, process = ?, \
             variety = ?, updated_at = ? WHERE id = ? AND user_id = ?"
        );
        assert_eq!(update.params.len(), 9);
    }

    #[test]
    fn a_shop_update_filters_by_the_id_and_the_user_id() {
        let statement = query::shop_update(SHOP_ID, USER_ID, &shop_values("店", None), AT).unwrap();
        assert_eq!(
            statement.sql,
            "UPDATE shops SET name = ?, address = ?, updated_at = ? WHERE id = ? AND user_id = ?"
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text("店".to_owned()),
                Value::Null,
                Value::Text(AT.to_owned()),
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
            ]
        );
    }

    #[test]
    fn an_archive_statement_sets_the_archived_at_and_the_updated_at() {
        let statement = query::shop_set_archived(SHOP_ID, USER_ID, Some(AT), AT).unwrap();
        assert_eq!(
            statement.sql,
            "UPDATE shops SET archived_at = ?, updated_at = ? WHERE id = ? AND user_id = ?"
        );
        assert_eq!(statement.params[0], Value::Text(AT.to_owned()));
        // アーカイブ解除は archived_at を NULL に戻す。
        let statement = query::product_set_archived(SHOP_ID, USER_ID, None, AT).unwrap();
        assert_eq!(
            statement.sql,
            "UPDATE products SET archived_at = ?, updated_at = ? WHERE id = ? AND user_id = ?"
        );
        assert_eq!(statement.params[0], Value::Null);
    }

    #[test]
    fn the_tag_queries_filter_by_the_user() {
        let delete = query::delete_product_flavor_tags(USER_ID, SHOP_ID);
        assert_eq!(
            delete.sql,
            "DELETE FROM product_flavor_tags WHERE product_id = ? AND user_id = ?"
        );
        // 束縛の順は SQL のプレースホルダと同じ (商品、利用者) である。
        assert_eq!(
            delete.params,
            vec![
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
            ]
        );
        let tag = query::insert_flavor_tag(SHOP_ID, USER_ID, "チョコ");
        assert_eq!(
            tag.sql,
            "INSERT INTO flavor_tags (id, user_id, name) VALUES (?, ?, ?) \
             ON CONFLICT (user_id, name) DO NOTHING"
        );
        let association = query::insert_product_flavor_tag(USER_ID, SHOP_ID, "チョコ");
        assert_eq!(
            association.sql,
            "INSERT INTO product_flavor_tags (user_id, product_id, tag_id) SELECT ?, ?, id FROM \
             flavor_tags WHERE user_id = ? AND name = ?"
        );
        assert_eq!(
            association.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
                Value::Text("チョコ".to_owned()),
            ]
        );
    }

    #[test]
    fn the_tag_list_is_ordered_by_the_name() {
        assert_eq!(
            query::flavor_tags_list(USER_ID).sql,
            "SELECT id, user_id, name FROM flavor_tags WHERE user_id = ? ORDER BY name ASC"
        );
    }

    #[test]
    fn the_product_flavor_notes_query_lists_the_given_products() {
        let statements = query::product_flavor_notes(USER_ID, &[SHOP_ID, "second"]);
        assert_eq!(statements.len(), 1, "2 件は 1 つの文に収まる");
        let statement = &statements[0];
        assert!(
            statement
                .sql
                .contains("WHERE pft.user_id = ? AND t.user_id = ? AND pft.product_id IN (?, ?)"),
            "{}",
            statement.sql
        );
        assert!(statement.sql.contains("ORDER BY t.name ASC"));
        let expected = vec![
            Value::Text(USER_ID.to_owned()),
            Value::Text(USER_ID.to_owned()),
            Value::Text(SHOP_ID.to_owned()),
            Value::Text("second".to_owned()),
        ];
        assert_eq!(statement.params, expected);
    }

    #[test]
    fn the_product_flavor_notes_query_is_split_within_the_bound_values() {
        // ページの上限 (200 件) を指定したときと同じ数の商品を渡す。
        let ids: Vec<String> = (0..200).map(|index| format!("id-{index}")).collect();
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        let statements = query::product_flavor_notes(USER_ID, &ids);
        // D1 は 1 つのクエリに 100 個まで値を束縛できるため、複数の文に分かれる。
        assert!(statements.len() > 1, "the query must be split");
        for statement in &statements {
            assert!(
                statement.params.len() <= query::MAX_BOUND_VALUES,
                "the bound values must fit in one query: {}",
                statement.params.len()
            );
            let placeholders = statement.sql.matches('?').count();
            assert_eq!(placeholders, statement.params.len());
        }
        let bound: usize = statements
            .iter()
            .map(|statement| statement.params.len() - 2)
            .sum();
        assert_eq!(bound, ids.len(), "every id must be bound once");
    }

    #[test]
    fn the_product_flavor_notes_query_of_no_products_is_empty() {
        assert!(query::product_flavor_notes(USER_ID, &[]).is_empty());
    }

    #[test]
    fn a_column_and_value_count_mismatch_is_rejected() {
        let insert = insert(&InsertQuery {
            table: SHOPS_TABLE,
            columns: &["name"],
            values: Vec::new(),
            id: SHOP_ID,
            user_id: USER_ID,
        });
        assert_eq!(insert, Err(QueryError::ColumnCountMismatch));
        let update = update(&UpdateQuery {
            table: SHOPS_TABLE,
            columns: &["name", "address"],
            values: vec![Value::Null],
            id: SHOP_ID,
            user_id: USER_ID,
        });
        assert_eq!(update, Err(QueryError::ColumnCountMismatch));
    }

    #[test]
    fn the_code_errors_map_to_the_internal_status() {
        assert_eq!(QueryError::ColumnCountMismatch.code().status(), 500);
        assert_eq!(QueryError::CursorKindMismatch.code().status(), 400);
        assert!(!QueryError::ColumnCountMismatch.message().is_empty());
    }
}

mod purchase_and_brew_queries {
    use super::*;
    use coffee_log_core::query::{self, BrewValues, PurchaseValues};

    /// 購入の列の別名 (結合の SQL の期待値に使う)。
    const PURCHASE_COLUMNS: &str = "p.id AS p_id, p.user_id AS p_user_id, \
        p.product_id AS p_product_id, p.shop_id AS p_shop_id, p.purchased_on AS p_purchased_on, \
        p.roast AS p_roast, p.roast_date AS p_roast_date, p.price_amount AS p_price_amount, \
        p.price_currency AS p_price_currency, p.weight_grams AS p_weight_grams, \
        p.photo_key AS p_photo_key, p.created_at AS p_created_at, p.updated_at AS p_updated_at, \
        p.archived_at AS p_archived_at";
    /// 商品の列の別名。
    const PRODUCT_COLUMNS: &str = "pr.id AS pr_id, pr.user_id AS pr_user_id, pr.name AS pr_name, \
        pr.producer AS pr_producer, pr.origin AS pr_origin, pr.region AS pr_region, \
        pr.process AS pr_process, pr.variety AS pr_variety, pr.created_at AS pr_created_at, \
        pr.updated_at AS pr_updated_at, pr.archived_at AS pr_archived_at";
    /// 店の列の別名。
    const SHOP_COLUMNS: &str = "sh.id AS sh_id, sh.user_id AS sh_user_id, sh.name AS sh_name, \
        sh.address AS sh_address, sh.created_at AS sh_created_at, sh.updated_at AS sh_updated_at, \
        sh.archived_at AS sh_archived_at";

    fn purchase_values<'a>(product_id: &'a str, shop_id: Option<&'a str>) -> PurchaseValues<'a> {
        PurchaseValues {
            product_id,
            shop_id,
            purchased_on: "2026-09-21",
            roast: None,
            roast_date: None,
            price_amount: None,
            price_currency: None,
            weight_grams: None,
        }
    }

    fn brew_values<'a>(purchase_id: &'a str) -> BrewValues<'a> {
        BrewValues {
            purchase_id,
            brewed_at: AT,
            dose_grams: None,
            water_grams: None,
            water_temp_c: None,
            brew_time_seconds: None,
            method: None,
            grind_setting: None,
            rating: None,
            notes: None,
        }
    }

    #[test]
    fn a_purchases_list_query_joins_the_product_and_the_shop() {
        let statement = query::purchases_list(USER_ID, Archived::Exclude, None, 50).unwrap();
        assert_eq!(
            statement.sql,
            format!(
                "SELECT {PURCHASE_COLUMNS}, {PRODUCT_COLUMNS}, {SHOP_COLUMNS} \
                 FROM purchases AS p \
                 INNER JOIN products AS pr ON pr.id = p.product_id AND pr.user_id = p.user_id \
                 LEFT JOIN shops AS sh ON sh.id = p.shop_id AND sh.user_id = p.user_id \
                 WHERE p.user_id = ? AND p.archived_at IS NULL \
                 ORDER BY p.purchased_on DESC, p.id ASC LIMIT ?"
            )
        );
        assert_eq!(
            statement.params,
            vec![Value::Text(USER_ID.to_owned()), Value::Integer(50)]
        );
    }

    #[test]
    fn a_purchases_list_query_with_a_cursor_uses_the_purchase_date() {
        let cursor = CursorKey::Date {
            on: "2026-09-21".to_owned(),
            id: SHOP_ID.to_owned(),
        };
        let statement =
            query::purchases_list(USER_ID, Archived::Exclude, Some(cursor), 10).unwrap();
        assert!(
            statement
                .sql
                .contains("AND (p.purchased_on < ? OR (p.purchased_on = ? AND p.id > ?))"),
            "{}",
            statement.sql
        );
        assert_eq!(statement.params.len(), 5);
        assert_eq!(statement.params[1], Value::Text("2026-09-21".to_owned()));
        assert_eq!(statement.params[3], Value::Text(SHOP_ID.to_owned()));
    }

    #[test]
    fn a_purchases_list_query_with_include_archived_has_no_archived_filter() {
        let statement = query::purchases_list(USER_ID, Archived::Include, None, 50).unwrap();
        assert!(
            !statement.sql.contains("archived_at IS NULL"),
            "include_archived must not add the archived filter: {}",
            statement.sql
        );
        assert!(statement.sql.contains("WHERE p.user_id = ?"));
    }

    #[test]
    fn a_purchases_cursor_of_another_kind_is_rejected() {
        let cursor = CursorKey::DateTime {
            at: AT.to_owned(),
            id: SHOP_ID.to_owned(),
        };
        assert_eq!(
            query::purchases_list(USER_ID, Archived::Exclude, Some(cursor), 50),
            Err(QueryError::CursorKindMismatch)
        );
    }

    #[test]
    fn a_brews_list_query_joins_the_purchase_the_product_and_the_shop() {
        let statement = query::brews_list(USER_ID, Archived::Exclude, None, 200).unwrap();
        assert_eq!(
            statement.sql,
            format!(
                "SELECT b.id AS b_id, b.user_id AS b_user_id, b.purchase_id AS b_purchase_id, \
                 b.brewed_at AS b_brewed_at, b.dose_grams AS b_dose_grams, \
                 b.water_grams AS b_water_grams, b.water_temp_c AS b_water_temp_c, \
                 b.brew_time_seconds AS b_brew_time_seconds, b.method AS b_method, \
                 b.grind_setting AS b_grind_setting, b.rating AS b_rating, b.notes AS b_notes, \
                 b.created_at AS b_created_at, b.updated_at AS b_updated_at, \
                 b.archived_at AS b_archived_at, {PURCHASE_COLUMNS}, {PRODUCT_COLUMNS}, \
                 {SHOP_COLUMNS} \
                 FROM brews AS b \
                 INNER JOIN purchases AS p ON p.id = b.purchase_id AND p.user_id = b.user_id \
                 INNER JOIN products AS pr ON pr.id = p.product_id AND pr.user_id = p.user_id \
                 LEFT JOIN shops AS sh ON sh.id = p.shop_id AND sh.user_id = p.user_id \
                 WHERE b.user_id = ? AND b.archived_at IS NULL \
                 ORDER BY b.brewed_at DESC, b.id ASC LIMIT ?"
            )
        );
        assert_eq!(
            statement.params,
            vec![Value::Text(USER_ID.to_owned()), Value::Integer(200)]
        );
    }

    #[test]
    fn a_brew_find_query_filters_by_the_id_and_the_user() {
        let statement = query::brew_find(USER_ID, SHOP_ID, Archived::Include);
        assert_eq!(
            statement.sql,
            format!(
                "SELECT b.id AS b_id, b.user_id AS b_user_id, b.purchase_id AS b_purchase_id, \
                 b.brewed_at AS b_brewed_at, b.dose_grams AS b_dose_grams, \
                 b.water_grams AS b_water_grams, b.water_temp_c AS b_water_temp_c, \
                 b.brew_time_seconds AS b_brew_time_seconds, b.method AS b_method, \
                 b.grind_setting AS b_grind_setting, b.rating AS b_rating, b.notes AS b_notes, \
                 b.created_at AS b_created_at, b.updated_at AS b_updated_at, \
                 b.archived_at AS b_archived_at, {PURCHASE_COLUMNS}, {PRODUCT_COLUMNS}, \
                 {SHOP_COLUMNS} \
                 FROM brews AS b \
                 INNER JOIN purchases AS p ON p.id = b.purchase_id AND p.user_id = b.user_id \
                 INNER JOIN products AS pr ON pr.id = p.product_id AND pr.user_id = p.user_id \
                 LEFT JOIN shops AS sh ON sh.id = p.shop_id AND sh.user_id = p.user_id \
                 WHERE b.id = ? AND b.user_id = ?"
            )
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
            ]
        );
    }

    #[test]
    fn a_purchase_find_query_joins_the_product_and_the_shop() {
        let statement = query::purchase_find(USER_ID, SHOP_ID, Archived::Include);
        assert!(
            statement
                .sql
                .contains("FROM purchases AS p INNER JOIN products AS pr ON pr.id = p.product_id"),
            "{}",
            statement.sql
        );
        assert!(
            statement
                .sql
                .contains("LEFT JOIN shops AS sh ON sh.id = p.shop_id AND sh.user_id = p.user_id"),
            "{}",
            statement.sql
        );
        assert!(
            statement.sql.ends_with("WHERE p.id = ? AND p.user_id = ?"),
            "{}",
            statement.sql
        );
    }

    #[test]
    fn a_purchase_insert_and_update_carry_every_field() {
        let insert = query::purchase_insert(
            SHOP_ID,
            USER_ID,
            &purchase_values("product-1", Some("shop-1")),
            AT,
            AT,
        )
        .unwrap();
        assert_eq!(
            insert.sql,
            "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, roast, \
             roast_date, price_amount, price_currency, weight_grams, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        );
        assert_eq!(
            insert.params,
            vec![
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
                Value::Text("product-1".to_owned()),
                Value::Text("shop-1".to_owned()),
                Value::Text("2026-09-21".to_owned()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Text(AT.to_owned()),
                Value::Text(AT.to_owned()),
            ]
        );
        let update =
            query::purchase_update(SHOP_ID, USER_ID, &purchase_values("product-1", None), AT)
                .unwrap();
        assert_eq!(
            update.sql,
            "UPDATE purchases SET product_id = ?, shop_id = ?, purchased_on = ?, roast = ?, \
             roast_date = ?, price_amount = ?, price_currency = ?, weight_grams = ?, updated_at = ? \
             WHERE id = ? AND user_id = ?"
        );
        assert_eq!(update.params.len(), 11);
        assert_eq!(update.params[1], Value::Null);
    }

    #[test]
    fn a_purchase_insert_carries_the_price_and_the_currency() {
        let values = PurchaseValues {
            product_id: "product-1",
            shop_id: None,
            purchased_on: "2026-09-21",
            roast: Some("中煎り"),
            roast_date: Some("2026-09-19"),
            price_amount: Some(1200),
            price_currency: Some("JPY"),
            weight_grams: Some(200),
        };
        let statement = query::purchase_insert(SHOP_ID, USER_ID, &values, AT, AT).unwrap();
        assert_eq!(statement.params[7], Value::Integer(1200));
        assert_eq!(statement.params[8], Value::Text("JPY".to_owned()));
        assert_eq!(statement.params[9], Value::Integer(200));
        // 価格が無いときは通貨コードも NULL にする (0007 の設計判断)。
        let statement = query::purchase_insert(
            SHOP_ID,
            USER_ID,
            &purchase_values("product-1", None),
            AT,
            AT,
        )
        .unwrap();
        assert_eq!(statement.params[7], Value::Null);
        assert_eq!(statement.params[8], Value::Null);
    }

    #[test]
    fn a_brew_insert_carries_the_decimal_values() {
        let values = BrewValues {
            purchase_id: "purchase-1",
            brewed_at: AT,
            dose_grams: Some(15.5),
            water_grams: Some(250.0),
            water_temp_c: Some(92.5),
            brew_time_seconds: Some(150),
            method: Some("ペーパードリップ"),
            grind_setting: Some("中細"),
            rating: Some(4),
            notes: Some("良い出来"),
        };
        let insert = query::brew_insert(SHOP_ID, USER_ID, &values, AT, AT).unwrap();
        assert_eq!(
            insert.sql,
            "INSERT INTO brews (id, user_id, purchase_id, brewed_at, dose_grams, water_grams, \
             water_temp_c, brew_time_seconds, method, grind_setting, rating, notes, created_at, \
             updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        );
        assert_eq!(insert.params[2], Value::Text("purchase-1".to_owned()));
        assert_eq!(insert.params[3], Value::Text(AT.to_owned()));
        assert_eq!(insert.params[4], Value::Real(15.5));
        assert_eq!(insert.params[5], Value::Real(250.0));
        assert_eq!(insert.params[6], Value::Real(92.5));
        assert_eq!(insert.params[7], Value::Integer(150));
        assert_eq!(insert.params[8], Value::Text("ペーパードリップ".to_owned()));
        assert_eq!(insert.params[9], Value::Text("中細".to_owned()));
        assert_eq!(insert.params[10], Value::Integer(4));
        assert_eq!(insert.params[11], Value::Text("良い出来".to_owned()));
        assert_eq!(insert.params.len(), 14);
        let update = query::brew_update(SHOP_ID, USER_ID, &brew_values("purchase-1"), AT).unwrap();
        assert_eq!(
            update.sql,
            "UPDATE brews SET purchase_id = ?, brewed_at = ?, dose_grams = ?, water_grams = ?, \
             water_temp_c = ?, brew_time_seconds = ?, method = ?, grind_setting = ?, rating = ?, \
             notes = ?, updated_at = ? WHERE id = ? AND user_id = ?"
        );
        assert_eq!(update.params[2], Value::Null);
        assert_eq!(update.params.len(), 13);
    }

    #[test]
    fn the_archive_statements_of_the_purchases_and_the_brews_keep_the_user() {
        let archive = query::purchase_set_archived(SHOP_ID, USER_ID, Some(AT), AT).unwrap();
        assert_eq!(
            archive.sql,
            "UPDATE purchases SET archived_at = ?, updated_at = ? WHERE id = ? AND user_id = ?"
        );
        assert_eq!(archive.params[0], Value::Text(AT.to_owned()));
        let unarchive = query::brew_set_archived(SHOP_ID, USER_ID, None, AT).unwrap();
        assert_eq!(
            unarchive.sql,
            "UPDATE brews SET archived_at = ?, updated_at = ? WHERE id = ? AND user_id = ?"
        );
        assert_eq!(unarchive.params[0], Value::Null);
    }
}

mod include_archived_parameter {
    use super::*;

    #[test]
    fn the_parameter_defaults_to_excluding_archived_rows() {
        assert_eq!(parse_include_archived(None), Ok(Archived::Exclude));
        assert_eq!(parse_include_archived(Some("false")), Ok(Archived::Exclude));
    }

    #[test]
    fn the_parameter_accepts_true() {
        assert_eq!(parse_include_archived(Some("true")), Ok(Archived::Include));
    }

    #[test]
    fn another_value_is_rejected_as_a_bad_request() {
        for text in ["", " ", "1", "0", "TRUE", "True", "yes", "true "] {
            assert_eq!(
                parse_include_archived(Some(text)),
                Err(IncludeArchivedError::NotABoolean),
                "{text} must be rejected"
            );
        }
        assert_eq!(IncludeArchivedError::NotABoolean.code().status(), 400);
        assert!(!IncludeArchivedError::NotABoolean.message().is_empty());
    }
}

mod conditions {
    use super::*;
    use coffee_log_core::query::{self, BrewValues, ProductValues, PurchaseValues, ShopValues};

    /// 検査の対象にする文の種類。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Kind {
        /// 一覧。既定では `archived_at IS NULL` を必ず付ける。
        List,
        /// 1 件の取得と更新、削除。`id` と `user_id` の両方で絞る。
        Row,
        /// 挿入。`id` と `user_id` の列と値を必ず持つ。
        Insert,
    }

    /// 検査の対象にする 1 件の文。
    struct Checked {
        name: &'static str,
        statement: Statement,
        kind: Kind,
        /// 既定の一覧か (`archived_at` の条件を検査するか)。
        default_archived: bool,
    }

    /// 利用者 ID の条件があるか。
    fn has_user_condition(statement: &Statement) -> bool {
        statement.sql.contains("user_id = ?")
    }

    /// アーカイブ済みを除く条件があるか。
    fn has_archived_condition(statement: &Statement) -> bool {
        statement.sql.contains("archived_at IS NULL")
    }

    /// 挿入の列と値に利用者 ID があるか。
    fn carries_user_id(statement: &Statement, user_id: &str) -> bool {
        statement.sql.contains("user_id")
            && statement.params.contains(&Value::Text(user_id.to_owned()))
    }

    /// モジュールが組み立てる文の一覧。
    fn every_statement() -> Vec<Checked> {
        let mut checked = Vec::new();
        for (name, archived, default_archived) in [
            ("shops list", Archived::Exclude, true),
            ("shops list with archived", Archived::Include, false),
        ] {
            checked.push(Checked {
                name,
                statement: query::shops_list(USER_ID, archived, None, 50).unwrap(),
                kind: Kind::List,
                default_archived,
            });
            checked.push(Checked {
                name: "products list",
                statement: query::products_list(USER_ID, archived, None, 50).unwrap(),
                kind: Kind::List,
                default_archived,
            });
        }
        let cursor = CursorKey::DateTime {
            at: AT.to_owned(),
            id: SHOP_ID.to_owned(),
        };
        for (name, archived, default_archived) in [
            ("purchases list", Archived::Exclude, true),
            ("purchases list with archived", Archived::Include, false),
            ("brews list", Archived::Exclude, true),
            ("brews list with archived", Archived::Include, false),
        ] {
            checked.push(Checked {
                name,
                statement: if name.starts_with("purchases") {
                    query::purchases_list(USER_ID, archived, None, 50).unwrap()
                } else {
                    query::brews_list(USER_ID, archived, None, 50).unwrap()
                },
                kind: Kind::List,
                default_archived,
            });
        }
        checked.push(Checked {
            name: "purchases list with a date cursor",
            statement: query::purchases_list(
                USER_ID,
                Archived::Exclude,
                Some(CursorKey::Date {
                    on: "2026-09-21".to_owned(),
                    id: SHOP_ID.to_owned(),
                }),
                50,
            )
            .unwrap(),
            kind: Kind::List,
            default_archived: true,
        });
        checked.push(Checked {
            name: "brews list with a cursor",
            statement: query::brews_list(USER_ID, Archived::Exclude, Some(cursor.clone()), 50)
                .unwrap(),
            kind: Kind::List,
            default_archived: true,
        });
        checked.push(Checked {
            name: "purchase find",
            statement: query::purchase_find(USER_ID, SHOP_ID, Archived::Include),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "brew find of the active rows",
            statement: query::brew_find(USER_ID, SHOP_ID, Archived::Exclude),
            kind: Kind::Row,
            default_archived: true,
        });
        checked.push(Checked {
            name: "purchase insert",
            statement: query::purchase_insert(
                SHOP_ID,
                USER_ID,
                &PurchaseValues {
                    product_id: "product-1",
                    shop_id: None,
                    purchased_on: "2026-09-21",
                    roast: None,
                    roast_date: None,
                    price_amount: None,
                    price_currency: None,
                    weight_grams: None,
                },
                AT,
                AT,
            )
            .unwrap(),
            kind: Kind::Insert,
            default_archived: false,
        });
        checked.push(Checked {
            name: "purchase update",
            statement: query::purchase_update(
                SHOP_ID,
                USER_ID,
                &PurchaseValues {
                    product_id: "product-1",
                    shop_id: None,
                    purchased_on: "2026-09-21",
                    roast: None,
                    roast_date: None,
                    price_amount: None,
                    price_currency: None,
                    weight_grams: None,
                },
                AT,
            )
            .unwrap(),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "purchase archive",
            statement: query::purchase_set_archived(SHOP_ID, USER_ID, Some(AT), AT).unwrap(),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "brew insert",
            statement: query::brew_insert(
                SHOP_ID,
                USER_ID,
                &BrewValues {
                    purchase_id: "purchase-1",
                    brewed_at: AT,
                    dose_grams: Some(15.5),
                    water_grams: None,
                    water_temp_c: None,
                    brew_time_seconds: None,
                    method: None,
                    grind_setting: None,
                    rating: None,
                    notes: None,
                },
                AT,
                AT,
            )
            .unwrap(),
            kind: Kind::Insert,
            default_archived: false,
        });
        checked.push(Checked {
            name: "brew update",
            statement: query::brew_update(
                SHOP_ID,
                USER_ID,
                &BrewValues {
                    purchase_id: "purchase-1",
                    brewed_at: AT,
                    dose_grams: None,
                    water_grams: None,
                    water_temp_c: None,
                    brew_time_seconds: None,
                    method: None,
                    grind_setting: None,
                    rating: None,
                    notes: None,
                },
                AT,
            )
            .unwrap(),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "brew unarchive",
            statement: query::brew_set_archived(SHOP_ID, USER_ID, None, AT).unwrap(),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "shops list with a cursor",
            statement: query::shops_list(USER_ID, Archived::Exclude, Some(cursor.clone()), 50)
                .unwrap(),
            kind: Kind::List,
            default_archived: true,
        });
        checked.push(Checked {
            name: "products list with a cursor",
            statement: query::products_list(USER_ID, Archived::Exclude, Some(cursor), 50).unwrap(),
            kind: Kind::List,
            default_archived: true,
        });
        checked.push(Checked {
            name: "shop find",
            statement: query::shop_find(USER_ID, SHOP_ID, Archived::Include),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "product find of the active rows",
            statement: query::product_find(USER_ID, SHOP_ID, Archived::Exclude),
            kind: Kind::Row,
            default_archived: true,
        });
        checked.push(Checked {
            name: "shop insert",
            statement: query::shop_insert(
                SHOP_ID,
                USER_ID,
                &ShopValues {
                    name: "店",
                    address: None,
                },
                AT,
                AT,
            )
            .unwrap(),
            kind: Kind::Insert,
            default_archived: false,
        });
        checked.push(Checked {
            name: "product insert",
            statement: query::product_insert(
                SHOP_ID,
                USER_ID,
                &ProductValues {
                    name: "豆",
                    producer: None,
                    origin: None,
                    region: None,
                    process: None,
                    variety: None,
                },
                AT,
                AT,
            )
            .unwrap(),
            kind: Kind::Insert,
            default_archived: false,
        });
        checked.push(Checked {
            name: "shop update",
            statement: query::shop_update(
                SHOP_ID,
                USER_ID,
                &ShopValues {
                    name: "店",
                    address: None,
                },
                AT,
            )
            .unwrap(),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "product update",
            statement: query::product_update(
                SHOP_ID,
                USER_ID,
                &ProductValues {
                    name: "豆",
                    producer: None,
                    origin: None,
                    region: None,
                    process: None,
                    variety: None,
                },
                AT,
            )
            .unwrap(),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "shop archive",
            statement: query::shop_set_archived(SHOP_ID, USER_ID, Some(AT), AT).unwrap(),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "product unarchive",
            statement: query::product_set_archived(SHOP_ID, USER_ID, None, AT).unwrap(),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "delete the product flavor tags",
            statement: query::delete_product_flavor_tags(USER_ID, SHOP_ID),
            kind: Kind::Row,
            default_archived: false,
        });
        checked.push(Checked {
            name: "insert a flavor tag",
            statement: query::insert_flavor_tag(SHOP_ID, USER_ID, "チョコ"),
            kind: Kind::Insert,
            default_archived: false,
        });
        checked.push(Checked {
            name: "insert a product flavor tag",
            statement: query::insert_product_flavor_tag(USER_ID, SHOP_ID, "チョコ"),
            kind: Kind::Insert,
            default_archived: false,
        });
        checked.push(Checked {
            name: "flavor tags list",
            statement: query::flavor_tags_list(USER_ID),
            kind: Kind::Row,
            default_archived: false,
        });
        for statement in query::product_flavor_notes(USER_ID, &[SHOP_ID]) {
            checked.push(Checked {
                name: "product flavor notes",
                statement,
                kind: Kind::Row,
                default_archived: false,
            });
        }
        checked
    }

    #[test]
    fn every_record_query_keeps_the_user_and_archived_conditions() {
        for checked in every_statement() {
            match checked.kind {
                Kind::List | Kind::Row => assert!(
                    has_user_condition(&checked.statement),
                    "the user filter is missing in {}: {}",
                    checked.name,
                    checked.statement.sql
                ),
                Kind::Insert => assert!(
                    carries_user_id(&checked.statement, USER_ID),
                    "the user id is missing in {}: {}",
                    checked.name,
                    checked.statement.sql
                ),
            }
            if checked.default_archived {
                assert!(
                    has_archived_condition(&checked.statement),
                    "the archived filter is missing in {}: {}",
                    checked.name,
                    checked.statement.sql
                );
            }
            let placeholders = checked.statement.sql.matches('?').count();
            assert_eq!(
                placeholders,
                checked.statement.params.len(),
                "every placeholder must have a value in {}: {}",
                checked.name,
                checked.statement.sql
            );
        }
    }

    #[test]
    fn the_checker_detects_an_omitted_user_condition() {
        let statement = Statement {
            sql: "SELECT id FROM shops WHERE id = ?".to_owned(),
            params: vec![Value::Text(SHOP_ID.to_owned())],
        };
        assert!(
            !has_user_condition(&statement),
            "a query without the user filter must be detected"
        );
        assert!(
            !carries_user_id(&statement, USER_ID),
            "an insert without the user id must be detected"
        );
    }

    #[test]
    fn the_checker_detects_an_omitted_archived_condition() {
        let statement = Statement {
            sql: "SELECT id FROM shops WHERE user_id = ?".to_owned(),
            params: vec![Value::Text(USER_ID.to_owned())],
        };
        assert!(
            !has_archived_condition(&statement),
            "a default list without the archived filter must be detected"
        );
        // 条件がある文は通る (検査そのものが常に失敗するわけではない)。
        assert!(has_archived_condition(&Statement {
            sql: "SELECT id FROM shops WHERE user_id = ? AND archived_at IS NULL".to_owned(),
            params: vec![Value::Text(USER_ID.to_owned())],
        }));
    }
}
