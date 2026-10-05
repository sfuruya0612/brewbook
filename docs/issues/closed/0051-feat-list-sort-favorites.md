# 購入、商品、店、抽出の一覧に並び替えとお気に入りを追加する

Created: 2026-10-03
Model: DeepSeek V4.1 Flash
Completed: 2026-10-05

## 背景

4 つの一覧の並び順は API が固定し、お気に入りを持つ仕組みが無い。

- 一覧は `frontend/src/screens/records/home.rs` (抽出)、`purchase_list.rs` (購入)、`product_list.rs` (商品)、`shop_list.rs` (店) が `frontend/src/screens/records/list_view.rs` の `RecordListView` を使う。同じ `RecordListView` は `frontend/src/screens/records/picker.rs` の `RecordPickerSheet` も使い、選択のシートでは `show_archived_toggle: false` を渡している。
- 並び順は `backend/brew_book_core/src/query.rs` の `shops_list`、`products_list`、`purchases_list`、`brews_list` が固定する (店と商品は `created_at` の降順、購入は `purchased_on` の降順、抽出は `brewed_at` の降順。同順位は ID の昇順)。
- カーソルは `backend/brew_book_core/src/cursor.rs` の `CursorKey` が日時と日付だけを運ぶ。
- ツールバーにあるのは 0050 で削除する「アーカイブ済みを含める」の切り替えだけである (`list_view.rs`)。
- `frontend/src/api/api_client.rs` には `get_json`、`post_json`、`patch_json`、`delete_json` があり、`PUT` を送るメソッドが無い。
- 所有者の要望は、購入、商品、ショップ、抽出のそれぞれにソート機能とお気に入り機能を付けることである。回答で、お気に入りは「星 + お気に入りのみ表示 (詳細画面にも星を置く)」、ソートは「各一覧 3 キー (昇順/降順の切り替え付き)」が選ばれた。

## 目的

4 つの一覧で、選んだキーと昇順または降順で並べ替えられ、お気に入りの記録だけに絞り込める。
各記録に星を付け外しでき、一覧と詳細の両方から操作できる (詳細画面の星は所有者の回答に含まれる)。

## 設計判断

- データベース: `backend/brew_book/migrations/0004_add_favorites.sql` を追加し、`shops`、`products`、`purchases`、`brews` に `favorited_at TEXT` (NULL は未設定) を足す。絞り込み用に `(user_id, favorited_at)` の 4 つのインデックスを足す。最終スキーマの台帳である ADR-0018 の表にも `favorited_at` を追記する。
- API の一覧: クエリパラメータに `sort` (並び順のキー)、`order` (`asc` または `desc`。既定は `desc`)、`favorite` (`true` のときだけお気に入りに絞る。既定は `false`) を足す。受け付ける `sort` のキーは次のとおりとし、それ以外は 400 にする。
  - 抽出 (`/api/brews`): `brewed_at` (既定)、`rating`、`dose_grams`
  - 購入 (`/api/purchases`): `purchased_on` (既定)、`price_amount`、`weight_grams`
  - 商品 (`/api/products`): `created_at` (既定)、`name`、`updated_at`
  - 店 (`/api/shops`): `created_at` (既定)、`name`、`updated_at`
- カーソル: `CursorKey` を、並び順のキーの名前 (`sort` と同じ値)、方向 (`order` と同じ値)、キーの値 (NULL のときは null)、ID を持つ形に広げる。キーの値の型はキーごとに決まる (日時と日付は文字列、数値は数値、`name` は文字列)。`sort` と `order` の組がカーソルと一致しないときは 400 にする (同じ一覧の中で `rating` と `dose_grams` のような同じ型のキーを区別し、方向の変更で前のカーソルを送ったときも重複や漏れを返さないため)。復号できない値も 400 にする。既存のカーソルとの互換は取らない (一覧の並び順を選べるようにするための後方互換を壊す変更)。
- 並び順の SQL: `ORDER BY キー <方向> NULLS LAST, id ASC` にする。数値のキー (`rating`、`dose_grams`、`price_amount`、`weight_grams`) は NULL を取り得るため、NULL は常に末尾にする。`name` の比較は索引、`ORDER BY`、カーソルの比較の 3 か所で `COLLATE NOCASE` に揃える。続きの条件は、カーソルのキーが NULL のときは `キー IS NULL AND id > ?`、それ以外は `(キー IS NULL) OR キー <比較> ? OR (キー = ? AND id > ?)` にする (方向で `<` と `>` を入れ替える)。
- 並び順のキーのインデックスを足す (`(user_id, name COLLATE NOCASE)`、`(user_id, updated_at DESC, id)`、`(user_id, price_amount)`、`(user_id, weight_grams)`、`(user_id, rating)`、`(user_id, dose_grams)`)。`NULLS LAST` と `COLLATE NOCASE` の比較のため索引が常に効くとは限らないが、想定規模 (1 利用者あたり購入 3,000 件、抽出 30,000 件) では許容し、性能の作り込みはしない。
- お気に入りの操作: `PUT /api/<資源>/:id/favorite` と `DELETE /api/<資源>/:id/favorite` の 8 経路を足す。`PUT` は `favorited_at` を現在時刻にし、`DELETE` は NULL にする。既に同じ状態のときは `favorited_at` と `updated_at` を変えずに 200 を返す (繰り返し呼んでも状態と応答が同じ)。認証が必要で、存在しない ID と他の利用者の ID は 404 にする。
- 応答: 4 つの記録の応答に `favorited_at` (文字列または null) を足す。結合した購入と抽出の応答では、入れ子の `product` と `shop` にも `favorited_at` を含める。エクスポート (FR-14) では、`favorited_at` を持つ店、商品、購入、抽出の 4 テーブルの列に含める (タグの 2 テーブルは持たない)。
- Frontend: `RecordListView` のツールバーに並び順の選択 (`select`)、昇順/降順の切り替え、お気に入りのみの切り替えを置く。並び順の選択肢は画面ごとに渡す。ツールバーを出すかどうかの prop を残し、選択のシート (`RecordPickerSheet`) では出さない (選択のシートは既定の並び順で、お気に入りの絞り込みを使わない)。並び順、方向、お気に入りのみを変えたときは、カーソルを捨てて先頭から読み直す (既存の `toggle_include_archived` と同じ)。行の右端に星のボタンを置き、押すと API を呼んで一覧を先頭から読み直す (`mark_records_changed` を使う)。星の押下は行の押下 (詳細を開く) を発火させない。キーボードでは、星のボタンの Enter と Space でも行の `onkeydown` に伝播させない。抽出と購入の詳細の画面、商品と店のフォームの画面にも星を置く。詳細の星は所有者の回答に含まれる。商品と店には詳細の経路が無い (`frontend/src/router.rs`) ため、フォームの画面を詳細と同じ位置づけにして星を置く。商品と店のフォームは新規と編集を兼ねるため、星は編集の画面 (id があるとき) にだけ置く (新規の登録では ID が無いため出さない。既存のアーカイブのボタンと同じ扱い)。
- 一覧の読み込みの型 (`RecordListView` の `RecordLoader`) は、カーソルに加えて並び順とお気に入りの条件を受け取る形にする。`RecordsApi` の一覧のメソッドは条件の型 (`ListOptions` など) を受け取る。`frontend/src/api/api_client.rs` に `put_json` を足す。
- i18n: `SortLabel`、`SortAscending`、`SortDescending`、`FavoritesOnlyLabel`、`FavoriteAddLabel`、`FavoriteRemoveLabel`、`SortCreatedAt` (「登録日」)、`SortUpdatedAt` (「更新日」) の 8 キーを足す。並び順のキーの名前は既存のキー (`BrewedAtLabel`、`PurchasedOnLabel`、`PriceLabel`、`WeightLabel`、`ProductNameLabel`、`ShopNameLabel`、`RatingLabel`、`DoseLabel`) を使う。
- PRD: 機能要求 FR-20 (一覧の並び替え) と FR-21 (お気に入り) を追加し、スコープの「やること」と用語に足す。成功指標の応答時間の行は店、商品、購入、抽出の一覧の API を対象にしており、`sort`、`order`、`favorite` はそのパラメータなので、新しい指標の行は足さず、性能の節の一覧の対象にこの 3 つのパラメータを含むことを明記する。
- 原本 (`docs/design/`) を先に更新する。`docs/design/components/Lists/README.md`、`components/ListRow/README.md`、`components/Home/README.md`、`components/bundle.css` (並び順の `select` と星の見た目)、各 `preview.html` を更新する。

採らなかった案は次のとおりである。

- 読み込み済みの行だけをクライアントで並べ替える: ページングの途中で全体の並びと食い違い、正しい並びにならないため却下する。
- お気に入りを端末の localStorage にだけ保存する: 他の端末と共有されず、一覧の絞り込みにも使えないため却下する (所有者の回答で却下)。
- お気に入りを先頭に並べるだけの並び順にする: 所有者の回答は「お気に入りのみ表示」であり、絞り込みの方が要望に合うため却下する。
- お気に入りを `PATCH` の本体で更新する: 一覧の星の切り替えがフォーム全体の値を送ることになり、他の項目を上書きする危険があるため却下する。
- お気に入りを真偽値の列 (`favorite INTEGER NOT NULL DEFAULT 0`) にする: D1 の結果は数値で返るため、真偽値への変換の補助が要る。0050 まで `archived_at` が使っていた「NULL で未設定」の形と同じにすれば、型の変換が要らず、いつお気に入りにしたかも残せるため却下する。
- カーソルに並び順のキー名を含めず、`sort` を変えたときに前のカーソルを受け付ける: 同じ型のキーを区別できず、重複や漏れのあるページが返るため却下する。

追加の API 呼び出しと権限は、上の 8 経路だけである。認証は既存のセッションを使う。

## 完了条件

- `backend/brew_book/migrations/0004_add_favorites.sql` が 4 テーブルに `favorited_at` を足し、4 つの絞り込みのインデックスと、並び順のキーのインデックスを足す (スキーマのテスト)。ADR-0018 の表にも `favorited_at` が追記されている。
- 4 つの一覧の API が `sort`、`order`、`favorite` を受け付け、指定したキーと方向で並べ、`favorite=true` でお気に入りだけを返す。許さない `sort`、`asc` と `desc` 以外の `order`、`true` と `false` 以外の `favorite` は 400 を返す (各一覧の正常系と入力不正のテスト)。
- 4 つの一覧のカーソルのページングが、全てのキーと両方の方向で、重複と漏れなく続きを返す。`sort` と `order` の組がカーソルと一致しないときは 400 を返す (既知の並びのテストと、カーソルの符号化と復号の PBT)。
- `PUT` と `DELETE /api/<資源>/:id/favorite` の 8 経路がお気に入りを付け外し、既に同じ状態のときは `favorited_at` と `updated_at` を変えずに 200 を返す。未認証は 401、他の利用者の ID は 404 を返す。8 経路を `backend/brew_book_core/src/routes.rs` の `ROUTES` と `backend/brew_book/tests/support/mod.rs` の `SUITE` に登録する (経路ごとのテストと、台帳の照合のテスト)。
- 4 つの記録の応答と、購入と抽出の入れ子の `product` と `shop` の応答に `favorited_at` が含まれる (API のテスト)。
- エクスポート (FR-14) の店、商品、購入、抽出の 4 テーブルの列に `favorited_at` が含まれる (`wrangler_export_api.rs` の更新)。
- Frontend の 4 つの一覧に並び順の選択と昇順/降順の切り替えとお気に入りのみの切り替えがあり、操作するとカーソルを捨てて API のパラメータが変わる (Frontend の自動テスト)。
- 選択のシート (`RecordPickerSheet`) には並び順とお気に入りの操作が出ない (Frontend の自動テスト)。
- 一覧の行と、抽出と購入の詳細と、商品と店の編集の画面の星でお気に入りを切り替えられ、星の押下と星への Enter と Space では詳細を開かない。商品と店の新規の登録の画面には星が出ない (Frontend の自動テスト)。
- お気に入りのみの切り替えで、お気に入りでない行が表示されない (Frontend の自動テスト)。
- i18n の 8 キーが足され、`KEY_COUNT` と日本語と英語の表の要素数が一致する (`test_i18n.rs` の更新)。
- PRD に FR-20 と FR-21 が追加され、スコープ、用語、性能の節が更新されている。`docs/design/` の一覧と行の原本 (README、`bundle.css`、`preview.html`) が新しい操作と星の見た目になっている。
- E2E のスクリーンショット比較の差分を確認し、意図した差分は `docs/design/screenshots/` を更新し、意図しない差分は直す (比較の結果を issue に記録する)。
- `mise run check` が通過する。

## 関連

- 0050 の完了後に着手する (一覧のツールバーと `include_archived` を置き換えるため)。
- 0039 のデザインシステムの `ListRow` と `IconButton` を拡張する。
- 0047 のヘッダーの変更とは独立である (詳細の星を `AppBar` の操作として置く場合だけ `ScreenAppBar` の操作の並びに足す)。

## 解決方法

購入、商品、店、抽出の 4 つの一覧に並び替えとお気に入りの絞り込みを足し、一覧の行と詳細 (抽出と購入) と商品と店の編集の画面に星を置いた。

- データベース: `backend/brew_book/migrations/0004_add_favorites.sql` を追加し、4 テーブルに `favorited_at TEXT` (NULL は未設定) を足し、絞り込み用の `(user_id, favorited_at)` の 4 インデックスと、並び順のキーのインデックス (`(user_id, name COLLATE NOCASE)`、`(user_id, updated_at DESC, id)`、`(user_id, price_amount)`、`(user_id, weight_grams)`、`(user_id, rating)`、`(user_id, dose_grams)`) を足した。ADR-0018 の最終スキーマの表にも `favorited_at` を追記した。
- API の一覧: `sort`、`order` (`asc` / `desc`、既定は `desc`)、`favorite` (`true` のときだけ絞る) を受け付け、資源ごとのキーだけを許し、それ以外は 400 にした。`ORDER BY キー <方向> NULLS LAST, id ASC` とし、`name` は索引、`ORDER BY`、カーソルの比較の 3 か所で `COLLATE NOCASE` に揃えた。
- カーソル: `CursorKey` を、並び順のキーの名前、方向、キーの値 (NULL のときは null)、ID を持つ形に広げた。`sort` と `order` の組がカーソルと一致しないときと、復号できない値は 400 にした。`encode` の `from_f64` の失敗は NaN と ±inf だけでこの経路に到達しないことをコメントに書いた。
- お気に入りの操作: `PUT` と `DELETE /api/<資源>/:id/favorite` の 8 経路を足した。既に同じ状態のときは `favorited_at` と `updated_at` を変えずに 200 を返す。未認証は 401、存在しない ID と他の利用者の ID は 404 にする。`routes.rs` の `ROUTES` と `tests/support/mod.rs` の `SUITE` に登録した。
- 応答: 4 つの記録の応答に `favorited_at` を足し、結合した購入と抽出の入れ子の `product` と `shop` にも含めた。エクスポート (FR-14) の 4 テーブルの列に `favorited_at` を含めた。
- Frontend: `RecordListView` のツールバーに並び順の選択と昇順/降順の切り替えとお気に入りのみの切り替えを置き、画面ごとの選択肢を渡すようにした。選択のシートには出さない。並び順、方向、お気に入りのみを変えたときはカーソルを捨てて先頭から読み直す。読み込み中に変えたときも、`load_pages` が条件をループのたびに読み直して新しい条件で読み直す (レビューの指摘、高)。行の右端の星でお気に入りを切り替え、行の押下 (詳細を開く) を発火させない。抽出と購入の詳細、商品と店の編集 (id があるとき) にも星を置いた。`frontend/src/api/api_client.rs` に `put_json` を足した。
- i18n: `SortLabel`、`SortAscending`、`SortDescending`、`FavoritesOnlyLabel`、`FavoriteAddLabel`、`FavoriteRemoveLabel`、`SortCreatedAt`、`SortUpdatedAt` の 8 キーを足した (`KEY_COUNT` は 221 から 229 になった)。
- PRD: FR-20 (一覧の並び替え) と FR-21 (お気に入り) を追加し、スコープと用語を更新し、性能の節の一覧の対象に `sort`、`order`、`favorite` を含むことを明記した。
- 原本: `docs/design/components/Lists/README.md`、`ListRow/README.md`、`Home/README.md`、`bundle.css` (並び順の `select` と星の見た目)、各 `preview.html` を更新した。
- テスト: `backend/brew_book/tests/schema.rs` (0004 の列とインデックス)、API テスト (4 つの一覧の `sort` / `order` / `favorite` の正常系と入力不正の 400、お気に入り 8 経路の付け外し・冪等 200・401・404、`favorited_at` の応答と入れ子の `product` / `shop`、カーソルのページングと `sort` / `order` 不一致の 400)、`test_query.rs` (全 9 キー × 両方向の `ORDER BY`)、`test_queries.rs` (SQL 台帳に `set_favorited_at`)、`test_cursor.rs` と `prop_cursor.rs` (符号化と復号の PBT)、`frontend/tests/test_records_lists_web.rs` (新規。ツールバー、星、選択のシート、行の押下との分離) を追加または更新した。`backend/pbt/tests/prop_cursor.proptest-regressions` を追加し、見つかったケースを固定した。

完了条件の検証:

- `0004_add_favorites.sql` が `favorited_at` とインデックスを足し、ADR-0018 の表にも追記: `schema.rs` の最終スキーマの検査が通過した。
- 4 つの一覧が `sort`、`order`、`favorite` を受け付け、指定したキーと方向で並べ、`favorite=true` で絞る。不正な値は 400: `wrangler_records_api.rs` の `wrangler_shops_list_sorts_and_filters` と `wrangler_products_list_sorts_and_filters`、`wrangler_purchases_brews_api.rs` の `wrangler_purchases_list_sorts_and_filters` と `wrangler_brews_list_sorts_and_filters` が並び順と絞り込みを検証し、4 つの `*_list_invalid_input_400` が `sort=unknown`、`order=up`、`favorite=yes`、`favorite=1` を 400 にすることを検証した。`backend:test-integration` 17 suite 通過。
- カーソルのページングが全てのキーと両方の方向で重複と漏れなく続きを返し、`sort` と `order` の組がカーソルと一致しないときは 400: `wrangler_shops_list_pages_with_a_sort_key` が `sort=name&order=asc&limit=2` で重複なく続きを返し、カーソルと違う組が 400 になることを検証した。`test_query.rs` の `every_sort_key_orders_with_its_column_and_direction` が全 9 キー × 両方向の `ORDER BY` を、`prop_cursor.rs` が符号化と復号の往復を検証した。
- 8 つのお気に入りの経路が付け外しし、同じ状態では 200 を返し、未認証は 401、他の利用者は 404: `wrangler_shops_favorite_put_and_delete`、`wrangler_products_favorite_put_and_delete`、`wrangler_purchases_favorite_put_and_delete`、`wrangler_brews_favorite_put_and_delete` が 4 資源の PUT と DELETE で検証し、台帳の照合 (`api_suite.rs`) も通過した。
- 4 つの記録と入れ子の `product` と `shop` の応答に `favorited_at` が含まれる: 一覧と単件取得と更新のテストが `favorited_at` を検査し、`assert_nested_product` と `assert_nested_shop` が入れ子の応答の `favorited_at` を検査した。
- エクスポートの 4 テーブルの列に `favorited_at` が含まれる: `wrangler_export_api.rs` の更新で確認した。
- Frontend の 4 つの一覧に並び替えとお気に入りの操作があり、操作でカーソルを捨ててパラメータが変わる: `test_records_lists_web.rs` の `the_four_lists_have_the_sort_and_favorite_toolbar` と `changing_the_sort_and_the_order_and_the_favorites_reloads_from_the_first_page` が検証した。
- 選択のシートには出ない: `the_picker_sheet_has_no_sort_or_favorite_controls` が検証した。
- 行と詳細と編集の星で切り替えられ、星の押下と Enter と Space では詳細を開かない。新規の登録には出ない: `the_star_on_a_row_toggles_the_favorite_without_opening_the_detail`、`the_detail_screens_have_the_star`、`the_edit_forms_have_the_star_and_the_new_forms_do_not` が検証した。
- お気に入りのみの切り替えでお気に入りでない行が表示されない: `the_favorites_only_filter_hides_the_records_that_are_not_favorites` が検証した。
- i18n の 8 キーと `KEY_COUNT` の一致: `test_i18n.rs` が通過した。
- PRD と `docs/design/` の原本: 上記のとおり。
- スクリーンショット比較: 実装の完了後にまとめて実行した E2E の 58 件の比較を確認し、原本 (Home、ListRow、Lists、Detail、RecordForms) を更新した。最終の比較は Home、BrewDetail、Purchases、ProductEdit が mean 0.00、ListRow が 0.30 である。PurchaseDetail、Products、Shops、ShopEdit の残差は複数の画面が 1 つの原本を共有することによる差で、今回の変更によるものではない。
- 既存のテストと E2E、`mise run check`: ユーザーの指示により 0049 から 0051 の実装の完了後に段ごとに実行した。fmt、lint、frontend:build、frontend:lint、formal、backend:test (68 suite)、backend:test-integration (17 suite)、frontend:test (43 suite)、frontend:test-web (11 suite)、frontend:test-same-origin (1 passed) の全てが通過した (2026-10-05)。

方針の方式を保ったままの実装詳細の乖離:

- `d1_binding.rs` のアーカイブのテストで 0004 以降を適用するようにした (一覧の API が `favorited_at` を読むため。check で見つかり、0050 の close で対応した)。
- `backend/pbt/tests/prop_cursor.proptest-regressions` を追加した (PBT が見つけた境界のケースを固定するため)。

レビューの指摘を受けて変えたもの (方式は変えていない):

- 読み込み中に並び順、方向、お気に入りのみを変えたときに新しい条件で読み直すように `load_pages` を直した (effect は signal を読んで再実行し、条件はループのたびに読み直す)。(レビューの指摘、高)
- 4 つの一覧の `sort` / `order` / `favorite` の正常系と入力不正の 400、お気に入り 8 経路の付け外し・冪等 200・401・404、`favorited_at` の応答と入れ子の `product` / `shop`、カーソルのページングと不一致 400 の API テストを追加した。(レビューの指摘、高)
- `test_query.rs` に全 9 キー × 両方向の `ORDER BY` の検査を足した。(レビューの指摘、高)
- `test_queries.rs` の SQL 台帳に `set_favorited_at` を足した。(レビューの指摘、低)
- `cursor.rs` の `encode` の `from_f64` の失敗が NaN と ±inf だけでこの経路に到達しないことをコメントに書いた。(レビューの指摘、低)
- `CHANGES.md` のエントリの種別を `[ADD]` から `[CHANGE]` に直した (カーソル形式の後方互換を壊すため)。(レビューの指摘、中)
- 記録の修正: スクリーンショット比較の数値と検証行を最終状態に合わせた。(レビューの指摘、低)
