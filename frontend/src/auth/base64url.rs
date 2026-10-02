//! WebAuthn が使う base64url (RFC 4648 の Section 5) の復号と符号化 (FR-1、FR-2)。
//!
//! サーバーとのオプションとクレデンシャルの JSON は、WebAuthn の `Base64url Encoding` に
//! 従って base64url の文字列で運ばれる。末尾の `=` は付けず、改行と空白とその他の文字を
//! 含めない (W3C WebAuthn Level 3 の 3)。このモジュールはそれに合わせて、パディングと空白を
//! 受け付けない。
//!
//! 復号はサーバーのオプションの `challenge`、`user.id`、`allowCredentials` の `id` を
//! ブラウザの API が受け取るバイト列にするために使い、符号化は `navigator.credentials` の
//! 応答 (クレデンシャル ID、`clientDataJSON`、`attestationObject`、署名) をサーバーに返す
//! JSON にするために使う。どちらも純粋な関数で、native のテストと PBT が守る (0040)。

/// base64url の復号の失敗。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// アルファベットに無い文字が含まれる (パディングの `=` を含む)。
    InvalidCharacter,
    /// 4 文字の組にならない長さである (端数が 1 文字など)。
    InvalidLength,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidCharacter => {
                write!(
                    formatter,
                    "the text has a character outside the base64url alphabet"
                )
            }
            Error::InvalidLength => {
                write!(formatter, "the length of the text is not valid base64url")
            }
        }
    }
}

impl std::error::Error for Error {}

/// base64url のアルファベット。RFC 4648 の Section 5 の URL とファイル名に安全な文字集合。
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// base64url を復号する。末尾のパディングは受け付けない (W3C WebAuthn Level 3 の 3)。
pub fn decode(input: &str) -> Result<Vec<u8>, Error> {
    let bytes = input.as_bytes();
    if bytes.len() % 4 == 1 {
        return Err(Error::InvalidLength);
    }
    let mut decoded = Vec::with_capacity(bytes.len() / 4 * 3 + 2);
    for chunk in bytes.chunks(4) {
        let mut values = [0u8; 4];
        for (index, byte) in chunk.iter().enumerate() {
            values[index] = value_of(*byte)?;
        }
        // 4 文字で 3 バイト、端数は 2 文字で 1 バイト、3 文字で 2 バイトになる。
        decoded.push((values[0] << 2) | (values[1] >> 4));
        if chunk.len() > 2 {
            decoded.push((values[1] << 4) | (values[2] >> 2));
        }
        if chunk.len() > 3 {
            decoded.push((values[2] << 6) | values[3]);
        }
    }
    Ok(decoded)
}

/// base64url に符号化する。パディングは付けない (W3C WebAuthn Level 3 の 3)。
pub fn encode(input: &[u8]) -> String {
    let mut encoded = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        encoded.push(ALPHABET[usize::from(first >> 2)] as char);
        encoded.push(ALPHABET[usize::from((first & 0x03) << 4 | second >> 4)] as char);
        if chunk.len() > 1 {
            encoded.push(ALPHABET[usize::from((second & 0x0f) << 2 | third >> 6)] as char);
        }
        if chunk.len() > 2 {
            encoded.push(ALPHABET[usize::from(third & 0x3f)] as char);
        }
    }
    encoded
}

/// 1 文字を 6 ビットの値にする。
fn value_of(byte: u8) -> Result<u8, Error> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'-' => Ok(62),
        b'_' => Ok(63),
        _ => Err(Error::InvalidCharacter),
    }
}
