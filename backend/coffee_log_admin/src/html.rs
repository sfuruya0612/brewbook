//! 管理者画面の HTML の組み立て (ADR-0008)。
//!
//! テンプレートエンジンのクレートは追加せず、Rust の文字列で組み立てる。
//! 利用者の表示名と発行したリンクは HTML エスケープして出力する。
//! 応答の組み立て (`worker::Response`) は呼び出し側が行い、ここは HTML の文字列だけを返す
//! (純粋な関数として単体テストできるようにするため)。

use crate::queries::UserRow;

/// HTML の特殊文字を実体参照にする。
///
/// 管理者画面に出す値は、利用者の表示名と発行したリンクだけである。どちらも外部から
/// 持ち込まれる値のため、属性値と本文の両方で安全なように 5 文字をエスケープする。
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

/// 利用者の一覧の HTML。表示名、作成日時、パスキーの数と、作成のフォームを出す (FR-17)。
pub fn users_page(users: &[UserRow]) -> String {
    let rows = if users.is_empty() {
        "<p>利用者はいません。</p>\n".to_owned()
    } else {
        users.iter().map(user_row).collect()
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="utf-8">
<title>利用者</title>
</head>
<body>
<h1>利用者</h1>
<table>
<thead>
<tr><th>表示名</th><th>作成日時</th><th>パスキーの数</th><th>登録用リンク</th></tr>
</thead>
<tbody>
{rows}</tbody>
</table>
<h2>利用者を作成</h2>
<form method="post" action="/users">
<label>表示名 <input type="text" name="display_name" maxlength="50"></label>
<button type="submit">作成</button>
</form>
</body>
</html>
"#
    )
}

/// 一覧の利用者 1 件の行。
fn user_row(user: &UserRow) -> String {
    format!(
        "<tr><td>{display_name}</td><td>{created_at}</td><td class=\"count\">{passkey_count}</td>\
<td><form method=\"post\" action=\"/users/{id}/tokens\">\
<button type=\"submit\">発行</button></form></td></tr>\n",
        display_name = escape(&user.display_name),
        created_at = escape(&user.created_at),
        passkey_count = user.passkey_count,
        id = escape(&user.id),
    )
}

/// 発行した登録用リンクの HTML。リンクはこの応答にだけ 1 回だけ載り、再表示できない (ADR-0008)。
pub fn token_page(link: &str) -> String {
    let link = escape(link);
    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="utf-8">
<title>登録用リンク</title>
</head>
<body>
<h1>登録用リンク</h1>
<p>このリンクは再表示できません。利用者に渡してください。</p>
<p>{link}</p>
<p><a href="/">利用者の一覧に戻る</a></p>
</body>
</html>
"#
    )
}

/// エラーの HTML。メッセージは英語にする (ログとエラーメッセージの規約)。
pub fn error_page(message: &str) -> String {
    let message = escape(message);
    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="utf-8">
<title>エラー</title>
</head>
<body>
<h1>エラー</h1>
<p>{message}</p>
<p><a href="/">利用者の一覧に戻る</a></p>
</body>
</html>
"#
    )
}
