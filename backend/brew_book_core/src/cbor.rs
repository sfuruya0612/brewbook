//! WebAuthn が使う範囲に限定した CBOR (RFC 8949) のデコーダ (ADR-0004)。
//!
//! 扱う型は integer、byte string、text string、array、map だけとする。
//! CTAP2 canonical CBOR は indefinite length を許さないため、受け付けない
//! (W3C WebAuthn Level 3 の 2.4 と 3)。
//! 入力の全体を使い切ることを要求し、余分なバイトが残る入力は拒否する。
//! 入力の長さの上限は [`MAX_INPUT_LEN`]、ネストの上限は [`MAX_DEPTH`] とし、
//! どちらも入力の長さに比例して動くことを保つために置く。

/// 入力の長さの上限。WebAuthn の attestation object と COSE の鍵はこれより十分小さい。
pub const MAX_INPUT_LEN: usize = 64 * 1024;

/// ネストの上限。WebAuthn の構造は 3 段 (attestation object、COSE の鍵、座標) までなので、
/// これを超える入力を拒否して、任意入力によるスタックの消費を防ぐ。
pub const MAX_DEPTH: usize = 16;

/// デコードの失敗。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 入力が長さの上限を超えている。
    TooLarge,
    /// アイテムの途中で入力が終わっている。
    UnexpectedEnd,
    /// 最初のアイテムの後に余分なバイトが残っている。
    TrailingBytes,
    /// indeterminate length (additional information の 31) を使っている。
    IndefiniteLength,
    /// text string が妥当な UTF-8 ではない。
    InvalidUtf8,
    /// integer が i64 の範囲に入らない。
    IntegerOutOfRange,
    /// WebAuthn が使わない型 (tag、simple、float と予約の additional information) を使っている。
    Unsupported,
    /// ネストが [`MAX_DEPTH`] を超えている。
    TooDeep,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::TooLarge => write!(
                formatter,
                "the input is longer than the {MAX_INPUT_LEN} byte limit"
            ),
            Error::UnexpectedEnd => {
                write!(formatter, "the input ends in the middle of a CBOR item")
            }
            Error::TrailingBytes => {
                write!(formatter, "the input has bytes after the first CBOR item")
            }
            Error::IndefiniteLength => write!(
                formatter,
                "the input uses an indefinite length, which CTAP2 canonical CBOR forbids"
            ),
            Error::InvalidUtf8 => write!(formatter, "a text string is not valid UTF-8"),
            Error::IntegerOutOfRange => write!(formatter, "an integer does not fit in i64"),
            Error::Unsupported => write!(
                formatter,
                "the input uses a CBOR type or additional information that WebAuthn does not use"
            ),
            Error::TooDeep => write!(
                formatter,
                "the nesting of the input is deeper than the limit of {MAX_DEPTH}"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// デコードした CBOR のアイテム。バイト列と text は入力から借用する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'a> {
    Integer(i64),
    ByteString(&'a [u8]),
    TextString(&'a str),
    Array(Vec<Value<'a>>),
    Map(Vec<(Value<'a>, Value<'a>)>),
}

impl<'a> Value<'a> {
    /// integer なら値を返す。
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// byte string なら中身を返す。
    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        match self {
            Value::ByteString(value) => Some(value),
            _ => None,
        }
    }

    /// text string なら中身を返す。
    pub fn as_text(&self) -> Option<&'a str> {
        match self {
            Value::TextString(value) => Some(value),
            _ => None,
        }
    }

    /// array なら要素を返す。
    pub fn as_array(&self) -> Option<&[Value<'a>]> {
        match self {
            Value::Array(values) => Some(values),
            _ => None,
        }
    }

    /// map なら組の並びを返す。並びは入力の順序を保つ。
    pub fn as_map(&self) -> Option<&[(Value<'a>, Value<'a>)]> {
        match self {
            Value::Map(entries) => Some(entries),
            _ => None,
        }
    }

    /// map から integer のキーで値を引く。同じキーが複数あるときは最初の組を返す。
    pub fn map_get_integer(&self, key: i64) -> Option<&Value<'a>> {
        self.as_map()?
            .iter()
            .find(|(entry_key, _)| entry_key.as_integer() == Some(key))
            .map(|(_, value)| value)
    }

    /// map から text のキーで値を引く。同じキーが複数あるときは最初の組を返す。
    pub fn map_get_text(&self, key: &str) -> Option<&Value<'a>> {
        self.as_map()?
            .iter()
            .find(|(entry_key, _)| entry_key.as_text() == Some(key))
            .map(|(_, value)| value)
    }
}

/// CBOR をデコードする。入力の全体を使い切ることを要求する。
pub fn decode(input: &[u8]) -> Result<Value<'_>, Error> {
    let (value, consumed) = decode_prefix(input)?;
    if consumed != input.len() {
        return Err(Error::TrailingBytes);
    }
    Ok(value)
}

/// 先頭の 1 つのアイテムをデコードし、消費した長さを返す。
///
/// authenticatorData に埋め込まれた COSE の公開鍵のように、後ろにデータ (拡張) が続く入力を
/// 扱うために使う。
pub fn decode_prefix(input: &[u8]) -> Result<(Value<'_>, usize), Error> {
    if input.len() > MAX_INPUT_LEN {
        return Err(Error::TooLarge);
    }
    parse(input, 0, 1)
}

/// 位置 `offset` から 1 つのアイテムをデコードする。`depth` は現在のネストの深さ。
fn parse(input: &[u8], offset: usize, depth: usize) -> Result<(Value<'_>, usize), Error> {
    if depth > MAX_DEPTH {
        return Err(Error::TooDeep);
    }
    let initial = *input.get(offset).ok_or(Error::UnexpectedEnd)?;
    let major = initial >> 5;
    let additional = initial & 0x1f;
    let (length, next) = read_length(input, offset + 1, additional)?;
    match major {
        0 => {
            let value = i64::try_from(length).map_err(|_| Error::IntegerOutOfRange)?;
            Ok((Value::Integer(value), next))
        }
        1 => {
            // major type 1 の値は -1 - length。
            let value = i64::try_from(length).map_err(|_| Error::IntegerOutOfRange)?;
            Ok((Value::Integer(-1 - value), next))
        }
        2 => {
            let (bytes, end) = slice(input, next, length)?;
            Ok((Value::ByteString(bytes), end))
        }
        3 => {
            let (bytes, end) = slice(input, next, length)?;
            let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
            Ok((Value::TextString(text), end))
        }
        4 => {
            let mut values = Vec::new();
            let mut cursor = next;
            for _ in 0..length {
                let (value, consumed) = parse(input, cursor, depth + 1)?;
                values.push(value);
                cursor = consumed;
            }
            Ok((Value::Array(values), cursor))
        }
        5 => {
            let mut entries = Vec::new();
            let mut cursor = next;
            for _ in 0..length {
                let (key, after_key) = parse(input, cursor, depth + 1)?;
                let (value, after_value) = parse(input, after_key, depth + 1)?;
                entries.push((key, value));
                cursor = after_value;
            }
            Ok((Value::Map(entries), cursor))
        }
        _ => Err(Error::Unsupported),
    }
}

/// additional information から長さと次の位置を読む。
fn read_length(input: &[u8], offset: usize, additional: u8) -> Result<(u64, usize), Error> {
    match additional {
        0..=23 => Ok((u64::from(additional), offset)),
        24..=27 => {
            let width = 1 << (additional - 24);
            let end = offset.checked_add(width).ok_or(Error::UnexpectedEnd)?;
            let bytes = input.get(offset..end).ok_or(Error::UnexpectedEnd)?;
            let mut length = 0u64;
            for byte in bytes {
                length = length << 8 | u64::from(*byte);
            }
            Ok((length, end))
        }
        28..=30 => Err(Error::Unsupported),
        _ => Err(Error::IndefiniteLength),
    }
}

/// 長さ `length` の範囲を切り出し、範囲の終わりの位置を返す。
/// 範囲が入力の外に出る場合は失敗する。
fn slice(input: &[u8], offset: usize, length: u64) -> Result<(&[u8], usize), Error> {
    let length = usize::try_from(length).map_err(|_| Error::UnexpectedEnd)?;
    let end = offset.checked_add(length).ok_or(Error::UnexpectedEnd)?;
    let bytes = input.get(offset..end).ok_or(Error::UnexpectedEnd)?;
    Ok((bytes, end))
}
