//! `cursor` の単体テスト。符号化の往復は PBT (`prop_cursor.rs`) が担う。

use coffee_log_core::cursor::{
    parse_page_size, CursorError, CursorKey, PageSizeError, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE,
};

const ID: &str = "9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60";

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
fn a_datetime_cursor_is_encoded_as_the_base64url_of_its_json() {
    let cursor = CursorKey::DateTime {
        at: "2026-09-21T12:34:56.789Z".to_owned(),
        id: ID.to_owned(),
    };
    assert_eq!(
        cursor.encode(),
        "eyJhdCI6IjIwMjYtMDktMjFUMTI6MzQ6NTYuNzg5WiIsImlkIjoiOWY4ZjFmMmUtNmIxYS00YTNjLThkMGUtMWIyYzNkNGU1ZjYwIn0"
    );
    assert!(
        !cursor.encode().contains('='),
        "the cursor must be unpadded"
    );
}

#[test]
fn a_date_cursor_is_encoded_as_the_base64url_of_its_json() {
    let cursor = CursorKey::Date {
        on: "2026-09-21".to_owned(),
        id: ID.to_owned(),
    };
    assert_eq!(
        cursor.encode(),
        "eyJvbiI6IjIwMjYtMDktMjEiLCJpZCI6IjlmOGYxZjJlLTZiMWEtNGEzYy04ZDBlLTFiMmMzZDRlNWY2MCJ9"
    );
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
    assert_eq!(
        coffee_log_core::query::QueryError::CursorKindMismatch
            .code()
            .status(),
        400
    );
    assert!(!CursorError::Invalid.message().is_empty());
}

#[test]
fn a_cursor_whose_payload_is_not_the_expected_json_is_rejected() {
    // base64url の "bm90IGpzb24" は "not json"。
    assert_eq!(CursorKey::decode("bm90IGpzb24"), Err(CursorError::Invalid));
    for payload in [
        r#"{"at":"2026-09-21T12:34:56.789Z"}"#,
        r#"{"id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        r#"{"at":"2026-09-21T12:34:56.789Z","on":"2026-09-21","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        r#"{"at":"2026-09-21T12:34:56.789Z","id":""}"#,
        r#"{"on":"2026-09-21","id":""}"#,
        r#"{"at":"2026-09-21T12:34:56.789Z","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60","extra":1}"#,
        r#"{"at":"2026-09-21T12:34:56Z","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        r#"{"at":"garbage","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        r#"{"on":"2026-02-30","id":"9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60"}"#,
        r#"{"on":"2026-09-21","id":1}"#,
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
