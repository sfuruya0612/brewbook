//! base64url の復号と符号化の単体テスト。

use brew_book_core::base64url::{decode, encode, Error};

/// RFC 4648 の Section 10 のテストベクタ (URL とファイル名に安全なアルファベット、パディングは省く)。
const RFC_4648_VECTORS: &[(&[u8], &str)] = &[
    (b"", ""),
    (b"f", "Zg"),
    (b"fo", "Zm8"),
    (b"foo", "Zm9v"),
    (b"foob", "Zm9vYg"),
    (b"fooba", "Zm9vYmE"),
    (b"foobar", "Zm9vYmFy"),
];

#[test]
fn the_rfc_4648_vectors_round_trip() {
    for (input, encoded) in RFC_4648_VECTORS {
        assert_eq!(encode(input), *encoded, "encoding {input:?}");
        assert_eq!(decode(encoded).unwrap(), *input, "decoding {encoded}");
    }
}

#[test]
fn the_rfc_4648_section_5_alphabet_is_used() {
    // RFC 4648 の Section 5 の例。0xfb 0xff は 62 番と 63 番の文字になる。
    assert_eq!(encode(&[0xfb, 0xff]), "-_8");
    assert_eq!(decode("-_8").unwrap(), vec![0xfb, 0xff]);
}

#[test]
fn the_empty_input_round_trips() {
    assert_eq!(encode(&[]), "");
    assert_eq!(decode("").unwrap(), Vec::<u8>::new());
}

#[test]
fn every_byte_value_round_trips() {
    let input: Vec<u8> = (0..=255).collect();
    assert_eq!(decode(&encode(&input)).unwrap(), input);
}

#[test]
fn padding_is_rejected() {
    // WebAuthn の base64url は末尾の `=` を省く (W3C WebAuthn Level 3 の 3)。
    assert_eq!(decode("Zg="), Err(Error::InvalidCharacter));
    assert_eq!(decode("Zg=="), Err(Error::InvalidCharacter));
    assert_eq!(decode("Zm8="), Err(Error::InvalidCharacter));
    assert_eq!(decode("Zg==="), Err(Error::InvalidLength));
}

#[test]
fn characters_outside_the_alphabet_are_rejected() {
    assert_eq!(decode("Zg\n"), Err(Error::InvalidCharacter));
    assert_eq!(decode("Zm9\n"), Err(Error::InvalidCharacter));
    assert_eq!(decode("Zm 9"), Err(Error::InvalidCharacter));
    assert_eq!(decode("Zm+9"), Err(Error::InvalidCharacter));
    assert_eq!(decode("Zm/9"), Err(Error::InvalidCharacter));
    assert_eq!(decode("Zm9!"), Err(Error::InvalidCharacter));
}

#[test]
fn a_length_with_a_remainder_of_one_is_rejected() {
    assert_eq!(decode("Z"), Err(Error::InvalidLength));
    assert_eq!(decode("Zm9vZ"), Err(Error::InvalidLength));
}

#[test]
fn non_canonical_encodings_decode_to_the_same_bytes() {
    // この復号器は RFC 4648 の任意の復号器として、未使用ビットがゼロでない符号化も受理する。
    // 「Zg」と「Zh」はどちらも 1 バイトの 0x66 になる。challenge の照合は文字列で行うため、
    // この性質が認証の判定に影響しない (W3C WebAuthn Level 3 の 7.1 と 7.2)。
    assert_eq!(decode("Zg"), decode("Zh"));
    assert_eq!(decode("Zg").unwrap(), vec![0x66]);
}

#[test]
fn the_error_messages_are_english() {
    for error in [Error::InvalidCharacter, Error::InvalidLength] {
        let message = error.to_string();
        assert!(
            message.is_ascii() && !message.is_empty(),
            "the message of {error:?} is not an English sentence: {message}"
        );
    }
}
