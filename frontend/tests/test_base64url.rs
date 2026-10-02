//! `auth::base64url` の単体テスト。
//!
//! パディングの有無の検査は Flutter の `test/web/passkey_interop_test.dart` を写す (0040)。
//! 任意の入力に対する性質は PBT (`pbt/tests/prop_base64url.rs`) が担う。

use brew_book_frontend::auth::base64url::{decode, encode, Error};

#[test]
fn decode_accepts_unpadded_text() {
    // 長さが 4 の倍数でない値も復号できる (W3C WebAuthn Level 3 の 3 はパディングを付けない)。
    assert_eq!(decode("AQID"), Ok(vec![1, 2, 3]));
    assert_eq!(decode("AQI"), Ok(vec![1, 2]));
    assert_eq!(decode("AQ"), Ok(vec![1]));
    assert_eq!(decode(""), Ok(Vec::new()));
}

#[test]
fn encode_does_not_add_padding() {
    assert_eq!(encode(&[1, 2, 3]), "AQID");
    assert_eq!(encode(&[1, 2]), "AQI");
    assert_eq!(encode(&[1]), "AQ");
    assert_eq!(encode(&[]), "");
}

#[test]
fn the_alphabet_uses_the_url_safe_characters() {
    // 62 は `-`、63 は `_` になる (RFC 4648 の Section 5 の URL とファイル名に安全な文字集合)。
    assert_eq!(encode(&[0xfb, 0xff, 0xff]), "-___");
    assert_eq!(decode("-___"), Ok(vec![0xfb, 0xff, 0xff]));
}

#[test]
fn decode_rejects_padding_and_whitespace() {
    assert_eq!(decode("AQ=="), Err(Error::InvalidCharacter));
    assert_eq!(decode("AQ I"), Err(Error::InvalidCharacter));
    assert_eq!(decode("AQ\n"), Err(Error::InvalidCharacter));
    assert_eq!(decode("AQ+/"), Err(Error::InvalidCharacter));
}

#[test]
fn decode_rejects_a_length_that_is_not_base64url() {
    // 端数が 1 文字の長さは 4 文字の組にならない。
    assert_eq!(decode("A"), Err(Error::InvalidLength));
    assert_eq!(decode("AQIDA"), Err(Error::InvalidLength));
}

#[test]
fn the_error_has_a_message() {
    assert!(!Error::InvalidCharacter.to_string().is_empty());
    assert!(!Error::InvalidLength.to_string().is_empty());
}
