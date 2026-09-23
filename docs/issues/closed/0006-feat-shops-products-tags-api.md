# 店と商品と Flavor Notes の API を実装する

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-23
対応 ADR: ADR-0006 (docs/adr/0006-data-model-and-archive.md)、ADR-0002 (docs/adr/0002-database-cloudflare-d1.md)
関連 PRD: FR-5、FR-6、FR-7、FR-8、FR-12、性能 (カーソル方式と応答時間 p95 200 ms)、成功指標 (データ分離とテスト網羅)
依存: 0001, 0002, 0003, 0005

## 背景

ADR-0006 は、店 (`shops`) と商品 (`products`) と Flavor Notes のタグ (`flavor_tags`、`product_flavor_tags`) の列と、`archived_at` による論理削除を決めた。
親をアーカイブしても子はアーカイブせず、アーカイブ済みの親を新規の子の参照先に指定すると 409 を返す。
すべてのクエリに `user_id` を含め、既定の一覧は `archived_at IS NULL` で絞る。
PRD の FR-6 から FR-8 は、店と商品の登録と編集、タグの付け替えと共有を要求する。
0003 でスキーマとクエリの共通部分ができ、0005 でセッションのミドルウェアができたが、記録を扱う API が無い。
本 issue は店と商品を扱い、購入と抽出は 0007、サジェストは 0008 が扱う。

## 目的

利用者が、店と商品を登録、閲覧、編集、アーカイブでき、商品に Flavor Notes のタグを付けられるようにする。

## 設計判断

### 経路の一覧

| 対象 | 経路 |
| --- | --- |
| 店 | `GET /api/shops`、`POST /api/shops`、`GET /api/shops/<ID>`、`PATCH /api/shops/<ID>`、`POST /api/shops/<ID>/archive`、`POST /api/shops/<ID>/unarchive` |
| 商品 | `GET /api/products`、`POST /api/products`、`GET /api/products/<ID>`、`PATCH /api/products/<ID>`、`POST /api/products/<ID>/archive`、`POST /api/products/<ID>/unarchive` |
| タグ | `GET /api/flavor-tags` |

- アーカイブと解除は繰り返し呼んでも 200 を返す (同じ状態への遷移はエラーにしない)。
  採らない案: 2 回目のアーカイブを 409 にする (クライアントの再送で不要な失敗になる)、`PATCH` のボディでアーカイブする (操作が 1 つにまとまり、冪等の扱いが曖昧になる)。

### 一覧と取得

- 一覧はカーソル方式とし、`limit` と `cursor` を受け取る。
  `limit` の既定は 50、最大は 200 とし、0 以下、整数でない値、200 を超える値は 400 を返す (PRD の性能)。
- 並び順は作成日時の降順と ID の昇順とする (PRD に指定が無いため決めた)。
  採らない案: 店名と商品名の昇順 (日本語の照合順序が環境で変わり、並びが安定しない)、更新日時の降順 (編集のたびに位置が変わり、利用者が探しにくい)。
- `include_archived=true` を指定するとアーカイブ済みの行も返す。既定は `false` とし、`true` と `false` 以外は 400 を返す。
- 単件取得はアーカイブ済みでも返す (FR-12)。
- 存在しない ID と他の利用者の ID は区別せず 404 を返す (ADR-0006)。
- 応答の JSON はスキーマの列名をそのまま使う (`created_at`、`archived_at` など)。
  商品の応答にはタグ名の配列 (`flavor_notes`) を含める (FR-8)。

### 入力の検証

- 店名と商品名は必須で、前後の空白を除いて空なら 400 を返す。
  住所と商品名以外の項目は NULL を許す (FR-6、FR-7)。
- 商品の Flavor Notes はタグ名の配列で受け取り、その配列で商品のタグを置き換える (FR-8)。
  タグ名は前後の空白を除いて空なら 400 を返し、保存時も前後の空白を除く。
  同じ利用者の同じタグ名は 1 つのタグとして共有し、どの商品からも参照されなくなったタグは削除しない。
- 自由記述の項目 (商品の Producer、Origin、Region、Process、Variety) も保存時に前後の空白を除く。
  サジェスト (0008) の重複の除去と前方一致を同じ値の扱いにするためである。
- `updated_at` は更新、アーカイブ、アーカイブ解除で現在時刻にする (ADR-0006)。

### クエリの組み立て

- SQL は `coffee_log_core` の 1 つのモジュールで組み立て、全てのクエリに `user_id` と `archived_at` の条件を必ず付ける (ADR-0006)。
  一覧と単件のクエリは共通の関数を通して組み立て、生成した SQL を単体テストで検証する。
  0007 と 0008 と 0010 と 0011 はこの共通部分を使う。
- アーカイブ済みの行が既定の一覧に出ないこと、他の利用者の行が返らないことを自動テストで検証する (ADR-0006、成功指標)。
- 物理削除は本 issue では行わない (アカウント削除の 0012 だけが行う。ADR-0006)。

## 完了条件

- FR-6 の受け入れ基準を満たす。
  店名の必須と空白の検証、住所の NULL、一覧と単件の取得、更新。
- FR-7 の受け入れ基準を満たす。
  商品名の必須と空白の検証、商品名以外の NULL、一覧と単件の取得、全項目の更新。
- FR-8 の受け入れ基準を満たす。
  タグの配列での置き換え、利用者ごとのタグの共有、空白のみのタグ名の 400、参照されなくなったタグの残存、取得結果のタグの配列、全タグの一覧の取得。
- FR-12 の受け入れ基準のうち店と商品の分を満たす。
  既定の一覧からの除外、`include_archived` での取得、単件の取得と解除、アーカイブ済みの店と商品を新規登録の参照先に指定した場合の 409 (購入と抽出の参照は 0007 が検査する)。
- FR-5 の受け入れ基準のうち店と商品の分を満たす (他の利用者の ID を指定した取得、更新、アーカイブ、解除が全て 404、一覧が自分の記録だけを返す)。
- カーソルの並び順と `limit` の検証 (0 以下、非整数、200 超は 400) が単体テストで通る。
- 想定規模 (1 利用者あたり商品 1,000 件、店 100 件) のデータで一覧と単件の取得の処理時間を測り、issue 本文に追記する。
  応答時間の目標 (p95 200 ms) の合否は、リリース後に本番の Workers Logs の `duration_ms` の集計で確認する (PRD の成功指標と同じ)。
- 利用者 ID と `archived_at` の条件の付け忘れを検出する単体テストが通る。
- 記録の内容 (商品名) を含むリクエストを実行し、その値がログの行に現れないことを結合テストで確認する (PRD の運用)。
- 経路の台帳 (0001) に本 issue の経路を追加し、認証が必要な全経路に正常系と未認証 401、入力を持つ経路に入力不正 400 のテストがあることを照合できる。
- `mise run check` が通過する。

## 関連

- 0003 がスキーマとクエリの共通部分を作る。
- 0005 がセッションのミドルウェアを作る。
- 0007 が購入と抽出を追加する。
- 0008 がサジェストを追加する。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. 一覧の応答に `next_cursor` を追加した (ページが `limit` に満たないときは null)。方針に指定は無いが、カーソル方式をクライアントから使うために必要である。
2. `PATCH` の意味を「項目が無ければ変更しない、`null` は NULL にする」とした。必須の店名・商品名に `null` を送った場合は 400 にする。
3. 更新はアーカイブ済みの行にもできる (単件取得と同じ `Archived::Include` で引くため)。
4. 受け取らない項目 (未知の列) は 400 で拒否する。`worker::Request::json` は `serde_wasm_bindgen` で読むため `deny_unknown_fields` が効かず、`records::read_input` で `serde_json` に通している (0007 の購入・抽出と同じ扱い)。
5. 自由記述とタグ名は前後の空白を除くだけで、空白だけの自由記述は空文字として保存する (NULL には変換しない)。タグ名の並びは名前の昇順 (SQLite のバイナリ照合) にする。
6. アーカイブの繰り返しでは `archived_at` を現在時刻で上書きする (方針は 200 を返すことだけを要求している)。
7. D1 は 1 クエリに 100 個までしか値を束縛できないため、商品のタグ名を引くクエリは 98 件ずつに分け、batch でまとめて実行する。
8. D1 の共通補助のうち `database`、`now_text`、`statement`、`execute_batch`、`execute_changes`、`new_id`、`random_bytes_16` を `auth/mod.rs` から `src/db.rs` へ移した (挙動は変えず、記録の API との重複を避けるため)。このうち `random_bytes_16` は呼び出し元が無いため `auth` から再公開せず、それ以外の 6 つを `auth` から再公開する。`prepared` は `db.rs` で新設した。`d1_check.rs` は店の SQL の定数 (`INSERT_SHOP` と `SHOP_COLUMNS`) をやめ、`query::shop_insert` と `query::shops_list` と `db::prepared` を使うようにした (SQL を 1 つのモジュールに集約する方針のため。利用者の投入の SQL はローカルの定数のまま)。
9. `seed::user_id` を `u32` 受け取りにし、10 以上の添字は 0 で埋めるようにした (数字の繰り返しでは 1 と 11 が衝突するため。10 以上を使うのは、テスト用の利用者 ID の一意性を検査する `the_test_user_ids_are_unique_and_well_formed` である。1 から 9 の値は従来と同じ)。
10. `mise.toml` の `backend:test-integration` に新しい結合テスト 2 ファイルを追加した (0003 と同じ扱い)。
11. アーカイブ済みの店と商品を参照先に指定した場合の 409 は、本 issue の経路に親を取るものが無いため実装していない (0007 が実装して検査する)。`Archived::Exclude` の単件取得は 0007 のために用意し、本 issue の経路からは呼ばない。
12. 商品の本体の書き込み (挿入または更新) とタグの置き換えは、別の D1 の呼び出しである。タグの置き換えが失敗すると商品の行や項目の更新は残り、挿入の再試行では同じ商品がもう 1 件できる。1 つの batch にまとめることもできるが、応答に使う値の準備と失敗時の扱いが複雑になるため、別の呼び出しのままとした。
13. 商品の登録と更新で `flavor_notes` に `null` を送った場合は 400 にする。タグは中間テーブルで持つため `null` で NULL にできず、全て外す操作は空の配列で表す。項目が無いときは変更しない (登録では空にする)。乖離 2 の「`null` は NULL にする」は他の項目に当てはめる。
14. タグの一覧 (`GET /api/flavor-tags`) は、方針の一覧の規約 (カーソル、`limit`、`include_archived`) を適用せず、利用者の全タグを名前の昇順で返す。利用者ごとのタグは少ない見込みで、方針にも指定が無いためである (ページングが要る規模になれば 0008 以降で扱う)。

## 想定規模での処理時間

2026-09-23 に `backend/` で `cargo test -p coffee_log --test wrangler_records_scale -- --nocapture` を実行した。
ローカルの `wrangler dev` とローカル D1 で HTTP の往復を計測し、各 10 回の最小、中央値、最大である。

```
products list (limit 50):      50 rows,  min 3 ms, median 4 ms, max 6 ms
products list (limit 200):     200 rows, min 6 ms, median 7 ms, max 7 ms
products single (the oldest):  1 row,    min 1 ms, median 2 ms, max 2 ms
products single (the newest):  1 row,    min 1 ms, median 2 ms, max 2 ms
shops list (100 shops):        100 rows, min 2 ms, median 3 ms, max 4 ms
shops single (1 shop):         1 row,    min 2 ms, median 2 ms, max 5 ms
```

単件の計測は、作成日時が最も古い行 (添字 0) と最も新しい行 (最後の添字) の 2 つである。

応答時間の目標 (p95 200 ms) の合否は、issue の記載どおりリリース後に本番の Workers Logs の `duration_ms` の集計で確認する。

## 解決方法

店、商品、Flavor Notes のタグの 13 経路を追加した。SQL は `coffee_log_core::query`、入力の検証は `coffee_log_core::records`、経路の処理は `coffee_log::records` に分けた。

- `backend/coffee_log_core/src/query.rs` に、店と商品の SQL を `list`、`find_one`、`insert`、`update` の共通関数の上に追加し (`shops_list`、`shop_find`、`shop_insert`、`shop_update`、`shop_set_archived` と `products_list`、`product_find`、`product_insert`、`product_update`、`product_set_archived`)、タグの SQL (`flavor_tags_list`、`insert_flavor_tag`、`insert_product_flavor_tag`、`delete_product_flavor_tags`、`product_flavor_notes`) を追加した。一覧は `Archived` と `CursorKey` から WHERE と ORDER BY (降順と ID の昇順) を組み立てる。全てのクエリが `user_id` を条件に持ち、一覧と単件のクエリが `archived_at` の条件を持つ (タグのクエリは持たない)。
- `backend/coffee_log_core/src/records.rs` を新設した。店名と商品名の検証 (`validate_name`)、自由記述の前後の空白の除去 (`trim_text`、`trim_optional`)、タグ名の検証と正規化 (`validate_flavor_notes`。空白のみは拒否、同じ名前は 1 つにまとめ、名前の昇順に並べる)。
- `backend/coffee_log_core/src/routes.rs` に 13 経路を追加した (店 6、商品 6、タグの一覧 1)。
- `backend/coffee_log/src/records/` を新設した。`mod.rs` (一覧のクエリパラメータ `ListParams` の読取、`next_cursor`、入力の読取 `read_input`、`PATCH` の `double_option`、名前の統合 `merge_name`、タグの置き換え `replace_flavor_notes`、タグの取得 `flavor_notes` と `flavor_notes_for`)、`shops.rs`、`products.rs`、`tags.rs` (経路の処理) を持つ。
- `backend/coffee_log/src/db.rs` を新設し、D1 の共通補助のうち `database`、`now_text`、`statement`、`execute_batch`、`execute_changes`、`new_id`、`random_bytes_16` を `auth/mod.rs` から移した (`auth` は `random_bytes_16` 以外の 6 つを再公開し、挙動は変えていない)。`prepared` は `db.rs` で新設した。`d1_check.rs` はローカルの SQL 定数をやめ、`query::shop_insert` と `query::shops_list` と `db::prepared` を使うようにした。
- `backend/coffee_log/src/lib.rs` は認証の経路と同じ形で 13 経路を振り分けるようにした。
- `backend/coffee_log/tests/wrangler_records_api.rs` (44 テスト。店、商品、タグ、ログ) と `wrangler_records_scale.rs` (1 テスト。想定規模の計測)、`backend/coffee_log/tests/test_records.rs` (3 テスト。`next_cursor` の並び順とテスト用の利用者 ID)、`backend/coffee_log_core/tests/test_records.rs` (1 テスト。入力の誤りと応答の対応)、`backend/pbt/tests/prop_records.rs` (6 テスト)、`backend/coffee_log_core/tests/test_query.rs` (20 テストを追加し 26 に) を追加した。`support/mod.rs` の `SUITE` に 13 経路の種別を追加し、`support/seed.rs` の `user_id` を `u32` 受け取りにして 10 以上は 0 埋めにした。
- `mise.toml` の `backend:test-integration` に新しい 2 つの結合テストを追加した。

完了条件の検証:

- FR-6: `shops::wrangler_shops_create_ok`、`shops::wrangler_shops_create_invalid_input_400`、`shops::wrangler_shops_update_ok`、`shops::wrangler_shops_update_invalid_input_400`、`shops::wrangler_shops_list_ok`、`shops::wrangler_shops_get_ok`、`coffee_log_core/tests/test_records.rs` の `every_input_error_maps_to_a_bad_request` が確認した。
- FR-7: `products::wrangler_products_create_ok` (名前だけの登録で他の項目は null)、`products::wrangler_products_create_invalid_input_400` (名前が無い、空、空白のみ、null、未知の項目)、`products::wrangler_products_update_ok` (全項目の更新と null での解除)、`products::wrangler_products_update_invalid_input_400`、`products::wrangler_products_list_ok`、`products::wrangler_products_get_ok` が確認した。
- FR-8: `products::wrangler_products_update_ok` (配列での置き換え、置き換え後の単件の取得、参照されなくなったタグの残存)、`products::wrangler_products_create_ok` (重複と空白の正規化)、`products::wrangler_products_create_invalid_input_400` (空白のみのタグ名、配列でない値、`null` の 400)、`products::wrangler_products_update_invalid_input_400` (空白のみのタグ名と `null` の 400)、`products::wrangler_products_list_ok` と `flavor_tags::wrangler_flavor_tags_list_ok` (共有するタグ、未参照のタグの残存、全タグの一覧) が確認した。
- FR-12: `shops::wrangler_shops_list_ok`、`products::wrangler_products_list_ok` (既定の一覧からの除外)、`shops::wrangler_shops_list_ok_with_include_archived_and_a_cursor` (店の `include_archived` とカーソル。商品の `include_archived` は `products::wrangler_products_list_ok` が確認する)、`shops::wrangler_shops_archive_and_unarchive_ok` (単件の取得、解除、繰り返しの 200、アーカイブ済みのまま更新できること)、`products::wrangler_products_archive_and_unarchive_ok` が確認した。アーカイブ済みの親を参照先に指定した場合の 409 は、本 issue の経路に親を取るものが無いため応答自体が無い (0007 が実装して検査する。issue の「## 実装詳細の乖離」11)。
- FR-5: `wrangler_shops_get_other_user_404`、`wrangler_shops_update_other_user_404`、`wrangler_shops_archive_other_user_404`、`wrangler_shops_unarchive_other_user_404` と商品の同名のテスト、`wrangler_shops_list_returns_only_own_records`、`wrangler_products_list_returns_only_own_records` が確認した。
- カーソルと `limit`: `coffee_log/tests/test_records.rs` の `the_next_cursor_uses_the_kind_of_the_ordering` と `a_page_without_the_full_count_has_no_next_cursor` が続きのカーソルの組み立て (日時と日付の種類) を、`coffee_log_core/tests/test_cursor.rs` の `the_page_size_rejects_zero_and_negative_values`、`the_page_size_rejects_values_that_are_not_integers`、`the_page_size_rejects_values_above_the_upper_bound` が 0 以下、非整数、200 超を 400 にすることを、`test_query.rs::list_builders` が並び順とカーソルの SQL を、`test_query.rs::include_archived_parameter` が `include_archived` の解釈を確認した。結合では `shops::wrangler_shops_list_ok_with_include_archived_and_a_cursor` と `products::wrangler_products_list_ok_with_a_cursor` (カーソルで 2 ページ目と 3 ページ目を引き、重複しないこと) が、`shops::wrangler_shops_list_invalid_input_400` (limit 0、-1、1.5、abc、201、999999999999 は 400) が確認した。
- 想定規模の計測: `wrangler_records_scale.rs` が店 100 件と商品 1,000 件を投入し、一覧 (50、200)、単件 (最も古い行と最も新しい行)、店の一覧と単件を測った (数値は issue の「## 想定規模での処理時間」にある)。
- 利用者 ID と `archived_at` の条件の付け忘れ: `test_query.rs::conditions::every_record_query_keeps_the_user_and_archived_conditions` (19 件の文) と `the_checker_detects_an_omitted_user_condition`、`the_checker_detects_an_omitted_archived_condition` が確認した。
- 記録の内容がログに出ないこと: `logging::wrangler_records_do_not_appear_in_the_log` が、このテスト専用のサーバーで (他のテストのログが混ざらない)、商品名 `MARKER-PRODUCT-NAME-7f3a` を含むリクエストの前後で `products_create` のログ行の数が増えるまで待ち、全行にその値が現れないことを確認した。
- 台帳とスイートの照合: `api_suite.rs::the_ledger_and_the_suite_match` が 13 経路の種別を照合した (認証が必要な全経路に正常系と未認証 401、入力を持つ 4 経路に入力不正 400)。
- `mise run check`: exit 0 (ベースラインからの新たな失敗は無し)。

方針からの乖離 (方式は変えていない): issue の「## 実装詳細の乖離」1 から 14 にある。

1. 一覧の応答に `next_cursor` を追加した (続きのカーソル。ページが `limit` に満たないときは null)。カーソル方式をクライアントから使うために必要である。
2. `PATCH` の意味を「項目が無ければ変更しない、`null` は NULL にする」とした (必須の店名・商品名の `null` と、配列の `flavor_notes` の `null` は 400)。
3. 更新はアーカイブ済みの行にもできる (単件取得と同じ `Archived::Include` で引くため)。
4. 受け取らない項目 (未知の列) は 400 で拒否する。`worker::Request::json` は `serde_wasm_bindgen` で読むため `deny_unknown_fields` が効かず、`records::read_input` で `serde_json` に通している (0007 の購入・抽出と同じ扱い)。
5. 自由記述とタグ名は前後の空白を除くだけで、空白だけの自由記述は空文字として保存する (NULL には変換しない)。タグ名の並びは名前の昇順 (SQLite のバイナリ照合) にする。
6. アーカイブの繰り返しでは `archived_at` を現在時刻で上書きする (方針は 200 を返すことだけを要求している)。
7. D1 は 1 クエリに 100 個までしか値を束縛できないため、商品のタグ名を引くクエリは 98 件ずつに分け、batch でまとめて実行する。
8. D1 の共通補助のうち `database`、`now_text`、`statement`、`execute_batch`、`execute_changes`、`new_id`、`random_bytes_16` を `auth/mod.rs` から `src/db.rs` へ移した。`random_bytes_16` は呼び出し元が無いため `auth` から再公開せず、それ以外の 6 つを再公開する。`prepared` は `db.rs` で新設した。`d1_check.rs` は店の SQL の定数をやめ、`query::shop_insert` と `query::shops_list` と `db::prepared` を使うようにした。
9. `seed::user_id` を `u32` 受け取りにし、10 以上の添字は 0 で埋めるようにした (数字の繰り返しでは 1 と 11 が衝突する。10 以上を使うのはテスト用の利用者 ID の検査である。1 から 9 の値は従来と同じ)。
10. `mise.toml` の `backend:test-integration` に新しい 2 つの結合テストを追加した。
11. アーカイブ済みの親を参照先に指定した場合の 409 は、本 issue の経路に親を取るものが無いため実装していない (0007 が実装して検査する)。
12. 商品の本体の書き込み (挿入または更新) とタグの置き換えは、別の D1 の呼び出しである。タグの置き換えが失敗すると商品の行や項目の更新は残る (1 つの batch にまとめると応答の準備と失敗時の扱いが複雑になるため)。
13. 商品の登録と更新で `flavor_notes` に `null` を送った場合は 400 にする (全て外す操作は空の配列で表す)。
14. タグの一覧は、方針の一覧の規約 (カーソル、`limit`、`include_archived`) を適用せず、利用者の全タグを名前の昇順で返す。
