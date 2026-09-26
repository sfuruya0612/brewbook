//! 管理者画面の HTML 生成 (src/html.rs) の単体テスト。
//!
//! 一覧と発行の応答が、利用者の表示名とリンクを HTML エスケープして 1 回だけ載せることを検査する。

use brew_book_admin::html::{error_page, escape, token_page, users_page};
use brew_book_admin::queries::UserRow;

/// テスト用の利用者の行。
fn user(id: &str, display_name: &str, created_at: &str, passkey_count: i64) -> UserRow {
    UserRow {
        id: id.to_owned(),
        display_name: display_name.to_owned(),
        created_at: created_at.to_owned(),
        passkey_count,
    }
}

#[test]
fn escape_replaces_the_html_special_characters() {
    assert_eq!(escape("&<>\"'"), "&amp;&lt;&gt;&quot;&#39;");
    // 特殊文字の無い文字列はそのまま返る (日本語を含む)。
    assert_eq!(escape("表示名 の 利用者"), "表示名 の 利用者");
}

#[test]
fn users_page_shows_the_display_name_created_at_and_passkey_count() {
    let page = users_page(&[user(
        "00000000-0000-4000-8000-000000000001",
        "一覧の利用者",
        "2026-09-01T00:00:00.000Z",
        2,
    )]);
    assert!(page.contains("一覧の利用者"), "{page}");
    assert!(page.contains("2026-09-01T00:00:00.000Z"), "{page}");
    assert!(page.contains("class=\"count\">2<"), "{page}");
    assert!(
        page.contains("action=\"/users/00000000-0000-4000-8000-000000000001/tokens\""),
        "{page}"
    );
    // 利用者の作成のフォーム (FR-17)。
    assert!(page.contains("action=\"/users\""), "{page}");
    assert!(page.contains("name=\"display_name\""), "{page}");
}

#[test]
fn users_page_escapes_the_display_name() {
    let page = users_page(&[user(
        "00000000-0000-4000-8000-000000000001",
        "<管理者> & \"利用者\"",
        "2026-09-01T00:00:00.000Z",
        1,
    )]);
    assert!(
        page.contains("&lt;管理者&gt; &amp; &quot;利用者&quot;"),
        "{page}"
    );
    assert!(!page.contains("<管理者>"), "{page}");
}

#[test]
fn users_page_shows_a_message_when_there_are_no_users() {
    let page = users_page(&[]);
    assert!(page.contains("利用者はいません"), "{page}");
    assert!(!page.contains("<tbody>\n\n"), "{page}");
}

#[test]
fn token_page_shows_the_link_once() {
    let link = "https://brewbook.example.workers.dev/register?token=abc";
    let page = token_page(link);
    assert_eq!(page.matches(link).count(), 1, "{page}");
    assert!(page.contains("再表示できません"), "{page}");
}

#[test]
fn token_page_escapes_the_link() {
    let link = "https://brewbook.example.workers.dev/register?token=a&b=c";
    let page = token_page(link);
    assert!(page.contains("token=a&amp;b=c"), "{page}");
    assert!(!page.contains("token=a&b=c"), "{page}");
}

#[test]
fn error_page_escapes_the_message() {
    let page = error_page("<script>alert(1)</script>");
    assert!(
        page.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
        "{page}"
    );
}
