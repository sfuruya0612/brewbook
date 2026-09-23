-- 購入の price_currency を NULL 許容にする (0007)。
--
-- 価格と通貨コードの組は片方だけを持たず、価格が無い購入では通貨コードも NULL にする
-- (0007 の設計判断)。初期スキーマ (0001) の price_currency は NOT NULL DEFAULT 'JPY' で、
-- 値が無いときに NULL を保存できない。
-- SQLite は列の NOT NULL を外す ALTER TABLE を持たないため、列を外して入れ直す。
-- テーブルを作り直す方法は、purchases が brews から参照されているため使わない
-- (外部キー制約を有効にしたまま親のテーブルを DROP できない)。
-- 列の位置は末尾に移る。SELECT は列名を明示するため、応答と保存の内容は変わらない。
ALTER TABLE purchases DROP COLUMN price_currency;
ALTER TABLE purchases ADD COLUMN price_currency TEXT;
