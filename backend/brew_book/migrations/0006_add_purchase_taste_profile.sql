-- 購入に、AI が推定した味と特徴を保存する (FR-24、ADR-0023)。
--
-- JSON の文字列で保存する。購入の応答では `taste_profile` のオブジェクトとして返す。
-- 推定をやり直したときは上書きし、AI の呼び出しに失敗したときは前の値を残す。
ALTER TABLE purchases ADD COLUMN taste_profile TEXT;
