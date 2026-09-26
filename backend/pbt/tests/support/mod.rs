//! PBT が使うテスト用の最小の CBOR エンコーダ。
//!
//! PBT は生成した CBOR の構造を符号化し、`brew_book_core` のデコーダで読み戻す。
//! 符号化は CTAP2 canonical CBOR の最短の形だけを使う (W3C WebAuthn Level 3 の 2.4)。

#![allow(dead_code)] // 補助は複数のテストクレートで共有するため、各クレートから見て未使用の項目がある

/// major type と長さ (または整数の値) のヘッダを符号化する。
pub fn encode_head(major: u8, value: u64) -> Vec<u8> {
    let mut bytes = vec![major << 5];
    match value {
        0..=23 => bytes[0] |= value as u8,
        24..=0xff => {
            bytes[0] |= 24;
            bytes.push(value as u8);
        }
        0x100..=0xffff => {
            bytes[0] |= 25;
            bytes.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            bytes[0] |= 26;
            bytes.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            bytes[0] |= 27;
            bytes.extend_from_slice(&value.to_be_bytes());
        }
    }
    bytes
}

/// integer を符号化する。
pub fn encode_integer(value: i64) -> Vec<u8> {
    if value >= 0 {
        encode_head(0, value as u64)
    } else {
        encode_head(1, (-1 - value) as u64)
    }
}

/// byte string を符号化する。
pub fn encode_bytes(value: &[u8]) -> Vec<u8> {
    let mut bytes = encode_head(2, value.len() as u64);
    bytes.extend_from_slice(value);
    bytes
}

/// text string を符号化する。
pub fn encode_text(value: &str) -> Vec<u8> {
    let mut bytes = encode_head(3, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
    bytes
}

/// array を符号化する。要素は符号化済みのバイト列で受け取る。
pub fn encode_array(items: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = encode_head(4, items.len() as u64);
    for item in items {
        bytes.extend_from_slice(item);
    }
    bytes
}

/// map を符号化する。組は符号化済みのキーと値で受け取る。
pub fn encode_map(entries: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = encode_head(5, entries.len() as u64);
    for (key, value) in entries {
        bytes.extend_from_slice(key);
        bytes.extend_from_slice(value);
    }
    bytes
}
