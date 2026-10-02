//! brewbook の Frontend (ADR-0017)。
//!
//! 画面の基盤 (経路の台帳、API クライアント、値の変換、翻訳、起動時のセッション確認) を持つ。
//! デザインシステムは 0039、認証の画面は 0040、記録の画面は 0041、統計は 0042、設定は 0043 が
//! 入れた。
//!
//! コメントの「Flutter の `frontend/lib/...`」は、移行前の Flutter 版の実装を指す (0045 で
//! 削除した)。同じ動きにする意図と、移行の判断の記録として残している。

pub mod api;
pub mod app;
pub mod auth;
pub mod i18n;
pub mod records;
pub mod router;
pub mod screens;
pub mod settings;
pub mod ui;

/// E2E のビルドであることの印 (0044)。
///
/// `frontend:test-same-origin` のハーネスが、配信するビルドがテスト用の feature `e2e` を
/// 有効にしたものであることを、ビルドの成果物と実行中の両方で確認するために使う。
pub const E2E_MARKER: &str = "brewbook-e2e-build";

/// E2E のビルドであることの印を `<html>` の `data-e2e` 属性に付ける (0044)。
///
/// feature `e2e` を有効にした wasm のビルドだけがこの関数を持ち、`main.rs` が起動の前に呼ぶ。
#[cfg(all(feature = "e2e", target_arch = "wasm32"))]
pub fn mark_e2e_build() {
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
    {
        let _ = element.set_attribute("data-e2e", E2E_MARKER);
    }
}
