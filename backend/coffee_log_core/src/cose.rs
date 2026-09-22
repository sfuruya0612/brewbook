//! COSE (RFC 9052) の鍵から EC2 の P-256 公開鍵を取り出す (ADR-0004)。
//!
//! 受け付けるのは kty が EC2 (2)、alg が ES256 (-7)、crv が P-256 (1)、
//! x と y が 32 バイトの byte string であるキーだけとする。
//! 鍵のラベルは RFC 9052 の 7.1 (COSE Key Common Parameters) と 13.1.1 (EC2) の値を使う。

use crate::cbor;

/// EC2 (P-256) の公開鍵の座標。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ec2PublicKey {
    pub x: [u8; 32],
    pub y: [u8; 32],
}

impl Ec2PublicKey {
    /// SEC1 の非圧縮形式 (0x04 || x || y)。`p256` の公開鍵の組み立てに使う。
    pub fn to_sec1_bytes(self) -> [u8; 65] {
        let mut bytes = [0u8; 65];
        bytes[0] = 0x04;
        bytes[1..33].copy_from_slice(&self.x);
        bytes[33..].copy_from_slice(&self.y);
        bytes
    }
}

/// COSE の鍵の検証の失敗。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 鍵の CBOR がデコードできない。
    Cbor(cbor::Error),
    /// 鍵が map ではない。
    NotAMap,
    /// kty が EC2 (2) ではない。
    UnsupportedKeyType,
    /// alg が ES256 (-7) ではない。
    UnsupportedAlgorithm,
    /// crv が P-256 (1) ではない。
    UnsupportedCurve,
    /// x か y が 32 バイトの byte string ではない。
    InvalidCoordinate,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Cbor(error) => write!(formatter, "the COSE key is not valid CBOR: {error}"),
            Error::NotAMap => write!(formatter, "the COSE key is not a map"),
            Error::UnsupportedKeyType => write!(formatter, "the COSE key type is not EC2"),
            Error::UnsupportedAlgorithm => write!(formatter, "the COSE algorithm is not ES256"),
            Error::UnsupportedCurve => write!(formatter, "the COSE curve is not P-256"),
            Error::InvalidCoordinate => {
                write!(formatter, "the COSE key has no 32 byte x and y coordinate")
            }
        }
    }
}

impl std::error::Error for Error {}

/// COSE の鍵の CBOR をデコードして EC2 の P-256 公開鍵を取り出す。
pub fn parse(input: &[u8]) -> Result<Ec2PublicKey, Error> {
    let value = cbor::decode(input).map_err(Error::Cbor)?;
    from_value(&value)
}

/// デコード済みの CBOR から EC2 の P-256 公開鍵を取り出す。
pub fn from_value(value: &cbor::Value<'_>) -> Result<Ec2PublicKey, Error> {
    if !matches!(value, cbor::Value::Map(_)) {
        return Err(Error::NotAMap);
    }
    // RFC 9052 の 7.1: kty はラベル 1、alg はラベル 3、crv はラベル -1、x は -2、y は -3。
    expect_integer(value, 1, 2, Error::UnsupportedKeyType)?;
    expect_integer(value, 3, -7, Error::UnsupportedAlgorithm)?;
    expect_integer(value, -1, 1, Error::UnsupportedCurve)?;
    Ok(Ec2PublicKey {
        x: coordinate(value, -2)?,
        y: coordinate(value, -3)?,
    })
}

/// integer のラベルが期待した値であることを確かめる。
fn expect_integer(
    value: &cbor::Value<'_>,
    label: i64,
    expected: i64,
    error: Error,
) -> Result<(), Error> {
    match value
        .map_get_integer(label)
        .and_then(cbor::Value::as_integer)
    {
        Some(found) if found == expected => Ok(()),
        _ => Err(error),
    }
}

/// 32 バイトの座標を読む。
fn coordinate(value: &cbor::Value<'_>, label: i64) -> Result<[u8; 32], Error> {
    let bytes = value
        .map_get_integer(label)
        .and_then(cbor::Value::as_bytes)
        .ok_or(Error::InvalidCoordinate)?;
    bytes.try_into().map_err(|_| Error::InvalidCoordinate)
}
