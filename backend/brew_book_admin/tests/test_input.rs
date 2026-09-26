//! フォームの入力の解釈と検証 (src/input.rs) の単体テスト。
//!
//! 表示名の境界値 (前後の空白、1 文字、50 文字、51 文字) は PBT
//! (`pbt/tests/prop_admin_input.rs`) が一般的な性質として検査し、ここでは個別の値を確かめる。

use brew_book_admin::input::{
    form_field, registration_link, validate_display_name, DisplayNameError,
};

#[test]
fn form_field_reads_and_decodes_a_field() {
    assert_eq!(
        form_field(
            "display_name=%E5%B1%B1%E7%94%B0+%E5%A4%AA%E9%83%8E",
            "display_name"
        ),
        Some("山田 太郎".to_owned())
    );
    // 複数の項目があっても名前で引ける。
    assert_eq!(
        form_field(
            "a=1&display_name=%E5%88%A9%E7%94%A8%E8%80%85&b=2",
            "display_name"
        ),
        Some("利用者".to_owned())
    );
}

#[test]
fn form_field_returns_none_for_a_missing_or_unreadable_body() {
    assert_eq!(form_field("", "display_name"), None);
    assert_eq!(form_field("other=1", "display_name"), None);
    // 値が空でも項目はある。検証が 400 にする (Missing)。
    assert_eq!(
        form_field("display_name=", "display_name"),
        Some(String::new())
    );
}

#[test]
fn validate_display_name_trims_the_input() {
    assert_eq!(
        validate_display_name("  利用者  ").expect("the name must be valid"),
        "利用者"
    );
    assert_eq!(
        validate_display_name("利用者").expect("the name must be valid"),
        "利用者"
    );
}

#[test]
fn validate_display_name_rejects_a_missing_name() {
    assert_eq!(validate_display_name(""), Err(DisplayNameError::Missing));
    assert_eq!(
        validate_display_name("   \t\n"),
        Err(DisplayNameError::Missing)
    );
}

#[test]
fn validate_display_name_accepts_fifty_characters_and_rejects_fifty_one() {
    let fifty = "あ".repeat(50);
    assert_eq!(
        validate_display_name(&fifty).expect("50 characters must be accepted"),
        fifty
    );
    let fifty_one = "あ".repeat(51);
    assert_eq!(
        validate_display_name(&fifty_one),
        Err(DisplayNameError::TooLong)
    );
    // 前後の空白は文字数に数えない。
    assert_eq!(
        validate_display_name(&format!("  {fifty}  ")).expect("50 characters must be accepted"),
        fifty
    );
}

#[test]
fn validate_display_name_messages_are_english() {
    assert_eq!(
        DisplayNameError::Missing.message(),
        "the display name is missing"
    );
    assert_eq!(
        DisplayNameError::TooLong.message(),
        "the display name must be at most 50 characters"
    );
}

#[test]
fn registration_link_appends_the_token_to_the_app_origin() {
    assert_eq!(
        registration_link("https://brewbook.example.workers.dev", "token-value"),
        "https://brewbook.example.workers.dev/register?token=token-value"
    );
    // オリジンの末尾の `/` は 1 つに正規化する。
    assert_eq!(
        registration_link("http://localhost:8787/", "token-value"),
        "http://localhost:8787/register?token=token-value"
    );
}
