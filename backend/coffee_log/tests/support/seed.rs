//! テストの下ごしらえ。
//!
//! 管理者画面 (0018) が無いため、利用者、登録用トークン、セッション、チャレンジ、パスキーの行は
//! `wrangler d1 execute` で直接入れる。値はテスト専用で、SQL のリテラルとして埋め込むため
//! 引用符を含めない (含む値は組み立てで落とす)。
//!
//! 生のトークンは D1 に保存できない (保存するのはハッシュだけ) ため、下ごしらえが生の値を返し、
//! テストが API の入力や Cookie に使う。

use coffee_log_core::auth::hash_secret;

/// テスト用の利用者 ID。同じ数字を並べた UUID v4 にする。
pub fn user_id(index: u8) -> String {
    let digit = char::from_digit(u32::from(index), 10).expect("the index must be one digit");
    let hex = digit.to_string();
    let group = |length: usize| hex.repeat(length);
    format!(
        "{}-{}-4{}-8{}-{}",
        group(8),
        group(4),
        group(3),
        group(3),
        group(12)
    )
}

/// 下ごしらえしたパスキー。
pub struct SeededPasskey {
    /// `passkey_credentials.id`。API のパスで指定する ID。
    pub id: String,
    /// クレデンシャル ID (base64url)。
    pub credential_id: String,
    /// パスキーの名前。
    pub name: String,
}

/// 下ごしらえの SQL の組み立て。
#[derive(Default)]
pub struct Seed {
    statements: Vec<String>,
    counter: u32,
}

impl Seed {
    pub fn new() -> Self {
        Self::default()
    }

    /// 利用者を入れる。
    pub fn user(&mut self, user_id: &str, display_name: &str, created_at: &str) -> &mut Self {
        self.push(format!(
            "INSERT INTO users (id, display_name, created_at) VALUES ({}, {}, {})",
            literal(user_id),
            literal(display_name),
            literal(created_at)
        ));
        self
    }

    /// 未使用の登録用トークンを入れる。生のトークンを返す。
    pub fn registration_token(&mut self, user_id: &str, expires_at: &str) -> String {
        let token = self.next_secret("registration-token");
        let id = self.next_id();
        let hash = hash_secret(&token);
        let statement = format!(
            "INSERT INTO registration_tokens (id, user_id, token_hash, expires_at, used_at) \
             VALUES ({}, {}, {}, {}, NULL)",
            literal(&id),
            literal(user_id),
            literal(&hash),
            literal(expires_at)
        );
        self.push(statement);
        token
    }

    /// 使用済みの登録用トークンを入れる。生のトークンを返す。
    pub fn used_registration_token(
        &mut self,
        user_id: &str,
        expires_at: &str,
        used_at: &str,
    ) -> String {
        let token = self.next_secret("used-registration-token");
        let id = self.next_id();
        let hash = hash_secret(&token);
        let statement = format!(
            "INSERT INTO registration_tokens (id, user_id, token_hash, expires_at, used_at) \
             VALUES ({}, {}, {}, {}, {})",
            literal(&id),
            literal(user_id),
            literal(&hash),
            literal(expires_at),
            literal(used_at)
        );
        self.push(statement);
        token
    }

    /// セッションを入れる。生のトークンを返す。
    pub fn session(&mut self, user_id: &str, expires_at: &str, created_at: &str) -> String {
        let token = self.next_secret("session-token");
        let id = self.next_id();
        let hash = hash_secret(&token);
        let statement = format!(
            "INSERT INTO sessions (id, user_id, token_hash, expires_at, created_at) \
             VALUES ({}, {}, {}, {}, {})",
            literal(&id),
            literal(user_id),
            literal(&hash),
            literal(expires_at),
            literal(created_at)
        );
        self.push(statement);
        token
    }

    /// パスキーの行を入れる。公開鍵は検証に使わないテスト用の固定値にする。
    pub fn passkey(
        &mut self,
        user_id: &str,
        name: &str,
        created_at: &str,
        last_used_at: Option<&str>,
    ) -> SeededPasskey {
        let counter = self.next_counter();
        let id = format!("passkey-{counter}");
        let credential_id = format!("credential-{counter}");
        let last_used = last_used_at.map_or_else(|| "NULL".to_owned(), literal);
        self.push(format!(
            "INSERT INTO passkey_credentials (id, user_id, credential_id, public_key, sign_count, \
             name, created_at, last_used_at) VALUES ({}, {}, {}, {}, 0, {}, {}, {})",
            literal(&id),
            literal(user_id),
            literal(&credential_id),
            literal("test-public-key"),
            literal(name),
            literal(created_at),
            last_used
        ));
        SeededPasskey {
            id,
            credential_id,
            name: name.to_owned(),
        }
    }

    /// チャレンジの行を入れる。有効期限の状態をテストから決められるようにする。
    pub fn challenge(
        &mut self,
        user_id: Option<&str>,
        kind: &str,
        challenge: &str,
        expires_at: &str,
    ) -> &mut Self {
        let user = user_id.map_or_else(|| "NULL".to_owned(), literal);
        let id = self.next_id();
        let statement = format!(
            "INSERT INTO webauthn_challenges (id, user_id, challenge, kind, expires_at) \
             VALUES ({}, {}, {}, {}, {})",
            literal(&id),
            user,
            literal(challenge),
            literal(kind),
            literal(expires_at)
        );
        self.push(statement);
        self
    }

    /// 下ごしらえの SQL。
    pub fn sql(&self) -> String {
        self.statements.join(";\n")
    }

    /// 次の連番。行の ID とテスト用の値に使う。
    fn next_counter(&mut self) -> u32 {
        self.counter += 1;
        self.counter
    }

    /// テスト専用の生の秘密値。
    fn next_secret(&mut self, prefix: &str) -> String {
        format!("{prefix}-{}", self.next_counter())
    }

    /// テスト専用の行の ID (UUID の形)。
    fn next_id(&mut self) -> String {
        let counter = self.next_counter();
        format!("00000000-0000-4000-8000-{counter:012}")
    }

    fn push(&mut self, statement: String) {
        self.statements.push(statement);
    }
}

/// 文字列を SQL のリテラルにする。
fn literal(value: &str) -> String {
    assert!(
        !value.contains('\''),
        "the test value must not contain a single quote: {value}"
    );
    format!("'{value}'")
}
