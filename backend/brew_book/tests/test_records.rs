//! `records` の単体テスト。一覧の続きのカーソルの組み立てと、テスト用の下ごしらえの値を確認する。

mod support;

use brew_book::records::ListParams;
use brew_book_core::cursor::{CursorKey, CursorValue, SortKey, SortOrder};

/// 並び順以外は固定のパラメータ。
fn list_params(limit: u32) -> ListParams {
    ListParams {
        limit,
        cursor: None,
        sort: SortKey::CreatedAt,
        order: SortOrder::Desc,
        favorite: false,
    }
}

#[test]
fn the_next_cursor_carries_the_sort_key_and_the_order() {
    let params = list_params(50);
    // 日時のキーでは文字列の値のカーソルを返す。
    let cursor = params
        .next_cursor(
            Some(CursorValue::Text("2026-09-23T00:00:00.000Z".to_owned())),
            "shop-1",
            50,
        )
        .expect("a full page must have a cursor");
    assert_eq!(
        CursorKey::decode(&cursor),
        Ok(CursorKey {
            sort: SortKey::CreatedAt,
            order: SortOrder::Desc,
            value: Some(CursorValue::Text("2026-09-23T00:00:00.000Z".to_owned())),
            id: "shop-1".to_owned(),
        })
    );

    // 並び順と方向は params のものをそのまま運ぶ (FR-20)。
    let mut params = list_params(50);
    params.sort = SortKey::PurchasedOn;
    params.order = SortOrder::Asc;
    let cursor = params
        .next_cursor(
            Some(CursorValue::Text("2026-09-23".to_owned())),
            "purchase-1",
            50,
        )
        .expect("a full page must have a cursor");
    assert_eq!(
        CursorKey::decode(&cursor),
        Ok(CursorKey {
            sort: SortKey::PurchasedOn,
            order: SortOrder::Asc,
            value: Some(CursorValue::Text("2026-09-23".to_owned())),
            id: "purchase-1".to_owned(),
        })
    );

    // 数値のキーと NULL の値もそのまま運ぶ。
    let mut params = list_params(50);
    params.sort = SortKey::Rating;
    let cursor = params
        .next_cursor(Some(CursorValue::Integer(4)), "brew-1", 50)
        .expect("a full page must have a cursor");
    assert_eq!(
        CursorKey::decode(&cursor),
        Ok(CursorKey {
            sort: SortKey::Rating,
            order: SortOrder::Desc,
            value: Some(CursorValue::Integer(4)),
            id: "brew-1".to_owned(),
        })
    );
    let cursor = params
        .next_cursor(None, "brew-1", 50)
        .expect("a full page must have a cursor");
    assert_eq!(
        CursorKey::decode(&cursor),
        Ok(CursorKey {
            sort: SortKey::Rating,
            order: SortOrder::Desc,
            value: None,
            id: "brew-1".to_owned(),
        })
    );
}

#[test]
fn a_page_without_the_full_count_has_no_next_cursor() {
    let params = list_params(50);
    assert_eq!(
        params.next_cursor(
            Some(CursorValue::Text("2026-09-23T00:00:00.000Z".to_owned())),
            "shop-1",
            49
        ),
        None
    );
}

#[test]
fn the_test_user_ids_are_unique_and_well_formed() {
    let ids = [
        support::seed::user_id(1),
        support::seed::user_id(10),
        support::seed::user_id(11),
    ];
    assert_ne!(ids[0], ids[1], "1 and 10 must not collide");
    assert_ne!(ids[1], ids[2], "10 and 11 must not collide");
    for id in &ids {
        let groups: Vec<&str> = id.split('-').collect();
        assert_eq!(groups.len(), 5, "the id must have 5 groups: {id}");
        assert_eq!(
            groups[0].len(),
            8,
            "the first group must be 8 characters: {id}"
        );
        assert_eq!(
            groups[1].len(),
            4,
            "the second group must be 4 characters: {id}"
        );
        assert!(
            groups[2].starts_with('4'),
            "the third group must start with the version 4: {id}"
        );
        assert!(
            groups[3].starts_with('8'),
            "the fourth group must start with the variant 8: {id}"
        );
        assert_eq!(
            groups[4].len(),
            12,
            "the last group must be 12 characters: {id}"
        );
    }
}
