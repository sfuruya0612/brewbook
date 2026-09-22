//! 乱数の取得 (ADR-0002)。
//!
//! Web Crypto の `crypto.getRandomValues` を、`worker` が re-export する `js_sys` 経由で呼ぶ。
//! js-sys 0.3.105 は `crypto` の型を持たないため、`globalThis.crypto` と `getRandomValues` を
//! `Reflect` で取り出して呼ぶ。`uuid`、`rand`、`getrandom` のクレートは追加しない。

/// 16 バイトの乱数を返す。Web Crypto を呼べない場合はエラーを返す。
#[cfg(target_arch = "wasm32")]
pub fn bytes_16() -> Result<[u8; 16], String> {
    use worker::js_sys::{Function, JsString, Reflect, Uint8Array};

    let global = worker::js_sys::global();
    let crypto = Reflect::get(&global, &JsString::from("crypto"))
        .map_err(|_| "globalThis.crypto is not available".to_owned())?;
    let get_random_values = Reflect::get(&crypto, &JsString::from("getRandomValues"))
        .map_err(|_| "crypto.getRandomValues is not available".to_owned())?;
    let get_random_values = Function::from(get_random_values);

    let mut buffer = [0_u8; 16];
    let array = Uint8Array::new_with_length(16);
    get_random_values
        .call1(&crypto, &array)
        .map_err(|_| "crypto.getRandomValues failed".to_owned())?;
    array.copy_to(&mut buffer);
    Ok(buffer)
}

/// ネイティブターゲットには Web Crypto が無い。Worker の実行時だけが乱数を取る。
#[cfg(not(target_arch = "wasm32"))]
pub fn bytes_16() -> Result<[u8; 16], String> {
    Err("random bytes are only available in the wasm worker".to_owned())
}
