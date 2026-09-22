//! `ids` の PBT。どんな 16 バイトからも UUID v4 の形が得られることを検証する。

use coffee_log_core::ids::{uuid_bytes, uuid_v4_from_bytes};
use proptest::prelude::*;

proptest! {
    /// UUID の文字列は、バージョンと variant のビットを除いて元の 16 バイトに戻る。
    /// WebAuthn の `user.id` に使う (ADR-0004)。
    #[test]
    fn a_uuid_round_trips_through_its_bytes(bytes in any::<[u8; 16]>()) {
        let uuid = uuid_v4_from_bytes(bytes);
        let mut expected = bytes;
        expected[6] = (expected[6] & 0x0f) | 0x40;
        expected[8] = (expected[8] & 0x3f) | 0x80;
        prop_assert_eq!(uuid_bytes(&uuid), Some(expected));
    }
    /// 36 文字の小文字 16 進で、ハイフンの位置とバージョンと variant が固定になる。
    #[test]
    fn uuid_v4_has_the_fixed_shape(bytes in any::<[u8; 16]>()) {
        let uuid = uuid_v4_from_bytes(bytes);
        prop_assert_eq!(uuid.len(), 36);
        let chars: Vec<char> = uuid.chars().collect();
        for (index, ch) in chars.iter().enumerate() {
            if matches!(index, 8 | 13 | 18 | 23) {
                prop_assert_eq!(*ch, '-');
            } else {
                prop_assert!(ch.is_ascii_digit() || ('a'..='f').contains(ch), "unexpected character {ch}");
            }
        }
        prop_assert_eq!(chars[14], '4');
        prop_assert!(matches!(chars[19], '8' | '9' | 'a' | 'b'));
    }

    /// バージョンと variant のビット以外は入力のバイトのままになる。
    #[test]
    fn uuid_v4_keeps_the_other_bits(bytes in any::<[u8; 16]>()) {
        let uuid = uuid_v4_from_bytes(bytes);
        let hex: String = uuid.chars().filter(|ch| *ch != '-').collect();
        let mut expected = bytes;
        expected[6] = (expected[6] & 0x0f) | 0x40;
        expected[8] = (expected[8] & 0x3f) | 0x80;
        for (index, byte) in expected.iter().enumerate() {
            prop_assert_eq!(&hex[index * 2..index * 2 + 2], format!("{byte:02x}"));
        }
    }
}
