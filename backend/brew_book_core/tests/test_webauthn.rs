//! WebAuthn の検証の単体テスト。
//!
//! 出典: W3C WebAuthn Level 3 の 16 節 (Test Vectors) <https://www.w3.org/TR/webauthn-3/#sctn-test-vectors>。
//! 16.2 (attestation none の ES256)、16.3 (packed の ES256)、16.4 (`crossOrigin` が true の none の ES256)、
//! 16.8 (packed の ES384) の値をそのまま使う。
//! 16.8 の COSE の鍵は、ES256 以外の公開鍵を拒否することを確かめるために、
//! `fmt` を none に置き換えた attestation object に埋め込んで使う。
//! 16.4 の認証の入力は UP と UV が両方立っている唯一の none の ES256 の組なので、成功の経路に使う。
//! 16.2 の認証の入力は UV が立っていないため、UV の欠落の経路に使う。

use brew_book_core::base64url;
use brew_book_core::cose;
use brew_book_core::webauthn::{
    check_sign_count, verify_authentication, verify_registration, AuthenticationInput, Error,
    RegistrationInput, SignCounter,
};

/// テストベクタが使う RP ID。
const RP_ID: &str = "example.org";

/// テストベクタが使う Origin。
const ORIGIN: &str = "https://example.org";

/// 16 進数の文字列をバイト列にする。
fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).unwrap())
        .collect()
}

/// 16 進数の文字列を、API の入力と同じ base64url の文字列にする。
fn encoded(text: &str) -> String {
    base64url::encode(&hex(text))
}

/// 16.4 の credential の秘密鍵で署名する。署名の対象は authenticatorData と
/// clientDataJSON の SHA-256 (W3C WebAuthn Level 3 の 7.2)。
fn sign(authenticator_data: &[u8], client_data_json: &[u8]) -> Vec<u8> {
    use p256::ecdsa::{signature::Signer, Signature, SigningKey};
    use sha2::{Digest, Sha256};

    let signing_key = SigningKey::from_slice(&hex(vectors::FOUR_CREDENTIAL_PRIVATE_KEY)).unwrap();
    let mut signed_data = authenticator_data.to_vec();
    signed_data.extend_from_slice(&Sha256::digest(client_data_json));
    let signature: Signature = signing_key.sign(&signed_data);
    signature.to_der().as_bytes().to_vec()
}

/// attestation object に埋め込まれた COSE の公開鍵を取り出す。
///
/// authData は attestation object の 30 バイト目 (`fmt` と `attStmt` の後) から始まり、
/// その 37 バイト目から aaguid 16 バイト、credential ID の長さ 2 バイト、credential ID 32 バイト、
/// COSE の公開鍵の順に並ぶ (W3C WebAuthn Level 3 の 6.1)。
fn embedded_public_key(attestation_object: &str) -> Vec<u8> {
    const AUTH_DATA_OFFSET: usize = 30;
    const BEFORE_PUBLIC_KEY_LEN: usize = 37 + 16 + 2 + 32;
    hex(attestation_object)[AUTH_DATA_OFFSET + BEFORE_PUBLIC_KEY_LEN..].to_vec()
}

/// 公開テストベクタの 16 進数。定数の並びは仕様の表記のままにする。
mod vectors {
    // 16.2 ES256 Credential with No Attestation (attestation は none)。
    pub const TWO_REGISTRATION_CHALLENGE: &str =
        "00c30fb78531c464d2b6771dab8d7b603c01162f2fa486bea70f283ae556e130";
    pub const TWO_REGISTRATION_CLIENT_DATA_JSON: &str = "7b2274797065223a22776562617574686e2e637265617465222c226368616c6c656e6765223a22414d4d507434557878475453746e63647134313759447742466938767049612d7077386f4f755657345441222c226f726967696e223a2268747470733a2f2f6578616d706c652e6f7267222c2263726f73734f726967696e223a66616c73652c22657874726144617461223a22636c69656e74446174614a534f4e206d617920626520657874656e6465642077697468206164646974696f6e616c206669656c647320696e20746865206675747572652c207375636820617320746869733a20426b5165446a646354427258426941774a544c453551227d";
    pub const TWO_REGISTRATION_ATTESTATION_OBJECT: &str = "a363666d74646e6f6e656761747453746d74a068617574684461746158a4bfabc37432958b063360d3ad6461c9c4735ae7f8edd46592a5e0f01452b2e4b559000000008446ccb9ab1db374750b2367ff6f3a1f0020f91f391db4c9b2fde0ea70189cba3fb63f579ba6122b33ad94ff3ec330084be4a5010203262001215820afefa16f97ca9b2d23eb86ccb64098d20db90856062eb249c33a9b672f26df61225820930a56b87a2fca66334b03458abf879717c12cc68ed73290af2e2664796b9220";
    pub const TWO_AUTHENTICATION_CHALLENGE: &str =
        "39c0e7521417ba54d43e8dc95174f423dee9bf3cd804ff6d65c857c9abf4d408";
    pub const TWO_AUTHENTICATION_AUTHENTICATOR_DATA: &str =
        "bfabc37432958b063360d3ad6461c9c4735ae7f8edd46592a5e0f01452b2e4b51900000000";
    pub const TWO_AUTHENTICATION_CLIENT_DATA_JSON: &str = "7b2274797065223a22776562617574686e2e676574222c226368616c6c656e6765223a224f63446e55685158756c5455506f334a5558543049393770767a7a59425039745a63685879617630314167222c226f726967696e223a2268747470733a2f2f6578616d706c652e6f7267222c2263726f73734f726967696e223a66616c73657d";
    pub const TWO_AUTHENTICATION_SIGNATURE: &str = "3046022100f50a4e2e4409249c4a853ba361282f09841df4dd4547a13a87780218deffcd380221008480ac0f0b93538174f575bf11a1dd5d78c6e486013f937295ea13653e331e87";

    // 16.3 ES256 Credential with Self Attestation (attestation は packed)。
    pub const THREE_REGISTRATION_CHALLENGE: &str =
        "7869c2b772d4b58eba9378cf8f29e26cf935aa77df0da89fa99c0bdc0a76f7e5";
    pub const THREE_REGISTRATION_CLIENT_DATA_JSON: &str = "7b2274797065223a22776562617574686e2e637265617465222c226368616c6c656e6765223a2265476e4374334c55745936366b336a506a796e6962506b31716e666644616966715a774c33417032392d55222c226f726967696e223a2268747470733a2f2f6578616d706c652e6f7267222c2263726f73734f726967696e223a66616c73652c22657874726144617461223a22636c69656e74446174614a534f4e206d617920626520657874656e6465642077697468206164646974696f6e616c206669656c647320696e20746865206675747572652c207375636820617320746869733a205539685458764b453255526b4d6e625f307859485667227d";
    pub const THREE_REGISTRATION_ATTESTATION_OBJECT: &str = "a363666d74667061636b65646761747453746d74a263616c672663736967584630440220067a20754ab925005dbf378097c92120031581c73228d1fb4f5b881bcd7da98302207fc7b147558c7c0eba3af18bd9d121fa3d3a26d17fe3f220272178f473b6006d68617574684461746158a4bfabc37432958b063360d3ad6461c9c4735ae7f8edd46592a5e0f01452b2e4b55d00000000df850e09db6afbdfab51697791506cfc0020455ef34e2043a87db3d4afeb39bbcb6cc32df9347c789a865ecdca129cbef58ca5010203262001215820eb151c8176b225cc651559fecf07af450fd85802046656b34c18f6cf193843c5225820927b8aa427a2be1b8834d233a2d34f61f13bfd44119c325d5896e183fee484f2";

    // 16.4 ES256 Credential with "crossOrigin": true in clientDataJSON (attestation は none)。
    pub const FOUR_REGISTRATION_CHALLENGE: &str =
        "3be5aacd03537142472340ab5969f240f1d87716e20b6807ac230655fa4b3b49";
    pub const FOUR_REGISTRATION_CLIENT_DATA_JSON: &str = "7b2274797065223a22776562617574686e2e637265617465222c226368616c6c656e6765223a224f2d57717a514e5463554a484930437257576e7951504859647862694332674872434d475666704c4f306b222c226f726967696e223a2268747470733a2f2f6578616d706c652e6f7267222c2263726f73734f726967696e223a747275652c22657874726144617461223a22636c69656e74446174614a534f4e206d617920626520657874656e6465642077697468206164646974696f6e616c206669656c647320696e20746865206675747572652c207375636820617320746869733a207a5a7175457444523944577170573574425754467567227d";
    pub const FOUR_REGISTRATION_ATTESTATION_OBJECT: &str = "a363666d74646e6f6e656761747453746d74a068617574684461746158a4bfabc37432958b063360d3ad6461c9c4735ae7f8edd46592a5e0f01452b2e4b54500000000883f4f6014f19c09d87aa38123be48d000206e1050c0d2ca2f07c755cb2c66a74c64fa43065c18f938354d9915db2bd5ce57a501020326200121582022200a473f90b11078851550d03b4e44a2279f8c4eca27b3153dedfe03e4e97d225820cbd0be95e746ad6f5a8191be11756e4c0420e72f65b466d39bc56b8b123a9c6e";
    pub const FOUR_CREDENTIAL_ID: &str =
        "6e1050c0d2ca2f07c755cb2c66a74c64fa43065c18f938354d9915db2bd5ce57";
    pub const FOUR_CREDENTIAL_PRIVATE_KEY: &str =
        "96c940e769bd9f1237c119f144fa61a4d56af0b3289685ae2bef7fb89620623d";
    pub const FOUR_AUTHENTICATION_CHALLENGE: &str =
        "876aa517ba83fdee65fcffdbca4c84eeae5d54f8041a1fc85c991e5bbb273137";
    pub const FOUR_AUTHENTICATION_AUTHENTICATOR_DATA: &str =
        "bfabc37432958b063360d3ad6461c9c4735ae7f8edd46592a5e0f01452b2e4b50500000000";
    pub const FOUR_AUTHENTICATION_CLIENT_DATA_JSON: &str = "7b2274797065223a22776562617574686e2e676574222c226368616c6c656e6765223a226832716c463771445f65356c5f505f62796b7945377135645650674547685f49584a6b655737736e4d5463222c226f726967696e223a2268747470733a2f2f6578616d706c652e6f7267222c2263726f73734f726967696e223a747275652c22657874726144617461223a22636c69656e74446174614a534f4e206d617920626520657874656e6465642077697468206164646974696f6e616c206669656c647320696e20746865206675747572652c207375636820617320746869733a2039327063545644304162792d713464746d6a36656667227d";
    pub const FOUR_AUTHENTICATION_SIGNATURE: &str = "3046022100eb12fcf23b12764c0f122e22371fab92e283879fd798f38ee1841c951b6e40e7022100c76237ff9db77b3c56f30837cda6a09acfa2e915544e609c0733b1184036d1cf";

    // 16.8 Packed Attestation with ES384 Credential の ES384 の COSE の鍵を、
    // fmt が none の attestation object に埋め込んだもの。authData は
    // sha256("example.org")、フラグ 0x45 (UP、UV、AT)、署名カウンタ 0、16.8 の aaguid、
    // 16.8 の credential ID、16.8 の COSE の鍵の順に並べる。
    pub const EIGHT_REGISTRATION_CHALLENGE: &str =
        "567b030b3e186bc1d169dd45b79f9e0d86f1fd63474da3eade5bdb8db379a0c3";
    pub const EIGHT_REGISTRATION_CLIENT_DATA_JSON: &str = "7b2274797065223a22776562617574686e2e637265617465222c226368616c6c656e6765223a22566e7344437a3459613848526164314674352d65445962785f574e4854615071336c76626a624e356f4d4d222c226f726967696e223a2268747470733a2f2f6578616d706c652e6f7267222c2263726f73734f726967696e223a66616c73657d";
    pub const EIGHT_NONE_ATTESTATION_OBJECT: &str = "a363666d74646e6f6e656761747453746d74a068617574684461746158c5bfabc37432958b063360d3ad6461c9c4735ae7f8edd46592a5e0f01452b2e4b54500000000e950dcda3bdae1d087cda380a897848b0020953ae2dd9f28b1a1d5802c83e1f65833bb9769a08de82d812bc27c13fc6f06a9a5010203382220022158304866bd8b01da789e9eb806e5eab05ae5a638542296ab057a2f1bbce9b58f8a08b9171390b58a37ac7fffc2c5f45857da2258302a0b024c7f4b72072a1f96bd30a7261aae9571dd39870eb29e55c0941c6b08e89629a1ea1216aa64ce57c2807bf3901a";
}

/// 登録の成功と拒否の経路。
mod registration {
    use super::*;

    #[test]
    fn a_published_none_es256_vector_registers() {
        let challenge = encoded(vectors::FOUR_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = encoded(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let input = RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        };
        let registered = verify_registration(&input).unwrap();
        assert_eq!(registered.credential_id, hex(vectors::FOUR_CREDENTIAL_ID));
        assert_eq!(registered.sign_count, 0);
        // 保存する COSE の公開鍵は、attestation object に埋め込まれた鍵そのもの。
        let public_key = cose::parse(&registered.cose_public_key).unwrap();
        assert_eq!(
            public_key.to_sec1_bytes().to_vec(),
            hex("0422200a473f90b11078851550d03b4e44a2279f8c4eca27b3153dedfe03e4e97dcbd0be95e746ad6f5a8191be11756e4c0420e72f65b466d39bc56b8b123a9c6e")
        );
    }

    #[test]
    fn the_result_is_usable_for_a_later_authentication() {
        let challenge = encoded(vectors::FOUR_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = encoded(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let registered = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap();
        // 16.4 の認証は、同じ組の公開鍵と credential ID を使う。
        assert_eq!(registered.credential_id, hex(vectors::FOUR_CREDENTIAL_ID));
        let authenticator_data = encoded(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        let authentication_client_data_json =
            encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let signature = encoded(vectors::FOUR_AUTHENTICATION_SIGNATURE);
        let authentication_challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let sign_count = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &authentication_challenge,
            client_data_json: &authentication_client_data_json,
            authenticator_data: &authenticator_data,
            signature: &signature,
            cose_public_key: &registered.cose_public_key,
        })
        .unwrap();
        assert_eq!(sign_count, 0);
    }

    #[test]
    fn a_packed_attestation_object_is_rejected() {
        let challenge = encoded(vectors::THREE_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::THREE_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = encoded(vectors::THREE_REGISTRATION_ATTESTATION_OBJECT);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::UnsupportedAttestationFormat);
    }

    #[test]
    fn a_statement_that_is_not_empty_is_rejected() {
        // 16.2 の attestation object の attStmt を空の map (a0) から map {1: 2} (a10102) に置き換える。
        const PREFIX: &str = "a363666d74646e6f6e656761747453746d74";
        let object = vectors::TWO_REGISTRATION_ATTESTATION_OBJECT;
        assert!(object.starts_with(PREFIX));
        let attestation_object = encoded(&format!("{PREFIX}a10102{}", &object[PREFIX.len() + 2..]));
        let challenge = encoded(vectors::TWO_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::TWO_REGISTRATION_CLIENT_DATA_JSON);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::AttestationStatementNotEmpty);
    }

    #[test]
    fn an_es384_public_key_is_rejected() {
        let challenge = encoded(vectors::EIGHT_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::EIGHT_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = encoded(vectors::EIGHT_NONE_ATTESTATION_OBJECT);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::PublicKey(cose::Error::UnsupportedAlgorithm));
    }

    #[test]
    fn a_different_origin_is_rejected() {
        let challenge = encoded(vectors::FOUR_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = encoded(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: "https://example.com",
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::OriginMismatch);
    }

    #[test]
    fn a_different_rp_id_is_rejected() {
        // rpIdHash が RP ID の SHA-256 と一致しない登録は拒否する。
        let challenge = encoded(vectors::FOUR_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = encoded(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let error = verify_registration(&RegistrationInput {
            rp_id: "example.com",
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::RpIdHashMismatch);
    }

    #[test]
    fn a_different_challenge_is_rejected() {
        let challenge = encoded(vectors::THREE_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = encoded(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::ChallengeMismatch);
    }

    #[test]
    fn a_get_client_data_json_is_rejected() {
        // 16.4 の認証の clientDataJSON は type が webauthn.get なので、登録では受け付けない。
        let challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let attestation_object = encoded(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::ClientDataTypeMismatch);
    }

    #[test]
    fn a_missing_user_verification_flag_is_rejected() {
        // 16.2 の登録は UP と AT だけが立っていて、UV が立っていない (ADR-0004 は UV を要求する)。
        let challenge = encoded(vectors::TWO_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::TWO_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = encoded(vectors::TWO_REGISTRATION_ATTESTATION_OBJECT);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::UserVerifiedMissing);
    }

    #[test]
    fn a_missing_attested_credential_data_flag_is_rejected() {
        // 16.4 の authData の先頭 37 バイトだけを残し、AT フラグ (0x40) を落とす。
        // byte string の長さは 164 から 37 (58 25) に変わる。
        const PREFIX_LEN: usize = 28;
        let object = hex(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let mut auth_data = object[PREFIX_LEN + 2..][..37].to_vec();
        auth_data[32] &= !0x40;
        let mut modified = object[..PREFIX_LEN].to_vec();
        modified.extend_from_slice(&[0x58, 0x25]);
        modified.extend_from_slice(&auth_data);
        let challenge = encoded(vectors::FOUR_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = base64url::encode(&modified);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::AttestedCredentialDataMissing);
    }

    #[test]
    fn a_missing_user_presence_flag_is_rejected() {
        // 16.4 の登録の authData から UP フラグ (0x01) を落とす。
        const PREFIX_LEN: usize = 28;
        const AUTH_DATA_LEN: usize = 164;
        let object = hex(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let mut auth_data = object[PREFIX_LEN + 2..][..AUTH_DATA_LEN].to_vec();
        auth_data[32] &= !0x01;
        let mut modified = object[..PREFIX_LEN].to_vec();
        modified.extend_from_slice(&[0x58, AUTH_DATA_LEN as u8]);
        modified.extend_from_slice(&auth_data);
        let challenge = encoded(vectors::FOUR_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = base64url::encode(&modified);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::UserPresentMissing);
    }

    #[test]
    fn extension_data_is_accepted_and_ignored() {
        // 16.4 の authData に ED フラグ (0x80) を立て、空の CBOR map (0xa0) を拡張として足す。
        // 拡張の中身は扱わない。
        const PREFIX_LEN: usize = 28;
        const AUTH_DATA_LEN: usize = 164;
        let object = hex(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let mut auth_data = object[PREFIX_LEN + 2..][..AUTH_DATA_LEN].to_vec();
        auth_data[32] |= 0x80;
        auth_data.push(0xa0);
        let mut modified = object[..PREFIX_LEN].to_vec();
        modified.extend_from_slice(&[0x58, (AUTH_DATA_LEN + 1) as u8]);
        modified.extend_from_slice(&auth_data);
        let challenge = encoded(vectors::FOUR_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = base64url::encode(&modified);
        let registered = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap();
        assert_eq!(registered.credential_id, hex(vectors::FOUR_CREDENTIAL_ID));
        assert_eq!(registered.sign_count, 0);
    }

    #[test]
    fn extension_data_that_is_not_a_cbor_map_is_rejected() {
        // ED フラグが立っていても、拡張が CBOR の map でなければ拒否する。
        const PREFIX_LEN: usize = 28;
        const AUTH_DATA_LEN: usize = 164;
        let object = hex(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let mut auth_data = object[PREFIX_LEN + 2..][..AUTH_DATA_LEN].to_vec();
        auth_data[32] |= 0x80;
        auth_data.push(0x01); // 整数 (map ではない)
        let mut modified = object[..PREFIX_LEN].to_vec();
        modified.extend_from_slice(&[0x58, (AUTH_DATA_LEN + 1) as u8]);
        modified.extend_from_slice(&auth_data);
        let challenge = encoded(vectors::FOUR_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        let attestation_object = base64url::encode(&modified);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::AuthenticatorDataShape);
    }

    #[test]
    fn a_non_canonical_challenge_is_rejected() {
        // base64url の "Zg" と "Zh" は復号すると同じ 1 バイト (0x66) になる。
        // challenge は文字列で照合するため、非正準の符号化は不一致として拒否する
        // (W3C WebAuthn Level 3 の 7.1 と 7.2)。
        let client_data_json =
            format!(r#"{{"type":"webauthn.create","challenge":"Zh","origin":"{ORIGIN}"}}"#);
        let client_data_json = base64url::encode(client_data_json.as_bytes());
        let attestation_object = encoded(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: "Zg",
            client_data_json: &client_data_json,
            attestation_object: &attestation_object,
        })
        .unwrap_err();
        assert_eq!(error, Error::ChallengeMismatch);
    }
}

/// ログインの成功と拒否の経路。
mod authentication {
    use super::*;

    /// 16.4 の登録の attestation object に埋め込まれた COSE の公開鍵。
    fn cose_public_key() -> Vec<u8> {
        embedded_public_key(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT)
    }

    #[test]
    fn a_published_none_es256_vector_authenticates() {
        let challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let authenticator_data = encoded(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        let signature = encoded(vectors::FOUR_AUTHENTICATION_SIGNATURE);
        let sign_count = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data: &authenticator_data,
            signature: &signature,
            cose_public_key: &cose_public_key(),
        })
        .unwrap();
        assert_eq!(sign_count, 0);
    }

    #[test]
    fn a_signature_from_another_vector_is_rejected() {
        // 16.4 の authenticatorData と clientDataJSON に、16.2 の署名を組み合わせると検証に失敗する。
        let challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let authenticator_data = encoded(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        let signature = encoded(vectors::TWO_AUTHENTICATION_SIGNATURE);
        let error = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data: &authenticator_data,
            signature: &signature,
            cose_public_key: &cose_public_key(),
        })
        .unwrap_err();
        assert_eq!(error, Error::SignatureInvalid);
    }

    #[test]
    fn a_missing_user_presence_flag_is_rejected() {
        // 16.4 の authData から UP フラグ (0x01) を落とす。署名は UP の検査より後なので、署名は変えない。
        let mut bytes = hex(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        bytes[32] &= !0x01;
        let authenticator_data = base64url::encode(&bytes);
        let challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let signature = encoded(vectors::FOUR_AUTHENTICATION_SIGNATURE);
        let error = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data: &authenticator_data,
            signature: &signature,
            cose_public_key: &cose_public_key(),
        })
        .unwrap_err();
        assert_eq!(error, Error::UserPresentMissing);
    }

    #[test]
    fn a_missing_user_verification_flag_is_rejected() {
        // 16.2 の認証は UP だけで UV が立っていない (ADR-0004 は UV を要求する)。
        let challenge = encoded(vectors::TWO_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::TWO_AUTHENTICATION_CLIENT_DATA_JSON);
        let authenticator_data = encoded(vectors::TWO_AUTHENTICATION_AUTHENTICATOR_DATA);
        let signature = encoded(vectors::TWO_AUTHENTICATION_SIGNATURE);
        let public_key = hex("a5010203262001215820afefa16f97ca9b2d23eb86ccb64098d20db90856062eb249c33a9b672f26df61225820930a56b87a2fca66334b03458abf879717c12cc68ed73290af2e2664796b9220");
        let error = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data: &authenticator_data,
            signature: &signature,
            cose_public_key: &public_key,
        })
        .unwrap_err();
        assert_eq!(error, Error::UserVerifiedMissing);
    }

    #[test]
    fn a_different_rp_id_is_rejected() {
        let challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let authenticator_data = encoded(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        let signature = encoded(vectors::FOUR_AUTHENTICATION_SIGNATURE);
        let error = verify_authentication(&AuthenticationInput {
            rp_id: "example.com",
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data: &authenticator_data,
            signature: &signature,
            cose_public_key: &cose_public_key(),
        })
        .unwrap_err();
        assert_eq!(error, Error::RpIdHashMismatch);
    }

    #[test]
    fn a_different_origin_is_rejected() {
        // clientDataJSON の origin が期待と一致しないログインは拒否する。
        let challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let authenticator_data = encoded(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        let signature = encoded(vectors::FOUR_AUTHENTICATION_SIGNATURE);
        let error = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: "https://example.com",
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data: &authenticator_data,
            signature: &signature,
            cose_public_key: &cose_public_key(),
        })
        .unwrap_err();
        assert_eq!(error, Error::OriginMismatch);
    }

    #[test]
    fn a_different_challenge_is_rejected() {
        let challenge = encoded(vectors::TWO_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let authenticator_data = encoded(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        let signature = encoded(vectors::FOUR_AUTHENTICATION_SIGNATURE);
        let error = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data: &authenticator_data,
            signature: &signature,
            cose_public_key: &cose_public_key(),
        })
        .unwrap_err();
        assert_eq!(error, Error::ChallengeMismatch);
    }

    #[test]
    fn a_create_client_data_json_is_rejected() {
        let challenge = encoded(vectors::FOUR_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        let authenticator_data = encoded(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        let signature = encoded(vectors::FOUR_AUTHENTICATION_SIGNATURE);
        let error = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data: &authenticator_data,
            signature: &signature,
            cose_public_key: &cose_public_key(),
        })
        .unwrap_err();
        assert_eq!(error, Error::ClientDataTypeMismatch);
    }

    #[test]
    fn the_sign_count_of_the_authenticator_data_is_returned() {
        // 16.4 の署名カウンタを 42 にして、公開されている秘密鍵 (16.4) で署名し直す。
        let mut authenticator_data = hex(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        authenticator_data[33..37].copy_from_slice(&42u32.to_be_bytes());
        let client_data_json = hex(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let signature = sign(&authenticator_data, &client_data_json);
        let challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let encoded_client_data_json = base64url::encode(&client_data_json);
        let encoded_authenticator_data = base64url::encode(&authenticator_data);
        let encoded_signature = base64url::encode(&signature);
        let sign_count = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &encoded_client_data_json,
            authenticator_data: &encoded_authenticator_data,
            signature: &encoded_signature,
            cose_public_key: &cose_public_key(),
        })
        .unwrap();
        assert_eq!(sign_count, 42);
    }
}

/// 署名カウンタの検査 (W3C WebAuthn Level 3 の 7.2 と ADR-0004)。
mod sign_count {
    use super::*;

    #[test]
    fn two_zero_counters_skip_the_check() {
        assert_eq!(check_sign_count(0, 0).unwrap(), SignCounter::Skipped);
    }

    #[test]
    fn a_counter_that_grew_is_accepted() {
        assert_eq!(check_sign_count(5, 6).unwrap(), SignCounter::Updated(6));
        assert_eq!(check_sign_count(0, 1).unwrap(), SignCounter::Updated(1));
        assert_eq!(
            check_sign_count(u32::MAX - 1, u32::MAX).unwrap(),
            SignCounter::Updated(u32::MAX)
        );
    }

    #[test]
    fn a_counter_that_did_not_grow_is_rejected() {
        assert_eq!(check_sign_count(5, 5), Err(Error::SignCounterRegressed));
        assert_eq!(check_sign_count(5, 4), Err(Error::SignCounterRegressed));
        assert_eq!(check_sign_count(1, 0), Err(Error::SignCounterRegressed));
    }
}

/// 壊れた入力の拒否。
mod malformed {
    use super::*;

    /// 登録の検証を、16.4 のチャレンジと clientDataJSON で行う。
    fn verify_broken_registration(attestation_object: &str) -> Error {
        let challenge = encoded(vectors::FOUR_REGISTRATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_REGISTRATION_CLIENT_DATA_JSON);
        verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            attestation_object,
        })
        .unwrap_err()
    }

    /// ログインの検証を、16.4 のチャレンジと clientDataJSON と公開鍵で行う。
    fn verify_broken_authentication(authenticator_data: &str) -> Error {
        let challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let signature = encoded(vectors::FOUR_AUTHENTICATION_SIGNATURE);
        let public_key = hex("a501020326200121582022200a473f90b11078851550d03b4e44a2279f8c4eca27b3153dedfe03e4e97d225820cbd0be95e746ad6f5a8191be11756e4c0420e72f65b466d39bc56b8b123a9c6e");
        verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data,
            signature: &signature,
            cose_public_key: &public_key,
        })
        .unwrap_err()
    }

    #[test]
    fn a_client_data_json_that_is_not_base64url_is_rejected() {
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &encoded(vectors::FOUR_REGISTRATION_CHALLENGE),
            client_data_json: "not base64url!",
            attestation_object: &encoded(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT),
        })
        .unwrap_err();
        assert_eq!(error, Error::Base64Url(base64url::Error::InvalidCharacter));
    }

    #[test]
    fn a_client_data_json_that_is_not_json_is_rejected() {
        let client_data_json = base64url::encode(b"not json");
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &encoded(vectors::FOUR_REGISTRATION_CHALLENGE),
            client_data_json: &client_data_json,
            attestation_object: &encoded(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT),
        })
        .unwrap_err();
        assert_eq!(error, Error::ClientDataJson);
    }

    #[test]
    fn a_client_data_json_without_the_required_field_is_rejected() {
        let client_data_json = base64url::encode(br#"{"type":"webauthn.create","challenge":"AA"}"#);
        let error = verify_registration(&RegistrationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &encoded(vectors::FOUR_REGISTRATION_CHALLENGE),
            client_data_json: &client_data_json,
            attestation_object: &encoded(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT),
        })
        .unwrap_err();
        assert_eq!(error, Error::ClientDataJson);
    }

    #[test]
    fn an_attestation_object_that_is_not_cbor_is_rejected() {
        assert_eq!(
            verify_broken_registration(&base64url::encode(&[0x19, 0x01])),
            Error::Cbor(brew_book_core::cbor::Error::UnexpectedEnd)
        );
    }

    #[test]
    fn an_attestation_object_that_is_not_a_map_is_rejected() {
        assert_eq!(
            verify_broken_registration(&base64url::encode(&[0x00])),
            Error::AttestationObjectShape
        );
    }

    #[test]
    fn an_attestation_object_without_auth_data_is_rejected() {
        // map { "fmt": "none", "attStmt": {} }
        let object = hex("a263666d74646e6f6e656761747453746d74a0");
        assert_eq!(
            verify_broken_registration(&base64url::encode(&object)),
            Error::AttestationObjectShape
        );
    }

    #[test]
    fn an_attestation_object_with_trailing_bytes_is_rejected() {
        let mut object = hex(vectors::FOUR_REGISTRATION_ATTESTATION_OBJECT);
        object.push(0x00);
        assert_eq!(
            verify_broken_registration(&base64url::encode(&object)),
            Error::Cbor(brew_book_core::cbor::Error::TrailingBytes)
        );
    }

    #[test]
    fn an_authenticator_data_that_is_too_short_is_rejected() {
        assert_eq!(
            verify_broken_authentication(&base64url::encode(&[0u8; 36])),
            Error::AuthenticatorDataShape
        );
    }

    #[test]
    fn an_authenticator_data_with_a_credential_id_beyond_the_input_is_rejected() {
        let mut bytes = hex(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        bytes[32] |= 0x40;
        bytes.extend_from_slice(&[0u8; 16]);
        bytes.extend_from_slice(&u16::MAX.to_be_bytes());
        assert_eq!(
            verify_broken_authentication(&base64url::encode(&bytes)),
            Error::AuthenticatorDataShape
        );
    }

    #[test]
    fn an_attested_credential_data_that_is_too_short_is_rejected() {
        // AT フラグを立てたまま、aaguid と credential ID の長さ (18 バイト) に満たない残りにする。
        let mut bytes = hex(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        bytes[32] |= 0x40;
        bytes.extend_from_slice(&[0u8; 5]);
        assert_eq!(
            verify_broken_authentication(&base64url::encode(&bytes)),
            Error::AuthenticatorDataShape
        );
    }

    #[test]
    fn an_authenticator_data_with_extension_bytes_but_no_flag_is_rejected() {
        let mut bytes = hex(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        bytes.push(0x00);
        assert_eq!(
            verify_broken_authentication(&base64url::encode(&bytes)),
            Error::AuthenticatorDataShape
        );
    }

    #[test]
    fn a_signature_that_is_not_der_is_rejected() {
        let challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let authenticator_data = encoded(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        let public_key = hex("a501020326200121582022200a473f90b11078851550d03b4e44a2279f8c4eca27b3153dedfe03e4e97d225820cbd0be95e746ad6f5a8191be11756e4c0420e72f65b466d39bc56b8b123a9c6e");
        let error = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data: &authenticator_data,
            signature: &base64url::encode(&[0x00]),
            cose_public_key: &public_key,
        })
        .unwrap_err();
        assert_eq!(error, Error::SignatureInvalid);
    }

    #[test]
    fn a_stored_public_key_that_is_not_cose_is_rejected() {
        let challenge = encoded(vectors::FOUR_AUTHENTICATION_CHALLENGE);
        let client_data_json = encoded(vectors::FOUR_AUTHENTICATION_CLIENT_DATA_JSON);
        let authenticator_data = encoded(vectors::FOUR_AUTHENTICATION_AUTHENTICATOR_DATA);
        let signature = encoded(vectors::FOUR_AUTHENTICATION_SIGNATURE);
        let error = verify_authentication(&AuthenticationInput {
            rp_id: RP_ID,
            origin: ORIGIN,
            expected_challenge: &challenge,
            client_data_json: &client_data_json,
            authenticator_data: &authenticator_data,
            signature: &signature,
            cose_public_key: &[],
        })
        .unwrap_err();
        assert_eq!(
            error,
            Error::PublicKey(cose::Error::Cbor(
                brew_book_core::cbor::Error::UnexpectedEnd
            ))
        );
    }

    #[test]
    fn the_error_messages_are_english() {
        // 0005 が API の応答に載せるため、メッセージは英語にする。
        for error in [
            Error::ClientDataJson,
            Error::ClientDataTypeMismatch,
            Error::ChallengeMismatch,
            Error::OriginMismatch,
            Error::AttestationObjectShape,
            Error::UnsupportedAttestationFormat,
            Error::AttestationStatementNotEmpty,
            Error::AuthenticatorDataShape,
            Error::RpIdHashMismatch,
            Error::UserPresentMissing,
            Error::UserVerifiedMissing,
            Error::AttestedCredentialDataMissing,
            Error::SignatureInvalid,
            Error::SignCounterRegressed,
            Error::Base64Url(base64url::Error::InvalidCharacter),
            Error::Cbor(brew_book_core::cbor::Error::TooLarge),
            Error::PublicKey(cose::Error::UnsupportedAlgorithm),
        ] {
            let message = error.to_string();
            assert!(
                message.is_ascii() && !message.is_empty(),
                "the message of {error:?} is not an English sentence: {message}"
            );
        }
    }
}
