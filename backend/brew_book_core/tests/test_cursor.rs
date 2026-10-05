//! `cursor` の単体テスト。符号化の往復は PBT (`prop_cursor.rs`) が担う。

use brew_book_core::cursor::{
    parse_page_size, parse_sort_key, CursorError, CursorKey, CursorValue, PageSizeError, SortKey,
    SortKeyError, SortOrder, SortOrderError, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE,
};

const ID: &str = "9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60";
const AT: &str = "2026-09-21T12:34:56.789Z";

#[test]
fn the_page_size_defaults_to_fifty_and_accepts_the_upper_bound() {
    assert_eq!(parse_page_size(None), Ok(DEFAULT_PAGE_SIZE));
    assert_eq!(parse_page_size(Some("50")), Ok(50));
    assert_eq!(parse_page_size(Some("1")), Ok(1));
    assert_eq!(parse_page_size(Some("200")), Ok(MAX_PAGE_SIZE));
}

#[test]
fn the_page_size_rejects_zero_and_negative_values() {
    assert_eq!(parse_page_size(Some("0")), Err(PageSizeError::NotPositive));
    assert_eq!(parse_page_size(Some("-1")), Err(PageSizeError::NotPositive));
    assert_eq!(
        parse_page_size(Some("-200")),
        Err(PageSizeError::NotPositive)
    );
}

#[test]
fn the_page_size_rejects_values_that_are_not_integers() {
    for text in ["", " ", "1.5", "abc", "50a", "0x10", "５"] {
        assert_eq!(
            parse_page_size(Some(text)),
            Err(PageSizeError::NotAnInteger),
            "{text} must be rejected"
        );
    }
}

#[test]
fn the_page_size_rejects_values_above_the_upper_bound() {
    assert_eq!(parse_page_size(Some("201")), Err(PageSizeError::TooLarge));
    assert_eq!(
        parse_page_size(Some("999999999999")),
        Err(PageSizeError::TooLarge)
    );
}

#[test]
fn the_sort_key_names_are_the_query_parameters_and_unknown_names_are_rejected() {
    for (key, name) in [
        (SortKey::BrewedAt, "brewed_at"),
        (SortKey::Rating, "rating"),
        (SortKey::DoseGrams, "dose_grams"),
        (SortKey::PurchasedOn, "purchased_on"),
        (SortKey::PriceAmount, "price_amount"),
        (SortKey::WeightGrams, "weight_grams"),
        (SortKey::CreatedAt, "created_at"),
        (SortKey::Name, "name"),
        (SortKey::UpdatedAt, "updated_at"),
    ] {
        assert_eq!(key.as_str(), name);
        assert_eq!(parse_sort_key(name, &[key]), Ok(key));
    }
    // 一覧が受け付けるキーの並びに無い名前は拒否する (FR-20)。
    assert_eq!(
        parse_sort_key("brewed_at", &[SortKey::CreatedAt]),
        Err(SortKeyError::Unknown)
    );
    assert_eq!(parse_sort_key("unknown", &[]), Err(SortKeyError::Unknown));
}

#[test]
fn the_sort_order_names_are_the_query_parameters_and_unknown_names_are_rejected() {
    assert_eq!(SortOrder::Asc.as_str(), "asc");
    assert_eq!(SortOrder::Desc.as_str(), "desc");
    assert_eq!(SortOrder::parse("asc"), Ok(SortOrder::Asc));
    assert_eq!(SortOrder::parse("desc"), Ok(SortOrder::Desc));
    for text in ["", "ASC", "up", "1"] {
        assert_eq!(SortOrder::parse(text), Err(SortOrderError::Unknown));
    }
}

#[test]
fn a_datetime_cursor_is_encoded_as_the_base64url_of_its_json() {
    let cursor = CursorKey {
        sort: SortKey::CreatedAt,
        order: SortOrder::Desc,
        value: Some(CursorValue::Text(AT.to_owned())),
        id: ID.to_owned(),
    };
    assert_eq!(
        cursor.encode(),
        "eyJzb3J0IjoiY3JlYXRlZF9hdCIsIm9yZGVyIjoiZGVzYyIsInZhbHVlIjoiMjAyNi0wOS0yMVQxMjozNDo1Ni43ODlaIiwiaWQiOiI5ZjhmMWYyZS02YjFhLTRhM2MtOGQwZS0xYjJjM2Q0ZTVmNjAifQ"
    );
    assert!(
        !cursor.encode().contains('='),
        "the cursor must be unpadded"
    );
}

#[test]
fn a_date_cursor_and_a_null_value_cursor_are_encoded_as_the_base64url_of_their_json() {
    let date = CursorKey {
        sort: SortKey::PurchasedOn,
        order: SortOrder::Asc,
        value: Some(CursorValue::Text("2026-09-21".to_owned())),
        id: ID.to_owned(),
    };
    assert_eq!(
        date.encode(),
        "eyJzb3J0IjoicHVyY2hhc2VkX29uIiwib3JkZXIiOiJhc2MiLCJ2YWx1ZSI6IjIwMjYtMDktMjEiLCJpZCI6IjlmOGYxZjJlLTZiMWEtNGEzYy04ZDBlLTFiMmMzZDRlNWY2MCJ9"
    );
    let null = CursorKey {
        sort: SortKey::Name,
        order: SortOrder::Asc,
        value: None,
        id: ID.to_owned(),
    };
    assert_eq!(
        null.encode(),
        "eyJzb3J0IjoibmFtZSIsIm9yZGVyIjoiYXNjIiwidmFsdWUiOm51bGwsImlkIjoiOWY4ZjFmMmUtNmIxYS00YTNjLThkMGUtMWIyYzNkNGU1ZjYwIn0"
    );
}

#[test]
fn a_numeric_cursor_is_encoded_as_the_base64url_of_its_json() {
    let integer = CursorKey {
        sort: SortKey::Rating,
        order: SortOrder::Desc,
        value: Some(CursorValue::Integer(4)),
        id: ID.to_owned(),
    };
    assert_eq!(
        integer.encode(),
        "eyJzb3J0IjoicmF0aW5nIiwib3JkZXIiOiJkZXNjIiwidmFsdWUiOjQsImlkIjoiOWY4ZjFmMmUtNmIxYS00YTNjLThkMGUtMWIyYzNkNGU1ZjYwIn0"
    );
    let real = CursorKey {
        sort: SortKey::DoseGrams,
        order: SortOrder::Asc,
        value: Some(CursorValue::Real(15.5)),
        id: ID.to_owned(),
    };
    assert_eq!(
        real.encode(),
        "eyJzb3J0IjoiZG9zZV9ncmFtcyIsIm9yZGVyIjoiYXNjIiwidmFsdWUiOjE1LjUsImlkIjoiOWY4ZjFmMmUtNmIxYS00YTNjLThkMGUtMWIyYzNkNGU1ZjYwIn0"
    );
}

#[test]
fn every_kind_of_cursor_round_trips() {
    for cursor in [
        CursorKey {
            sort: SortKey::CreatedAt,
            order: SortOrder::Desc,
            value: Some(CursorValue::Text(AT.to_owned())),
            id: ID.to_owned(),
        },
        CursorKey {
            sort: SortKey::PurchasedOn,
            order: SortOrder::Asc,
            value: Some(CursorValue::Text("2026-09-21".to_owned())),
            id: ID.to_owned(),
        },
        CursorKey {
            sort: SortKey::Name,
            order: SortOrder::Asc,
            value: Some(CursorValue::Text("豆".to_owned())),
            id: ID.to_owned(),
        },
        CursorKey {
            sort: SortKey::Rating,
            order: SortOrder::Desc,
            value: Some(CursorValue::Integer(4)),
            id: ID.to_owned(),
        },
        CursorKey {
            sort: SortKey::DoseGrams,
            order: SortOrder::Asc,
            value: Some(CursorValue::Real(15.5)),
            id: ID.to_owned(),
        },
        CursorKey {
            sort: SortKey::WeightGrams,
            order: SortOrder::Desc,
            value: None,
            id: ID.to_owned(),
        },
    ] {
        assert_eq!(CursorKey::decode(&cursor.encode()), Ok(cursor));
    }
}

#[test]
fn a_cursor_that_is_not_base64url_is_rejected() {
    for text in [
        "", "a", "!!!!", "abc=",
        "AB", // 復号すると 1 バイトになるが、JSON にならない
    ] {
        assert_eq!(
            CursorKey::decode(text),
            Err(CursorError::Invalid),
            "{text} must be rejected"
        );
    }
}

#[test]
fn every_page_size_and_cursor_error_maps_to_400() {
    assert_eq!(PageSizeError::NotAnInteger.code().status(), 400);
    assert_eq!(PageSizeError::NotPositive.code().status(), 400);
    assert_eq!(PageSizeError::TooLarge.code().status(), 400);
    assert_eq!(CursorError::Invalid.code().status(), 400);
    assert_eq!(SortKeyError::Unknown.code().status(), 400);
    assert_eq!(SortOrderError::Unknown.code().status(), 400);
    assert_eq!(
        brew_book_core::query::QueryError::CursorMismatch
            .code()
            .status(),
        400
    );
    assert!(!CursorError::Invalid.message().is_empty());
    assert!(!SortKeyError::Unknown.message().is_empty());
    assert!(!SortOrderError::Unknown.message().is_empty());
}

#[test]
fn a_cursor_whose_payload_is_not_the_expected_json_is_rejected() {
    // base64url の "bm90IGpzb24" は "not json"。
    assert_eq!(CursorKey::decode("bm90IGpzb24"), Err(CursorError::Invalid));
    for payload in [
        r#"{"sort":"created_at","order":"desc","value":"2026-09-21T12:34:56.789Z"}"#,
        r#"{"order":"desc","value":"2026-09-21T12:34:56.789Z","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        r#"{"sort":"created_at","order":"desc","value":"2026-09-21T12:34:56.789Z","id":""}"#,
        r#"{"sort":"created_at","order":"desc","value":"2026-09-21T12:34:56.789Z","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60","extra":1}"#,
        // 知らない並び順のキー。
        r#"{"sort":"unknown","order":"desc","value":"2026-09-21T12:34:56.789Z","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        // 知らない方向。
        r#"{"sort":"created_at","order":"up","value":"2026-09-21T12:34:56.789Z","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        // 日時のキーに日時でない文字列。
        r#"{"sort":"created_at","order":"desc","value":"2026-09-21T12:34:56Z","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        r#"{"sort":"created_at","order":"desc","value":"garbage","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        // 日付のキーに実在しない日付。
        r#"{"sort":"purchased_on","order":"desc","value":"2026-02-30","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        // 数値のキーに文字列。
        r#"{"sort":"rating","order":"desc","value":"4","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        // 整数のキーに小数。
        r#"{"sort":"rating","order":"desc","value":4.5,"id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        // 文字列のキーに数値。
        r#"{"sort":"name","order":"desc","value":4,"id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        // 値の型が JSON のオブジェクト。
        r#"{"sort":"name","order":"desc","value":{},"id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        // id が数値。
        r#"{"sort":"created_at","order":"desc","value":"2026-09-21T12:34:56.789Z","id":1}"#,
    ] {
        let encoded = base64url(payload);
        assert_eq!(
            CursorKey::decode(&encoded),
            Err(CursorError::Invalid),
            "{payload} must be rejected"
        );
    }
}

/// テストの入力を作るための base64url の符号化。実装とは独立に書く。
fn base64url(text: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::new();
    for chunk in text.as_bytes().chunks(3) {
        let first = u32::from(chunk[0]);
        let second = u32::from(chunk.get(1).copied().unwrap_or(0));
        let third = u32::from(chunk.get(2).copied().unwrap_or(0));
        let triple = (first << 16) | (second << 8) | third;
        output.push(ALPHABET[(triple >> 18) as usize & 0x3f] as char);
        output.push(ALPHABET[(triple >> 12) as usize & 0x3f] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[(triple >> 6) as usize & 0x3f] as char);
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[triple as usize & 0x3f] as char);
        }
    }
    output
}
