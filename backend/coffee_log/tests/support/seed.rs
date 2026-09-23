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
/// 10 以上の添字は 0 で埋めて一意にする (数字の繰り返しでは 1 と 11 が衝突するため)。
pub fn user_id(index: u32) -> String {
    let group = |length: usize| {
        if index < 10 {
            let digit = char::from_digit(index, 10).expect("the index must be a digit");
            digit.to_string().repeat(length)
        } else {
            format!("{index:0>length$}")
        }
    };
    format!(
        "{}-{}-4{}-8{}-{}",
        group(8),
        group(4),
        group(3),
        group(3),
        group(12)
    )
}

/// 下ごしらえした店。
#[derive(Debug, Clone)]
pub struct SeededShop {
    /// `shops.id`。API のパスで指定する ID。
    pub id: String,
    /// 店名。
    pub name: String,
}

/// 下ごしらえした商品。
#[derive(Debug, Clone)]
pub struct SeededProduct {
    /// `products.id`。API のパスで指定する ID。
    pub id: String,
    /// 商品名。
    pub name: String,
}

/// 下ごしらえした購入。
#[derive(Debug, Clone)]
pub struct SeededPurchase {
    /// `purchases.id`。API のパスで指定する ID。
    pub id: String,
    /// 参照する商品の ID。
    pub product_id: String,
    /// 購入日。
    pub purchased_on: String,
}

/// 下ごしらえした抽出。
#[derive(Debug, Clone)]
pub struct SeededBrew {
    /// `brews.id`。API のパスで指定する ID。
    pub id: String,
    /// 参照する購入の ID。
    pub purchase_id: String,
    /// 抽出日時。
    pub brewed_at: String,
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

    /// 店の行を入れる。住所と `archived_at` は任意。
    pub fn shop(
        &mut self,
        user_id: &str,
        name: &str,
        address: Option<&str>,
        created_at: &str,
        updated_at: &str,
        archived_at: Option<&str>,
    ) -> SeededShop {
        let id = self.next_id();
        self.push(format!(
            "INSERT INTO shops (id, user_id, name, address, created_at, updated_at, archived_at) \
             VALUES ({}, {}, {}, {}, {}, {}, {})",
            literal(&id),
            literal(user_id),
            literal(name),
            address.map_or_else(|| "NULL".to_owned(), literal),
            literal(created_at),
            literal(updated_at),
            archived_at.map_or_else(|| "NULL".to_owned(), literal)
        ));
        SeededShop {
            id,
            name: name.to_owned(),
        }
    }

    /// 商品の行を入れる。商品名以外の項目は NULL にする。
    pub fn product(
        &mut self,
        user_id: &str,
        name: &str,
        created_at: &str,
        updated_at: &str,
        archived_at: Option<&str>,
    ) -> SeededProduct {
        let id = self.next_id();
        self.push(format!(
            "INSERT INTO products (id, user_id, name, created_at, updated_at, archived_at) \
             VALUES ({}, {}, {}, {}, {}, {})",
            literal(&id),
            literal(user_id),
            literal(name),
            literal(created_at),
            literal(updated_at),
            archived_at.map_or_else(|| "NULL".to_owned(), literal)
        ));
        SeededProduct {
            id,
            name: name.to_owned(),
        }
    }

    /// Flavor Notes のタグの行を入れる。タグの ID を返す。
    pub fn flavor_tag(&mut self, user_id: &str, name: &str) -> String {
        let id = self.next_id();
        self.push(format!(
            "INSERT INTO flavor_tags (id, user_id, name) VALUES ({}, {}, {})",
            literal(&id),
            literal(user_id),
            literal(name)
        ));
        id
    }

    /// 商品とタグの対応の行を入れる。
    pub fn product_flavor_tag(&mut self, user_id: &str, product_id: &str, tag_id: &str) {
        self.push(format!(
            "INSERT INTO product_flavor_tags (user_id, product_id, tag_id) VALUES ({}, {}, {})",
            literal(user_id),
            literal(product_id),
            literal(tag_id)
        ));
    }

    /// 購入の行を入れる。店、価格、通貨コード、重量、写真は任意。
    // 下ごしらえの引数はテーブルの列をそのまま受ける (テストが列を選んで投入できるようにする)。
    #[allow(clippy::too_many_arguments)]
    pub fn purchase(
        &mut self,
        user_id: &str,
        product_id: &str,
        shop_id: Option<&str>,
        purchased_on: &str,
        created_at: &str,
        updated_at: &str,
        archived_at: Option<&str>,
    ) -> SeededPurchase {
        let id = self.next_id();
        let shop = shop_id.map_or_else(|| "NULL".to_owned(), literal);
        self.push(format!(
            "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, created_at, \
             updated_at, archived_at) VALUES ({}, {}, {}, {}, {}, {}, {}, {})",
            literal(&id),
            literal(user_id),
            literal(product_id),
            shop,
            literal(purchased_on),
            literal(created_at),
            literal(updated_at),
            archived_at.map_or_else(|| "NULL".to_owned(), literal)
        ));
        SeededPurchase {
            id,
            product_id: product_id.to_owned(),
            purchased_on: purchased_on.to_owned(),
        }
    }

    /// 抽出の行を入れる。数値の項目は入れず、NULL のままにする。
    pub fn brew(
        &mut self,
        user_id: &str,
        purchase_id: &str,
        brewed_at: &str,
        created_at: &str,
        updated_at: &str,
        archived_at: Option<&str>,
    ) -> SeededBrew {
        let id = self.next_id();
        self.push(format!(
            "INSERT INTO brews (id, user_id, purchase_id, brewed_at, created_at, updated_at, \
             archived_at) VALUES ({}, {}, {}, {}, {}, {}, {})",
            literal(&id),
            literal(user_id),
            literal(purchase_id),
            literal(brewed_at),
            literal(created_at),
            literal(updated_at),
            archived_at.map_or_else(|| "NULL".to_owned(), literal)
        ));
        SeededBrew {
            id,
            purchase_id: purchase_id.to_owned(),
            brewed_at: brewed_at.to_owned(),
        }
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
