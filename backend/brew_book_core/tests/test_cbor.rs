//! CBOR のデコーダの単体テスト。
//!
//! 境界値と、PBT で実現できない意図的なエラーパスを確かめる (所有者の Rust のテスト規約)。

use coffee_log_core::cbor::{decode, decode_prefix, Error, Value, MAX_INPUT_LEN};

#[test]
fn a_map_with_text_keys_is_decoded_in_order() {
    // { "fmt": "none", "attStmt": {}, "authData": h'0102' }
    let input = [
        0xa3, 0x63, b'f', b'm', b't', 0x64, b'n', b'o', b'n', b'e', 0x67, b'a', b't', b't', b'S',
        b't', b'm', b't', 0xa0, 0x68, b'a', b'u', b't', b'h', b'D', b'a', b't', b'a', 0x42, 0x01,
        0x02,
    ];
    let value = decode(&input).unwrap();
    assert_eq!(value.map_get_text("fmt").unwrap().as_text(), Some("none"));
    assert_eq!(
        value.map_get_text("attStmt").unwrap().as_map(),
        Some(&[][..])
    );
    assert_eq!(
        value.map_get_text("authData").unwrap().as_bytes(),
        Some(&[0x01, 0x02][..])
    );
    assert_eq!(value.map_get_text("missing"), None);
    assert_eq!(value.map_get_text("fmt").unwrap().as_bytes(), None);
}

#[test]
fn integers_of_each_width_are_decoded() {
    let cases: &[(&[u8], i64)] = &[
        (&[0x00], 0),
        (&[0x17], 23),
        (&[0x18, 0x18], 24),
        (&[0x18, 0xff], 255),
        (&[0x19, 0x01, 0x00], 256),
        (&[0x1a, 0x00, 0x01, 0x00, 0x00], 65536),
        (
            &[0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00],
            4294967296,
        ),
        (
            &[0x1b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            i64::MAX,
        ),
        (&[0x20], -1),
        (&[0x37], -24),
        (&[0x38, 0x63], -100),
        (
            &[0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            i64::MIN,
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(
            decode(input).unwrap(),
            Value::Integer(*expected),
            "{input:?}"
        );
    }
}

#[test]
fn integers_that_do_not_fit_in_i64_are_rejected() {
    assert_eq!(
        decode(&[0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
        Err(Error::IntegerOutOfRange)
    );
    assert_eq!(
        decode(&[0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
        Err(Error::IntegerOutOfRange)
    );
}

#[test]
fn byte_strings_and_text_strings_are_decoded() {
    assert_eq!(decode(&[0x40]).unwrap(), Value::ByteString(&[]));
    assert_eq!(decode(&[0x60]).unwrap(), Value::TextString(""));
    assert_eq!(
        decode(&[0x43, 0x01, 0x02, 0x03]).unwrap(),
        Value::ByteString(&[1, 2, 3])
    );
    // 「豆」は UTF-8 で 3 バイト。
    assert_eq!(
        decode(&[0x63, 0xe8, 0xb1, 0x86]).unwrap(),
        Value::TextString("豆")
    );
}

#[test]
fn a_text_string_that_is_not_utf8_is_rejected() {
    assert_eq!(decode(&[0x62, 0xff, 0xff]), Err(Error::InvalidUtf8));
}

#[test]
fn arrays_and_nested_maps_are_decoded() {
    // [ 1, [ 2 ], { 1: h'00' } ]
    let input = [0x83, 0x01, 0x81, 0x02, 0xa1, 0x01, 0x41, 0x00];
    let value = decode(&input).unwrap();
    let array = value.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0], Value::Integer(1));
    assert_eq!(array[1], Value::Array(vec![Value::Integer(2)]));
    assert_eq!(
        array[2].map_get_integer(1).unwrap().as_bytes(),
        Some(&[0x00][..])
    );
}

#[test]
fn an_empty_array_and_an_empty_map_are_decoded() {
    assert_eq!(decode(&[0x80]).unwrap(), Value::Array(Vec::new()));
    assert_eq!(decode(&[0xa0]).unwrap(), Value::Map(Vec::new()));
}

#[test]
fn a_map_keeps_duplicate_keys_and_the_first_one_is_found() {
    // { 1: 2, 1: 3 }
    let value = decode(&[0xa2, 0x01, 0x02, 0x01, 0x03]).unwrap();
    assert_eq!(value.as_map().unwrap().len(), 2);
    assert_eq!(value.map_get_integer(1).unwrap().as_integer(), Some(2));
}

#[test]
fn a_prefix_can_be_decoded_with_its_length() {
    // 先頭のアイテムだけをデコードし、残りは呼び出し側が扱う (authData の拡張など)。
    let (value, consumed) = decode_prefix(&[0x01, 0x02, 0x03]).unwrap();
    assert_eq!(value, Value::Integer(1));
    assert_eq!(consumed, 1);
    assert_eq!(decode_prefix(&[0x42, 0xaa, 0xbb, 0xcc]).unwrap().1, 3);
    assert_eq!(decode_prefix(&[]), Err(Error::UnexpectedEnd));
}

#[test]
fn types_that_web_authn_does_not_use_are_rejected() {
    assert_eq!(decode(&[0xc0, 0x00]), Err(Error::Unsupported)); // tag 0
    assert_eq!(decode(&[0xf4]), Err(Error::Unsupported)); // false
    assert_eq!(
        decode(&[0xfb, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        Err(Error::Unsupported)
    ); // 0.0
    assert_eq!(decode(&[0x1c]), Err(Error::Unsupported)); // 予約の additional information
    assert_eq!(decode(&[0x1f]), Err(Error::IndefiniteLength));
}

#[test]
fn an_input_that_ends_in_the_middle_is_rejected() {
    assert_eq!(decode(&[0x19, 0x01]), Err(Error::UnexpectedEnd));
    assert_eq!(decode(&[0x43, 0x01, 0x02]), Err(Error::UnexpectedEnd));
    assert_eq!(decode(&[0x82, 0x01]), Err(Error::UnexpectedEnd));
    assert_eq!(decode(&[0xa1, 0x01]), Err(Error::UnexpectedEnd));
    assert_eq!(decode(&[]), Err(Error::UnexpectedEnd));
}

#[test]
fn the_length_limit_is_the_boundary_of_the_rejection() {
    // 上限ちょうどの入力は、中身が正しければ受理する (上限は入力全体の長さに掛かる)。
    // 0x59 は 2 バイトの長さを持つ byte string で、65533 バイトの本体と合わせて 65536 バイト。
    let mut input = vec![0x59, 0xff, 0xfd];
    input.extend(vec![0x00; MAX_INPUT_LEN - 3]);
    assert!(decode(&input).is_ok());
    // 1 バイト増やすと拒否する。
    input.push(0x00);
    assert_eq!(decode(&input), Err(Error::TooLarge));
}

#[test]
fn the_error_messages_are_english() {
    for error in [
        Error::TooLarge,
        Error::UnexpectedEnd,
        Error::TrailingBytes,
        Error::IndefiniteLength,
        Error::InvalidUtf8,
        Error::IntegerOutOfRange,
        Error::Unsupported,
        Error::TooDeep,
    ] {
        let message = error.to_string();
        assert!(
            message.is_ascii() && !message.is_empty(),
            "the message of {error:?} is not an English sentence: {message}"
        );
    }
}
