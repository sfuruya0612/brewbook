-- お気に入りと一覧の並び替えを追加する (0051、FR-20、FR-21)。
--
-- 店、商品、購入、抽出の 4 テーブルに favorited_at (NULL は未設定) を足す。
-- 0050 まで archived_at が使っていた「NULL で未設定」の形に揃え、いつお気に入りにしたかも
-- 残す (0051 の設計判断)。SQLite は ALTER TABLE ... ADD COLUMN で列を末尾に足すため、
-- 列の位置は SELECT の列名の明示に影響しない。

ALTER TABLE shops ADD COLUMN favorited_at TEXT;
ALTER TABLE products ADD COLUMN favorited_at TEXT;
ALTER TABLE purchases ADD COLUMN favorited_at TEXT;
ALTER TABLE brews ADD COLUMN favorited_at TEXT;

-- お気に入りだけに絞る一覧のインデックス (FR-21)。
CREATE INDEX idx_shops_user_favorited_at ON shops (user_id, favorited_at);
CREATE INDEX idx_products_user_favorited_at ON products (user_id, favorited_at);
CREATE INDEX idx_purchases_user_favorited_at ON purchases (user_id, favorited_at);
CREATE INDEX idx_brews_user_favorited_at ON brews (user_id, favorited_at);

-- 並び順のキーのインデックス (FR-20)。名前は大文字と小文字を区別しない比較に揃える。
-- NULLS LAST と COLLATE NOCASE の比較のため索引が常に効くとは限らないが、想定規模では
-- 許容し、性能の作り込みはしない (0051 の設計判断)。
CREATE INDEX idx_shops_user_name ON shops (user_id, name COLLATE NOCASE);
CREATE INDEX idx_shops_user_updated_at ON shops (user_id, updated_at DESC, id);
CREATE INDEX idx_products_user_name ON products (user_id, name COLLATE NOCASE);
CREATE INDEX idx_products_user_updated_at ON products (user_id, updated_at DESC, id);
CREATE INDEX idx_purchases_user_price_amount ON purchases (user_id, price_amount);
CREATE INDEX idx_purchases_user_weight_grams ON purchases (user_id, weight_grams);
CREATE INDEX idx_brews_user_rating ON brews (user_id, rating);
CREATE INDEX idx_brews_user_dose_grams ON brews (user_id, dose_grams);
