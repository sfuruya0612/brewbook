# 購入と抽出の API を実装する

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-23
対応 ADR: ADR-0006 (docs/adr/0006-data-model-and-archive.md)、ADR-0002 (docs/adr/0002-database-cloudflare-d1.md)
関連 PRD: FR-5、FR-9、FR-11、FR-12、性能 (カーソル方式と応答時間 p95 200 ms)、成功指標 (データ分離とテスト網羅)
依存: 0001, 0002, 0003, 0005, 0006

## 背景

ADR-0006 は、購入 (`purchases`) が商品と店を参照し、抽出 (`brews`) が購入だけを参照すると決めた。
商品と店は購入から結合して返し、店は LEFT JOIN で結合して `shop_id` が NULL の購入では `shop` を null にする。
購入は購入日と価格 (通貨の最小単位の整数と ISO 4217 の通貨コード、既定値 JPY) と重量を持ち、為替変換はしない (PRD のやらないこと)。
抽出の項目は購入と抽出日時以外を全て任意とし、自由記述は保存時に前後の空白を除く (0006 の規則)。
PRD の FR-9 と FR-11 は、購入と抽出の登録、閲覧、編集と、ネストした参照の取得を要求する。
0006 で店と商品の API ができた。本 issue が購入と抽出を追加する。

## 目的

利用者が、商品を買った 1 回を購入として、コーヒーを淹れた 1 回を抽出として登録、閲覧、編集、アーカイブでき、抽出から使った豆と店をたどれるようにする。

## 設計判断

### 経路の一覧

| 対象 | 経路 |
| --- | --- |
| 購入 | `GET /api/purchases`、`POST /api/purchases`、`GET /api/purchases/<ID>`、`PATCH /api/purchases/<ID>`、`POST /api/purchases/<ID>/archive`、`POST /api/purchases/<ID>/unarchive` |
| 抽出 | `GET /api/brews`、`POST /api/brews`、`GET /api/brews/<ID>`、`PATCH /api/brews/<ID>`、`POST /api/brews/<ID>/archive`、`POST /api/brews/<ID>/unarchive` |

- アーカイブと解除の扱いは 0006 と同じ (繰り返しは 200)。
- 購入の応答には `product` と `shop` をネストしたオブジェクトとして含み、店が無い購入は `shop` を null にする (FR-9)。
- 抽出の応答には `purchase` をネストし、その中に `product` と `shop` を含める (FR-11)。
- 購入の応答に `photo_key` を含める (クライアントが写真の有無を知るため)。写真の操作は 0009 が扱う。
- 一覧の並び順は、購入が購入日の降順と ID の昇順、抽出が抽出日時の降順と ID の昇順とする (FR-9、FR-11)。
- 単件取得はアーカイブ済みでも返す (FR-12)。

### 入力の検証

- 購入は商品と購入日を必須とし、欠けていれば 400 を返す。
  購入日は `YYYY-MM-DD` の形式で実在する日付だけを受け付け、それ以外は 400 を返す (タイムゾーンを持たない日付として保存する。ADR-0002)。
  店は省略でき、更新で null を送ると外せる。
  存在しないか他の利用者の商品と店は 404、アーカイブ済みの商品と店は 409 を返す (FR-9)。
  参照先の ID を変更する更新でも同じ検査をする (親をアーカイブした後にその親へ付け替えられないようにするため。PRD は新規登録だけを挙げるが、同じ理由が更新にも当てはまる)。
- 価格は 0 以上の整数、通貨コードは ISO 4217 の 3 文字の英大文字だけを受け付ける (それ以外は 400)。
  価格だけを指定して通貨コードを省略した場合は JPY を保存し、価格が無いときは通貨コードも null にする。
  価格と通貨コードの組は片方だけを持たない (更新で価格を null にすると通貨コードも null にする)。
  為替換算は行わない (通貨コードを記録するだけ。PRD のやらないこと)。
  重量は 0 以上の整数だけを受け付ける。
- 抽出は購入と抽出日時を必須とし、欠けていれば 400 を返す。
  抽出日時は ISO 8601 の UTC (末尾が `Z`、小数秒はミリ秒 3 桁) だけを受け付け、それ以外は 400 を返す (ADR-0002 の保存形式)。
  存在しないか他の利用者の購入は 404、アーカイブ済みの購入は 409 を返す (FR-11)。
  参照先の購入を変更する更新でも同じ検査をする (アーカイブ済みの購入に付け替えられないようにするため)。
  豆の量、湯量、湯の温度は 0 以上の小数で小数第 1 位まで、時間は 0 以上の整数、評価は 1 から 5 の整数だけを受け付ける (それ以外は 400)。
  小数の検査は `serde_json` の数値の文字列表現に対して行い、浮動小数点の丸め誤差に依存しない。
- 抽出の入力では `purchase_id` 以外の参照を受け付けない (商品と店は購入からたどる。ADR-0006)。
- 更新の入力で受け取らない列 (ID、`created_at`、`archived_at` など) は無視せず 400 を返す。
  採らない案: 未知の列を無視する (クライアントの項目名の間違いが黙って保存されないため)。
- `updated_at` は更新、アーカイブ、アーカイブ解除で現在時刻にする (ADR-0006)。

### クエリの組み立て

- 0006 の共通部分を使い、全てのクエリに `user_id` と `archived_at` の条件を付ける。
  抽出の一覧と単件の取得は、購入、商品、店の結合を 1 回の SQL で行う (ADR-0006)。
  結合の SQL を単体テストで検証する。

## 完了条件

- FR-9 の受け入れ基準を満たす。
  商品と購入日の必須、店の省略と null での解除、参照の 404 と 409、通貨コードの既定値と検証、重量と価格の検証、ネストした応答と店の null、購入日の降順と ID の昇順の一覧。
- FR-11 の受け入れ基準を満たす。
  購入と抽出日時の必須、負の値の 400、参照の 404 と 409、小数第 2 位以下の 400、時間と評価の検証、ネストした応答 (購入、商品、店)、抽出日時の降順と ID の昇順の一覧。
- FR-12 の受け入れ基準のうち購入と抽出の分を満たす。
  既定の一覧からの除外、`include_archived` での取得、単件の取得と解除、アーカイブ済みの購入を抽出の参照先に指定した場合の 409、親 (商品、店、購入) をアーカイブしても子から親をたどれる。
- FR-5 の受け入れ基準のうち購入と抽出の分を満たす (他の利用者の ID を指定した取得、更新、アーカイブ、解除が全て 404、一覧が自分の記録だけを返す)。
- 購入日と抽出日時の形式の検証 (不正な形式、実在しない日付、オフセット付きの日時) が単体テストで通る。
- アーカイブ済みの購入への付け替えが 409 になる。
- 想定規模 (1 利用者あたり購入 3,000 件、抽出 30,000 件) のデータで一覧と単件の取得の処理時間を測り、issue 本文に追記する。
  応答時間の目標 (p95 200 ms) の合否は、リリース後に本番の Workers Logs の `duration_ms` の集計で確認する (PRD の成功指標と同じ)。
- 結合の SQL と、利用者 ID と `archived_at` の条件の付け忘れを検出する単体テストが通る。
- 経路の台帳 (0001) に本 issue の経路を追加し、認証が必要な全経路に正常系と未認証 401、入力を持つ経路に入力不正 400 のテストがあることを照合できる。
- `mise run check` が通過する。

## 関連

- 0006 が店と商品の API とクエリの共通部分を作る。
- 0008 が本 issue の列を使うサジェストを追加する。
- 0009 が購入の写真を追加する。
- 0010 が購入と抽出の統計を追加する。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. スキーマの変更 (migration 0002 `0002_purchases_price_currency_nullable.sql`) を追加した。0003 の `purchases.price_currency` は `NOT NULL DEFAULT 'JPY'` で、方針の「価格が無いときは通貨コードも null にする」を満たせない。`ALTER TABLE purchases DROP COLUMN price_currency` と `ADD COLUMN price_currency TEXT` で NULL を許すようにした (SQLite は NOT NULL を外せず、`purchases` は `brews` から参照されるためテーブルの作り直しは外部キー制約に抵触するため採らない)。列の位置は末尾に移るが、SQL は列名を明示するため応答と保存の内容は変わらない。適用時に既存行の通貨コードの値は失われ NULL になる (本番の D1 はまだ作成されておらず、影響する行は無い)。適用後のスキーマは、NULL 許容、`brews` から `purchases` への外部キー、インデックス `idx_purchases_user_archived_purchased_on` の存在を `wrangler_the_price_currency_column_allows_null` が確認する。本番の D1 への適用は 0017 のデプロイ時になる。
2. ネストした商品に `flavor_notes` を含める。FR-7 の「商品の項目」に Flavor Notes が含まれ、0006 の商品の応答と形を揃えるため、結合のあとに商品 ID を 98 件ずつのクエリに分け、1 つの batch でまとめて引いて付ける (D1 の束縛の上限のため)。
3. 参照先を変えない更新はアーカイブ済みの親でも 200 にする。方針の「参照先の ID を変更する更新でも同じ検査」と付け替えの理由に合わせ、親をアーカイブしても購入や抽出を編集できるようにした (参照先を変更するときは 404 または 409)。
4. 価格と通貨コードの組の細部。作成で価格だけを指定したときは JPY、価格が無いときは通貨コードも null。更新で価格を null にすると通貨コードも null、価格があるときに通貨コードだけを null にする更新は組が片方だけになるため 400。価格が無いのに通貨コードだけを指定する入力は、作成でも更新でも 400 にする。通貨コードを省略した更新は現在の通貨コードを保ち、無ければ JPY。
5. 整数の上限。価格、重量、時間は D1 の整数 (32 ビット) に収まる値だけを受け付け、超える値は 400 にする (500 にしない)。
6. 小数の検査。`serde_json` の数値の文字列表現 (`Number::to_string`) を `coffee_log_core::records::validate_decimal` に渡す。指数表記のまま残る値 (`1e300`) は 400、正規化で小数第 1 位になる値 (`12.50` から `12.5`) は受け付ける。小数を保存するため、`coffee_log_core::query::Value` に `Real(f64)` を追加し、`Value` と `Statement` などの派生から `Eq` を外した (`PartialEq` は残る。`Eq` を要求する呼び出し元は無い)。
7. 結合した行の読み取り。`coffee_log_core::query` が列に `p_id` のような一意の別名を付け、Worker は平坦な行の型で読んでからネストした応答に組み立てる (項目の重複は許容)。
8. テストハーネスの追加。`support/mod.rs` に `execute_sql_file` (`wrangler d1 execute --file`) を追加し、想定規模の投入を 1 プロセスで行う。D1 の 1 文の長さの上限 (100 KB) に合わせ、`ROWS_PER_STATEMENT` を 500 から 400 にした (500 行の抽出の INSERT は約 110 KB になり `SQLITE_TOOBIG` で失敗するため)。
9. `mise.toml` の `backend:test-integration` に新しい結合テストを追加した (0006 と同じ扱い)。

## 想定規模での処理時間

2026-09-23 に `backend/` で `cargo test -p coffee_log --test wrangler_records_scale -- --nocapture` を実行した。
店 100 件、商品 1,000 件、購入 3,000 件、抽出 30,000 件を投入し、ローカルの `wrangler dev` とローカル D1 で HTTP の往復を計測した。各 10 回の最小、中央値、最大である。

```
purchases list (limit 50):      50 rows, min 4 ms,  median 5 ms,  max 8 ms
purchases list (limit 200):    200 rows, min 9 ms,  median 10 ms, max 11 ms
purchases single (the oldest):   1 row,  min 2 ms,  median 3 ms,  max 3 ms
purchases single (the newest):   1 row,  min 2 ms,  median 2 ms,  max 3 ms
brews list (limit 50):          50 rows, min 5 ms,  median 5 ms,  max 7 ms
brews list (limit 200):        200 rows, min 12 ms, median 12 ms, max 13 ms
brews single (the oldest):       1 row,  min 2 ms,  median 2 ms,  max 2 ms
brews single (the newest):       1 row,  min 2 ms,  median 2 ms,  max 3 ms
```

単件の計測は、作成日時が最も古い行 (添字 0) と最も新しい行 (最後の添字) の 2 つである。
中央値は測定回数が偶数のときは中央の 2 つの平均とする。
応答時間の目標 (p95 200 ms) の合否は、issue の記載どおりリリース後に本番の Workers Logs の `duration_ms` の集計で確認する。

## 解決方法

購入と抽出の 12 経路を追加した。SQL は `coffee_log_core::query`、入力の検証は `coffee_log_core::records`、経路の処理は `coffee_log::records` に分けた。

- `backend/coffee_log_core/src/query.rs` に購入と抽出の SQL を追加した (`purchases_list`、`purchase_find`、`purchase_insert`、`purchase_update`、`purchase_set_archived` と `brews_list`、`brew_find`、`brew_insert`、`brew_update`、`brew_set_archived`)。一覧と単件は商品と店 (抽出は購入も) を結合し、列に一意の別名を付ける。全てのクエリが `user_id` を条件に持ち、既定の一覧は `Archived::Exclude` で `archived_at IS NULL` を付け、単件の取得と参照の検査は `Archived::Include` でアーカイブ済みも引く (挿入と更新の文に `archived_at` の条件は無い)。
- `backend/coffee_log_core/src/records.rs` に、購入日の検証 (`validate_day`。実在する日付だけ)、抽出日時の検証 (`validate_timestamp`。ミリ秒 3 桁の UTC だけ)、小数 (`validate_decimal`。小数第 1 位まで) と整数 (`validate_count`。0 以上で 32 ビットに収まる値) の検証を追加した。
- `backend/coffee_log/src/records/purchases.rs` と `brews.rs` を新設した (経路の処理。ネストした応答の組み立ては `PurchaseJoinRow::into_response` と `BrewJoinRow::into_response`)。`records/mod.rs` に参照の検査 (`require_product`、`require_shop`、`require_purchase`) と Flavor Notes の付与 (`attach_flavor_notes`) を追加した。`db.rs` は `Value::Real` を `D1Type::Real` に対応させ、`tests/support/seed.rs` に `SeededPurchase`、`SeededBrew` と `purchase`、`brew` を追加した。
- `backend/coffee_log/migrations/0002_purchases_price_currency_nullable.sql` を追加した (通貨コードを NULL 許容にする。乖離 1)。
- `backend/coffee_log/src/lib.rs` は同じ形で 12 経路を振り分けるようにした。`backend/coffee_log_core/src/routes.rs` に 12 経路を追加した (購入 6、抽出 6)。
- `backend/coffee_log/tests/wrangler_purchases_brews_api.rs` (45 テスト)、`backend/coffee_log_core/tests/test_query.rs` (11 テストを追加し 37 に)、`backend/coffee_log_core/tests/test_records.rs` (6 テストを追加し 7 に)、`backend/pbt/tests/prop_records.rs` (5 テストを追加し 11 に) を追加した。`support/mod.rs` の `SUITE` に 12 経路の種別を追加し、`execute_sql_file` (`wrangler d1 execute --file`) を追加した。`wrangler_records_scale.rs` は `ROWS_PER_STATEMENT` を 500 から 400 にし (1 文の長さの上限のため)、購入と抽出の計測を追加した。
- `mise.toml` の `backend:test-integration` に新しい結合テストを追加した。

完了条件の検証:

- FR-9: `purchases::wrangler_purchases_create_ok` (店の省略、通貨コードの既定 JPY、重量と価格、ネストした商品と店)、`purchases::wrangler_purchases_create_invalid_input_400` (商品と購入日の必須、日付と価格と重量の検証、価格が無いのに通貨コードだけを指定する入力の 400)、`purchases::wrangler_purchases_create_reference_404_and_409`、`purchases::wrangler_purchases_list_ok` (購入日の降順と ID の昇順)、`purchases::wrangler_purchases_update_ok` (店の `null` での解除、価格と通貨コードの組)、`purchases::wrangler_purchases_update_invalid_input_400` (価格が無い購入への通貨コードだけの 400) が確認した。ネストした商品の `flavor_notes` の実値は `assert_nested_product` が確認した (乖離 2)。
- FR-11: `brews::wrangler_brews_create_ok` (小数第 1 位の値 (0、0.0、0.1、-0.0 を含む)、ネストした購入と商品と店)、`brews::wrangler_brews_create_invalid_input_400` (購入と抽出日時の必須、負の値、小数第 2 位以下、時間と評価、オフセット付き日時、`purchase_id` 以外の参照)、`brews::wrangler_brews_list_ok` (抽出日時の降順と ID の昇順)、`brews::wrangler_brews_create_reference_404_and_409` が確認した。
- 乖離 3 (参照先を変えない更新はアーカイブ済みの親でも 200) は、`purchases::wrangler_purchases_update_reference_404_and_409` と `brews::wrangler_brews_update_reference_404_and_409` が、親 (商品と店、購入) をアーカイブしてから同じ参照先で更新して 200 になることを確認した。
- migration 0002 の適用後のスキーマは `wrangler_the_price_currency_column_allows_null` が確認した (価格の通貨の NULL 許容、`brews` から `purchases` への外部キー、インデックス `idx_purchases_user_archived_purchased_on` の存在)。
- FR-12: 既定の一覧からの除外と `include_archived` は `purchases::wrangler_purchases_list_ok_with_include_archived_and_a_cursor` と `brews::wrangler_brews_list_ok_with_include_archived_and_a_cursor` が、単件の取得と解除と繰り返しの 200 は `purchases::wrangler_purchases_archive_and_unarchive_ok` と `brews::wrangler_brews_archive_and_unarchive_ok` が、アーカイブ済みの購入を参照先にした 409 は `brews::wrangler_brews_create_reference_404_and_409` と `brews::wrangler_brews_update_reference_404_and_409` が、親 (商品、店、購入) をアーカイブしても子からたどれることは `purchases::wrangler_purchases_keep_tracing_the_archived_parents` と `brews::wrangler_brews_keep_tracing_the_archived_parents` が確認した。
- FR-5: `purchases::wrangler_purchases_get_other_user_404`、`purchases::wrangler_purchases_archive_other_user_404` (アーカイブと解除の両方で他の利用者の ID を使う)、`brews::wrangler_brews_get_other_user_404`、`brews::wrangler_brews_archive_other_user_404` が 404 を確認し、他の利用者の購入と抽出の更新 404 は `purchases::wrangler_purchases_update_reference_404_and_409` と `brews::wrangler_brews_update_reference_404_and_409` の中で確認している (更新の 404 の専用テストは無い)。一覧は `purchases::wrangler_purchases_list_returns_only_own_records` と `brews::wrangler_brews_list_returns_only_own_records` が確認した。
- 購入日と抽出日時の形式: `coffee_log_core/tests/test_records.rs` の `the_day_validation_accepts_existing_dates_only` と `the_timestamp_validation_accepts_the_fixed_utc_form_only` (不正な形式、実在しない日付 `2026-02-30`、オフセット付き `+09:00`、ミリ秒なし、小文字の `z`)、`pbt/tests/prop_records.rs` の `a_value_of_another_length_is_not_a_day` と `a_value_of_another_length_or_suffix_is_not_a_timestamp` が確認した。
- アーカイブ済みの購入への付け替え: `brews::wrangler_brews_update_reference_404_and_409` (参照を変更すると 409、変更しなければ 200) が確認した。
- 結合の SQL と条件の付け忘れ: `coffee_log_core/tests/test_query.rs` の `purchase_and_brew_queries` に、一覧と結合の SQL の全文 (INNER JOIN と LEFT JOIN、結合条件、`user_id`、`archived_at`、ORDER BY、カーソル) を確認するテストがあり、`test_query.rs::conditions::every_record_query_keeps_the_user_and_archived_conditions` (33 件の文) が条件の付け忘れを確認した。
- 想定規模の計測: `wrangler_records_scale.rs` が店 100 件、商品 1,000 件、購入 3,000 件、抽出 30,000 件を投入して計測した (数値は issue の「## 想定規模での処理時間」にある)。
- 台帳とスイートの照合: `api_suite.rs::the_ledger_and_the_suite_match` が 12 経路の種別を照合した。
- `mise run check`: exit 0 (ベースラインからの新たな失敗は無し)。

方針からの乖離 (方式は変えていない): issue の「## 実装詳細の乖離」1 から 9 にある。

1. migration 0002 を追加した。`price_currency` が `NOT NULL DEFAULT 'JPY'` のままでは「価格が無いときは通貨コードも null にする」を満たせないため (既存行の値は失われ NULL になるが、本番の D1 は未作成)。
2. ネストした商品に `flavor_notes` を含めた。FR-7 の商品の項目に含まれ、0006 の商品の応答と形を揃えるため (98 件ずつのクエリを 1 つの batch で引く)。
3. 参照先を変えない更新はアーカイブ済みの親でも 200 にした。方針の検査は参照先を変更する更新に対するものであり、親をアーカイブしても子を編集できるようにするため。
4. 価格と通貨コードの組の細部 (価格だけなら JPY、価格が無ければ通貨コードも null、価格があるときの通貨コードの null と、価格が無いときの通貨コードだけの指定は 400) を決めた。組が片方だけにならないようにするため。
5. 整数の上限 (32 ビット) を 400 にした。D1 の整数に収まらない値を 500 にしないため。
6. 小数を `serde_json` の数値の文字列表現で検査し、`Value` に `Real(f64)` を足して `Eq` を外した。浮動小数点の丸め誤差に依存せず、D1 に小数を保存するため。
7. 結合した行に一意の別名を付けた。商品と店と購入で同じ列名が衝突するため。
8. `execute_sql_file` と `ROWS_PER_STATEMENT = 400` を追加した。想定規模の投入を 1 プロセスで行い、D1 の 1 文の長さの上限 (100 KB) に収めるため。
9. `mise.toml` の `backend:test-integration` に新しい結合テストを追加した。新しい結合テストを `mise run check` の対象にするため。
