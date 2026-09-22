//! UUID v4 の組み立て (ADR-0002)。
//!
//! 乱数は呼び出し側 (Worker) が Web Crypto から 16 バイト取って渡す。
//! このモジュールはバイト列から UUID v4 の文字列を組み立てる純粋な関数だけを持つ。

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
