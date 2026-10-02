//! web-sys の `navigator.credentials` でパスキーを操作する実装 (FR-1、FR-2、ADR-0004)。
//!
//! `PublicKeyCredential` と `CredentialsContainer` は web-sys の型を使う。オプションの辞書は
//! web-sys の型で組み立てる (0040 の設計判断。足りない項目があれば js-sys の `Reflect` を使う)。
//! `serde-wasm-bindgen` は使わず、依存を増やさない (ADR-0001 の規約)。

use serde_json::{Map, Value};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AttestationConveyancePreference, AuthenticatorAssertionResponse,
    AuthenticatorAttestationResponse, AuthenticatorSelectionCriteria, CredentialCreationOptions,
    CredentialRequestOptions, CredentialsContainer, PublicKeyCredential,
    PublicKeyCredentialCreationOptions, PublicKeyCredentialDescriptor,
    PublicKeyCredentialParameters, PublicKeyCredentialRequestOptions, PublicKeyCredentialRpEntity,
    PublicKeyCredentialType, PublicKeyCredentialUserEntity, UserVerificationRequirement,
};

use super::base64url;
use super::passkey::{CreationOptions, PasskeyClient, PasskeyError, PasskeyFuture, RequestOptions};

/// web-sys の `navigator.credentials` を使うパスキーの実装。
pub struct WebPasskeyClient;

impl PasskeyClient for WebPasskeyClient {
    fn create_credential(&self, options: CreationOptions) -> PasskeyFuture<Map<String, Value>> {
        Box::pin(create_credential(options))
    }

    fn get_credential(&self, options: RequestOptions) -> PasskeyFuture<Map<String, Value>> {
        Box::pin(get_credential(options))
    }
}

/// バイト列を JS の `Uint8Array` に写す (コピー)。
///
/// web-sys の `*_with_u8_slice` は wasm のメモリへのビューを JS に渡すため、辞書に保持
/// させると、その後の確保で中身が変わる (0040 のレビューで、テストが実際に壊れることを
/// 検出した)。コピーを作る `Uint8Array::from` と `*_with_u8_array` を使う。
fn bytes_to_js(value: &[u8]) -> js_sys::Uint8Array {
    js_sys::Uint8Array::from(value)
}

/// 登録のオプションを `navigator.credentials.create` が受け取る辞書にする。
///
/// チャレンジと利用者の ID はバイト列のまま渡す (`_u8_slice` の組み立てを使う)。
/// テストから辞書の中身を確認できるように公開する。
pub fn creation_options_to_js(options: &CreationOptions) -> CredentialCreationOptions {
    let challenge = bytes_to_js(&options.challenge);
    let user_id = bytes_to_js(&options.user_id);
    let rp = PublicKeyCredentialRpEntity::new(&options.rp_name);
    if !options.rp_id.is_empty() {
        rp.set_id(&options.rp_id);
    }
    let user = PublicKeyCredentialUserEntity::new_with_u8_array(
        &options.user_name,
        &options.user_display_name,
        &user_id,
    );
    let algorithms = js_sys::Array::new();
    for algorithm in &options.algorithms {
        algorithms.push(
            PublicKeyCredentialParameters::new(*algorithm, PublicKeyCredentialType::PublicKey)
                .as_ref(),
        );
    }
    let public_key = PublicKeyCredentialCreationOptions::new_with_u8_array(
        &challenge,
        algorithms.as_ref(),
        &rp,
        &user,
    );
    public_key.set_timeout(options.timeout);
    public_key.set_attestation(attestation_of(&options.attestation));
    let selection = AuthenticatorSelectionCriteria::new();
    selection.set_resident_key(&options.resident_key);
    selection.set_user_verification(user_verification_of(&options.user_verification));
    public_key.set_authenticator_selection(&selection);
    let creation = CredentialCreationOptions::new();
    creation.set_public_key(&public_key);
    creation
}

/// ログインの要求のオプションを `navigator.credentials.get` が受け取る辞書にする。
pub fn request_options_to_js(options: &RequestOptions) -> CredentialRequestOptions {
    let challenge = bytes_to_js(&options.challenge);
    let public_key = PublicKeyCredentialRequestOptions::new_with_u8_array(&challenge);
    // rp_id が空のときは設定しない (ブラウザの既定を使う。登録の辞書と同じ扱い。
    // 0040 のレビューの指摘)。
    if !options.rp_id.is_empty() {
        public_key.set_rp_id(&options.rp_id);
    }
    public_key.set_timeout(options.timeout);
    public_key.set_user_verification(user_verification_of(&options.user_verification));
    let descriptors = js_sys::Array::new();
    for id in &options.allow_credentials {
        let id = bytes_to_js(id);
        descriptors.push(
            PublicKeyCredentialDescriptor::new_with_u8_array(
                &id,
                PublicKeyCredentialType::PublicKey,
            )
            .as_ref(),
        );
    }
    public_key.set_allow_credentials(descriptors.as_ref());
    let request = CredentialRequestOptions::new();
    request.set_public_key(&public_key);
    request
}

/// `navigator.credentials.create` でパスキーを作り、サーバーに返す JSON にする。
async fn create_credential(options: CreationOptions) -> Result<Map<String, Value>, PasskeyError> {
    let js_options = creation_options_to_js(&options);
    let promise = credentials()?
        .create_with_options(&js_options)
        .map_err(js_failure)?;
    let value = JsFuture::from(promise).await.map_err(js_failure)?;
    if value.is_null() || value.is_undefined() {
        // 利用者が選択を取り消すと null が返る (W3C WebAuthn Level 3 の 5.1.4)。
        return Err(PasskeyError::cancelled("the credential is null"));
    }
    let credential: PublicKeyCredential = value
        .dyn_into()
        .map_err(|_| PasskeyError::failed("the result is not a PublicKeyCredential"))?;
    let response: AuthenticatorAttestationResponse = credential
        .response()
        .dyn_into()
        .map_err(|_| PasskeyError::failed("the response is not an attestation response"))?;

    let mut result = Map::new();
    result.insert("id".to_string(), Value::String(credential.id()));
    result.insert("type".to_string(), Value::String(credential.type_()));
    let mut body = Map::new();
    body.insert(
        "clientDataJSON".to_string(),
        Value::String(base64url::encode(&bytes(&response.client_data_json()))),
    );
    body.insert(
        "attestationObject".to_string(),
        Value::String(base64url::encode(&bytes(&response.attestation_object()))),
    );
    result.insert("response".to_string(), Value::Object(body));
    Ok(result)
}

/// `navigator.credentials.get` でパスキーを使い、サーバーに返す JSON にする。
async fn get_credential(options: RequestOptions) -> Result<Map<String, Value>, PasskeyError> {
    let js_options = request_options_to_js(&options);
    let promise = credentials()?
        .get_with_options(&js_options)
        .map_err(js_failure)?;
    let value = JsFuture::from(promise).await.map_err(js_failure)?;
    if value.is_null() || value.is_undefined() {
        // 利用者が選択を取り消すと null が返る (W3C WebAuthn Level 3 の 5.1.4)。
        return Err(PasskeyError::cancelled("the credential is null"));
    }
    let credential: PublicKeyCredential = value
        .dyn_into()
        .map_err(|_| PasskeyError::failed("the result is not a PublicKeyCredential"))?;
    let response: AuthenticatorAssertionResponse = credential
        .response()
        .dyn_into()
        .map_err(|_| PasskeyError::failed("the response is not an assertion response"))?;

    let mut result = Map::new();
    // サーバーは保存済みのクレデンシャル ID と照合する (base64url。Flutter と同じ)。
    result.insert(
        "id".to_string(),
        Value::String(base64url::encode(&bytes(&credential.raw_id()))),
    );
    result.insert("type".to_string(), Value::String(credential.type_()));
    let mut body = Map::new();
    body.insert(
        "clientDataJSON".to_string(),
        Value::String(base64url::encode(&bytes(&response.client_data_json()))),
    );
    body.insert(
        "authenticatorData".to_string(),
        Value::String(base64url::encode(&bytes(&response.authenticator_data()))),
    );
    body.insert(
        "signature".to_string(),
        Value::String(base64url::encode(&bytes(&response.signature()))),
    );
    result.insert("response".to_string(), Value::Object(body));
    Ok(result)
}

/// `navigator.credentials` を返す。
fn credentials() -> Result<CredentialsContainer, PasskeyError> {
    web_sys::window()
        .map(|window| window.navigator().credentials())
        .ok_or_else(|| PasskeyError::unsupported("window is not available"))
}

/// JS の例外を失敗の種類と原因にする。
///
/// 例外の名前 (`name`) から種類を決める (Flutter の `WebPasskeyClient.kindOf` と同じ)。
/// テストから呼べるように公開する (wasm のテストで JS のオブジェクトを渡して検査する。
/// 0040 のレビューの指摘)。
pub fn js_failure(error: wasm_bindgen::JsValue) -> PasskeyError {
    let name = js_sys::Reflect::get(&error, &wasm_bindgen::JsValue::from_str("name"))
        .ok()
        .and_then(|value| value.as_string());
    let cause = match error.as_string() {
        Some(message) => message,
        None => format!("{error:?}"),
    };
    PasskeyError::new(PasskeyError::kind_of_error_name(name.as_deref()), cause)
}

/// ArrayBuffer をバイト列にする。
fn bytes(buffer: &js_sys::ArrayBuffer) -> Vec<u8> {
    js_sys::Uint8Array::new(buffer).to_vec()
}

/// attestation の伝達の希望を web-sys の型にする。既定は `none`。
fn attestation_of(value: &str) -> AttestationConveyancePreference {
    match value {
        "indirect" => AttestationConveyancePreference::Indirect,
        "direct" => AttestationConveyancePreference::Direct,
        "enterprise" => AttestationConveyancePreference::Enterprise,
        _ => AttestationConveyancePreference::None,
    }
}

/// 利用者の確認の要求を web-sys の型にする。既定は `required`。
fn user_verification_of(value: &str) -> UserVerificationRequirement {
    match value {
        "preferred" => UserVerificationRequirement::Preferred,
        "discouraged" => UserVerificationRequirement::Discouraged,
        _ => UserVerificationRequirement::Required,
    }
}
