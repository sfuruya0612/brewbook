//! CBOR のデコーダの PBT。
//!
//! テスト用の最小のエンコーダ (`support`) で生成した構造を符号化し、デコードの往復と、
//! 値の型ごとの不変条件 (長さの上限、ネストの上限、余分なバイト、indefinite length) を検証する。

mod support;

use coffee_log_core::cbor::{decode, decode_prefix, Error, Value, MAX_DEPTH, MAX_INPUT_LEN};
use proptest::prelude::*;
use support::{encode_array, encode_bytes, encode_integer, encode_map, encode_text};

/// PBT が生成する CBOR の構造。
#[derive(Debug, Clone)]
enum Tree {
    Integer(i64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Tree>),
    Map(Vec<(Tree, Tree)>),
}

/// CBOR の構造を生成する。
fn tree() -> impl Strategy<Value = Tree> {
    let leaf = prop_oneof![
        any::<i64>().prop_map(Tree::Integer),
        prop::collection::vec(any::<u8>(), 0..24).prop_map(Tree::Bytes),
        ".{0,16}".prop_map(Tree::Text),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Tree::Array),
            prop::collection::vec((inner.clone(), inner), 0..4).prop_map(Tree::Map),
        ]
    })
}

/// 生成した構造を CBOR に符号化する。
fn encode_of(tree: &Tree) -> Vec<u8> {
    match tree {
        Tree::Integer(value) => encode_integer(*value),
        Tree::Bytes(value) => encode_bytes(value),
        Tree::Text(value) => encode_text(value),
        Tree::Array(items) => {
            let items: Vec<Vec<u8>> = items.iter().map(encode_of).collect();
            encode_array(&items)
        }
        Tree::Map(entries) => {
            let entries: Vec<(Vec<u8>, Vec<u8>)> = entries
                .iter()
                .map(|(key, value)| (encode_of(key), encode_of(value)))
                .collect();
            encode_map(&entries)
        }
    }
}

/// 生成した構造と同じ形の `Value` を作る。
fn value_of(tree: &Tree) -> Value<'_> {
    match tree {
        Tree::Integer(value) => Value::Integer(*value),
        Tree::Bytes(value) => Value::ByteString(value),
        Tree::Text(value) => Value::TextString(value),
        Tree::Array(items) => Value::Array(items.iter().map(value_of).collect()),
        Tree::Map(entries) => Value::Map(
            entries
                .iter()
                .map(|(key, value)| (value_of(key), value_of(value)))
                .collect(),
        ),
    }
}

/// ネストした array を `depth` 段作る。中身は integer の 0。
fn nested(depth: usize) -> Vec<u8> {
    let mut encoded = encode_integer(0);
    for _ in 0..depth {
        encoded = encode_array(&[encoded]);
    }
    encoded
}

proptest! {
    /// 生成した構造は、符号化してデコードすると同じ形に戻る。
    #[test]
    fn every_generated_structure_round_trips(tree in tree()) {
        let encoded = encode_of(&tree);
        prop_assert!(encoded.len() <= MAX_INPUT_LEN);
        prop_assert_eq!(decode(&encoded), Ok(value_of(&tree)));
        prop_assert_eq!(decode_prefix(&encoded), Ok((value_of(&tree), encoded.len())));
    }

    /// 長さの上限を超える入力は、中身によらず拒否する。
    #[test]
    fn an_input_over_the_limit_is_rejected(
        prefix in prop::collection::vec(any::<u8>(), 0..8),
        extra in 1..=8usize,
    ) {
        let mut bytes = vec![0u8; MAX_INPUT_LEN + extra];
        bytes[..prefix.len()].copy_from_slice(&prefix);
        prop_assert_eq!(decode(&bytes), Err(Error::TooLarge));
        prop_assert_eq!(decode_prefix(&bytes), Err(Error::TooLarge));
    }

    /// 最初のアイテムの後に余分なバイトがある入力は拒否する。
    #[test]
    fn trailing_bytes_are_rejected(tree in tree()) {
        let mut encoded = encode_of(&tree);
        encoded.push(0x00);
        prop_assert_eq!(decode(&encoded), Err(Error::TrailingBytes));
    }

    /// ネストの上限を超える入力は拒否し、上限までなら受け入れる。
    #[test]
    fn nesting_is_bounded(depth in 1..MAX_DEPTH * 2) {
        let encoded = nested(depth);
        if depth < MAX_DEPTH {
            prop_assert!(decode(&encoded).is_ok(), "depth {} must be accepted", depth);
        } else {
            prop_assert_eq!(decode(&encoded), Err(Error::TooDeep), "depth {}", depth);
        }
    }

    /// indefinite length は、後ろに何が続いても拒否する (CTAP2 canonical CBOR は禁止する)。
    #[test]
    fn indefinite_length_is_rejected(
        first in prop_oneof![
            Just(0x5fu8),
            Just(0x7f),
            Just(0x9f),
            Just(0xbf),
            Just(0xff)
        ],
        rest in prop::collection::vec(any::<u8>(), 0..16),
    ) {
        let mut encoded = vec![first];
        encoded.extend_from_slice(&rest);
        prop_assert_eq!(decode(&encoded), Err(Error::IndefiniteLength));
    }

    /// 任意のバイト列 (64 KiB 以下) を与えても panic しない (CI のクラッシュ耐性の検査)。
    #[test]
    fn arbitrary_input_does_not_panic(bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        let _ = decode(&bytes);
        let _ = decode_prefix(&bytes);
    }
}
