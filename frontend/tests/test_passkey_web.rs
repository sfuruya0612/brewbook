//! ブラウザで動くパスキーの変換のテスト (0040)。
//!
//! サーバーのオプションから `navigator.credentials` が受け取る辞書を組み立てる部分を検査する。
//! `navigator.credentials` の呼び出し自体は、仮想認証器を使う 0044 の E2E が担う。
//! Flutter の `test/web/passkey_interop_test.dart` の JS との境界の検査を Rust に写す。

#![cfg(target_arch = "wasm32")]

use brew_book_frontend::auth::passkey_web::{
    creation_options_to_js, js_failure, request_options_to_js,
};
use brew_book_frontend::auth::{CreationOptions, PasskeyErrorKind, RequestOptions};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// wasm-bindgen の型を `JsValue` として扱う (Reflect で読むため)。
fn as_value<T: AsRef<JsValue>>(value: &T) -> &JsValue {
    value.as_ref()
}

/// オブジェクトの項目を読む。
fn field(value: &JsValue, name: &str) -> JsValue {
    js_sys::Reflect::get(value, &JsValue::from_str(name))
        .unwrap_or_else(|_| panic!("the {name} field must be readable"))
}

/// Uint8Array の値をバイト列にする。
fn bytes(value: &JsValue) -> Vec<u8> {
    js_sys::Uint8Array::new(value).to_vec()
}

/// 登録のオプションが `navigator.credentials.create` の辞書になることを検査する。
#[wasm_bindgen_test]
fn the_creation_options_become_the_browser_dictionary() {
    let options = CreationOptions {
        challenge: vec![1, 2, 3],
        rp_id: "localhost".to_string(),
        rp_name: "brewbook".to_string(),
        user_id: vec![1],
        user_name: "u1".to_string(),
        user_display_name: "u1".to_string(),
        algorithms: vec![-7],
        timeout: 300000,
        attestation: "none".to_string(),
        resident_key: "preferred".to_string(),
        user_verification: "required".to_string(),
    };

    let creation = creation_options_to_js(&options);
    let public_key = field(as_value(&creation), "publicKey");

    assert_eq!(bytes(&field(&public_key, "challenge")), vec![1, 2, 3]);
    let rp = field(&public_key, "rp");
    assert_eq!(
        field(&rp, "id").as_string().as_deref(),
        Some("localhost"),
        "the relying party id must be set"
    );
    assert_eq!(field(&rp, "name").as_string().as_deref(), Some("brewbook"));
    let user = field(&public_key, "user");
    assert_eq!(bytes(&field(&user, "id")), vec![1]);
    assert_eq!(field(&user, "name").as_string().as_deref(), Some("u1"));
    assert_eq!(
        field(&user, "displayName").as_string().as_deref(),
        Some("u1")
    );
    let algorithms = field(&public_key, "pubKeyCredParams")
        .dyn_into::<js_sys::Array>()
        .expect("pubKeyCredParams must be an array");
    assert_eq!(algorithms.length(), 1);
    let algorithm = algorithms.get(0);
    assert_eq!(field(&algorithm, "alg").as_f64(), Some(-7.0));
    assert_eq!(
        field(&algorithm, "type").as_string().as_deref(),
        Some("public-key")
    );
    assert_eq!(field(&public_key, "timeout").as_f64(), Some(300000.0));
    assert_eq!(
        field(&public_key, "attestation").as_string().as_deref(),
        Some("none")
    );
    let selection = field(&public_key, "authenticatorSelection");
    assert_eq!(
        field(&selection, "residentKey").as_string().as_deref(),
        Some("preferred")
    );
    assert_eq!(
        field(&selection, "userVerification").as_string().as_deref(),
        Some("required")
    );
}

/// ログインのオプションが `navigator.credentials.get` の辞書になることを検査する。
#[wasm_bindgen_test]
fn the_request_options_become_the_browser_dictionary() {
    let options = RequestOptions {
        challenge: vec![1, 2, 3],
        rp_id: "localhost".to_string(),
        user_verification: "required".to_string(),
        allow_credentials: vec![vec![1]],
        timeout: 300000,
    };

    let request = request_options_to_js(&options);
    let public_key = field(as_value(&request), "publicKey");

    assert_eq!(bytes(&field(&public_key, "challenge")), vec![1, 2, 3]);
    assert_eq!(
        field(&public_key, "rpId").as_string().as_deref(),
        Some("localhost")
    );
    assert_eq!(field(&public_key, "timeout").as_f64(), Some(300000.0));
    assert_eq!(
        field(&public_key, "userVerification")
            .as_string()
            .as_deref(),
        Some("required")
    );
    let allowed = field(&public_key, "allowCredentials")
        .dyn_into::<js_sys::Array>()
        .expect("allowCredentials must be an array");
    assert_eq!(allowed.length(), 1);
    let descriptor = allowed.get(0);
    assert_eq!(bytes(&field(&descriptor, "id")), vec![1]);
    assert_eq!(
        field(&descriptor, "type").as_string().as_deref(),
        Some("public-key")
    );
}

/// discoverable なパスキーを対象にするときは、空の `allowCredentials` を渡す (FR-2)。
#[wasm_bindgen_test]
fn an_empty_allow_credentials_stays_an_empty_array() {
    let options = RequestOptions {
        challenge: vec![1],
        rp_id: "localhost".to_string(),
        user_verification: "required".to_string(),
        allow_credentials: Vec::new(),
        timeout: 0,
    };

    let request = request_options_to_js(&options);
    let public_key = field(as_value(&request), "publicKey");
    let allowed = field(&public_key, "allowCredentials")
        .dyn_into::<js_sys::Array>()
        .expect("allowCredentials must be an array");

    assert_eq!(allowed.length(), 0);
}

/// JS の例外の名前から失敗の種類を決めることを検査する (Flutter の
/// `test/web/passkey_interop_test.dart` の `kindOf` の検査を写す。0040 のレビューの指摘)。
#[wasm_bindgen_test]
fn a_js_error_name_decides_the_failure_kind() {
    for (name, expected) in [
        ("NotAllowedError", PasskeyErrorKind::Cancelled),
        ("AbortError", PasskeyErrorKind::Cancelled),
        ("NotSupportedError", PasskeyErrorKind::Unsupported),
        ("InvalidStateError", PasskeyErrorKind::Failed),
    ] {
        let error = js_sys::Object::new();
        js_sys::Reflect::set(&error, &JsValue::from_str("name"), &JsValue::from_str(name))
            .expect("the name must be settable");
        let failure = js_failure(error.into());
        assert_eq!(failure.kind, expected, "{name}");
        assert!(
            !failure.cause.is_empty(),
            "the cause must describe the failure: {name}"
        );
    }
}

/// 空の rpId は設定しない (ブラウザの既定を使う) ことを検査する (0040 のレビューの指摘)。
#[wasm_bindgen_test]
fn an_empty_rp_id_is_not_set() {
    let options = RequestOptions {
        challenge: vec![1],
        rp_id: String::new(),
        user_verification: "required".to_string(),
        allow_credentials: Vec::new(),
        timeout: 0,
    };

    let request = request_options_to_js(&options);
    let public_key = field(as_value(&request), "publicKey");

    assert!(
        field(&public_key, "rpId").is_undefined(),
        "the empty rpId must not be set"
    );
}

/// attestation と userVerification の列挙の写像を検査する (0040 のレビューの指摘)。
#[wasm_bindgen_test]
fn the_enum_values_are_mapped() {
    for (value, expected) in [
        ("none", "none"),
        ("indirect", "indirect"),
        ("direct", "direct"),
        ("enterprise", "enterprise"),
        ("", "none"),
    ] {
        let options = CreationOptions {
            challenge: vec![1],
            rp_id: "localhost".to_string(),
            rp_name: "brewbook".to_string(),
            user_id: vec![1],
            user_name: "u1".to_string(),
            user_display_name: "u1".to_string(),
            algorithms: vec![-7],
            timeout: 0,
            attestation: value.to_string(),
            resident_key: "preferred".to_string(),
            user_verification: "required".to_string(),
        };
        let creation = creation_options_to_js(&options);
        let public_key = field(as_value(&creation), "publicKey");
        assert_eq!(
            field(&public_key, "attestation").as_string().as_deref(),
            Some(expected),
            "attestation = {value:?}"
        );
    }

    for (value, expected) in [
        ("required", "required"),
        ("preferred", "preferred"),
        ("discouraged", "discouraged"),
        ("", "required"),
    ] {
        let options = RequestOptions {
            challenge: vec![1],
            rp_id: "localhost".to_string(),
            user_verification: value.to_string(),
            allow_credentials: Vec::new(),
            timeout: 0,
        };
        let request = request_options_to_js(&options);
        let public_key = field(as_value(&request), "publicKey");
        assert_eq!(
            field(&public_key, "userVerification")
                .as_string()
                .as_deref(),
            Some(expected),
            "userVerification = {value:?}"
        );
    }
}
