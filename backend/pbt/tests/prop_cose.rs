//! COSE の鍵のパーサの PBT。
//!
//! EC2 の P-256 の鍵の生成と、値の型ごとの不変条件を検証する。
//! テスト用の最小のエンコーダ (`support`) で鍵を組み立てる。

mod support;

use coffee_log_core::cbor::{self, MAX_INPUT_LEN};
use coffee_log_core::cose::{self, Ec2PublicKey, Error};
use proptest::prelude::*;
use support::{encode_array, encode_bytes, encode_integer, encode_map, encode_text};

/// COSE の鍵を組み立てる。`extra` は追加のパラメータ (kid など)。
fn key(kty: i64, alg: i64, crv: i64, x: &[u8], y: &[u8], extra: &[(i64, Vec<u8>)]) -> Vec<u8> {
    // RFC 9052 の 7.1: kty は 1、alg は 3、crv は -1、x は -2、y は -3。
    let mut entries = vec![
        (encode_integer(1), encode_integer(kty)),
        (encode_integer(3), encode_integer(alg)),
        (encode_integer(-1), encode_integer(crv)),
        (encode_integer(-2), encode_bytes(x)),
        (encode_integer(-3), encode_bytes(y)),
    ];
    for (label, value) in extra {
        entries.push((encode_integer(*label), value.clone()));
    }
    encode_map(&entries)
}

/// 32 バイトの座標を生成する。
fn coordinate() -> impl Strategy<Value = [u8; 32]> {
    any::<[u8; 32]>()
}

/// ES256 (alg -7、crv P-256) 以外の値を生成する。
fn not_es256() -> impl Strategy<Value = i64> {
    any::<i64>().prop_filter("not ES256", |value| *value != -7)
}

/// P-256 (crv 1) 以外の値を生成する。
fn not_p256() -> impl Strategy<Value = i64> {
    any::<i64>().prop_filter("not P-256", |value| *value != 1)
}

/// EC2 (kty 2) 以外の値を生成する。
fn not_ec2() -> impl Strategy<Value = i64> {
    any::<i64>().prop_filter("not EC2", |value| *value != 2)
}

/// 32 バイト以外の長さの座標を生成する。
fn wrong_length_coordinate() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..=64)
        .prop_filter("not 32 bytes", |value| value.len() != 32)
}

proptest! {
    /// 長さの上限を超える入力は、COSE の経路でも拒否する (CBOR のデコーダに委譲する)。
    #[test]
    fn an_input_over_the_limit_is_rejected(
        prefix in prop::collection::vec(any::<u8>(), 0..8),
        extra in 1..=8usize,
    ) {
        let mut bytes = vec![0u8; MAX_INPUT_LEN + extra];
        bytes[..prefix.len()].copy_from_slice(&prefix);
        prop_assert_eq!(cose::parse(&bytes), Err(Error::Cbor(cbor::Error::TooLarge)));
    }

    /// 生成した ES256 の鍵は、そのままの座標で読み出せる。
    #[test]
    fn a_generated_es256_key_is_parsed(x in coordinate(), y in coordinate()) {
        let encoded = key(2, -7, 1, &x, &y, &[]);
        prop_assert_eq!(cose::parse(&encoded), Ok(Ec2PublicKey { x, y }));
    }

    /// kty が EC2 でない鍵は拒否する。
    #[test]
    fn a_key_type_that_is_not_ec2_is_rejected(kty in not_ec2(), x in coordinate(), y in coordinate()) {
        prop_assert_eq!(
            cose::parse(&key(kty, -7, 1, &x, &y, &[])),
            Err(Error::UnsupportedKeyType)
        );
    }

    /// alg が ES256 でない鍵は拒否する。
    #[test]
    fn an_algorithm_that_is_not_es256_is_rejected(alg in not_es256(), x in coordinate(), y in coordinate()) {
        prop_assert_eq!(
            cose::parse(&key(2, alg, 1, &x, &y, &[])),
            Err(Error::UnsupportedAlgorithm)
        );
    }

    /// crv が P-256 でない鍵は拒否する。
    #[test]
    fn a_curve_that_is_not_p256_is_rejected(crv in not_p256(), x in coordinate(), y in coordinate()) {
        prop_assert_eq!(
            cose::parse(&key(2, -7, crv, &x, &y, &[])),
            Err(Error::UnsupportedCurve)
        );
    }

    /// x か y が 32 バイトでない鍵は拒否する。
    #[test]
    fn a_coordinate_of_the_wrong_length_is_rejected(
        short in wrong_length_coordinate(),
        correct in coordinate(),
    ) {
        prop_assert_eq!(
            cose::parse(&key(2, -7, 1, &short, &correct, &[])),
            Err(Error::InvalidCoordinate)
        );
        prop_assert_eq!(
            cose::parse(&key(2, -7, 1, &correct, &short, &[])),
            Err(Error::InvalidCoordinate)
        );
    }

    /// 追加のパラメータがあっても、座標の読み出しは変わらない。
    #[test]
    fn extra_parameters_do_not_change_the_coordinates(
        x in coordinate(),
        y in coordinate(),
        kid in "[a-z0-9]{0,8}",
    ) {
        let extra = vec![
            (2, encode_text(&kid)),
            (4, encode_array(&[])),
            (99, encode_integer(0)),
        ];
        prop_assert_eq!(cose::parse(&key(2, -7, 1, &x, &y, &extra)), Ok(Ec2PublicKey { x, y }));
    }

    /// 任意のバイト列でも panic せず、ES256 の鍵として読めた値は canonical な形でも同じ結果になる。
    #[test]
    fn parsed_keys_are_stable(bytes in prop::collection::vec(any::<u8>(), 0..64)) {
        if let Ok(parsed) = cose::parse(&bytes) {
            let canonical = key(2, -7, 1, &parsed.x, &parsed.y, &[]);
            prop_assert_eq!(cose::parse(&canonical), Ok(parsed));
        }
    }
}
