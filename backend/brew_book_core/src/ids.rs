//! UUID v4 の組み立て (ADR-0002)。
//!
//! 乱数は呼び出し側 (Worker) が Web Crypto から 16 バイト取って渡す。
//! このモジュールはバイト列から UUID v4 の文字列を組み立てる純粋な関数だけを持つ。

/// UUID の文字列表記の長さ (ハイフンを含む 36 文字)。
pub const UUID_TEXT_LEN: usize = 36;

/// 16 バイトの乱数から UUID v4 の文字列 (小文字、ハイフン付き) を組み立てる。
///
/// 6 バイト目の上位 4 ビットを `0100` (バージョン 4)、8 バイト目の上位 2 ビットを
/// `10` (RFC 4122 の variant) にする。それ以外のビットは入力のまま使う。
pub fn uuid_v4_from_bytes(bytes: [u8; 16]) -> String {
    let mut bytes = bytes;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(32);
    for byte in bytes {
        hex.push(HEX[(byte >> 4) as usize] as char);
        hex.push(HEX[(byte & 0x0f) as usize] as char);
    }
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// UUID の文字列 (ハイフン付き) を 16 バイトに戻す。
///
/// `uuid_v4_from_bytes` が作る小文字の表記のほか、大文字の 16 進も受け付ける。
/// 長さが 36 文字でない、区切りの位置が違う、16 進の数字でない文字がある場合は None を返す。
/// WebAuthn の `user.id` は利用者の UUID の 16 バイトを入れる (ADR-0004)。
pub fn uuid_bytes(uuid: &str) -> Option<[u8; 16]> {
    let bytes = uuid.as_bytes();
    if bytes.len() != UUID_TEXT_LEN {
        return None;
    }
    const SEPARATORS: [usize; 4] = [8, 13, 18, 23];
    let mut nibbles = [0u8; 32];
    let mut index = 0;
    for (position, byte) in bytes.iter().enumerate() {
        if SEPARATORS.contains(&position) {
            if *byte != b'-' {
                return None;
            }
            continue;
        }
        nibbles[index] = hex_value(*byte)?;
        index += 1;
    }
    let mut result = [0u8; 16];
    for (index, slot) in result.iter_mut().enumerate() {
        *slot = (nibbles[index * 2] << 4) | nibbles[index * 2 + 1];
    }
    Some(result)
}

/// 16 進の 1 文字を 4 ビットの値にする。
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
