//! `auth::base64url` の PBT。任意のバイト列と文字列に対する性質 (ADR-0013、0040)。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::auth::base64url::{decode, encode};
use proptest::prelude::*;

// 任意のバイト列を符号化して復号すると元に戻る。
proptest! {
    #[test]
    fn encode_then_decode_is_the_identity(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        prop_assert_eq!(decode(&encode(&bytes)), Ok(bytes));
    }

    /// 符号化の出力はアルファベットの文字だけで、パディングと空白を含まない
    /// (W3C WebAuthn Level 3 の 3)。
    #[test]
    fn the_encoded_text_uses_the_alphabet_only(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        let encoded = encode(&bytes);
        prop_assert!(encoded
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'));
        prop_assert!(!encoded.contains('='));
    }

    /// 復号は任意の文字列で panic せず、成功するのはアルファベットの文字だけで 4 文字の組に
    /// なるとき (端数が 1 文字でないとき) だけである。
    #[test]
    fn decode_succeeds_only_for_the_alphabet_and_a_valid_length(text in ".{0,64}") {
        let alphabet_only = text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
        let valid_length = text.len() % 4 != 1;
        prop_assert_eq!(decode(&text).is_ok(), alphabet_only && valid_length);
    }

    /// 復号に成功した文字列は、正規の符号化を経ても同じバイト列に戻る。
    #[test]
    fn decoding_the_canonical_encoding_returns_the_same_bytes(text in "[A-Za-z0-9_\\-]{0,64}") {
        prop_assume!(text.len() % 4 != 1);
        let bytes = decode(&text).expect("the alphabet-only text must decode");
        prop_assert_eq!(decode(&encode(&bytes)), Ok(bytes));
    }
}
