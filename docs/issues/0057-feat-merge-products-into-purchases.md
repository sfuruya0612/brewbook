# 商品を購入に統合し、商品のテーブルと API と画面を無くす

Created: 2026-10-14
Model: Claude Fable 5.1

## 背景

所有者の要望は TODO (`docs/issues/TODO.md`) の「購入と商品の項目を統合したい」である。
2026-10-13 の会話で所有者は次を決めた。

- DB ごと統合する。
  purchases に商品名、Producer、Origin、Region、Process、Variety を持たせ、Flavor Notes も購入に直接付ける。
  products テーブル、商品の API、商品の画面、ナビゲーションの「商品」を無くす。
  既存のデータはマイグレーションで購入へコピーする。
- 同じ商品を 2 回目以降に買うときは、購入の一覧と詳細の「この購入をもとに登録」から、商品の項目と店と Roast を引き継いだ新規フォームを開く。
  購入日と写真は引き継がない。
- 写真を選んで推測を受けたら、推測できた項目は入力済みの欄でも上書きする。
- 既存の商品のお気に入りは捨てる。
  購入が 1 件も無い商品も捨てる。

TODO の 2 つ目の小項目 (「この購入をもとに登録」) は issue 0058 で扱い、本 issue は残りの 3 つを扱う。

現状は次のとおりである。

- ADR-0006 は、同じ商品を複数回買う場合の扱いとして「商品マスタ + 購入履歴」に分けることを決め、検討した選択肢のうち「購入ごとに 1 レコード (商品マスタ無し)」を「同じ商品を 2 回買うと商品の項目が重複する」を理由に採らなかった。
  PRD (`docs/prd/brewbook.md`) の「## 背景と課題」は、この重複の説明が筆者の提示した選択肢であり、所有者が確認した事実ではないと書いている。
- スキーマ (`backend/brew_book/migrations/0001_initial_schema.sql`) では、`products` が商品名、producer、origin、region、process、variety を持ち、`purchases.product_id` が `products` を NOT NULL の外部キーで参照する。
  Flavor Notes は中間テーブル `product_flavor_tags` (主キー `(product_id, tag_id)`) で商品に付く。
  `favorited_at` は `0004_add_favorites.sql` で 4 テーブルに足した。
  `brews` は `purchases` だけを参照する。
- 購入は自前の名前を持たず、一覧 (`frontend/src/screens/records/purchase_list.rs`)、ホーム (`frontend/src/screens/records/home.rs`)、抽出のフォーム (`frontend/src/screens/records/brew_form.rs`) の行の題は `purchase.product.name` である。
- 商品の API は `backend/brew_book_core/src/routes.rs` の台帳に 8 経路ある (`products_list`、`products_create`、`products_get`、`products_update`、`products_favorite_put`、`products_favorite_delete`、`products_delete`、`products_delete_impact`)。
  処理は `backend/brew_book/src/records/products.rs` にあり、`ProductResponse` は購入と抽出の応答の入れ子 (`PurchaseResponse.product`、`backend/brew_book/src/records/purchases.rs`) にも使う。
  同ファイルの `NameFilter` は商品の一覧の名前の完全一致の絞り込み (`GET /api/products?name=`) で、写真からの推測の照合 (FR-19) だけが使う。
  SQL は `backend/brew_book_core/src/query.rs` の `PRODUCTS_TABLE`、`PRODUCT_COLUMNS`、`PRODUCT_SORT_KEYS`、`PURCHASES_FROM`、`BREWS_FROM` (products との JOIN)、`ListQuery.name` (商品の一覧だけが指定する)、`products_list`、`product_find`、`product_insert`、`product_update`、`product_delete`、`product_purchases_delete`、`product_brews_delete`、`product_purchase_photo_keys`、`product_delete_impact`、`product_flavor_notes`、`delete_product_flavor_tags`、`insert_product_flavor_tag` にある。
- 購入の登録は `product_id` を必須で受け取り (`purchases.rs` の `CreateInput`)、更新は省略できるが null を拒否する (`UpdateInput`)。
  参照先は `backend/brew_book/src/records/mod.rs` の `require_product` で検査する。
  Flavor Notes の付与は同ファイルの `attach_flavor_notes`、`replace_flavor_notes`、`flavor_notes`、`flavor_notes_for` が商品に対して行う。
- サジェスト (FR-13、`backend/brew_book/src/records/suggestions.rs`) の 8 項目のうち Producer、Origin、Region、Process、Variety の 5 項目は、`query.rs` の `SuggestionItem::table` が `PRODUCTS_TABLE` を返し、products テーブルから引く。
- 写真からの推測 (FR-19、`backend/brew_book/src/records/purchase_suggestions.rs`) の応答は商品の項目を `product` の入れ子に持つ (`backend/brew_book_core/src/suggestion.rs` の `ProductSuggestion`)。
  `ProductSuggestion.flavor_notes` は `Vec<String>` で、推測できないときは空の配列になる (null にはならない)。
  Frontend (`frontend/src/screens/records/purchase_form.rs`) は、推測を受けると `frontend/src/records/forms.rs` の `apply_purchase_suggestion` で Roast、Roast Date、価格、重量を空の欄にだけ入れる。
  商品が未選択のときに限り、推測した商品名で `GET /api/products?name=` を引き、同ファイルの `product_match` で一致した商品を選ぶか、「推測した内容で商品を登録」のシート (商品のフォームの重複) を出す。
  `frontend/pbt/tests/prop_records.rs` の `the_suggestion_never_overwrites_an_input` と `a_selected_product_is_never_replaced` がこの挙動を固定している。
  応答の形 (`product` の入れ子) と「空の入力欄にだけ反映し、入力済みの値を上書きしない」規則は PRD の FR-19 と issue 0034 が定めており、ADR-0016 は Workers AI の採用と無料枠と検証を決めるだけでこの 2 点を含まない。
- Frontend の商品の画面は `/products` (`product_list.rs`)、`/products/new` と `/products/:id/edit` (`product_form.rs`) で、`frontend/src/router.rs` の経路と `frontend/src/screens/records/mod.rs` の `NAV_ENTRIES` (6 項目) に入っている。
  購入のフォームは `RecordPickerSheet<Product>` で既存の商品を選ぶ。
  購入の詳細 (`purchase_detail.rs`) と抽出の詳細 (`brew_detail.rs`) は商品のタイルから `ProductEdit` へ移る。
  `frontend/src/records/api.rs` の `list_path` の `name` 引数は `products` だけが使う。
- エクスポート (FR-14、`backend/brew_book/src/export.rs`) の `ExportResponse` はトップレベルに `products` と `product_flavor_tags` の配列を持つ。
- 商品の削除 (FR-23、`products.rs` の `delete`) は購入、抽出、写真、Flavor Notes の対応を連鎖で消す。
  アカウント削除の順序は `query.rs` の `ACCOUNT_DELETE_TABLES` にある。
- スキーマの検査 (`backend/brew_book/tests/schema.rs`) は SQLite を起動せず、手書きのパーサで DDL を読む。
  扱える文は `CREATE TABLE`、`CREATE [UNIQUE] INDEX`、`DROP INDEX`、`ALTER TABLE ... ADD COLUMN` と `DROP COLUMN` だけで、それ以外は panic する。
  `the_migration_keeps_the_primary_keys` は `product_flavor_tags` だけを列の組の主キーとして特別扱いする。
- マイグレーションの適用の経路は 2 つある。
  `mise run db-migrate` (`wrangler d1 migrations apply --local`) と、`backend/brew_book/tests/d1_binding.rs` の `wrangler_the_migration_removing_the_archive_keeps_the_rows` が使う `tests/support/mod.rs` の `execute_sql_file` (`wrangler d1 execute --file`) である。
  後者は 0002 の状態で行を入れ、0003 以降を名前順に適用して行が残ることを確かめる。
- E2E (`backend/brew_book/tests/wrangler_same_origin_e2e.rs`) は画面 (`Products`、`ProductEdit` を含む) と部品のスクリーンショットを `docs/design/screenshots/` の原本と比較し、必須の比較の件数を検査する。
- デザインの原本で商品の画面、商品のタイル、商品の行、ピッカーの商品、「商品を登録」の文言、6 項目のドロワーとレールに触れるのは、`docs/design/README.md`、`components` の `RecordForms`、`Lists`、`ListRow`、`Detail`、`ReferenceTile`、`AppBar`、`WideLayout`、`Home`、`Button` の README、`tokens.json` の用法の記述、`Home`、`WideLayout`、`Lists`、`RecordForms`、`Detail`、`ReferenceTile` の `preview.html` である。
  これとは別に、`components` の `preview.html` のうち `Cover` を除く 19 個と `components/bundle.css` に共通の CSS のコメント「参照先のタイル (購入、商品、店をたどる)」がある。
- 統計 (`backend/brew_book/src/records/stats.rs`、`backend/brew_book_core/src/stats.rs`)、管理者 Worker (`backend/brew_book_admin`)、負荷試験 (`load/`) は商品を参照しない。

商品と購入を分けた理由 (商品の項目の重複を避ける) は、上記のとおり所有者が確認した事実ではない。
統合すると、購入のフォームが 1 つで完結し、写真からの推測も商品の照合と登録のシートを経ずに購入の欄へ直接入る。

## 目的

利用者は、商品を別の記録として登録せずに、購入のフォームで商品名、Producer、Origin、Region、Process、Variety、Flavor Notes を直接入力して購入を登録できる。

## 設計判断

所有者が 2026-10-13 に決めたことは次のとおりである (決定の記録は ADR-0022 に置く)。

- 統合は DB、API、画面の全てで行い、商品の記録を無くす。
  画面だけの統合 (DB と API に商品を残し、購入ごとに商品を 1 件自動で作る) と、購入のフォームへの商品の項目の埋め込み (商品マスタを残す) は、使われない商品マスタと JOIN が残るため採らない。
- 写真からの推測は、推測できた項目を入力済みの欄でも上書きする。
  推測できなかった項目は欄を消さない。
- 既存のデータの商品のお気に入りは引き継がない。
  購入が 1 件も無い商品は、購入日が無く購入にできないため捨てる。

本 issue を 0058 と分けた理由は、「この購入をもとに登録」が本 issue の後に独立して実装、検証、close でき、本 issue だけでも統合が完結するためである。
本 issue の残りは分けない。
スキーマ、API、画面、テスト、ドキュメントは同じ統合の決定を共有し、片方だけ close すると購入の応答の形と画面が食い違うためである。

筆者が置いた実装上の判断は次のとおりである。

- スキーマ: マイグレーション `backend/brew_book/migrations/0005_merge_products_into_purchases.sql` を足す。
  - purchases の列を `id、user_id、shop_id (NULL 許容)、purchased_on、roast、roast_date、price_amount、price_currency、weight_grams、photo_key、name (NOT NULL)、producer、origin、region、process、variety、created_at、updated_at、favorited_at` とする。
    `product_id` は無くす。
  - Flavor Notes の対応は `purchase_flavor_tags (user_id、purchase_id、tag_id)` とする (主キー `(purchase_id, tag_id)`。ADR-0006 の規則に従い `user_id` も持つ)。
    `flavor_tags` は変えない。
  - SQLite の `ALTER TABLE ... DROP COLUMN` は外部キー制約に使われている列 (`product_id`) を消せないため、purchases は作り直す。
    `PRAGMA defer_foreign_keys = true` を先頭に置く。
    新しい purchases (`purchases_new`) を作り、既存の行と products の列を `INSERT ... SELECT` でコピーする (既存の全列を列名で明示して移す)。
    `purchase_flavor_tags` を `product_flavor_tags` と purchases の JOIN から作る。
    古い purchases を DROP して `purchases_new` を purchases に RENAME し、purchases のインデックス (`idx_purchases_user_purchased_on`、`idx_purchases_user_favorited_at`、`idx_purchases_user_price_amount`、`idx_purchases_user_weight_grams`) を作り直す。
    `product_flavor_tags` と `products` とそのインデックスを DROP する。
    brews の外部キー (`REFERENCES purchases (id)`) は、DROP の時点の違反を `defer_foreign_keys` でトランザクションの終わりまで遅らせ、RENAME 後の purchases が満たす。
    D1 は全てのクエリとマイグレーションで外部キー制約を検査し、`PRAGMA foreign_keys` は変えられず、`PRAGMA defer_foreign_keys` を同じトランザクションの終わりまで有効にする (2026-10-13 に Cloudflare のドキュメント `d1/sql-api/foreign-keys` と `d1/reference/migrations` で確認)。
    マイグレーションのファイル全体が 1 つのトランザクションで適用されることは、2 つの経路 (`mise run db-migrate` と `d1_binding.rs` の `wrangler d1 execute --file`) の両方で、brews を持つ購入が残ることを確かめて検証する。
    どちらかの経路で 1 つのトランザクションにならない場合は、その事実と代替案 (brews も作り直す) を issue に書いてから進める。
  - `schema.rs` のパーサに `PRAGMA`、`INSERT ... SELECT`、`DROP TABLE`、`ALTER TABLE ... RENAME TO` の解釈を足す。
    `the_migration_keeps_the_primary_keys` の列の組の主キーの特別扱いを `purchase_flavor_tags` に変える。
  - `d1_binding.rs` に、0004 の状態で店、購入のある商品と無い商品、購入が 2 件ある商品、Flavor Notes 付きの商品、写真のキーとお気に入りと店と価格を持つ購入、価格の無い購入、抽出を入れ、0005 以降を順に適用した後の状態を確かめるテストを足す。
    購入が 2 件ある商品は、`INSERT ... SELECT` と `purchase_flavor_tags` の JOIN で行が増える誤りを検出するために入れる。適用の後も購入は 2 件のままで、それぞれが商品の 6 項目と Flavor Notes を持つことを確かめる。
  - 購入が 1 件も無い商品の件数は、staging と production に適用するときに `SELECT COUNT(*) FROM products WHERE id NOT IN (SELECT product_id FROM purchases)` で数えて記録する。
    staging への適用は本 issue の完了条件に含める。所有者の承認を得てから (本 issue の運用。グローバル規約の外部リソースの変更の扱い)、`README.md` のデプロイ手順と `mise.toml` が定める順で `mise run db-migrate-staging` でスキーマを適用し、続けて `mise run deploy-staging` でデプロイする (新しいコードは purchases の新しい列を読むため、適用せずにデプロイすると購入の経路が壊れる)。適用の直前に staging の件数を `backend/brew_book` で `wrangler d1 execute DB --env staging --remote --command "SELECT COUNT(*) FROM products WHERE id NOT IN (SELECT product_id FROM purchases)"` を実行して数え (`README.md` の「リモートに触れるコマンドは必ず `--env` を付け」に従う)、close 時の「## 解決方法」に記録する。
    production への適用は本 issue の範囲外である。main への push で Workers Builds がデプロイの直前にマイグレーションを適用する (ADR-0021) ため、本 issue で production を操作しない。production の件数は所有者が push の前に同じ SQL (`--env production --remote`) で数えて記録する (リリースの手順であり、本 issue の完了条件ではない)。
    却下した案: 0055 のように「staging での確認: 未実施」と記録して close する案。マイグレーションを適用せずにデプロイできず、FR-19 の staging での確認 (PRD の成功指標) がマイグレーション適用を前提にするため、未実施のまま close すると成功指標を確かめられない。
- API:
  - 商品の 8 経路を台帳とスイートから消す。
    `GET /api/flavor-tags` は残す (タグは購入に付く。FR-8 のタグの一覧の要求は残る)。
    `NameFilter`、`ListQuery.name`、`list_qualified` の名前の条件、`list_path` の `name` 引数は使い手が無くなるため消す。
  - 購入の登録と更新の入力から `product_id` を消し、`name` (登録では必須。前後の空白を除いて空なら 400)、`producer`、`origin`、`region`、`process`、`variety` (任意。文字列)、`flavor_notes` (任意。タグ名の配列) を足す。
    更新では `name` を省略でき (変えない)、送るときは登録と同じ検証をする。`producer`、`origin`、`region`、`process`、`variety` に `null` を送ると NULL にする (商品の `UpdateInput` と同じ扱い)。`name` と `flavor_notes` の `null` は 400 にする (商品の `UpdateInput` と同じ)。省いた項目は変えない。
    `flavor_notes` は、更新で省略すると変えず、空の配列を送ると全て外す。
    登録で省略するとタグ無しになる。
  - 購入の応答から `product` の入れ子を消し、`name`、`producer`、`origin`、`region`、`process`、`variety`、`flavor_notes` を購入の直下に置く。
    抽出の応答の `purchase` も同じ形にする。
  - 購入の一覧の並び順のキー (`PURCHASE_SORT_KEYS`) は変えない。
    商品の一覧の名前の並び替えを購入に引き継ぐことは TODO に無いため扱わない。
  - サジェストの Producer、Origin、Region、Process、Variety は purchases テーブルから引く。
    経路と項目名は変えない。
  - 写真からの推測の応答は `product` の入れ子をやめ、`name`、`producer`、`origin`、`region`、`process`、`variety`、`flavor_notes` を直下に置く (購入の応答と同じ形)。
    `flavor_notes` は配列のままとし、推測できないときは空の配列にする (現行と同じ)。
    商品の項目が 1 つも推測できないときに `product` を null にする規則は無くなる。
    モデルへのプロンプトと検証 (`suggestion.rs`) も同じ形にする。
  - 購入の削除で `purchase_flavor_tags` の対応も消す。
    `flavor_tags` の行は残す (ADR-0006)。
    削除の影響の件数 (抽出の件数) は変えない。
  - エクスポートの `products` と `product_flavor_tags` を消し、`purchase_flavor_tags` を足す。
    purchases の行は新しい列を全て持つ。
    `ACCOUNT_DELETE_TABLES` を新しいテーブルの並びにする。
- Frontend:
  - 商品の画面 (`product_list.rs`、`product_form.rs`)、経路 (`/products`、`/products/new`、`/products/:id/edit`)、`NAV_ENTRIES` の「商品」 (6 項目から 5 項目にし、レールの `active` の添字を詰める)、`RecordsApi` の商品の関数、`Product` と `ProductInput` の型、商品に関する i18n のキーを消す。
  - 購入のフォームは、商品のピッカーの代わりに商品名 (必須)、Producer、Origin、Region、Process、Variety (5 項目はサジェスト付き)、Flavor Notes (タグ入力) を持つ。
    項目の並びは商品名、Producer、Origin と Region、Process と Variety、Flavor Notes、店、購入日、Roast、Roast Date、価格と通貨、重量、写真とする。
    「推測した内容で商品を登録」のシートは消す。
  - 購入の詳細は商品名、Producer、Origin、Region、Process、Variety、Flavor Notes を出す。
    購入と抽出の詳細の商品のタイルは消す。
  - 写真からの推測の適用 (`apply_purchase_suggestion`) は、推測した値が null でない項目を入力済みでも上書きし、null の項目は変えない。
    Flavor Notes は、推測した配列が空でなければタグの集合を置き換え、空なら変えない (写真に Flavor Notes が無いときに手で入れたタグを消さないため)。
    通貨は推測しないため変えない。
    `product_match`、`ProductMatch`、`suggested_product_name` は消す。
- ドキュメント:
  - ADR-0022 (商品を購入に統合する) を足し、ADR-0006 の「商品マスタと購入履歴を分ける」決定と `products`、`product_flavor_tags` の 2 テーブルを置き換える。
    ADR-0018 の最終スキーマの表と ADR-0020 の商品の削除の記述に、ADR-0022 による変更の注記を足す。
    ADR-0016 は変えない (推測の応答の形と上書きの規則は PRD の FR-19 と issue 0034 が定めており、ADR-0016 の決定ではない)。
  - PRD の用語、概要、背景と課題、目的、成功指標 (p95 の対象と FR-19 の staging での確認の基準)、ユースケース、スコープ、FR-5、FR-7、FR-8、FR-9、FR-11 (抽出の応答の `purchase` の中の `product`)、FR-13、FR-14、FR-15、FR-19、FR-20、FR-21、FR-23、非機能要求の性能 (p95 の対象) と想定規模、制約と前提、関連資料を更新し、「2026-10-13 に所有者は…」の決定の記録を足す。
    `README.md` の p95 の表 (「店、商品、購入、抽出の一覧と単件の取得」) も直す。
  - デザインの原本を更新する。
    `docs/design/README.md`、`components` の `RecordForms`、`Lists`、`ListRow`、`Detail`、`ReferenceTile`、`AppBar`、`WideLayout`、`Home`、`Button` の README、`tokens.json` の用法の記述、`Home`、`WideLayout`、`Lists`、`RecordForms`、`Detail`、`ReferenceTile` の `preview.html` から商品の画面、商品のタイル、`ListRow/README.md` の「商品の行」、`ReferenceTile/README.md` のピッカーの「商品」、`docs/design/README.md` の記録の種類の印の「商品は `Icons.spa_outlined`」、「商品を登録」の文言を消し、ドロワーとレールを 5 項目にする。
    `components` の `preview.html` のうち `Cover` を除く 19 個と `components/bundle.css` の CSS のコメント「参照先のタイル (購入、商品、店をたどる)」を「参照先のタイル (購入、店をたどる)」にし、写しの `frontend/src/ui/design.css` の同じコメントも追随させる (0048 と同じく原本を先に更新する)。
    E2E のスクリーンショット比較の差分を確認し、意図した差分は `docs/design/screenshots/` の原本を更新し、意図しない差分は直す (比較の結果を issue に記録する。0047、0050、0056 と同じ扱い)。
    E2E の `screens()` から `Products` と `ProductEdit` を、`form_screens()` から `ProductNew` と `ProductEdit` を外し、`nav_menu` の項目を 5 つにし、`register_sheet_scrolls` (「推測した内容で商品を登録する」のシート) を消し、読まれなくなる `Fixture.product_id` を消す (`cargo clippy -D warnings` の未使用の警告を避ける)。必須の比較の件数の計算は変えない。
    `COMPONENTS` の `Chip` の取得元は `screen: "Products"` であるため、`PurchaseDetail` (Flavor Notes の `TagChip` が `.chip` を持つ) に移し、設計の 10 部品の比較を保つ (`COMPONENTS` は 8 件のままで、`Feedback` と `Button` は別に取る)。E2E の下ごしらえの購入はマイグレーションの後に Flavor Notes を持つ (`product_flavor_tags` から `purchase_flavor_tags` へ移る) ため、`.chip` が取れる。
    却下した案: `COMPONENTS` から `Chip` を消す案。設計の 10 部品の比較が 9 件に減り、`docs/design/screenshots/Chip-*.png` が未使用になる。
- 扱わないこと: 「この購入をもとに登録」 (issue 0058)、サジェストへの商品名の追加、購入の一覧の商品名での並び替えと絞り込み、既存の商品のお気に入りの引き継ぎ、production へのマイグレーションの適用と production の購入の無い商品の件数の記録 (main への push で Workers Builds が適用する。ADR-0021)、負荷試験 (`load/`) と管理者 Worker の変更。

## 完了条件

- `backend/brew_book/migrations/0005_merge_products_into_purchases.sql` があり、ローカルの D1 (`mise run db-migrate`) に適用できる。
  適用の前に店、購入のある商品と無い商品、購入が 2 件ある商品、Flavor Notes 付きの商品、写真のキーとお気に入りと店と価格を持つ購入、価格の無い購入、抽出を入れたローカルの D1 で、適用の後に購入が商品の 6 項目と Flavor Notes を持ち、購入が 2 件ある商品の購入が 2 件のままそれぞれ商品の 6 項目と Flavor Notes を持ち、既存の列 (`shop_id`、`purchased_on`、`roast`、`roast_date`、`price_amount`、`price_currency`、`weight_grams`、`photo_key`、`created_at`、`updated_at`、`favorited_at`) の値が変わらず、抽出が購入を参照したまま残り、`products` と `product_flavor_tags` が無いことを `wrangler d1 execute --local` で確かめる (手順と結果を close 時の「## 解決方法」に書く)。
- `backend/brew_book/tests/d1_binding.rs` に、上と同じ下ごしらえを 0004 の状態で入れて 0005 以降を `execute_sql_file` で順に適用し、同じ結果を確かめるテストがある。
- `backend/brew_book/tests/schema.rs` のパーサが `PRAGMA`、`INSERT ... SELECT`、`DROP TABLE`、`ALTER TABLE ... RENAME TO` を解釈し、新しいスキーマ (purchases の列、`purchase_flavor_tags` の列と主キーと外部キー、purchases のインデックス 4 つ、`products` と `product_flavor_tags` が無いこと) を検査する。
- 台帳 (`backend/brew_book_core/src/routes.rs`) とスイート (`backend/brew_book/tests/support/mod.rs`) から商品の 8 経路が消え、`GET /api/flavor-tags` は残る。
  台帳とスイートが一致する。
- `POST /api/purchases` と `PATCH /api/purchases/:id` が `name`、`producer`、`origin`、`region`、`process`、`variety`、`flavor_notes` を受け付け、`product_id` を受け付けない (未知の項目の扱いは既存の入力と同じ)。
  `POST` では `name` が必須で、無いか空か空白だけなら 400 になる。
  `PATCH` では `name` を省くと変えず、送った `name` が空か空白だけなら 400、`producer`、`origin`、`region`、`process`、`variety` の `null` は NULL、`name` と `flavor_notes` の `null` は 400、省いた項目は変えない。
  `flavor_notes` の置き換えは FR-8 の規則 (配列で置き換え、同じ利用者の同じタグ名は 1 つ、空のタグ名は 400、参照されなくなったタグは残す) に従う。
- 購入の応答 (一覧、単件、登録、更新) と抽出の応答の `purchase` が `product` の入れ子を持たず、`name`、`producer`、`origin`、`region`、`process`、`variety`、`flavor_notes` を直下に持つ。
- 購入の一覧の `sort` が受け付けるキーは `purchased_on`、`price_amount`、`weight_grams` のままである。
- サジェストの Producer、Origin、Region、Process、Variety が購入の値から返る。
- `POST /api/purchase-suggestions` の応答が `product` の入れ子を持たず、`name`、`producer`、`origin`、`region`、`process`、`variety`、`flavor_notes`、`roast`、`roast_date`、`price_amount`、`weight_grams` を直下に持つ。
  推測できない項目は null、`flavor_notes` は推測できないときに空の配列になる。
- 購入の削除で `purchase_flavor_tags` の対応が消え、`flavor_tags` の行は残る。
- `GET /api/export` の応答が `shops`、`flavor_tags`、`purchase_flavor_tags`、`purchases`、`brews` の 5 配列を持ち、`products` と `product_flavor_tags` を持たない。
  purchases の行が新しい列を全て持つ。
- アカウント削除が `purchase_flavor_tags` を含む新しいテーブルの並びで全行を消す。
- `ListQuery.name`、`list_qualified` の名前の条件、`NameFilter`、`list_path` の `name` 引数が無い。
- Frontend に `/products`、`/products/new`、`/products/:id/edit` の経路と商品の画面が無く、ナビゲーションの項目が抽出、購入、店、統計、設定の 5 つである。
- 購入のフォームが商品名 (必須)、Producer、Origin、Region、Process、Variety (サジェスト付き)、Flavor Notes (タグ入力) を持ち、商品のピッカーと「推測した内容で商品を登録」のシートが無い。
  商品名が空のときの保存は検証エラーになり、API を呼ばない。
- 購入の詳細が商品名、Producer、Origin、Region、Process、Variety、Flavor Notes を出し、購入と抽出の詳細に商品のタイルが無い。
- 写真からの推測で、推測した値が null でない項目は入力済みの欄でも上書きされ、null の項目は変わらない。
  Flavor Notes は推測した配列が空でなければ置き換わり、空なら変わらない。
  通貨は変わらない。
- SQL の組み立ての単体テスト (`backend/brew_book_core/tests/test_query.rs`) が、購入の INSERT と UPDATE の新しい列、products との JOIN が無いこと、`purchase_flavor_tags` の INSERT と DELETE と SELECT、購入の削除の連鎖、サジェストのテーブル名を確かめる。
  商品の SQL と名前の絞り込みのテストは消す。
  新しい SQL が `backend/brew_book/tests/test_queries.rs` の検査の対象に入る。
- 結合テスト (`backend/brew_book/tests/wrangler_records_api.rs`、`wrangler_purchases_brews_api.rs`、`wrangler_suggestions_api.rs`、`wrangler_purchase_suggestions_api.rs`、`wrangler_export_api.rs`、`wrangler_account_api.rs`、`wrangler_records_scale.rs`、`wrangler_stats_api.rs`、`wrangler_stats_scale.rs`、`wrangler_same_origin_e2e.rs`、`wrangler_photos_api.rs`、`d1_binding.rs`) と `backend/brew_book/tests/support/seed.rs` が新しい形に合う。
  商品の API のテストが消え、購入の商品の項目と Flavor Notes の登録、更新、応答、削除の連鎖、エクスポート、アカウント削除、サジェストのテストがある。
  `wrangler_records_scale.rs` の想定規模から商品 1,000 件を外す。
- `backend/brew_book_core/tests/test_suggestion.rs` と `backend/pbt/tests/prop_suggestion.rs`、`prop_query.rs` が新しい形に合う。
- Frontend の単体テスト (`frontend/tests/test_records_api.rs`、`test_records_forms.rs`、`test_records_display.rs`、`test_records_screens.rs`、`test_router.rs`) とブラウザテスト (`test_records_lists_web.rs`、`test_records_screens_web.rs`、`test_app_nav_web.rs`) と `frontend/pbt/tests/prop_records.rs` が新しい形に合う。
  商品のテストが消え、購入のフォームの商品の項目、推測の上書き (値のある項目は上書き、null の項目は保持、Flavor Notes は空の配列なら保持) の PBT、ナビゲーションの 5 項目のテストがある。
- i18n のキーの増減に合わせて `frontend/src/i18n/keys.rs` の `KEY_COUNT` と `ALL`、`ja.rs` と `en.rs` の表、`frontend/tests/test_i18n.rs` の期待値を更新する。
- E2E の `screens()` と `form_screens()` に商品の画面が無く、`nav_menu` が 5 項目で、`register_sheet_scrolls` と `Fixture.product_id` が無く、`COMPONENTS` の `Chip` の取得元が `PurchaseDetail` で、`COMPONENTS` が 8 件のままで、必須の比較の件数の検査が通る。
  スクリーンショット比較の差分を確認し、意図した差分で `docs/design/screenshots/` の原本を更新し、比較の結果を close 時の「## 解決方法」に記録する。
- デザインの原本 (「## 設計判断」に列挙した README、`tokens.json`、`preview.html`、`bundle.css`) に商品の画面、商品のタイル、商品の行、ピッカーの商品、「商品を登録」の文言、6 項目のドロワーとレール、CSS のコメントの「商品」が無く、`frontend/src/ui/design.css` の同じコメントにも「商品」が無い。
  `docs/design` 配下で「商品」を含む記述は、購入の項目の「商品名」だけである。
- ADR-0022 がある。
  ADR-0006、ADR-0018、ADR-0020 の Status と本文に ADR-0022 による置き換えの注記がある。
  ADR-0016 は変わらない。
  PRD と `README.md` が「## 設計判断」のとおり更新されている。
- 所有者の承認の後、staging の購入の無い商品の件数を `backend/brew_book` で `wrangler d1 execute DB --env staging --remote` を実行して数えて記録し、`mise run db-migrate-staging` で 0005 を適用し、`mise run deploy-staging` でデプロイする。適用と件数を close 時の「## 解決方法」に記録する。
  承認が得られるまでは close しない (0055 と同じ扱い)。
- staging へのデプロイの後、実写真を `POST /api/purchase-suggestions` に送り、応答が新しい形 (直下の項目と、推測できない項目の null、`flavor_notes` の配列) を満たすことを確かめ、結果を close 時の「## 解決方法」に記録する (PRD の成功指標の FR-19 の確認)。
- `CHANGES.md` の `## develop` に `[CHANGE]` のエントリがある。
- 扱わない範囲: 「この購入をもとに登録」 (issue 0058)、サジェストへの商品名の追加、購入の一覧の商品名での並び替えと絞り込み、既存の商品のお気に入りの引き継ぎ、production へのマイグレーションの適用と production の購入の無い商品の件数の記録、負荷試験と管理者 Worker の変更は扱わない。
- `mise run check` が通る。

## 関連

- issue 0058 (「この購入をもとに登録」。本 issue の統合の後に実装する。TODO の 2 つ目の小項目に対応する)。
- ADR-0006 (データモデル)、ADR-0016 (写真からの推測)、ADR-0018 (最終スキーマ)、ADR-0020 (記録の削除)。
- issue 0034 (写真からの推測の商品の照合と登録の導線を本 issue で消す)、0056 (商品の削除の連鎖を本 issue で消す)。
