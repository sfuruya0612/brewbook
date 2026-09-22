-- D1 の初期スキーマ (ADR-0006 の 11 テーブル)。
--
-- 主キーはサーバーで生成する UUID v4 の文字列、日時は ISO 8601 UTC の文字列、
-- 日付はタイムゾーンを持たない YYYY-MM-DD の文字列で保存する (ADR-0002)。
-- 外部キー制約を定義し、物理削除はアカウント削除 (0012) だけが子のテーブルから順に行う。
-- カスケード削除は使わない (ADR-0006)。

-- 利用者。表示名は管理者が管理者画面で利用者を識別するためだけに使う。
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- 登録用トークン。生の値は保存せず、ハッシュだけを保存する。
CREATE TABLE registration_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    token_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used_at TEXT
);

-- パスキー。credential_id は全利用者で一意にする (ログインが credential ID で引くため)。
CREATE TABLE passkey_credentials (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    credential_id TEXT NOT NULL UNIQUE,
    public_key TEXT NOT NULL,
    sign_count INTEGER NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT
);

-- WebAuthn のチャレンジ。ログイン用の行は user_id を持たず、チャレンジ値で引く。
CREATE TABLE webauthn_challenges (
    id TEXT PRIMARY KEY,
    user_id TEXT REFERENCES users (id),
    challenge TEXT NOT NULL,
    kind TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

-- セッション。生のトークンは保存せず、ハッシュだけを保存する。
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    token_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- 店。住所は任意。
CREATE TABLE shops (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    name TEXT NOT NULL,
    address TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

-- 商品。商品名以外は任意。
CREATE TABLE products (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    name TEXT NOT NULL,
    producer TEXT,
    origin TEXT,
    region TEXT,
    process TEXT,
    variety TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

-- Flavor Notes のタグ。利用者ごとに名前で一意にする (FR-8)。
CREATE TABLE flavor_tags (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    name TEXT NOT NULL,
    UNIQUE (user_id, name)
);

-- 商品とタグの対応。中間テーブルも user_id を持つ (ADR-0006)。
CREATE TABLE product_flavor_tags (
    user_id TEXT NOT NULL REFERENCES users (id),
    product_id TEXT NOT NULL REFERENCES products (id),
    tag_id TEXT NOT NULL REFERENCES flavor_tags (id),
    PRIMARY KEY (product_id, tag_id)
);

-- 購入。店は任意 (もらい物など店が無い購入を認める)。通貨コードの既定値は JPY。
CREATE TABLE purchases (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    product_id TEXT NOT NULL REFERENCES products (id),
    shop_id TEXT REFERENCES shops (id),
    purchased_on TEXT NOT NULL,
    roast TEXT,
    roast_date TEXT,
    price_amount INTEGER,
    price_currency TEXT NOT NULL DEFAULT 'JPY',
    weight_grams INTEGER,
    photo_key TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

-- 抽出。購入だけを参照し、商品と店の ID は重複して持たない。
CREATE TABLE brews (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    purchase_id TEXT NOT NULL REFERENCES purchases (id),
    brewed_at TEXT NOT NULL,
    dose_grams REAL,
    water_grams REAL,
    water_temp_c REAL,
    brew_time_seconds INTEGER,
    method TEXT,
    grind_setting TEXT,
    rating INTEGER,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

-- 一覧、サジェスト、統計の複合インデックス (利用者 ID、archived_at、並び順のキー)。
-- 一覧は並び順のキーの降順と ID の昇順で返すため、ID まで含める。
CREATE INDEX idx_shops_user_archived_created_at
    ON shops (user_id, archived_at, created_at DESC, id);
CREATE INDEX idx_products_user_archived_created_at
    ON products (user_id, archived_at, created_at DESC, id);
CREATE INDEX idx_purchases_user_archived_purchased_on
    ON purchases (user_id, archived_at, purchased_on DESC, id);
CREATE INDEX idx_brews_user_archived_brewed_at
    ON brews (user_id, archived_at, brewed_at DESC, id);
