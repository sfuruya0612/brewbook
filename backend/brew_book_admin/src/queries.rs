//! 管理者 Worker の SQL (FR-17)。
//!
//! SQL はこのモジュールだけが持つ。扱うテーブルは users、registration_tokens、
//! passkey_credentials の 3 つだけとする (ADR-0008)。単体テストが [`STATEMENTS`] を列挙し、
//! 現れるテーブル名が [`TABLES`] と一致することを照合する。

/// 一覧に出す利用者 1 件。パスキーの数は部分クエリで数える。
#[derive(Debug, serde::Deserialize)]
pub struct UserRow {
    /// `users.id`。
    pub id: String,
    /// `users.display_name`。
    pub display_name: String,
    /// `users.created_at`。
    pub created_at: String,
    /// その利用者のパスキーの数。
    pub passkey_count: i64,
}

/// 利用者の存在の確認に使う行。
#[derive(Debug, serde::Deserialize)]
pub struct UserIdRow {
    /// `users.id`。
    pub id: String,
}

/// 利用者の一覧。表示名、作成日時、パスキーの数を返す (FR-17)。
pub const SELECT_USERS: &str = "SELECT users.id, users.display_name, users.created_at, \
                                (SELECT COUNT(*) FROM passkey_credentials \
                                 WHERE passkey_credentials.user_id = users.id) AS passkey_count \
                                FROM users ORDER BY users.created_at DESC, users.id ASC";

/// 利用者の存在の確認。
pub const SELECT_USER: &str = "SELECT id FROM users WHERE id = ?";

/// 利用者の作成。
pub const INSERT_USER: &str = "INSERT INTO users (id, display_name, created_at) VALUES (?, ?, ?)";

/// 利用者の未使用の登録用トークンの削除 (再発行の前に呼ぶ)。
pub const DELETE_UNUSED_TOKENS: &str =
    "DELETE FROM registration_tokens WHERE user_id = ? AND used_at IS NULL";

/// 登録用トークンの発行。生の値は保存せず、ハッシュだけを保存する (ADR-0004)。
pub const INSERT_TOKEN: &str =
    "INSERT INTO registration_tokens (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)";

/// この Worker が実行する SQL の全て。単体テストが列挙に使う。
pub const STATEMENTS: &[&str] = &[
    SELECT_USERS,
    SELECT_USER,
    INSERT_USER,
    DELETE_UNUSED_TOKENS,
    INSERT_TOKEN,
];

/// この Worker が扱う 3 つのテーブル (ADR-0008)。
pub const TABLES: [&str; 3] = ["users", "registration_tokens", "passkey_credentials"];

/// SQL に現れるテーブル名を集める。
///
/// `FROM`、`INTO`、`UPDATE`、`JOIN` の直後の語をテーブル名として読む。列名や別名は読まないため、
/// 追加した SQL が扱うテーブルをこの関数で列挙できる。
pub fn table_names(sql: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut read_next = false;
    for word in
        sql.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
    {
        if read_next {
            if !word.is_empty() {
                names.push(word.to_ascii_lowercase());
                read_next = false;
            }
            continue;
        }
        if matches!(
            word.to_ascii_uppercase().as_str(),
            "FROM" | "INTO" | "UPDATE" | "JOIN"
        ) {
            read_next = true;
        }
    }
    names
}
