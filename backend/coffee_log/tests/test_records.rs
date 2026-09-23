//! `records` の単体テスト。一覧の続きのカーソルの組み立てと、テスト用の下ごしらえの値を確認する。

mod support;

use coffee_log::records::ListParams;
use coffee_log_core::cursor::CursorKey;
use coffee_log_core::query::{Archived, OrderKind};

/// 並び順のキーの種類以外は固定のパラメータ。
fn params(limit: u32) -> ListParams {
    ListParams {
        limit,
        cursor: None,
        archived: Archived::Exclude,
    }
}

#[test]
fn the_next_cursor_uses_the_kind_of_the_ordering() {
    let params = params(50);
    // 日時の並び順では日時のカーソルを返す。
    let cursor = params
        .next_cursor(
            OrderKind::DateTime,
            Some(("2026-09-23T00:00:00.000Z", "shop-1")),
            50,
        )
        .expect("a full page must have a cursor");
    assert_eq!(
        CursorKey::decode(&cursor),
        Ok(CursorKey::DateTime {
            at: "2026-09-23T00:00:00.000Z".to_owned(),
            id: "shop-1".to_owned(),
        })
    );
    // 日付の並び順では日付のカーソルを返す (購入のように日付で並ぶ一覧のため)。
    let cursor = params
        .next_cursor(OrderKind::Date, Some(("2026-09-23", "purchase-1")), 50)
        .expect("a full page must have a cursor");
    assert_eq!(
        CursorKey::decode(&cursor),
        Ok(CursorKey::Date {
            on: "2026-09-23".to_owned(),
            id: "purchase-1".to_owned(),
        })
    );
}

#[test]
fn a_page_without_the_full_count_has_no_next_cursor() {
    let params = params(50);
    assert_eq!(
        params.next_cursor(
            OrderKind::DateTime,
            Some(("2026-09-23T00:00:00.000Z", "shop-1")),
            49
        ),
        None
    );
    assert_eq!(params.next_cursor(OrderKind::DateTime, None, 50), None);
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
