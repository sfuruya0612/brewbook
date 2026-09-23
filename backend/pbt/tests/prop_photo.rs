//! `photo` の PBT。キーの往復と、UUID でない部分や他の利用者のキーの拒否を検査する。
//!
//! 個別の誤り (プレフィックスや拡張子の欠落、上位のディレクトリへ移動する並び) は
//! 単体テスト (`test_photo.rs`) が担う。

use coffee_log_core::ids::{uuid_bytes, uuid_v4_from_bytes};
use coffee_log_core::photo::{parse_pending_key, pending_key, photo_key};
use proptest::prelude::*;

/// 利用者 ID と購入 ID に使う値。実際の ID (UUID) と同じ文字だけにする。
fn identifier() -> impl Strategy<Value = String> {
    "[0-9a-f-]{1,36}".prop_map(String::from)
}

proptest! {
    /// `pending/<利用者 ID>/<UUID>.jpg` は組み立てて解析すると元の UUID に戻る。
    #[test]
    fn a_pending_key_round_trips(user_id in identifier(), bytes in any::<[u8; 16]>()) {
        let uuid = uuid_v4_from_bytes(bytes);
        let key = pending_key(&user_id, &uuid);
        prop_assert_eq!(parse_pending_key(&user_id, &key), Some(uuid.as_str()));
    }

    /// 他の利用者の ID では解析できない。他の利用者の `pending/` のオブジェクトを取り込めない (FR-10)。
    #[test]
    fn a_pending_key_of_another_user_does_not_parse(
        user_id in identifier(),
        other in identifier(),
        bytes in any::<[u8; 16]>(),
    ) {
        prop_assume!(user_id != other);
        let key = pending_key(&other, &uuid_v4_from_bytes(bytes));
        prop_assert_eq!(parse_pending_key(&user_id, &key), None);
    }

    /// UUID の部分が UUID でなければ解析できない。
    #[test]
    fn a_pending_key_without_a_uuid_does_not_parse(user_id in identifier(), uuid in identifier()) {
        let key = format!("pending/{user_id}/{uuid}.jpg");
        let expected = if uuid_bytes(&uuid).is_some() {
            Some(uuid.as_str())
        } else {
            None
        };
        prop_assert_eq!(parse_pending_key(&user_id, &key), expected);
    }

    /// 紐づけ済みのキーは `pending/` のキーとして解析できない。
    #[test]
    fn a_photo_key_is_not_a_pending_key(
        user_id in identifier(),
        purchase_id in identifier(),
        bytes in any::<[u8; 16]>(),
    ) {
        let key = photo_key(&user_id, &purchase_id, &uuid_v4_from_bytes(bytes));
        prop_assert_eq!(parse_pending_key(&user_id, &key), None);
    }
}
