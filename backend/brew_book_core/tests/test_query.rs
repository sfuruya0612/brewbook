//! `query` の単体テスト。組み立てた SQL が利用者 ID の条件を必ず含むこと、
//! 値がプレースホルダで渡ることを確認する。
//!
//! 組み立てる関数ごとの SQL は [`record_queries`]、条件の付け忘れの検出は [`conditions`] が検査する。

use brew_book_core::cursor::{CursorKey, CursorValue, SortKey, SortOrder};
use brew_book_core::query::{QueryError, Statement, Value};

const USER_ID: &str = "9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60";
const SHOP_ID: &str = "0d2b6f5e-3a4c-4a7b-9c8d-7e6f5a4b3c2d";
const AT: &str = "2026-09-21T12:34:56.789Z";

mod list_builders {
    use super::*;
    use brew_book_core::cursor::{CursorValue, SortKey, SortOrder};
    use brew_book_core::query::{list, ListQuery};

    fn shop_query(cursor: Option<CursorKey>, limit: u32) -> ListQuery<'static> {
        ListQuery {
            table: "shops",
            columns: "id, user_id, name, address, created_at, updated_at",
            user_id: USER_ID,
            sort: SortKey::CreatedAt,
            order: SortOrder::Desc,
            favorite_only: false,
            cursor,
            limit,
            name: None,
        }
    }

    /// キーの値が文字列のカーソルを作る。
    fn text_cursor(sort: SortKey, value: &str, order: SortOrder) -> CursorKey {
        CursorKey {
            sort,
            order,
            value: Some(CursorValue::Text(value.to_owned())),
            id: SHOP_ID.to_owned(),
        }
    }

    #[test]
    fn a_list_query_filters_by_the_user() {
        let statement = list(&shop_query(None, 50)).unwrap();
        assert_eq!(
            statement.sql,
            "SELECT id, user_id, name, address, created_at, updated_at FROM shops \
             WHERE user_id = ? \
             ORDER BY created_at DESC NULLS LAST, id ASC LIMIT ?"
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
    fn a_list_query_with_a_cursor_adds_the_keyset_condition() {
        let cursor = text_cursor(SortKey::CreatedAt, AT, SortOrder::Desc);
        let statement = list(&shop_query(Some(cursor), 10)).unwrap();
        assert_eq!(
            statement.sql,
            "SELECT id, user_id, name, address, created_at, updated_at FROM shops \
             WHERE user_id = ? \
             AND (created_at IS NULL OR created_at < ? OR (created_at = ? AND id > ?)) \
             ORDER BY created_at DESC NULLS LAST, id ASC LIMIT ?"
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text(AT.to_owned()),
                Value::Text(AT.to_owned()),
                Value::Text(SHOP_ID.to_owned()),
                Value::Integer(10),
            ]
        );
    }

    #[test]
    fn a_list_query_of_a_date_order_key_uses_the_date_column() {
        let query = ListQuery {
            table: "purchases",
            columns: "id, user_id, purchased_on",
            user_id: USER_ID,
            sort: SortKey::PurchasedOn,
            order: SortOrder::Desc,
            favorite_only: false,
            cursor: Some(text_cursor(
                SortKey::PurchasedOn,
                "2026-09-21",
                SortOrder::Desc,
            )),
            limit: 200,
            name: None,
        };
        let statement = list(&query).unwrap();
        assert!(statement.sql.contains(
            "AND (purchased_on IS NULL OR purchased_on < ? OR (purchased_on = ? AND id > ?))"
        ));
        assert!(statement
            .sql
            .contains("ORDER BY purchased_on DESC NULLS LAST, id ASC LIMIT ?"));
        assert_eq!(statement.params.len(), 5);
        assert_eq!(statement.params[4], Value::Integer(200));
    }

    #[test]
    fn an_ascending_list_query_compares_the_other_way() {
        let cursor = text_cursor(SortKey::CreatedAt, AT, SortOrder::Asc);
        let mut query = shop_query(Some(cursor), 10);
        query.order = SortOrder::Asc;
        let statement = list(&query).unwrap();
        assert!(statement
            .sql
            .contains("AND (created_at IS NULL OR created_at > ? OR (created_at = ? AND id > ?))"));
        assert!(statement
            .sql
            .contains("ORDER BY created_at ASC NULLS LAST, id ASC LIMIT ?"));
    }

    #[test]
    fn a_cursor_with_a_null_key_continues_from_the_null_rows() {
        let cursor = CursorKey {
            sort: SortKey::CreatedAt,
            order: SortOrder::Desc,
            value: None,
            id: SHOP_ID.to_owned(),
        };
        let statement = list(&shop_query(Some(cursor), 50)).unwrap();
        assert_eq!(
            statement.sql,
            "SELECT id, user_id, name, address, created_at, updated_at FROM shops \
             WHERE user_id = ? AND (created_at IS NULL AND id > ?) \
             ORDER BY created_at DESC NULLS LAST, id ASC LIMIT ?"
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text(SHOP_ID.to_owned()),
                Value::Integer(50),
            ]
        );
    }

    #[test]
    fn a_numeric_cursor_binds_the_number_with_its_type() {
        let integer = CursorKey {
            sort: SortKey::Rating,
            order: SortOrder::Desc,
            value: Some(CursorValue::Integer(4)),
            id: SHOP_ID.to_owned(),
        };
        let query = ListQuery {
            table: "brews",
            columns: "id, user_id, rating",
            user_id: USER_ID,
            sort: SortKey::Rating,
            order: SortOrder::Desc,
            favorite_only: false,
            cursor: Some(integer),
            limit: 50,
            name: None,
        };
        let statement = list(&query).unwrap();
        assert!(statement
            .sql
            .contains("AND (rating IS NULL OR rating < ? OR (rating = ? AND id > ?))"));
        assert!(statement
            .sql
            .contains("ORDER BY rating DESC NULLS LAST, id ASC"));
        assert_eq!(
            statement.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Integer(4),
                Value::Integer(4),
                Value::Text(SHOP_ID.to_owned()),
                Value::Integer(50),
            ]
        );

        let real = CursorKey {
            sort: SortKey::DoseGrams,
            order: SortOrder::Asc,
            value: Some(CursorValue::Real(15.5)),
            id: SHOP_ID.to_owned(),
        };
        let query = ListQuery {
            table: "brews",
            columns: "id, user_id, dose_grams",
            user_id: USER_ID,
            sort: SortKey::DoseGrams,
            order: SortOrder::Asc,
            favorite_only: false,
            cursor: Some(real),
            limit: 50,
            name: None,
        };
        let statement = list(&query).unwrap();
        assert!(statement
            .sql
            .contains("AND (dose_grams IS NULL OR dose_grams > ? OR (dose_grams = ? AND id > ?))"));
        assert!(statement.params.contains(&Value::Real(15.5)));
    }

    #[test]
    fn a_name_cursor_compares_without_case() {
        let cursor = text_cursor(SortKey::Name, "Ethiopia", SortOrder::Asc);
        let query = ListQuery {
            table: "shops",
            columns: "id, user_id, name",
            user_id: USER_ID,
            sort: SortKey::Name,
            order: SortOrder::Asc,
            favorite_only: false,
            cursor: Some(cursor),
            limit: 50,
            name: None,
        };
        let statement = list(&query).unwrap();
        assert!(statement.sql.contains(
            "AND (name IS NULL OR name COLLATE NOCASE > ? OR (name COLLATE NOCASE = ? AND id > ?))"
        ));
        assert!(statement
            .sql
            .contains("ORDER BY name COLLATE NOCASE ASC NULLS LAST, id ASC"));
    }

    #[test]
    fn every_sort_key_orders_with_its_column_and_direction() {
        // 9 つのキーすべてが、両方の方向で自分の列と NULLS LAST と ID の昇順で並ぶ (FR-20)。
        let cases = [
            (SortKey::BrewedAt, "brewed_at", false),
            (SortKey::Rating, "rating", false),
            (SortKey::DoseGrams, "dose_grams", false),
            (SortKey::PurchasedOn, "purchased_on", false),
            (SortKey::PriceAmount, "price_amount", false),
            (SortKey::WeightGrams, "weight_grams", false),
            (SortKey::CreatedAt, "created_at", false),
            (SortKey::Name, "name", true),
            (SortKey::UpdatedAt, "updated_at", false),
        ];
        for (sort, column, nocase) in cases {
            for (order, direction) in [(SortOrder::Asc, "ASC"), (SortOrder::Desc, "DESC")] {
                let mut query = shop_query(None, 50);
                query.sort = sort;
                query.order = order;
                let statement = list(&query).unwrap();
                let ordered = if nocase {
                    format!("ORDER BY {column} COLLATE NOCASE {direction} NULLS LAST, id ASC")
                } else {
                    format!("ORDER BY {column} {direction} NULLS LAST, id ASC")
                };
                assert!(
                    statement.sql.contains(&ordered),
                    "the {sort:?} {order:?} query must contain {ordered}: {}",
                    statement.sql
                );
            }
        }
    }

    #[test]
    fn a_favorite_only_list_filters_by_the_favorited_column() {
        let mut query = shop_query(None, 50);
        query.favorite_only = true;
        let statement = list(&query).unwrap();
        assert_eq!(
            statement.sql,
            "SELECT id, user_id, name, address, created_at, updated_at FROM shops \
             WHERE user_id = ? AND favorited_at IS NOT NULL \
             ORDER BY created_at DESC NULLS LAST, id ASC LIMIT ?"
        );
        assert_eq!(statement.params.len(), 2);
    }

    #[test]
    fn a_list_query_with_a_name_filters_by_the_exact_name_ignoring_case() {
        let mut query = shop_query(None, 50);
        query.name = Some("Ethiopia");
        let statement = list(&query).unwrap();
        assert_eq!(
            statement.sql,
            "SELECT id, user_id, name, address, created_at, updated_at FROM shops \
             WHERE user_id = ? AND name = ? COLLATE NOCASE \
             ORDER BY created_at DESC NULLS LAST, id ASC LIMIT ?"
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text("Ethiopia".to_owned()),
                Value::Integer(50),
            ]
        );
    }

    #[test]
    fn a_cursor_of_another_key_or_order_is_rejected() {
        // 別のキーのカーソル。
        let other_key = text_cursor(SortKey::PurchasedOn, "2026-09-21", SortOrder::Desc);
        assert_eq!(
            list(&shop_query(Some(other_key), 50)),
            Err(QueryError::CursorMismatch)
        );
        // 別の方向のカーソル。
        let other_order = text_cursor(SortKey::CreatedAt, AT, SortOrder::Asc);
        assert_eq!(
            list(&shop_query(Some(other_order), 50)),
            Err(QueryError::CursorMismatch)
        );
    }

    #[test]
    fn every_list_query_keeps_the_user_condition() {
        for sort in [
            SortKey::CreatedAt,
            SortKey::Name,
            SortKey::UpdatedAt,
            SortKey::BrewedAt,
            SortKey::Rating,
            SortKey::DoseGrams,
        ] {
            for order in [SortOrder::Desc, SortOrder::Asc] {
                let value = match sort.value_kind() {
                    brew_book_core::cursor::SortValueKind::Integer => Some(CursorValue::Integer(4)),
                    brew_book_core::cursor::SortValueKind::Real => Some(CursorValue::Real(15.5)),
                    _ => Some(CursorValue::Text(AT.to_owned())),
                };
                for cursor in [
                    None,
                    Some(CursorKey {
                        sort,
                        order,
                        value: value.clone(),
                        id: SHOP_ID.to_owned(),
                    }),
                    Some(CursorKey {
                        sort,
                        order,
                        value: None,
                        id: SHOP_ID.to_owned(),
                    }),
                ] {
                    let query = ListQuery {
                        table: "brews",
                        columns: "id, user_id, brewed_at",
                        user_id: USER_ID,
                        sort,
                        order,
                        favorite_only: false,
                        cursor,
                        limit: 50,
                        name: None,
                    };
                    let statement = list(&query).unwrap();
                    assert!(
                        statement.sql.contains("WHERE user_id = ?"),
                        "the user filter is missing: {}",
                        statement.sql
                    );
                    assert_eq!(statement.params[0], Value::Text(USER_ID.to_owned()));
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
}

mod record_queries {
    use super::*;
    use brew_book_core::query::{
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
        let statement = query::shop_find(USER_ID, SHOP_ID);
        assert_eq!(
            statement.sql,
            "SELECT id, user_id, name, address, created_at, updated_at, favorited_at FROM shops \
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
        assert_eq!(QueryError::CursorMismatch.code().status(), 400);
        assert!(!QueryError::ColumnCountMismatch.message().is_empty());
    }
}

mod purchase_and_brew_queries {
    use super::*;
    use brew_book_core::query::{self, BrewValues, PurchaseValues};

    /// 購入の列の別名 (結合の SQL の期待値に使う)。
    const PURCHASE_COLUMNS: &str = "p.id AS p_id, p.user_id AS p_user_id, \
        p.product_id AS p_product_id, p.shop_id AS p_shop_id, p.purchased_on AS p_purchased_on, \
        p.roast AS p_roast, p.roast_date AS p_roast_date, p.price_amount AS p_price_amount, \
        p.price_currency AS p_price_currency, p.weight_grams AS p_weight_grams, \
        p.photo_key AS p_photo_key, p.created_at AS p_created_at, p.updated_at AS p_updated_at, \
        p.favorited_at AS p_favorited_at";
    /// 商品の列の別名。
    const PRODUCT_COLUMNS: &str = "pr.id AS pr_id, pr.user_id AS pr_user_id, pr.name AS pr_name, \
        pr.producer AS pr_producer, pr.origin AS pr_origin, pr.region AS pr_region, \
        pr.process AS pr_process, pr.variety AS pr_variety, pr.created_at AS pr_created_at, \
        pr.updated_at AS pr_updated_at, pr.favorited_at AS pr_favorited_at";
    /// 店の列の別名。
    const SHOP_COLUMNS: &str = "sh.id AS sh_id, sh.user_id AS sh_user_id, sh.name AS sh_name, \
        sh.address AS sh_address, sh.created_at AS sh_created_at, sh.updated_at AS sh_updated_at, \
        sh.favorited_at AS sh_favorited_at";

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
        let statement = query::purchases_list(
            USER_ID,
            SortKey::PurchasedOn,
            SortOrder::Desc,
            false,
            None,
            50,
        )
        .unwrap();
        assert_eq!(
            statement.sql,
            format!(
                "SELECT {PURCHASE_COLUMNS}, {PRODUCT_COLUMNS}, {SHOP_COLUMNS} \
                 FROM purchases AS p \
                 INNER JOIN products AS pr ON pr.id = p.product_id AND pr.user_id = p.user_id \
                 LEFT JOIN shops AS sh ON sh.id = p.shop_id AND sh.user_id = p.user_id \
                 WHERE p.user_id = ? \
                 ORDER BY p.purchased_on DESC NULLS LAST, p.id ASC LIMIT ?"
            )
        );
        assert_eq!(
            statement.params,
            vec![Value::Text(USER_ID.to_owned()), Value::Integer(50)]
        );
    }

    #[test]
    fn a_purchases_list_query_with_a_cursor_uses_the_purchase_date() {
        let cursor = CursorKey {
            sort: SortKey::PurchasedOn,
            order: SortOrder::Desc,
            value: Some(CursorValue::Text("2026-09-21".to_owned())),
            id: SHOP_ID.to_owned(),
        };
        let statement = query::purchases_list(
            USER_ID,
            SortKey::PurchasedOn,
            SortOrder::Desc,
            false,
            Some(cursor),
            10,
        )
        .unwrap();
        assert!(
            statement.sql.contains(
                "AND (p.purchased_on IS NULL OR p.purchased_on < ? OR (p.purchased_on = ? AND p.id > ?))"
            ),
            "{}",
            statement.sql
        );
        assert_eq!(statement.params.len(), 5);
        assert_eq!(statement.params[1], Value::Text("2026-09-21".to_owned()));
        assert_eq!(statement.params[3], Value::Text(SHOP_ID.to_owned()));
    }

    #[test]
    fn a_purchases_cursor_of_another_key_is_rejected() {
        let cursor = CursorKey {
            sort: SortKey::PriceAmount,
            order: SortOrder::Desc,
            value: Some(CursorValue::Integer(1200)),
            id: SHOP_ID.to_owned(),
        };
        assert_eq!(
            query::purchases_list(
                USER_ID,
                SortKey::PurchasedOn,
                SortOrder::Desc,
                false,
                Some(cursor),
                50,
            ),
            Err(QueryError::CursorMismatch)
        );
    }

    #[test]
    fn a_brews_list_query_joins_the_purchase_the_product_and_the_shop() {
        let statement = query::brews_list(
            USER_ID,
            SortKey::BrewedAt,
            SortOrder::Desc,
            false,
            None,
            200,
        )
        .unwrap();
        assert_eq!(
            statement.sql,
            format!(
                "SELECT b.id AS b_id, b.user_id AS b_user_id, b.purchase_id AS b_purchase_id, \
                 b.brewed_at AS b_brewed_at, b.dose_grams AS b_dose_grams, \
                 b.water_grams AS b_water_grams, b.water_temp_c AS b_water_temp_c, \
                 b.brew_time_seconds AS b_brew_time_seconds, b.method AS b_method, \
                 b.grind_setting AS b_grind_setting, b.rating AS b_rating, b.notes AS b_notes, \
                 b.created_at AS b_created_at, b.updated_at AS b_updated_at, \
                 b.favorited_at AS b_favorited_at, \
                 {PURCHASE_COLUMNS}, {PRODUCT_COLUMNS}, {SHOP_COLUMNS} \
                 FROM brews AS b \
                 INNER JOIN purchases AS p ON p.id = b.purchase_id AND p.user_id = b.user_id \
                 INNER JOIN products AS pr ON pr.id = p.product_id AND pr.user_id = p.user_id \
                 LEFT JOIN shops AS sh ON sh.id = p.shop_id AND sh.user_id = p.user_id \
                 WHERE b.user_id = ? \
                 ORDER BY b.brewed_at DESC NULLS LAST, b.id ASC LIMIT ?"
            )
        );
        assert_eq!(
            statement.params,
            vec![Value::Text(USER_ID.to_owned()), Value::Integer(200)]
        );
    }

    #[test]
    fn a_brew_find_query_filters_by_the_id_and_the_user() {
        let statement = query::brew_find(USER_ID, SHOP_ID);
        assert_eq!(
            statement.sql,
            format!(
                "SELECT b.id AS b_id, b.user_id AS b_user_id, b.purchase_id AS b_purchase_id, \
                 b.brewed_at AS b_brewed_at, b.dose_grams AS b_dose_grams, \
                 b.water_grams AS b_water_grams, b.water_temp_c AS b_water_temp_c, \
                 b.brew_time_seconds AS b_brew_time_seconds, b.method AS b_method, \
                 b.grind_setting AS b_grind_setting, b.rating AS b_rating, b.notes AS b_notes, \
                 b.created_at AS b_created_at, b.updated_at AS b_updated_at, \
                 b.favorited_at AS b_favorited_at, \
                 {PURCHASE_COLUMNS}, {PRODUCT_COLUMNS}, {SHOP_COLUMNS} \
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
        let statement = query::purchase_find(USER_ID, SHOP_ID);
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
}

mod suggestions {
    //! サジェストのクエリ (FR-13) の単体テスト。
    //!
    //! 8 つの項目名とテーブルと列の対応、値のまとめ方と並び順、`LIKE` のパターンのエスケープを
    //! 確認する。エスケープが任意の値で往復することは PBT (`prop_query.rs`) が担う。

    use super::*;
    use brew_book_core::query::{
        self, parse_suggestion_field, SuggestionFieldError, SuggestionItem, SUGGESTION_LIMIT,
    };

    #[test]
    fn the_eight_field_names_are_accepted_and_the_others_are_rejected() {
        for (name, item) in [
            ("producer", SuggestionItem::Producer),
            ("origin", SuggestionItem::Origin),
            ("region", SuggestionItem::Region),
            ("process", SuggestionItem::Process),
            ("variety", SuggestionItem::Variety),
            ("roast", SuggestionItem::Roast),
            ("method", SuggestionItem::Method),
            ("grind_setting", SuggestionItem::GrindSetting),
        ] {
            assert_eq!(
                parse_suggestion_field(name),
                Ok(item),
                "{name} must be accepted"
            );
        }
        for name in [
            "",
            // 大文字と小文字を区別する (PRD の経路の表記のままだけを受け付ける)。
            "Producer",
            "grindSetting",
            "GrindSetting",
            "notes",
            "name",
            "shop",
            "producers",
            "origin_name",
        ] {
            assert_eq!(
                parse_suggestion_field(name),
                Err(SuggestionFieldError::Unknown),
                "{name} must be rejected"
            );
        }
        assert_eq!(SuggestionFieldError::Unknown.code().status(), 400);
        assert!(!SuggestionFieldError::Unknown.message().is_empty());
    }

    #[test]
    fn the_query_of_each_field_uses_its_table_and_column() {
        for (item, table, column) in [
            (SuggestionItem::Producer, "products", "producer"),
            (SuggestionItem::Origin, "products", "origin"),
            (SuggestionItem::Region, "products", "region"),
            (SuggestionItem::Process, "products", "process"),
            (SuggestionItem::Variety, "products", "variety"),
            (SuggestionItem::Roast, "purchases", "roast"),
            (SuggestionItem::Method, "brews", "method"),
            (SuggestionItem::GrindSetting, "brews", "grind_setting"),
        ] {
            assert_eq!(item.table(), table);
            assert_eq!(item.column(), column);
            let statement = query::suggestions(USER_ID, item, "エチ");
            assert!(
                statement
                    .sql
                    .starts_with(&format!("SELECT {column} AS value FROM {table} ")),
                "{}",
                statement.sql
            );
        }
    }

    #[test]
    fn a_suggestion_query_groups_the_values_and_orders_them() {
        let statement = query::suggestions(USER_ID, SuggestionItem::Producer, "エチ");
        assert_eq!(
            statement.sql,
            "SELECT producer AS value FROM products WHERE user_id = ? \
             AND lower(producer) LIKE lower(?) || '%' ESCAPE '\\' GROUP BY producer \
             ORDER BY MAX(updated_at) DESC, value ASC LIMIT ?"
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text("エチ".to_owned()),
                Value::Integer(20),
            ]
        );
        assert_eq!(SUGGESTION_LIMIT, 20);
    }

    #[test]
    fn the_pattern_escapes_the_wildcards_and_the_escape_character() {
        // ワイルドカードとエスケープ文字を含む入力の正確な出力を 1 例で確認する。
        // 任意の入力での復元は PBT (`prop_query.rs`) が担う。
        let statement = query::suggestions(USER_ID, SuggestionItem::Method, "100%_a\\b");
        assert_eq!(
            statement.params[1],
            Value::Text("100\\%\\_a\\\\b".to_owned())
        );
        assert_eq!(statement.params[2], Value::Integer(20));
    }
}

mod export_queries {
    use super::*;
    use brew_book_core::query;

    #[test]
    fn an_export_query_carries_every_column_of_the_table_without_a_limit() {
        let statement =
            query::export_rows(query::SHOPS_TABLE, query::SHOP_COLUMNS, "id ASC", USER_ID);
        assert_eq!(
            statement.sql,
            "SELECT id, user_id, name, address, created_at, updated_at, favorited_at FROM shops \
             WHERE user_id = ? ORDER BY id ASC"
        );
        assert_eq!(statement.params, vec![Value::Text(USER_ID.to_owned())]);
        assert!(
            !statement.sql.contains("LIMIT"),
            "the export must carry every row: {}",
            statement.sql
        );
        assert!(
            !statement.sql.contains(USER_ID),
            "the user id must not appear in the SQL: {}",
            statement.sql
        );
    }

    #[test]
    fn an_export_query_of_the_product_flavor_tags_orders_by_the_primary_key() {
        let statement = query::export_rows(
            query::PRODUCT_FLAVOR_TAGS_TABLE,
            query::PRODUCT_FLAVOR_TAG_COLUMNS,
            "product_id ASC, tag_id ASC",
            USER_ID,
        );
        assert_eq!(
            statement.sql,
            "SELECT user_id, product_id, tag_id FROM product_flavor_tags \
             WHERE user_id = ? ORDER BY product_id ASC, tag_id ASC"
        );
        assert_eq!(statement.params, vec![Value::Text(USER_ID.to_owned())]);
    }

    #[test]
    fn every_export_query_of_the_six_tables_keeps_the_user_condition() {
        for (table, columns, order_by) in [
            (query::SHOPS_TABLE, query::SHOP_COLUMNS, "id ASC"),
            (query::PRODUCTS_TABLE, query::PRODUCT_COLUMNS, "id ASC"),
            (
                query::FLAVOR_TAGS_TABLE,
                query::FLAVOR_TAG_COLUMNS,
                "id ASC",
            ),
            (
                query::PRODUCT_FLAVOR_TAGS_TABLE,
                query::PRODUCT_FLAVOR_TAG_COLUMNS,
                "product_id ASC, tag_id ASC",
            ),
            (query::PURCHASES_TABLE, query::PURCHASE_COLUMNS, "id ASC"),
            (query::BREWS_TABLE, query::BREW_COLUMNS, "id ASC"),
        ] {
            let statement = query::export_rows(table, columns, order_by, USER_ID);
            assert!(
                statement.sql.contains("WHERE user_id = ?"),
                "the user filter is missing in {table}: {}",
                statement.sql
            );
            assert_eq!(
                statement.params,
                vec![Value::Text(USER_ID.to_owned())],
                "the export of {table} must bind only the user"
            );
            let placeholders = statement.sql.matches('?').count();
            assert_eq!(
                placeholders,
                statement.params.len(),
                "every placeholder must have a value in {table}: {}",
                statement.sql
            );
        }
    }
}

mod deletion_queries {
    use super::*;
    use brew_book_core::query;

    #[test]
    fn every_delete_statement_filters_by_id_and_user() {
        for (name, statement, table) in [
            (
                "shop",
                query::shop_delete(USER_ID, SHOP_ID),
                query::SHOPS_TABLE,
            ),
            (
                "product",
                query::product_delete(USER_ID, SHOP_ID),
                query::PRODUCTS_TABLE,
            ),
            (
                "purchase",
                query::purchase_delete(USER_ID, SHOP_ID),
                query::PURCHASES_TABLE,
            ),
            (
                "brew",
                query::brew_delete(USER_ID, SHOP_ID),
                query::BREWS_TABLE,
            ),
        ] {
            assert_eq!(
                statement.sql,
                format!("DELETE FROM {table} WHERE id = ? AND user_id = ?"),
                "{name}"
            );
            assert_eq!(
                statement.params,
                vec![
                    Value::Text(SHOP_ID.to_owned()),
                    Value::Text(USER_ID.to_owned()),
                ],
                "{name}"
            );
            assert!(
                !statement.sql.contains(USER_ID) && !statement.sql.contains(SHOP_ID),
                "the values must not appear in the SQL of {name}: {}",
                statement.sql
            );
        }
    }

    #[test]
    fn the_shop_delete_clears_the_shop_of_the_purchases_and_updates_the_timestamp() {
        let statement = query::shop_clear_purchases(USER_ID, SHOP_ID, AT);
        assert_eq!(
            statement.sql,
            "UPDATE purchases SET shop_id = NULL, updated_at = ? \
             WHERE shop_id = ? AND user_id = ?"
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text(AT.to_owned()),
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
            ]
        );
    }

    #[test]
    fn the_product_delete_carries_the_children_and_the_photo_keys() {
        let brews = query::product_brews_delete(USER_ID, SHOP_ID);
        assert_eq!(
            brews.sql,
            "DELETE FROM brews WHERE user_id = ? AND purchase_id IN \
             (SELECT id FROM purchases WHERE product_id = ? AND user_id = ?)"
        );
        assert_eq!(
            brews.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
            ]
        );

        let purchases = query::product_purchases_delete(USER_ID, SHOP_ID);
        assert_eq!(
            purchases.sql,
            "DELETE FROM purchases WHERE product_id = ? AND user_id = ?"
        );
        assert_eq!(
            purchases.params,
            vec![
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
            ]
        );

        let keys = query::product_purchase_photo_keys(USER_ID, SHOP_ID);
        assert_eq!(
            keys.sql,
            "SELECT photo_key FROM purchases \
             WHERE product_id = ? AND user_id = ? AND photo_key IS NOT NULL"
        );
        assert_eq!(
            keys.params,
            vec![
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
            ]
        );

        let purchase_brews = query::purchase_brews_delete(USER_ID, SHOP_ID);
        assert_eq!(
            purchase_brews.sql,
            "DELETE FROM brews WHERE purchase_id = ? AND user_id = ?"
        );
    }

    #[test]
    fn the_impact_queries_count_the_rows_of_the_user() {
        let shops = query::shop_delete_impact(USER_ID, SHOP_ID);
        assert_eq!(
            shops.sql,
            "SELECT COUNT(*) AS count FROM purchases WHERE shop_id = ? AND user_id = ?"
        );
        assert_eq!(
            shops.params,
            vec![
                Value::Text(SHOP_ID.to_owned()),
                Value::Text(USER_ID.to_owned()),
            ]
        );

        let purchases = query::purchase_delete_impact(USER_ID, SHOP_ID);
        assert_eq!(
            purchases.sql,
            "SELECT COUNT(*) AS count FROM brews WHERE purchase_id = ? AND user_id = ?"
        );

        let products = query::product_delete_impact(USER_ID, SHOP_ID);
        assert!(products.sql.contains("AS purchases"), "{}", products.sql);
        assert!(products.sql.contains("AS brews"), "{}", products.sql);
        assert_eq!(products.params.len(), 5);
        let placeholders = products.sql.matches('?').count();
        assert_eq!(placeholders, products.params.len());
    }
}

mod conditions {
    use super::*;
    use brew_book_core::query::{self, BrewValues, ProductValues, PurchaseValues, ShopValues};

    /// 検査の対象にする文の種類。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Kind {
        /// 一覧。
        List,
        /// 1 件の取得と更新、削除。`id` と `user_id` の両方で絞る。
        Row,
        /// エクスポート (FR-14)。利用者の全行を引くため、`user_id` だけで絞る。
        Export,
        /// 挿入。`id` と `user_id` の列と値を必ず持つ。
        Insert,
    }

    /// 検査の対象にする 1 件の文。
    struct Checked {
        name: &'static str,
        statement: Statement,
        kind: Kind,
    }

    /// 利用者 ID の条件があるか。
    fn has_user_condition(statement: &Statement) -> bool {
        statement.sql.contains("user_id = ?")
    }

    /// 挿入の列と値に利用者 ID があるか。
    fn carries_user_id(statement: &Statement, user_id: &str) -> bool {
        statement.sql.contains("user_id")
            && statement.params.contains(&Value::Text(user_id.to_owned()))
    }

    /// モジュールが組み立てる文の一覧。
    fn every_statement() -> Vec<Checked> {
        let mut checked = Vec::new();
        checked.push(Checked {
            name: "shops list",
            statement: query::shops_list(
                USER_ID,
                SortKey::CreatedAt,
                SortOrder::Desc,
                false,
                None,
                50,
            )
            .unwrap(),
            kind: Kind::List,
        });
        checked.push(Checked {
            name: "products list",
            statement: query::products_list(
                USER_ID,
                SortKey::CreatedAt,
                SortOrder::Desc,
                false,
                None,
                50,
                None,
            )
            .unwrap(),
            kind: Kind::List,
        });
        checked.push(Checked {
            name: "products list with a name and favorites",
            statement: query::products_list(
                USER_ID,
                SortKey::Name,
                SortOrder::Asc,
                true,
                None,
                50,
                Some("名前"),
            )
            .unwrap(),
            kind: Kind::List,
        });
        let cursor = CursorKey {
            sort: SortKey::CreatedAt,
            order: SortOrder::Desc,
            value: Some(CursorValue::Text(AT.to_owned())),
            id: SHOP_ID.to_owned(),
        };
        checked.push(Checked {
            name: "purchases list",
            statement: query::purchases_list(
                USER_ID,
                SortKey::PurchasedOn,
                SortOrder::Desc,
                false,
                None,
                50,
            )
            .unwrap(),
            kind: Kind::List,
        });
        checked.push(Checked {
            name: "brews list",
            statement: query::brews_list(
                USER_ID,
                SortKey::BrewedAt,
                SortOrder::Desc,
                false,
                None,
                50,
            )
            .unwrap(),
            kind: Kind::List,
        });
        checked.push(Checked {
            name: "purchases list with a date cursor",
            statement: query::purchases_list(
                USER_ID,
                SortKey::PurchasedOn,
                SortOrder::Desc,
                false,
                Some(CursorKey {
                    sort: SortKey::PurchasedOn,
                    order: SortOrder::Desc,
                    value: Some(CursorValue::Text("2026-09-21".to_owned())),
                    id: SHOP_ID.to_owned(),
                }),
                50,
            )
            .unwrap(),
            kind: Kind::List,
        });
        checked.push(Checked {
            name: "brews list with a cursor",
            statement: query::brews_list(
                USER_ID,
                SortKey::BrewedAt,
                SortOrder::Desc,
                false,
                Some(CursorKey {
                    sort: SortKey::BrewedAt,
                    order: SortOrder::Desc,
                    value: Some(CursorValue::Text(AT.to_owned())),
                    id: SHOP_ID.to_owned(),
                }),
                50,
            )
            .unwrap(),
            kind: Kind::List,
        });
        checked.push(Checked {
            name: "purchase find",
            statement: query::purchase_find(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "brew find",
            statement: query::brew_find(USER_ID, SHOP_ID),
            kind: Kind::Row,
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
        });
        checked.push(Checked {
            name: "shops list with a cursor",
            statement: query::shops_list(
                USER_ID,
                SortKey::CreatedAt,
                SortOrder::Desc,
                false,
                Some(cursor.clone()),
                50,
            )
            .unwrap(),
            kind: Kind::List,
        });
        checked.push(Checked {
            name: "products list with a cursor",
            statement: query::products_list(
                USER_ID,
                SortKey::CreatedAt,
                SortOrder::Desc,
                false,
                Some(cursor),
                50,
                None,
            )
            .unwrap(),
            kind: Kind::List,
        });
        checked.push(Checked {
            name: "set the favorite",
            statement: query::set_favorited_at(query::SHOPS_TABLE, SHOP_ID, USER_ID, Some(AT), AT),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "shop delete",
            statement: query::shop_delete(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "shop clear purchases",
            statement: query::shop_clear_purchases(USER_ID, SHOP_ID, AT),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "shop delete impact",
            statement: query::shop_delete_impact(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "product delete",
            statement: query::product_delete(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "product brews delete",
            statement: query::product_brews_delete(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "product purchases delete",
            statement: query::product_purchases_delete(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "product purchase photo keys",
            statement: query::product_purchase_photo_keys(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "product delete impact",
            statement: query::product_delete_impact(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "purchase delete",
            statement: query::purchase_delete(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "purchase brews delete",
            statement: query::purchase_brews_delete(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "purchase delete impact",
            statement: query::purchase_delete_impact(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "brew delete",
            statement: query::brew_delete(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "shop find",
            statement: query::shop_find(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "product find",
            statement: query::product_find(USER_ID, SHOP_ID),
            kind: Kind::Row,
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
        });
        checked.push(Checked {
            name: "delete the product flavor tags",
            statement: query::delete_product_flavor_tags(USER_ID, SHOP_ID),
            kind: Kind::Row,
        });
        checked.push(Checked {
            name: "insert a flavor tag",
            statement: query::insert_flavor_tag(SHOP_ID, USER_ID, "チョコ"),
            kind: Kind::Insert,
        });
        checked.push(Checked {
            name: "insert a product flavor tag",
            statement: query::insert_product_flavor_tag(USER_ID, SHOP_ID, "チョコ"),
            kind: Kind::Insert,
        });
        checked.push(Checked {
            name: "flavor tags list",
            statement: query::flavor_tags_list(USER_ID),
            kind: Kind::Row,
        });
        for statement in query::product_flavor_notes(USER_ID, &[SHOP_ID]) {
            checked.push(Checked {
                name: "product flavor notes",
                statement,
                kind: Kind::Row,
            });
        }
        checked.push(Checked {
            name: "suggestions",
            statement: query::suggestions(USER_ID, query::SuggestionItem::Producer, "エチ"),
            kind: Kind::List,
        });
        // エクスポート (FR-14)。6 テーブルの全行を引く。
        for (name, table, columns, order_by) in [
            (
                "export the shops",
                query::SHOPS_TABLE,
                query::SHOP_COLUMNS,
                "id ASC",
            ),
            (
                "export the products",
                query::PRODUCTS_TABLE,
                query::PRODUCT_COLUMNS,
                "id ASC",
            ),
            (
                "export the flavor tags",
                query::FLAVOR_TAGS_TABLE,
                query::FLAVOR_TAG_COLUMNS,
                "id ASC",
            ),
            (
                "export the product flavor tags",
                query::PRODUCT_FLAVOR_TAGS_TABLE,
                query::PRODUCT_FLAVOR_TAG_COLUMNS,
                "product_id ASC, tag_id ASC",
            ),
            (
                "export the purchases",
                query::PURCHASES_TABLE,
                query::PURCHASE_COLUMNS,
                "id ASC",
            ),
            (
                "export the brews",
                query::BREWS_TABLE,
                query::BREW_COLUMNS,
                "id ASC",
            ),
        ] {
            checked.push(Checked {
                name,
                statement: query::export_rows(table, columns, order_by, USER_ID),
                kind: Kind::Export,
            });
        }
        checked
    }

    #[test]
    fn every_record_query_keeps_the_user_condition() {
        for checked in every_statement() {
            match checked.kind {
                Kind::List | Kind::Row | Kind::Export => assert!(
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
}
