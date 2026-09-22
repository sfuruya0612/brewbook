//! 乱数の取得 (ADR-0002)。
//!
//! Web Crypto の `crypto.getRandomValues` を、`worker` が re-export する `js_sys` 経由で呼ぶ。
//! js-sys 0.3.105 は `crypto` の型を持たないため、`globalThis.crypto` と `getRandomValues` を
//! `Reflect` で取り出して呼ぶ。`uuid`、`rand`、`getrandom` のクレートは追加しない。

/// 16 バイトの乱数を返す。UUID v4 の組み立てに使う。Web Crypto を呼べない場合はエラーを返す。
#[cfg(target_arch = "wasm32")]
pub fn bytes_16() -> Result<[u8; 16], String> {
    let mut buffer = [0_u8; 16];
    fill(&mut buffer)?;
    Ok(buffer)
}

/// 32 バイトの乱数を返す。登録用トークン、チャレンジ、セッションのトークンに使う (ADR-0004)。
#[cfg(target_arch = "wasm32")]
pub fn bytes_32() -> Result<[u8; 32], String> {
    let mut buffer = [0_u8; 32];
    fill(&mut buffer)?;
    Ok(buffer)
}

/// ネイティブターゲットには Web Crypto が無い。Worker の実行時だけが乱数を取る。
#[cfg(not(target_arch = "wasm32"))]
pub fn bytes_16() -> Result<[u8; 16], String> {
    Err("random bytes are only available in the wasm worker".to_owned())
}

/// ネイティブターゲットには Web Crypto が無い。Worker の実行時だけが乱数を取る。
#[cfg(not(target_arch = "wasm32"))]
pub fn bytes_32() -> Result<[u8; 32], String> {
    Err("random bytes are only available in the wasm worker".to_owned())
}

/// `crypto.getRandomValues` でバッファを埋める。
#[cfg(target_arch = "wasm32")]
fn fill(buffer: &mut [u8]) -> Result<(), String> {
    use worker::js_sys::{Function, JsString, Reflect, Uint8Array};

    let global = worker::js_sys::global();
    let crypto = Reflect::get(&global, &JsString::from("crypto"))
        .map_err(|_| "globalThis.crypto is not available".to_owned())?;
    let get_random_values = Reflect::get(&crypto, &JsString::from("getRandomValues"))
        .map_err(|_| "crypto.getRandomValues is not available".to_owned())?;
    let get_random_values = Function::from(get_random_values);

    let length = u32::try_from(buffer.len())
        .map_err(|_| "the requested random length is too large".to_owned())?;
    let array = Uint8Array::new_with_length(length);
    get_random_values
        .call1(&crypto, &array)
        .map_err(|_| "crypto.getRandomValues failed".to_owned())?;
    array.copy_to(buffer);
    Ok(())
}
