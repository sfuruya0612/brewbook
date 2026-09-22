//! `ids` の単体テスト。往復と形の検査は PBT (`prop_ids.rs`) が担う。

use coffee_log_core::ids::uuid_v4_from_bytes;

#[test]
fn all_zero_bytes_set_the_version_and_variant_bits() {
    let uuid = uuid_v4_from_bytes([0x00; 16]);
    assert_eq!(uuid, "00000000-0000-4000-8000-000000000000");
}

#[test]
fn all_one_bytes_keep_the_other_bits() {
    let uuid = uuid_v4_from_bytes([0xff; 16]);
    assert_eq!(uuid, "ffffffff-ffff-4fff-bfff-ffffffffffff");
}

#[test]
fn a_known_byte_sequence_becomes_the_expected_lowercase_uuid() {
    let bytes = [
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88,
    ];
    let uuid = uuid_v4_from_bytes(bytes);
    assert_eq!(uuid, "12345678-9abc-4ef0-9122-334455667788");
}
