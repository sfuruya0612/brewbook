//! base64url の復号と符号化の PBT。
//!
//! WebAuthn の base64url は、URL とファイル名に安全なアルファベットを使い、末尾の `=` を省く
//! (W3C WebAuthn Level 3 の 3)。

use brew_book_core::base64url::{decode, encode, Error};
use proptest::prelude::*;

proptest! {
    /// 任意のバイト列は、符号化して復号すると元に戻る。
    #[test]
    fn every_byte_string_round_trips(input in prop::collection::vec(any::<u8>(), 0..64)) {
        prop_assert_eq!(decode(&encode(&input)), Ok(input.clone()));
    }

    /// 符号化の結果は、base64url のアルファベットとパディング無しだけになる。
    #[test]
    fn the_encoding_stays_inside_the_alphabet(input in prop::collection::vec(any::<u8>(), 0..64)) {
        let encoded = encode(&input);
        prop_assert!(
            encoded
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
            "the encoding has a character outside the alphabet: {}",
            encoded
        );
        prop_assert!(!encoded.contains('='));
    }

    /// アルファベットに無い文字を含む入力は、長さによらず拒否する。
    #[test]
    fn a_character_outside_the_alphabet_is_rejected(
        input in "[A-Za-z0-9_-]{0,16}",
        bad in prop::sample::select(vec!['=', '+', '/', '\n', ' ', '!', '.']),
    ) {
        let text = format!("{input}{bad}");
        prop_assert!(decode(&text).is_err(), "{:?}", text);
    }

    /// 4 文字の組にならない長さの入力は拒否する。
    #[test]
    fn a_length_with_a_remainder_of_one_is_rejected(input in "[A-Za-z0-9_-]{0,15}") {
        let text = format!("{input}A");
        prop_assume!(text.len() % 4 == 1);
        prop_assert_eq!(decode(&text), Err(Error::InvalidLength));
    }
}
