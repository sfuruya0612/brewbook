//! UI の文言 (FR-16)。
//!
//! 表示する文字列は型付きのキー ([`Key`]) と日本語と英語の表で持ち、画面のコードには表示する
//! 文字列を直接書かない (FR-16)。表は移行前の Flutter 版の ARB (`frontend/lib/l10n/app_ja.arb` と
//! `app_en.arb`。0045 で削除) から写した 227 キーで、複数形と選択は無い。
//!
//! 画面は [`t`] で現在の言語の文言を引く。`{name}` の入る文言は [`t_args`] で値を差し込む。
//! 現在の言語は起動時に [`set_language`] で決める (端末またはブラウザの言語が日本語なら日本語、
//! それ以外は英語)。

use std::cell::Cell;

mod en;
mod ja;
mod keys;

pub use keys::{Key, KEY_COUNT};

/// UI の言語 (FR-16)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Language {
    /// 日本語。
    Japanese,
    /// 英語。
    English,
}

impl Language {
    /// 言語コード (`ja`、`en`)。
    pub fn code(self) -> &'static str {
        match self {
            Self::Japanese => "ja",
            Self::English => "en",
        }
    }
}

/// 端末またはブラウザの言語から UI の言語を決める (FR-16)。
///
/// 日本語 (`ja`、`ja-JP`) なら日本語、それ以外と未指定は英語にする。対応する言語は日本語と
/// 英語の 2 つだけなので、英語以外の言語は英語にする。
pub fn resolve_language(language: Option<&str>) -> Language {
    match language {
        Some(tag)
            if tag
                .split(['-', '_'])
                .next()
                .is_some_and(|part| part.eq_ignore_ascii_case("ja")) =>
        {
            Language::Japanese
        }
        _ => Language::English,
    }
}

thread_local! {
    /// 現在の UI の言語。起動時に [`set_language`] で決める。
    static CURRENT_LANGUAGE: Cell<Language> = const { Cell::new(Language::English) };
}

/// 現在の UI の言語。
pub fn current_language() -> Language {
    CURRENT_LANGUAGE.with(Cell::get)
}

/// UI の言語を設定する。起動時に 1 回呼ぶ。
pub fn set_language(language: Language) {
    CURRENT_LANGUAGE.with(|cell| cell.set(language));
}

/// 現在の言語の文言を返す (FR-16)。
pub fn t(key: Key) -> &'static str {
    text(current_language(), key)
}

/// 指定した言語の文言を返す。
pub fn text(language: Language, key: Key) -> &'static str {
    match language {
        Language::Japanese => ja::JA[key.index()],
        Language::English => en::EN[key.index()],
    }
}

/// 現在の言語の文言の `{name}` を引数の値で置き換える (FR-16)。
///
/// 引数の名前は ARB のプレースホルダの名前と同じにする。表に無い名前を渡しても何も
/// 置き換わらない。
pub fn t_args(key: Key, args: &[(&str, &str)]) -> String {
    text_args(current_language(), key, args)
}

/// 指定した言語の文言の `{name}` を引数の値で置き換える。
pub fn text_args(language: Language, key: Key, args: &[(&str, &str)]) -> String {
    let mut value = text(language, key).to_string();
    for (name, replacement) in args {
        value = value.replace(&format!("{{{name}}}"), replacement);
    }
    value
}

/// ブラウザの言語 (`navigator.language`) を返す。Web 以外では None。
#[cfg(target_arch = "wasm32")]
pub fn browser_language() -> Option<String> {
    web_sys::window()
        .map(|window| window.navigator().language())
        .flatten()
}

/// ブラウザの言語 (`navigator.language`) を返す。Web 以外では None。
#[cfg(not(target_arch = "wasm32"))]
pub fn browser_language() -> Option<String> {
    None
}
