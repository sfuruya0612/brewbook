//! `auth` の PBT。往復と、時刻の比較の性質を検査する。

use coffee_log_core::auth::{
    encode_secret, hash_secret, is_expired, session_cookie, session_token, validate_passkey_name,
    PASSKEY_NAME_MAX_CHARS, SECRET_LEN,
};
use coffee_log_core::base64url;
use coffee_log_core::datetime::{format_epoch_millis, MAX_EPOCH_MILLIS, MIN_EPOCH_MILLIS};
use proptest::prelude::*;

proptest! {
    /// 32 バイトの乱数は base64url を経由して同じバイト列に戻る。
    #[test]
    fn a_secret_round_trips_through_base64url(bytes in prop::array::uniform32(any::<u8>())) {
        let encoded = encode_secret(bytes);
        prop_assert_eq!(base64url::decode(&encoded), Ok(bytes.to_vec()));
        prop_assert_eq!(encoded.len(), (SECRET_LEN * 4).div_ceil(3));
    }

    /// セッションの Cookie からは、埋めたトークンがそのまま取り出せる。
    #[test]
    fn the_session_cookie_round_trips(
        token in prop::array::uniform32(any::<u8>()),
        max_age in -1_i64..3_000_000_000_i64,
    ) {
        let token = encode_secret(token);
        let cookie = session_cookie(&token, max_age);
        prop_assert_eq!(session_token(&cookie), Some(token.as_str()));
    }

    /// 期限の比較は、符号化した文字列の比較と epoch ミリ秒の比較が一致する
    /// (辞書順の比較が時刻の順と一致し、`is_expired` がそれを用いていること)。
    #[test]
    fn the_expiry_comparison_matches_the_epoch_millis(
        expires_millis in MIN_EPOCH_MILLIS..=MAX_EPOCH_MILLIS,
        now_millis in MIN_EPOCH_MILLIS..=MAX_EPOCH_MILLIS,
    ) {
        let expires_at = format_epoch_millis(expires_millis).expect("a valid time must format");
        let now = format_epoch_millis(now_millis).expect("a valid time must format");
        prop_assert_eq!(is_expired(&expires_at, &now), expires_millis <= now_millis);
    }

    /// 名前の検証は、前後の空白を除いた 1 文字以上 50 文字以下だけを受け付ける。
    #[test]
    fn a_validated_name_is_non_empty_and_within_the_limit(name in ".*") {
        let trimmed = name.trim();
        match validate_passkey_name(&name) {
            Ok(validated) => {
                prop_assert_eq!(validated.as_str(), trimmed);
                prop_assert!(!validated.is_empty());
                prop_assert!(validated.chars().count() <= PASSKEY_NAME_MAX_CHARS);
            }
            Err(_) => {
                prop_assert!(trimmed.is_empty() || trimmed.chars().count() > PASSKEY_NAME_MAX_CHARS);
            }
        }
    }

    /// ハッシュは常に小文字の 16 進 64 文字で、異なる秘密値では異なる。
    #[test]
    fn the_hash_is_a_lowercase_hex_digest(first in ".*", second in ".*") {
        let first_hash = hash_secret(&first);
        prop_assert_eq!(first_hash.len(), 64);
        prop_assert!(first_hash.chars().all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase()));
        if first != second {
            prop_assert_ne!(first_hash, hash_secret(&second));
        }
    }
}
