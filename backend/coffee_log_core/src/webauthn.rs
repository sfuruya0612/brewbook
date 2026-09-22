//! WebAuthn の登録とログインの検証 (ADR-0004)。
//!
//! 検証は、時刻、乱数、D1 に依存しない純粋な関数として実装する。
//! 対応する範囲は ES256 (COSE alg -7) と attestation none だけで、次を検証する。
//!
//! - `clientDataJSON` の type (登録は `webauthn.create`、ログインは `webauthn.get`)、
//!   challenge、origin (W3C WebAuthn Level 3 の 5.8.1 と 7.1 と 7.2)
//! - `authenticatorData` の rpIdHash とフラグ (UP と UV、登録時は AT) (6.1)
//! - attestation object の CBOR デコードと `fmt: none` の受け入れ (8.7)
//! - COSE の鍵 (EC2、P-256) の取り出し (RFC 9052)
//! - ES256 の署名検証。署名の対象は authenticatorData と clientDataJSON の SHA-256 (7.2)
//!
//! 署名カウンタは [`check_sign_count`] に分け、保存値の更新は呼び出し側 (0005) が行う
//! (W3C WebAuthn Level 3 の 7.2 と ADR-0004)。
//!
//! 参照する仕様: W3C WebAuthn Level 3 (<https://www.w3.org/TR/webauthn-3/>)。

use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::base64url;
use crate::cbor;
use crate::cose;

/// 登録の `clientDataJSON` の type (W3C WebAuthn Level 3 の 5.8.1)。
pub const REGISTRATION_TYPE: &str = "webauthn.create";

/// ログインの `clientDataJSON` の type (W3C WebAuthn Level 3 の 5.8.1)。
pub const AUTHENTICATION_TYPE: &str = "webauthn.get";

/// authenticatorData のフラグのビット (W3C WebAuthn Level 3 の 6.1)。
const FLAG_USER_PRESENT: u8 = 0x01;
const FLAG_USER_VERIFIED: u8 = 0x04;
const FLAG_ATTESTED_CREDENTIAL_DATA: u8 = 0x40;
const FLAG_EXTENSION_DATA: u8 = 0x80;

/// authenticatorData の先頭の固定長 (rpIdHash 32 バイト、フラグ 1 バイト、署名カウンタ 4 バイト)。
const AUTHENTICATOR_DATA_HEADER_LEN: usize = 37;

/// attested credential data の先頭の aaguid の長さ (W3C WebAuthn Level 3 の 6.1)。
const AAGUID_LEN: usize = 16;

/// 検証の失敗。呼び出し側が拒否の理由を区別できるようにする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// 入力の base64url が復号できない。
    Base64Url(base64url::Error),
    /// `clientDataJSON` が JSON として読めない、または type と challenge と origin が無い。
    ClientDataJson,
    /// `clientDataJSON` の type が検証の種別と一致しない。
    ClientDataTypeMismatch,
    /// `clientDataJSON` の challenge が期待した値と一致しない。
    ChallengeMismatch,
    /// `clientDataJSON` の origin が期待した値と一致しない。
    OriginMismatch,
    /// CBOR がデコードできない。
    Cbor(cbor::Error),
    /// attestation object が map ではない、または fmt と attStmt と authData が揃っていない。
    AttestationObjectShape,
    /// attestation の形式が none ではない (ADR-0004 は none だけを受け付ける)。
    UnsupportedAttestationFormat,
    /// `fmt: none` なのに attestation statement が空の map ではない (W3C WebAuthn Level 3 の 8.7)。
    AttestationStatementNotEmpty,
    /// authenticatorData の長さか構造が不正である。
    AuthenticatorDataShape,
    /// rpIdHash が RP ID の SHA-256 と一致しない。
    RpIdHashMismatch,
    /// UP フラグが立っていない。
    UserPresentMissing,
    /// UV フラグが立っていない (ADR-0004 は `userVerification: required` を要求する)。
    UserVerifiedMissing,
    /// 登録なのに attested credential data (AT フラグ) が無い。
    AttestedCredentialDataMissing,
    /// 認証器の公開鍵が EC2 の P-256 ではない (ADR-0004 は ES256 だけを受け付ける)。
    PublicKey(cose::Error),
    /// 署名が検証できない。
    SignatureInvalid,
    /// 署名カウンタが保存値以下に後退している (ADR-0004)。
    SignCounterRegressed,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Base64Url(error) => {
                write!(formatter, "the input is not valid base64url: {error}")
            }
            Error::ClientDataJson => write!(
                formatter,
                "clientDataJSON is not valid JSON with a type, a challenge and an origin"
            ),
            Error::ClientDataTypeMismatch => {
                write!(
                    formatter,
                    "the clientDataJSON type does not match the ceremony"
                )
            }
            Error::ChallengeMismatch => write!(
                formatter,
                "the clientDataJSON challenge does not match the expected challenge"
            ),
            Error::OriginMismatch => write!(
                formatter,
                "the clientDataJSON origin does not match the expected origin"
            ),
            Error::Cbor(error) => write!(formatter, "a CBOR structure is not valid: {error}"),
            Error::AttestationObjectShape => write!(
                formatter,
                "the attestation object is not a map with fmt, attStmt and authData"
            ),
            Error::UnsupportedAttestationFormat => {
                write!(formatter, "the attestation format is not none")
            }
            Error::AttestationStatementNotEmpty => {
                write!(
                    formatter,
                    "the none attestation statement is not an empty map"
                )
            }
            Error::AuthenticatorDataShape => {
                write!(formatter, "the authenticatorData has an invalid structure")
            }
            Error::RpIdHashMismatch => {
                write!(
                    formatter,
                    "the authenticatorData rpIdHash does not match the RP ID"
                )
            }
            Error::UserPresentMissing => write!(formatter, "the user present flag is not set"),
            Error::UserVerifiedMissing => write!(formatter, "the user verified flag is not set"),
            Error::AttestedCredentialDataMissing => {
                write!(formatter, "the attested credential data flag is not set")
            }
            Error::PublicKey(error) => write!(
                formatter,
                "the credential public key is not an ES256 EC2 P-256 key: {error}"
            ),
            Error::SignatureInvalid => {
                write!(
                    formatter,
                    "the signature is not valid for the credential public key"
                )
            }
            Error::SignCounterRegressed => {
                write!(formatter, "the signature counter did not increase")
            }
        }
    }
}

impl std::error::Error for Error {}

/// 登録の検証の入力。base64url の文字列は API の入力 JSON と同じ形で受け取る。
#[derive(Debug, Clone, Copy)]
pub struct RegistrationInput<'a> {
    /// Relying Party ID (例: `coffee-log.example.workers.dev`)。
    pub rp_id: &'a str,
    /// RP の Origin (例: `https://coffee-log.example.workers.dev`)。
    pub origin: &'a str,
    /// 期待するチャレンジ (base64url)。0005 が発行して D1 に保存した値。
    pub expected_challenge: &'a str,
    /// `clientDataJSON` (base64url)。
    pub client_data_json: &'a str,
    /// attestation object (base64url)。
    pub attestation_object: &'a str,
}

/// 登録で得られるクレデンシャル。0005 はこれを D1 に保存する (ADR-0006)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredCredential {
    /// authenticatorData の credential ID。ログインの credential の id と照合する。
    pub credential_id: Vec<u8>,
    /// COSE の公開鍵の CBOR。`passkey_credentials` の「公開鍵 (COSE)」として保存する。
    pub cose_public_key: Vec<u8>,
    /// authenticatorData の署名カウンタ。
    pub sign_count: u32,
}

/// ログインの検証の入力。base64url の文字列は API の入力 JSON と同じ形で受け取る。
#[derive(Debug, Clone, Copy)]
pub struct AuthenticationInput<'a> {
    /// Relying Party ID (例: `coffee-log.example.workers.dev`)。
    pub rp_id: &'a str,
    /// RP の Origin (例: `https://coffee-log.example.workers.dev`)。
    pub origin: &'a str,
    /// 期待するチャレンジ (base64url)。0005 が発行して D1 に保存した値。
    pub expected_challenge: &'a str,
    /// `clientDataJSON` (base64url)。
    pub client_data_json: &'a str,
    /// `authenticatorData` (base64url)。
    pub authenticator_data: &'a str,
    /// 署名 (base64url)。ES256 の ASN.1 DER 形式。
    pub signature: &'a str,
    /// D1 に保存した COSE の公開鍵 (ADR-0006)。
    pub cose_public_key: &'a [u8],
}

/// 登録を検証し、保存するクレデンシャルを返す (W3C WebAuthn Level 3 の 7.1)。
pub fn verify_registration(input: &RegistrationInput<'_>) -> Result<RegisteredCredential, Error> {
    // clientDataJSON は base64url で運ばれる。復号したバイト列は ClientData が借用する。
    let client_data_json = base64url::decode(input.client_data_json).map_err(Error::Base64Url)?;
    let client_data = parse_client_data(&client_data_json)?;
    // 手順 7 から 9: type と challenge。challenge は base64url の文字列で照合する。
    if client_data.type_ != REGISTRATION_TYPE {
        return Err(Error::ClientDataTypeMismatch);
    }
    check_challenge(input.expected_challenge, client_data.challenge)?;
    // 手順 10 と 11: origin。同一オリジンで配信するため、期待する origin と完全に一致することを求める。
    if client_data.origin != input.origin {
        return Err(Error::OriginMismatch);
    }

    // 手順 12: attestation object の CBOR デコード。
    let encoded_attestation =
        base64url::decode(input.attestation_object).map_err(Error::Base64Url)?;
    let attestation = cbor::decode(&encoded_attestation).map_err(Error::Cbor)?;
    if !matches!(attestation, cbor::Value::Map(_)) {
        return Err(Error::AttestationObjectShape);
    }
    let format = attestation
        .map_get_text("fmt")
        .and_then(cbor::Value::as_text)
        .ok_or(Error::AttestationObjectShape)?;
    let statement = attestation
        .map_get_text("attStmt")
        .and_then(cbor::Value::as_map)
        .ok_or(Error::AttestationObjectShape)?;
    let encoded_authenticator_data = attestation
        .map_get_text("authData")
        .and_then(cbor::Value::as_bytes)
        .ok_or(Error::AttestationObjectShape)?;
    // 手順 14 から 16: `fmt: none` を受け入れ、attStmt は空の map であることを求める (8.7)。
    if format != "none" {
        return Err(Error::UnsupportedAttestationFormat);
    }
    if !statement.is_empty() {
        return Err(Error::AttestationStatementNotEmpty);
    }

    let authenticator_data = parse_authenticator_data(encoded_authenticator_data)?;
    // 手順 15: rpIdHash。手順 16 と 17: UP と UV。手順 18: AT は登録で必要である。
    check_rp_id_hash(authenticator_data.rp_id_hash, input.rp_id)?;
    check_user_flags(authenticator_data.flags)?;
    let attested = authenticator_data
        .attested_credential_data
        .ok_or(Error::AttestedCredentialDataMissing)?;
    Ok(RegisteredCredential {
        credential_id: attested.credential_id.to_vec(),
        cose_public_key: attested.cose_public_key.to_vec(),
        sign_count: authenticator_data.sign_count,
    })
}

/// ログインを検証し、authenticatorData の署名カウンタを返す (W3C WebAuthn Level 3 の 7.2)。
///
/// 署名カウンタの検査と保存値の更新は呼び出し側が [`check_sign_count`] で行う (ADR-0004)。
pub fn verify_authentication(input: &AuthenticationInput<'_>) -> Result<u32, Error> {
    // clientDataJSON は base64url で運ばれる。復号したバイト列は ClientData が借用する。
    let client_data_json = base64url::decode(input.client_data_json).map_err(Error::Base64Url)?;
    let client_data = parse_client_data(&client_data_json)?;
    // 手順 13 と 14: type と challenge。
    if client_data.type_ != AUTHENTICATION_TYPE {
        return Err(Error::ClientDataTypeMismatch);
    }
    check_challenge(input.expected_challenge, client_data.challenge)?;
    // 手順 15: origin。
    if client_data.origin != input.origin {
        return Err(Error::OriginMismatch);
    }

    let encoded_authenticator_data =
        base64url::decode(input.authenticator_data).map_err(Error::Base64Url)?;
    let authenticator_data = parse_authenticator_data(&encoded_authenticator_data)?;
    // 手順 16 から 19: rpIdHash と UP と UV。
    check_rp_id_hash(authenticator_data.rp_id_hash, input.rp_id)?;
    check_user_flags(authenticator_data.flags)?;

    // 手順 20 から 22: 署名の検証。
    let signature = base64url::decode(input.signature).map_err(Error::Base64Url)?;
    verify_signature(
        input.cose_public_key,
        &encoded_authenticator_data,
        &client_data_json,
        &signature,
    )?;
    Ok(authenticator_data.sign_count)
}

/// 署名カウンタの検査の結果 (W3C WebAuthn Level 3 の 7.2 と ADR-0004)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignCounter {
    /// 保存値と今回の値が両方 0 なので検査を省略する。
    Skipped,
    /// 今回の値が保存値より大きい。呼び出し側が保存値をこの値に更新する。
    Updated(u32),
}

/// 署名カウンタを検査する (W3C WebAuthn Level 3 の 7.2 と ADR-0004)。
///
/// 保存値と今回の値が両方 0 なら検査を省略し、今回の値が保存値以下なら拒否し、
/// 大きければ受け入れて更新後の値を返す。保存値の更新は呼び出し側 (0005) が行う。
pub fn check_sign_count(stored: u32, current: u32) -> Result<SignCounter, Error> {
    if stored == 0 && current == 0 {
        return Ok(SignCounter::Skipped);
    }
    if current <= stored {
        return Err(Error::SignCounterRegressed);
    }
    Ok(SignCounter::Updated(current))
}

/// `clientDataJSON` のうち検証に使うフィールド (W3C WebAuthn Level 3 の 5.8.1)。
///
/// 仕様は将来のフィールドの追加を許すため、未知のフィールドは無視する。
#[derive(Debug, Deserialize)]
struct ClientData<'a> {
    #[serde(rename = "type")]
    type_: &'a str,
    challenge: &'a str,
    origin: &'a str,
}

/// base64url の `clientDataJSON` を復号して JSON として読む。
/// 借用した `ClientData` を返すため、復号したバイト列は呼び出し側が保持する。
fn parse_client_data(input: &[u8]) -> Result<ClientData<'_>, Error> {
    serde_json::from_slice(input).map_err(|_| Error::ClientDataJson)
}

/// 期待するチャレンジと `clientDataJSON` のチャレンジを、base64url の文字列で照合する。
/// W3C WebAuthn Level 3 の 7.1 と 7.2 は、challenge が期待する値の base64url 符号化と
/// 等しいことを求める。非正準の符号化 (未使用ビットがゼロでない) も不一致として拒否する。
fn check_challenge(expected: &str, actual: &str) -> Result<(), Error> {
    if expected != actual {
        return Err(Error::ChallengeMismatch);
    }
    Ok(())
}

/// rpIdHash が RP ID の SHA-256 であることを確かめる (W3C WebAuthn Level 3 の 6.1)。
fn check_rp_id_hash(rp_id_hash: &[u8], rp_id: &str) -> Result<(), Error> {
    if rp_id_hash != Sha256::digest(rp_id.as_bytes()).as_slice() {
        return Err(Error::RpIdHashMismatch);
    }
    Ok(())
}

/// UP と UV のフラグが立っていることを確かめる (W3C WebAuthn Level 3 の 6.1)。
fn check_user_flags(flags: u8) -> Result<(), Error> {
    if flags & FLAG_USER_PRESENT == 0 {
        return Err(Error::UserPresentMissing);
    }
    if flags & FLAG_USER_VERIFIED == 0 {
        return Err(Error::UserVerifiedMissing);
    }
    Ok(())
}

/// authenticatorData の構造 (W3C WebAuthn Level 3 の 6.1)。
struct AuthenticatorData<'a> {
    rp_id_hash: &'a [u8],
    flags: u8,
    sign_count: u32,
    /// AT フラグが立っているときだけ存在する。
    attested_credential_data: Option<AttestedCredentialData<'a>>,
}

/// authenticatorData の attested credential data (W3C WebAuthn Level 3 の 6.1)。
struct AttestedCredentialData<'a> {
    credential_id: &'a [u8],
    /// authData に埋め込まれた COSE の公開鍵の CBOR。
    cose_public_key: &'a [u8],
}

/// authenticatorData を解析する。
///
/// 先頭は rpIdHash 32 バイト、フラグ 1 バイト、署名カウンタ 4 バイトで、
/// AT フラグが立っていれば attested credential data が続き、ED フラグが立っていれば拡張が続く。
/// 拡張は扱わないため、ED フラグが立っていないのに残りがある入力は拒否する。
fn parse_authenticator_data(input: &[u8]) -> Result<AuthenticatorData<'_>, Error> {
    if input.len() < AUTHENTICATOR_DATA_HEADER_LEN {
        return Err(Error::AuthenticatorDataShape);
    }
    let rp_id_hash = &input[..32];
    let flags = input[32];
    let sign_count = u32::from_be_bytes([input[33], input[34], input[35], input[36]]);
    let rest = &input[AUTHENTICATOR_DATA_HEADER_LEN..];
    let (attested_credential_data, trailing) = if flags & FLAG_ATTESTED_CREDENTIAL_DATA != 0 {
        let (attested, trailing) = parse_attested_credential_data(rest)?;
        (Some(attested), trailing)
    } else {
        (None, rest)
    };
    if !trailing.is_empty() {
        if flags & FLAG_EXTENSION_DATA == 0 {
            return Err(Error::AuthenticatorDataShape);
        }
        // 拡張の中身は扱わないが、CBOR の map であることだけは確認する
        // (W3C WebAuthn Level 3 の 6.1 と 7.1)。
        let extensions = cbor::decode(trailing).map_err(|_| Error::AuthenticatorDataShape)?;
        if extensions.as_map().is_none() {
            return Err(Error::AuthenticatorDataShape);
        }
    }
    Ok(AuthenticatorData {
        rp_id_hash,
        flags,
        sign_count,
        attested_credential_data,
    })
}

/// attested credential data を解析し、COSE の公開鍵が ES256 (EC2、P-256) であることを確かめる。
fn parse_attested_credential_data(
    input: &[u8],
) -> Result<(AttestedCredentialData<'_>, &[u8]), Error> {
    let header = input
        .get(..AAGUID_LEN + 2)
        .ok_or(Error::AuthenticatorDataShape)?;
    let credential_id_len = usize::from(u16::from_be_bytes([
        header[AAGUID_LEN],
        header[AAGUID_LEN + 1],
    ]));
    let key_offset = AAGUID_LEN + 2 + credential_id_len;
    let key_and_trailing = input
        .get(key_offset..)
        .ok_or(Error::AuthenticatorDataShape)?;
    let credential_id = &input[AAGUID_LEN + 2..key_offset];
    let (key, consumed) = cbor::decode_prefix(key_and_trailing).map_err(Error::Cbor)?;
    // ES256 (EC2、P-256) 以外の公開鍵を受け付けない (ADR-0004)。
    cose::from_value(&key).map_err(Error::PublicKey)?;
    Ok((
        AttestedCredentialData {
            credential_id,
            cose_public_key: &key_and_trailing[..consumed],
        },
        &key_and_trailing[consumed..],
    ))
}

/// ES256 の署名を検証する (W3C WebAuthn Level 3 の 7.2)。
///
/// 署名の対象は authenticatorData と clientDataJSON の SHA-256 の連結である。
fn verify_signature(
    cose_public_key: &[u8],
    authenticator_data: &[u8],
    client_data_json: &[u8],
    signature: &[u8],
) -> Result<(), Error> {
    let public_key = cose::parse(cose_public_key).map_err(Error::PublicKey)?;
    let verifying_key = VerifyingKey::from_sec1_bytes(&public_key.to_sec1_bytes())
        .map_err(|_| Error::SignatureInvalid)?;
    let signature = Signature::from_der(signature).map_err(|_| Error::SignatureInvalid)?;
    let client_data_hash = Sha256::digest(client_data_json);
    let mut signed_data = Vec::with_capacity(authenticator_data.len() + client_data_hash.len());
    signed_data.extend_from_slice(authenticator_data);
    signed_data.extend_from_slice(&client_data_hash);
    verifying_key
        .verify(&signed_data, &signature)
        .map_err(|_| Error::SignatureInvalid)
}
