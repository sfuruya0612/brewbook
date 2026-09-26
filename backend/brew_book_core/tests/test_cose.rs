//! COSE の鍵から EC2 の P-256 公開鍵を取り出す処理の単体テスト。
//!
//! 16 進数の値は W3C WebAuthn Level 3 の 16 節 (Test Vectors) の COSE の鍵を使う
//! <https://www.w3.org/TR/webauthn-3/#sctn-test-vectors>。
//! 拒否の経路は、公開された鍵のバイト列を書き換えて作る。
//! PBT で実現できない意図的なエラーパスを確かめる。

use brew_book_core::cbor;
use brew_book_core::cose::{self, Ec2PublicKey, Error};

/// 16.2 の ES256 の COSE の鍵 (kty 2、alg -7、crv 1、x と y が 32 バイト)。
const ES256_KEY: &str = "a5010203262001215820afefa16f97ca9b2d23eb86ccb64098d20db90856062eb249c33a9b672f26df61225820930a56b87a2fca66334b03458abf879717c12cc68ed73290af2e2664796b9220";

/// 16.8 の ES384 の COSE の鍵 (kty 2、alg -35、crv 2、x と y が 48 バイト)。
const ES384_KEY: &str = "a5010203382220022158304866bd8b01da789e9eb806e5eab05ae5a638542296ab057a2f1bbce9b58f8a08b9171390b58a37ac7fffc2c5f45857da2258302a0b024c7f4b72072a1f96bd30a7261aae9571dd39870eb29e55c0941c6b08e89629a1ea1216aa64ce57c2807bf3901a";

/// 16.2 の鍵のバイト位置 (RFC 9052 の並びのまま)。
mod offsets {
    /// kty のラベル (1)。
    pub const KEY_TYPE_LABEL: usize = 1;
    /// kty の値 (2 = EC2)。
    pub const KEY_TYPE: usize = 2;
    /// alg のラベル (3)。
    pub const ALGORITHM_LABEL: usize = 3;
    /// alg の値 (-7 = ES256)。
    pub const ALGORITHM: usize = 4;
    /// crv のラベル (-1)。
    pub const CURVE_LABEL: usize = 5;
    /// crv の値 (1 = P-256)。
    pub const CURVE: usize = 6;
    /// x の byte string のヘッダ (58 20)。
    pub const X_TYPE: usize = 8;
    /// x の長さの前置き (1 バイト)。
    pub const X_LENGTH: usize = 9;
    /// x の値の先頭。
    pub const X: usize = 10;
    /// y のラベル (-3)。
    pub const Y_LABEL: usize = 42;
    /// y の長さの前置き (1 バイト)。
    pub const Y_LENGTH: usize = 44;
    /// y の値の先頭。
    pub const Y: usize = 45;
}

/// 16 進数の文字列をバイト列にする。
fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).unwrap())
        .collect()
}

/// 16.2 の COSE の鍵から期待する座標。
fn expected_public_key() -> Ec2PublicKey {
    Ec2PublicKey {
        x: hex("afefa16f97ca9b2d23eb86ccb64098d20db90856062eb249c33a9b672f26df61")
            .try_into()
            .unwrap(),
        y: hex("930a56b87a2fca66334b03458abf879717c12cc68ed73290af2e2664796b9220")
            .try_into()
            .unwrap(),
    }
}

/// 16.2 の鍵のバイト列を写して書き換える。
fn edited(edit: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut bytes = hex(ES256_KEY);
    edit(&mut bytes);
    bytes
}

#[test]
fn a_published_es256_key_is_parsed() {
    assert_eq!(cose::parse(&hex(ES256_KEY)).unwrap(), expected_public_key());
    assert_eq!(
        cose::parse(&hex(ES256_KEY))
            .unwrap()
            .to_sec1_bytes()
            .to_vec(),
        [
            vec![0x04],
            hex("afefa16f97ca9b2d23eb86ccb64098d20db90856062eb249c33a9b672f26df61"),
            hex("930a56b87a2fca66334b03458abf879717c12cc68ed73290af2e2664796b9220"),
        ]
        .concat()
    );
}

#[test]
fn an_es384_key_is_rejected() {
    assert_eq!(
        cose::parse(&hex(ES384_KEY)),
        Err(Error::UnsupportedAlgorithm)
    );
}

#[test]
fn a_key_that_is_not_a_map_is_rejected() {
    assert_eq!(
        cose::from_value(&cbor::Value::Integer(2)),
        Err(Error::NotAMap)
    );
    assert_eq!(
        cose::from_value(&cbor::Value::Array(Vec::new())),
        Err(Error::NotAMap)
    );
}

#[test]
fn a_key_type_that_is_not_ec2_is_rejected() {
    // kty を RSA (3) にする。
    let key = edited(|bytes| bytes[offsets::KEY_TYPE] = 3);
    assert_eq!(cose::parse(&key), Err(Error::UnsupportedKeyType));
    // kty のラベルを変えて、kty が無い状態にする。
    let key = edited(|bytes| bytes[offsets::KEY_TYPE_LABEL] = 2);
    assert_eq!(cose::parse(&key), Err(Error::UnsupportedKeyType));
}

#[test]
fn an_algorithm_that_is_not_es256_is_rejected() {
    // alg を EdDSA (-8) にする。
    let key = edited(|bytes| bytes[offsets::ALGORITHM] = 0x27);
    assert_eq!(cose::parse(&key), Err(Error::UnsupportedAlgorithm));
    // alg のラベルを変えて、alg が無い状態にする。
    let key = edited(|bytes| bytes[offsets::ALGORITHM_LABEL] = 4);
    assert_eq!(cose::parse(&key), Err(Error::UnsupportedAlgorithm));
}

#[test]
fn a_curve_that_is_not_p256_is_rejected() {
    // crv を P-384 (2) にする。
    let key = edited(|bytes| bytes[offsets::CURVE] = 2);
    assert_eq!(cose::parse(&key), Err(Error::UnsupportedCurve));
    // crv のラベルを変えて、crv が無い状態にする。
    let key = edited(|bytes| bytes[offsets::CURVE_LABEL] = 0x23);
    assert_eq!(cose::parse(&key), Err(Error::UnsupportedCurve));
}

#[test]
fn a_coordinate_that_is_not_32_bytes_is_rejected() {
    // x を 8 バイトにする。
    let key = edited(|bytes| {
        bytes[offsets::X_LENGTH] = 8;
        bytes.drain(offsets::X + 8..offsets::Y_LABEL);
    });
    assert_eq!(cose::parse(&key), Err(Error::InvalidCoordinate));
    // y を 8 バイトにする。
    let key = edited(|bytes| {
        bytes[offsets::Y_LENGTH] = 8;
        bytes.truncate(offsets::Y + 8);
    });
    assert_eq!(cose::parse(&key), Err(Error::InvalidCoordinate));
    // x の値を byte string ではなく text string にする (長さの前置きの 0x20 はそのまま使う)。
    let key = edited(|bytes| {
        bytes[offsets::X_TYPE] = 0x78;
        bytes[offsets::X..offsets::Y_LABEL].fill(b'a');
    });
    assert_eq!(cose::parse(&key), Err(Error::InvalidCoordinate));
    // y の組を落とし、map の組の数を 4 にする。
    let key = edited(|bytes| {
        bytes[0] = 0xa4;
        bytes.truncate(offsets::Y_LABEL);
    });
    assert_eq!(cose::parse(&key), Err(Error::InvalidCoordinate));
}

#[test]
fn extra_parameters_do_not_break_the_key() {
    // kid (2) と key_ops (4) を足し、map の組の数を 7 にする。
    let key = edited(|bytes| {
        bytes[0] = 0xa7;
        // 2: "kid"
        bytes.extend_from_slice(&[0x02, 0x63, b'k', b'i', b'd']);
        // 4: [2]
        bytes.extend_from_slice(&[0x04, 0x81, 0x02]);
    });
    assert_eq!(cose::parse(&key).unwrap(), expected_public_key());
}

#[test]
fn a_key_that_is_not_cbor_is_rejected() {
    assert_eq!(
        cose::parse(&[]),
        Err(Error::Cbor(cbor::Error::UnexpectedEnd))
    );
    assert_eq!(
        cose::parse(&[0x19]),
        Err(Error::Cbor(cbor::Error::UnexpectedEnd))
    );
    // 0x00 は integer なので、CBOR としては正しいが map ではない。
    assert_eq!(cose::parse(&[0x00]), Err(Error::NotAMap));
    assert_eq!(
        cose::parse(&[0x00, 0x00]),
        Err(Error::Cbor(cbor::Error::TrailingBytes))
    );
    assert_eq!(
        cose::parse(&[0x9f]),
        Err(Error::Cbor(cbor::Error::IndefiniteLength))
    );
}

#[test]
fn the_error_messages_are_english() {
    for error in [
        Error::Cbor(cbor::Error::UnexpectedEnd),
        Error::NotAMap,
        Error::UnsupportedKeyType,
        Error::UnsupportedAlgorithm,
        Error::UnsupportedCurve,
        Error::InvalidCoordinate,
    ] {
        let message = error.to_string();
        assert!(
            message.is_ascii() && !message.is_empty(),
            "the message of {error:?} is not an English sentence: {message}"
        );
    }
}
