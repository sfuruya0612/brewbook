-- アーカイブ (論理削除) を廃止する (ADR-0018)。
--
-- 店、商品、購入、抽出の 4 テーブルから archived_at の列を削除し、列を含まないインデックスを
-- 作り直す。列を参照するインデックスが残っていると DROP COLUMN が失敗するため、先に消す。
-- 既存の行は削除せず、アーカイブ済みだった行も通常の記録として残す (DELETE は書かない)。
DROP INDEX idx_shops_user_archived_created_at;
DROP INDEX idx_products_user_archived_created_at;
DROP INDEX idx_purchases_user_archived_purchased_on;
DROP INDEX idx_brews_user_archived_brewed_at;

ALTER TABLE shops DROP COLUMN archived_at;
ALTER TABLE products DROP COLUMN archived_at;
ALTER TABLE purchases DROP COLUMN archived_at;
ALTER TABLE brews DROP COLUMN archived_at;

-- 一覧と統計の複合インデックス (利用者 ID、並び順のキー)。
-- 一覧は並び順のキーの降順と ID の昇順で返すため、ID まで含める。
CREATE INDEX idx_shops_user_created_at
    ON shops (user_id, created_at DESC, id);
CREATE INDEX idx_products_user_created_at
    ON products (user_id, created_at DESC, id);
CREATE INDEX idx_purchases_user_purchased_on
    ON purchases (user_id, purchased_on DESC, id);
CREATE INDEX idx_brews_user_brewed_at
    ON brews (user_id, brewed_at DESC, id);
