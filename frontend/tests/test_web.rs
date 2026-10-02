//! ブラウザで動かすテスト (dioxus:test-web) の配線を検査する (0037)。
//!
//! wasm-bindgen-test を headless の Chrome (mise の chromedriver) で実行できることを確認する。
//! ブラウザの API を使う検証は 0038 以降で足す。

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// テストのバイナリが wasm としてブラウザで実行されることを検査する。
#[wasm_bindgen_test]
fn test_binary_runs_on_wasm() {
    assert_eq!(std::env::consts::ARCH, "wasm32");
}
