//! brewbook の Frontend の起動 (ADR-0017)。
//!
//! 画面の基盤は `brew_book_frontend` のクレートに置き、このファイルは Web の起動だけを行う。

/// Web の起動。ブラウザでアプリを立ち上げる。
#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus::launch(brew_book_frontend::app::App);
}

/// Web 以外は対象外 (ADR-0017)。テストと lint が native でも通るための空の main。
#[cfg(not(target_arch = "wasm32"))]
fn main() {}
