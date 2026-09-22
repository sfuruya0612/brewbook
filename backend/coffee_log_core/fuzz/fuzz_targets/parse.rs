#![no_main]

//! CBOR と COSE のパーサ、および WebAuthn の検証の Fuzzing ターゲット (issue 0004)。
//!
//! 任意のバイト列を与えて panic しないことを検証する。CI では実行せず、手元で実行する
//! (cargo fuzz は nightly を要する。ADR-0009 の版固定と二重管理を避ける)。

use libfuzzer_sys::fuzz_target;
use sha2::{Digest, Sha256};

fuzz_target!(|data: &[u8]| {
    // CBOR のデコーダ。長さの上限とネストの上限に加えて、範囲の切り出しが panic しないこと。
    let _ = coffee_log_core::cbor::decode(data);
    // COSE の鍵のパーサ。
    let _ = coffee_log_core::cose::parse(data);
    // base64url の復号。WebAuthn の入力 JSON が運ぶ文字列を扱う。
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = coffee_log_core::base64url::decode(text);
    }

    // 登録の検証。clientDataJSON は JSON のバイト列を base64url で符号化した形で渡し、
    // その challenge を入力の符号化と一致させることで、attestation object の解析に到達させる。
    let challenge = coffee_log_core::base64url::encode(data);
    let registration_json = format!(
        r#"{{"type":"webauthn.create","challenge":"{challenge}","origin":"https://example.org"}}"#
    );
    let registration_client_data_json =
        coffee_log_core::base64url::encode(registration_json.as_bytes());
    let registration = coffee_log_core::webauthn::RegistrationInput {
        rp_id: "example.org",
        origin: "https://example.org",
        expected_challenge: &challenge,
        client_data_json: &registration_client_data_json,
        attestation_object: &challenge,
    };
    let _ = coffee_log_core::webauthn::verify_registration(&registration);

    // ログインの検証。authenticatorData は RP ID のハッシュで始め、フラグの検査と
    // 署名カウンタの読み取りにも到達させる (入力はフラグの位置から続く)。
    let mut authenticator_data = Sha256::digest(b"example.org").to_vec();
    authenticator_data.extend_from_slice(data);
    let authenticator_data = coffee_log_core::base64url::encode(&authenticator_data);
    let authentication_json = format!(
        r#"{{"type":"webauthn.get","challenge":"{challenge}","origin":"https://example.org"}}"#
    );
    let authentication_client_data_json =
        coffee_log_core::base64url::encode(authentication_json.as_bytes());
    let authentication = coffee_log_core::webauthn::AuthenticationInput {
        rp_id: "example.org",
        origin: "https://example.org",
        expected_challenge: &challenge,
        client_data_json: &authentication_client_data_json,
        authenticator_data: &authenticator_data,
        signature: &challenge,
        cose_public_key: data,
    };
    let _ = coffee_log_core::webauthn::verify_authentication(&authentication);
});
