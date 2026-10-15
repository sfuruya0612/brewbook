# ADR-0022: 商品を購入に統合する

Created: 2026-10-14
Model: Claude Fable 5.1
Status: Accepted

## 背景

ADR-0006 は、同じ商品を複数回買う場合の扱いとして商品マスタと購入履歴を分け、商品名、Producer、Origin、Region、Process、Variety、Flavor Notes を商品に、店、Roast、Roast Date、価格、重量、写真を購入に持たせた。
検討した選択肢の「購入ごとに 1 レコード (商品マスタ無し)」は「同じ商品を 2 回買うと商品の項目が重複する」を理由に採らなかった。
この重複の説明は筆者が選択肢の提示で示したものであり、所有者が確認した事実ではない (PRD の「## 背景と課題」)。

分けた結果、購入の登録には商品の登録か選択が先に要り、購入のフォームは商品のピッカーと「商品を登録」のシートを持つ。
写真からの推測 (PRD の FR-19) も、推測した商品名で商品の一覧を名前で引き、一致が無ければ「推測した内容で商品を登録」のシートを経てから購入の欄に入る。
抽出の一覧と単件の取得は purchases、products、shops の 3 つの結合を要する。

2026-10-13 に所有者は、TODO の「購入と商品の項目を統合したい」を受けて次を決めた (issue 0057、0058)。

- DB ごと統合する。
  purchases に商品名、Producer、Origin、Region、Process、Variety を持たせ、Flavor Notes も購入に直接付ける。
  products テーブル、商品の API、商品の画面、ナビゲーションの「商品」を無くす。
  既存のデータはマイグレーションで購入へ写す。
- 同じ商品を 2 回目以降に買うときは、購入の一覧と詳細の「この購入をもとに登録」から、商品の項目と店と Roast を引き継いだ新規のフォームを開く (issue 0058)。
  購入日と写真は引き継がない。
- 写真を選んで推測を受けたら、推測できた項目は入力済みの欄でも上書きする。
- 既存の商品のお気に入りは捨てる。
  購入が 1 件も無い商品も捨てる。

## 決定

- purchases に `name` (NOT NULL)、`producer`、`origin`、`region`、`process`、`variety` を持たせ、`product_id` を無くす。
- Flavor Notes の対応は `purchase_flavor_tags` (`user_id`、`purchase_id`、`tag_id`。主キーは `(purchase_id, tag_id)`) とする。
  中間テーブルも `user_id` を持つ (ADR-0006)。`flavor_tags` は変えない。
- `products` と `product_flavor_tags` と products のインデックス 4 つ (`idx_products_user_created_at`、`idx_products_user_favorited_at`、`idx_products_user_name`、`idx_products_user_updated_at`) を消す。
- 最終スキーマの利用者データのテーブルは次の 5 つとする (PRD の FR-14 のエクスポートと FR-15 のアカウント削除の対象)。
  ADR-0018 の最終スキーマの表の products、product_flavor_tags、purchases の行を本表が置き換える。

| テーブル | 主な列 | 参照 |
| --- | --- | --- |
| shops | id、user_id、店名、住所、created_at、updated_at、favorited_at | users |
| flavor_tags | id、user_id、タグ名 | users |
| purchase_flavor_tags | user_id、purchase_id、tag_id | users、purchases、flavor_tags |
| purchases | id、user_id、shop_id (NULL 許容)、purchased_on、roast、roast_date、price_amount、price_currency、weight_grams、photo_key、name、producer、origin、region、process、variety、created_at、updated_at、favorited_at | users、shops |
| brews | id、user_id、purchase_id、brewed_at、dose_grams、water_grams、water_temp_c、brew_time_seconds、method、grind_setting、rating、notes、created_at、updated_at、favorited_at | users、purchases |

- purchases のインデックスは `idx_purchases_user_purchased_on` (user_id、purchased_on DESC、id)、`idx_purchases_user_favorited_at`、`idx_purchases_user_price_amount`、`idx_purchases_user_weight_grams` の 4 つのままとする (ADR-0018 と同じ定義で作り直す)。
  購入の一覧の並び順のキー (`purchased_on`、`price_amount`、`weight_grams`) は変えない。
- 商品の API の 8 経路 (`/api/products` の一覧、登録、取得、更新、お気に入りの付け外し、削除、削除の影響) と、商品の一覧の名前の絞り込み (`name`) を消す。
  `GET /api/flavor-tags` は残す (タグは購入に付く)。
- 購入の登録 (`POST /api/purchases`) は `name` を必須 (前後の空白を除いて空なら 400) とし、`producer`、`origin`、`region`、`process`、`variety` (任意の文字列) と `flavor_notes` (任意のタグ名の配列。省略するとタグ無し) を受け付ける。
  更新 (`PATCH /api/purchases/:id`) は `name` を省略でき (変えない)、送るときは登録と同じ検証をする。
  5 項目の `null` は NULL にし、`name` と `flavor_notes` の `null` は 400 にする。
  `flavor_notes` は省略すると変えず、空の配列を送ると全て外す。タグの置き換えの規則は FR-8 のまま (同じ利用者の同じタグ名は 1 つ、空のタグ名は 400、参照されなくなったタグは残す) とする。
- 購入の応答 (一覧、単件、登録、更新) と抽出の応答の `purchase` は `product` の入れ子を持たず、`name`、`producer`、`origin`、`region`、`process`、`variety`、`flavor_notes` を購入の直下に持つ。
- サジェスト (FR-13) の Producer、Origin、Region、Process、Variety は purchases から引く。経路と項目名は変えない。
- 写真からの推測 (FR-19) の応答は `product` の入れ子をやめ、商品の項目を購入の項目と同じ直下に置く。
  推測できない項目は null、`flavor_notes` は推測できないときに空の配列とする。
  クライアントは、推測できた項目 (null でない項目) を入力済みの欄でも上書きし、null の項目は変えない。
  Flavor Notes は推測した配列が空でなければタグの集合を置き換え、空なら変えない。通貨は推測の対象でないため変えない。
  ADR-0016 (Workers AI の採用、無料枠、検証) は変えない。応答の形と適用の規則は PRD の FR-19 が定める。
- 購入の削除 (ADR-0020) は `purchase_flavor_tags` の対応も消し、`flavor_tags` の行は残す。
  商品の削除の経路と連鎖 (商品の購入、抽出、写真、タグの対応を消す) は無くなる。
- Frontend は商品の画面 (`/products`、`/products/new`、`/products/:id/edit`) とナビゲーションの「商品」を消し、ナビゲーションを抽出、購入、店、統計、設定の 5 項目にする。
  購入のフォームは商品名 (必須)、Producer、Origin、Region、Process、Variety (サジェスト付き)、Flavor Notes (タグ入力) を商品のピッカーの代わりに持つ。
  購入の詳細はこれらの項目を出し、購入と抽出の詳細の商品のタイルは無くなる。
- 既存のデータはマイグレーション `backend/brew_book/migrations/0005_merge_products_into_purchases.sql` で移す。
  購入の 6 項目は商品の値を写し、`purchase_flavor_tags` は `product_flavor_tags` をその商品の購入ごとに写す。
  購入が 1 件も無い商品は捨てる (購入日が無く購入にできない)。商品のお気に入りは引き継がない (商品 1 件に購入が複数あり、どの購入に付けるか決められない)。
  捨てる商品の件数は、staging と production への適用の前に `SELECT COUNT(*) FROM products WHERE id NOT IN (SELECT product_id FROM purchases)` で数えて issue に記録する。
- マイグレーションは purchases を作り直す。
  SQLite の `ALTER TABLE ... DROP COLUMN` は外部キー制約に使われている列 (`product_id`) を消せず、D1 は外部キー制約を常に検査するためである。
  `PRAGMA defer_foreign_keys = true` で brews の参照の違反をトランザクションの終わりまで遅らせ、移す行を制約の無い退避テーブル (`purchases_merged`) に入れ、purchases を `DROP TABLE` してから同じ名前で作り直し、退避した行を入れ直してから退避テーブルを消す。
- 本 ADR は ADR-0006 の「商品マスタと購入履歴を分ける」決定、`products` と `product_flavor_tags` の 2 テーブル、「商品のタグは配列で置き換える」規則、検討した選択肢の「購入ごとに 1 レコード (商品マスタ無し)」の却下を部分置き換えにする。
  データモデルの他の決定 (抽出は購入だけを参照する、`user_id` を全クエリの条件に含める、タグは利用者ごとに名前で一意、価格の持ち方など) は残る。
  ADR-0018 の最終スキーマの表と、ADR-0020 の商品の削除の記述も部分置き換えにする。
- PRD の用語、概要、背景と課題、目的、成功指標、ユースケース、スコープ、FR-5、FR-7 (欠番)、FR-8、FR-9、FR-11、FR-13、FR-14、FR-15、FR-19、FR-20、FR-21、FR-23、非機能要求、制約と前提、関連資料と、`README.md` の p95 の表と、`docs/design/` の原本を本 ADR に合わせて改訂する。
- 新しい API と権限は追加しない。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| 画面だけ統合する (DB と API に商品を残し、購入ごとに商品を 1 件自動で作る) | 使われない商品マスタと JOIN が残り、商品の API と画面の整合を保つ手間が続く。所有者が DB ごとの統合を選んだ |
| 購入のフォームに商品の項目を埋め込み、商品マスタは残す | 同上。商品の照合 (どの商品に紐づけるか) の規則が要る |
| 既存の商品のお気に入りを購入へ引き継ぐ | 商品 1 件に購入が複数あり、どの購入に付けるか決められない。所有者が捨てると決めた |
| 購入が 1 件も無い商品を購入にして残す | 購入日が必須で、作れない。所有者が捨てると決めた |
| 写真からの推測を空の欄にだけ反映する (従来の FR-19 の規則を保つ) | 所有者が上書きを選んだ。写真を選ぶ操作が推測を使う意思表示になる |
| 新しい purchases を別名 (`purchases_new`) で作り、古い purchases を DROP して RENAME する | SQLite は遅延した外部キー違反を件数で数え、親の行の削除 (`DROP TABLE` の暗黙の DELETE) で増やし、同じ名前の親のテーブルへの行の挿入で減らす。RENAME は件数を戻さないため、brews を持つ購入があると COMMIT が `FOREIGN KEY constraint failed` になる (2026-10-14 に SQLite 3.51.0 で確認) |
| brews も作り直す | 退避テーブルの方式で brews を触らずに済む。想定規模の抽出 30,000 件を書き直す必要が無い |
| 商品名を購入の一覧の並び順のキーに足す | TODO に無い。扱わない |

## 結果

- 購入のフォームが 1 つで完結し、写真からの推測は商品の照合と登録のシートを経ずに購入の欄へ直接入る。
- 同じ商品の 2 回目以降の購入は、商品の項目を購入ごとに持つ (重複を受け入れる)。
  再入力の手間は「この購入をもとに登録」 (issue 0058) で減らす。
- 購入と抽出の応答の形と、エクスポートの形 (`products` と `product_flavor_tags` が消え `purchase_flavor_tags` が増える) が変わる (後方互換の無い変更)。
- 抽出の一覧と単件の取得の結合は purchases と shops の 2 つになる。
- ナビゲーションが 5 項目になり、画面の数が 3 つ減る。E2E のスクリーンショット比較の対象から商品の画面が消える。
- 想定規模 (PRD の非機能要求) から商品 1,000 件が消える。
- 0005 の staging への適用は issue 0057 で所有者の承認を得て行い、production への適用は main への push で Workers Builds がデプロイの直前に行う (ADR-0021)。
