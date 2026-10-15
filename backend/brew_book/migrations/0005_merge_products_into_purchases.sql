-- 商品を購入に統合する (0057、ADR-0022)。
--
-- purchases に商品名、producer、origin、region、process、variety を持たせ、product_id を無くす。
-- Flavor Notes の対応は product_flavor_tags から purchase_flavor_tags (購入とタグの対応) に移す。
-- products と product_flavor_tags は消す。購入が 1 件も無い商品と商品のお気に入りは引き継がない
-- (0057 の設計判断)。
--
-- SQLite の ALTER TABLE ... DROP COLUMN は外部キー制約に使われている列 (product_id) を消せない
-- ため、purchases は作り直す。D1 は外部キー制約を常に検査するため、brews が参照する purchases を
-- DROP する時点の違反は defer_foreign_keys でトランザクションの終わりまで遅らせる。
--
-- 作り直しは、別名のテーブルを作って RENAME する形にしない。SQLite は遅延した違反を件数で数え、
-- 親の行の削除 (DROP TABLE の暗黙の DELETE) で増やし、同じ名前の親のテーブルへの行の挿入で減らす。
-- RENAME は件数を戻さないため、brews を持つ購入があると COMMIT が FOREIGN KEY constraint failed に
-- なる (2026-10-14 に SQLite 3.51.0 で確認)。そのため、移す行を制約の無いテーブルに退避し、
-- purchases を DROP してから同じ名前で作り直し、退避した行を入れ直す。入れ直しで違反の件数が
-- 0 に戻り、COMMIT が通る。
PRAGMA defer_foreign_keys = true;

-- 移す行の退避先。制約を持たず、商品の対応を移すために product_id も持つ。
CREATE TABLE purchases_merged (
    id TEXT,
    user_id TEXT,
    product_id TEXT,
    shop_id TEXT,
    purchased_on TEXT,
    roast TEXT,
    roast_date TEXT,
    price_amount INTEGER,
    price_currency TEXT,
    weight_grams INTEGER,
    photo_key TEXT,
    name TEXT,
    producer TEXT,
    origin TEXT,
    region TEXT,
    process TEXT,
    variety TEXT,
    created_at TEXT,
    updated_at TEXT,
    favorited_at TEXT
);

-- 既存の購入と、その商品の項目を退避する。既存の全列を列名で明示して移す。
INSERT INTO purchases_merged (
    id, user_id, product_id, shop_id, purchased_on, roast, roast_date, price_amount,
    price_currency, weight_grams, photo_key, name, producer, origin, region, process, variety,
    created_at, updated_at, favorited_at
)
SELECT
    p.id, p.user_id, p.product_id, p.shop_id, p.purchased_on, p.roast, p.roast_date,
    p.price_amount, p.price_currency, p.weight_grams, p.photo_key, pr.name, pr.producer,
    pr.origin, pr.region, pr.process, pr.variety, p.created_at, p.updated_at, p.favorited_at
FROM purchases AS p
INNER JOIN products AS pr ON pr.id = p.product_id;

-- 古い purchases を消す (インデックスも一緒に消える)。brews の参照の違反は COMMIT まで遅れる。
DROP TABLE purchases;

-- 新しい purchases。店は任意 (もらい物など店が無い購入を認める)。商品名以外の商品の項目は任意。
CREATE TABLE purchases (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users (id),
    shop_id TEXT REFERENCES shops (id),
    purchased_on TEXT NOT NULL,
    roast TEXT,
    roast_date TEXT,
    price_amount INTEGER,
    price_currency TEXT,
    weight_grams INTEGER,
    photo_key TEXT,
    name TEXT NOT NULL,
    producer TEXT,
    origin TEXT,
    region TEXT,
    process TEXT,
    variety TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    favorited_at TEXT
);

-- 退避した行を入れ直す。
INSERT INTO purchases (
    id, user_id, shop_id, purchased_on, roast, roast_date, price_amount, price_currency,
    weight_grams, photo_key, name, producer, origin, region, process, variety,
    created_at, updated_at, favorited_at
)
SELECT
    id, user_id, shop_id, purchased_on, roast, roast_date, price_amount, price_currency,
    weight_grams, photo_key, name, producer, origin, region, process, variety,
    created_at, updated_at, favorited_at
FROM purchases_merged;

-- 購入とタグの対応。中間テーブルも user_id を持つ (ADR-0006)。
CREATE TABLE purchase_flavor_tags (
    user_id TEXT NOT NULL REFERENCES users (id),
    purchase_id TEXT NOT NULL REFERENCES purchases (id),
    tag_id TEXT NOT NULL REFERENCES flavor_tags (id),
    PRIMARY KEY (purchase_id, tag_id)
);

-- 商品のタグの対応を、その商品の購入ごとに移す。
INSERT INTO purchase_flavor_tags (user_id, purchase_id, tag_id)
SELECT m.user_id, m.id, pft.tag_id
FROM product_flavor_tags AS pft
INNER JOIN purchases_merged AS m ON m.product_id = pft.product_id;

-- purchases のインデックスを作り直す (0003、0004 と同じ定義)。
CREATE INDEX idx_purchases_user_purchased_on
    ON purchases (user_id, purchased_on DESC, id);
CREATE INDEX idx_purchases_user_favorited_at ON purchases (user_id, favorited_at);
CREATE INDEX idx_purchases_user_price_amount ON purchases (user_id, price_amount);
CREATE INDEX idx_purchases_user_weight_grams ON purchases (user_id, weight_grams);

-- 退避先と、商品のテーブルとインデックスを消す。
DROP TABLE purchases_merged;
DROP INDEX idx_products_user_created_at;
DROP INDEX idx_products_user_favorited_at;
DROP INDEX idx_products_user_name;
DROP INDEX idx_products_user_updated_at;
DROP TABLE product_flavor_tags;
DROP TABLE products;
