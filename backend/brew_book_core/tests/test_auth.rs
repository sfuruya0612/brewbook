//! `auth` の単体テスト。往復と不変条件の検査は PBT (`prop_auth.rs`) が担う。

use coffee_log_core::auth::{
    encode_secret, expired_session_cookie, expiry_from, hash_secret, is_expired, session_cookie,
    session_token, validate_passkey_name, NameError, PASSKEY_NAME_MAX_CHARS,
};

#[test]
fn a_name_of_exactly_the_limit_is_accepted() {
    let name = "a".repeat(PASSKEY_NAME_MAX_CHARS);
    assert_eq!(validate_passkey_name(&name), Ok(name));
}

#[test]
fn a_name_over_the_limit_is_rejected() {
    let name = "a".repeat(PASSKEY_NAME_MAX_CHARS + 1);
    assert_eq!(validate_passkey_name(&name), Err(NameError::TooLong));
}

#[test]
fn the_limit_counts_characters_not_bytes() {
    // 50 文字のマルチバイト文字は受け付け、51 文字は拒否する。
    let accepted = "あ".repeat(PASSKEY_NAME_MAX_CHARS);
    assert_eq!(validate_passkey_name(&accepted), Ok(accepted));
    let rejected = "あ".repeat(PASSKEY_NAME_MAX_CHARS + 1);
    assert_eq!(validate_passkey_name(&rejected), Err(NameError::TooLong));
}

#[test]
fn a_name_that_is_empty_after_trimming_is_rejected() {
    assert_eq!(validate_passkey_name(""), Err(NameError::Missing));
    assert_eq!(validate_passkey_name("   "), Err(NameError::Missing));
    assert_eq!(validate_passkey_name("\t\n"), Err(NameError::Missing));
}

#[test]
fn the_name_is_trimmed() {
    assert_eq!(
        validate_passkey_name("  マイキー  "),
        Ok("マイキー".to_owned())
    );
}

#[test]
fn the_expiry_at_the_current_time_is_expired() {
    assert!(is_expired(
        "2026-09-21T00:00:00.000Z",
        "2026-09-21T00:00:00.000Z"
    ));
    assert!(is_expired(
        "2026-09-21T00:00:00.000Z",
        "2026-09-21T00:00:00.001Z"
    ));
    assert!(!is_expired(
        "2026-09-21T00:00:00.001Z",
        "2026-09-21T00:00:00.000Z"
    ));
}

#[test]
fn the_expiry_is_formatted_as_the_fixed_length_iso_8601_utc() {
    assert_eq!(
        expiry_from(0, 300).expect("the expiry must format"),
        "1970-01-01T00:05:00.000Z"
    );
    assert_eq!(
        expiry_from(0, 0).expect("the expiry must format"),
        "1970-01-01T00:00:00.000Z"
    );
}

#[test]
fn the_expiry_is_formatted_from_a_negative_time() {
    assert_eq!(
        expiry_from(-1_000, 1).expect("the expiry must format"),
        "1970-01-01T00:00:00.000Z"
    );
}

#[test]
fn the_hash_is_the_sha256_of_the_secret_in_lowercase_hex() {
    assert_eq!(
        hash_secret("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(hash_secret("").len(), 64);
}

#[test]
fn the_secret_is_the_base64url_of_the_32_bytes() {
    assert_eq!(encode_secret([0; 32]), "A".repeat(43));
}

#[test]
fn the_session_cookie_carries_the_required_attributes() {
    assert_eq!(
        session_cookie("token-value", 2_592_000),
        "session=token-value; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=2592000"
    );
}

#[test]
fn the_expired_session_cookie_has_no_value_and_no_max_age() {
    assert_eq!(
        expired_session_cookie(),
        "session=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0"
    );
}

#[test]
fn the_token_is_taken_from_a_header_with_other_cookies() {
    assert_eq!(
        session_token("theme=dark; session=abc.def; language=ja"),
        Some("abc.def")
    );
    assert_eq!(session_token("session=abc"), Some("abc"));
    assert_eq!(session_token(" session = abc "), Some("abc"));
}

#[test]
fn a_header_without_the_session_cookie_has_no_token() {
    assert_eq!(session_token(""), None);
    assert_eq!(session_token("theme=dark"), None);
    // 名前が一致しない Cookie は対象にしない。
    assert_eq!(session_token("xsession=abc"), None);
    assert_eq!(session_token("session-extra=abc"), None);
    // 空の値は無いものとして扱う。
    assert_eq!(session_token("session="), None);
    assert_eq!(session_token("session=; theme=dark"), None);
}
