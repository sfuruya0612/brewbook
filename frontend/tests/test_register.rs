//! `screens::register` の送信の前の判定の単体テスト (0040)。
//!
//! 画面のハンドラが名前の検証 (1 文字以上 50 文字以下) とトークンの有無を API の呼び出しの
//! 前に決めることを検査する (0040 のレビューの指摘。完了条件「範囲外は画面で拒否する」)。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::auth::PASSKEY_NAME_MAX_CHARS;
use brew_book_frontend::i18n::Key;
use brew_book_frontend::screens::{register_submit, RegisterSubmit};

#[test]
fn a_valid_name_with_a_token_is_sent() {
    assert_eq!(
        register_submit("  マイキー  ", Some("tok")),
        RegisterSubmit::Send {
            token: "tok".to_string(),
            name: "マイキー".to_string(),
        }
    );
}

#[test]
fn an_out_of_range_name_is_rejected_before_the_api() {
    for value in ["", "   ", &"あ".repeat(PASSKEY_NAME_MAX_CHARS + 1)] {
        assert_eq!(
            register_submit(value, Some("tok")),
            RegisterSubmit::NameError(Key::PasskeyNameError),
            "{value:?}"
        );
    }
}

#[test]
fn a_missing_token_is_not_sent() {
    for token in [None, Some("")] {
        assert_eq!(
            register_submit("マイキー", token),
            RegisterSubmit::MissingToken,
            "{token:?}"
        );
    }
}
