//! ブラウザで動かすテスト (frontend:test-web) の配線と、ブラウザの API を使うコードを検査する (0037、0038)。
//!
//! wasm-bindgen-test を headless の Chrome (mise の chromedriver) で実行する。

#![cfg(target_arch = "wasm32")]

use brew_book_frontend::i18n::{browser_language, resolve_language, Language};
use brew_book_frontend::records::clock::{Clock, DeviceClock};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// テストのバイナリが wasm としてブラウザで実行されることを検査する。
#[wasm_bindgen_test]
fn test_binary_runs_on_wasm() {
    assert_eq!(std::env::consts::ARCH, "wasm32");
}

/// ブラウザの言語が読めることを検査する (FR-16)。読めない場合はここで失敗させる。
#[wasm_bindgen_test]
fn the_browser_language_is_readable() {
    let tag = browser_language();
    assert!(tag.is_some(), "the browser must expose navigator.language");
    let language = resolve_language(tag.as_deref());
    assert!(matches!(language, Language::Japanese | Language::English));
}

/// 端末の時計が妥当なローカル日時と UTC オフセットを返すことを検査する (FR-9、FR-18)。
#[wasm_bindgen_test]
fn the_device_clock_returns_a_plausible_local_time() {
    let clock = DeviceClock;
    let now = clock.now();
    assert!(now.year >= 2026, "the clock must be after the release");
    assert!(now.year <= 9999, "the year must be in the value's range");
    assert!((1..=12).contains(&now.month), "the month must be 1..=12");
    assert!((1..=31).contains(&now.day), "the day must be 1..=31");
    assert!(now.hour < 24, "the hour must be 0..=23");
    assert!(now.minute < 60, "the minute must be 0..=59");
    let offset = clock.utc_offset_minutes();
    assert!(
        (-840..=840).contains(&offset),
        "the UTC offset must be in the API's range: {offset}"
    );
}
