# アーカイブ機能を画面、API、データベースから削除する

Created: 2026-10-03
Model: DeepSeek V4.1 Flash

## 背景

アーカイブ (論理削除) は ADR-0006 の決定で、店、商品、購入、抽出の 4 つの記録に実装されている。所有者はアーカイブ機能が不要と決めた。既存のアーカイブ済みの記録は削除せず、通常の記録として扱う。

現在の実装は次のとおりである。

- データベース: `backend/brew_book/migrations/0001_initial_schema.sql` の `shops`、`products`、`purchases`、`brews` の `archived_at` 列と、`idx_shops_user_archived_created_at`、`idx_products_user_archived_created_at`、`idx_purchases_user_archived_purchased_on`、`idx_brews_user_archived_brewed_at` の 4 つのインデックス。
- 共有ライブラリ: `backend/brew_book_core/src/query.rs` の `Archived`、`IncludeArchivedError`、`parse_include_archived`、`ListQuery::archived`、`FindQuery::archived`、`set_archived` と `*_set_archived`、`SHOP_COLUMNS` などの列の並び、`backend/brew_book_core/src/stats.rs` の集計の `archived_at` の条件。
- API: `backend/brew_book_core/src/routes.rs` の `shops_archive`、`shops_unarchive`、`products_archive`、`products_unarchive`、`purchases_archive`、`purchases_unarchive`、`brews_archive`、`brews_unarchive` の 8 経路と、`backend/brew_book/src/lib.rs` の振り分け。`backend/brew_book/src/records/mod.rs` の `ListParams` の `include_archived`、`purchase_exists` (`query::purchase_find` を `Archived::Include` で呼ぶ)、アーカイブ済みの参照先を 409 で拒否する `require_product`、`require_shop`、`require_purchase`。`backend/brew_book/src/d1_check.rs` の `Archived::Exclude` と `ShopRow.archived_at`。
- Frontend: `frontend/src/screens/records/list_view.rs` のアーカイブ済みを含める切り替え、`frontend/src/records/list.rs` の `include_archived`、`toggle_include_archived`、`PageRequest.include_archived`、`frontend/src/ui/list_row.rs` の `ArchivedBadge`、`frontend/src/screens/records/mod.rs` の `archive_button`、各フォームのアーカイブのボタンと状態、`frontend/src/screens/records/picker.rs` の `show_archived_toggle: false`、`frontend/src/records/api.rs` の `set_*_archived`、`frontend/src/records/models.rs` の `archived_at` と `is_archived`、`frontend/src/i18n/{keys,ja,en}.rs` の `ArchiveButton`、`UnarchiveButton`、`IncludeArchivedLabel`、`ArchivedMessage`、`UnarchivedMessage`、`ArchivedBadge` の 6 キー。
- テスト: `backend/brew_book/tests/support/seed.rs` に `archived_at` を引数に取る 9 つのヘルパー (`shop`、`product`、`product_with_texts`、`purchase`、`purchase_with_roast`、`brew`、`brew_with_texts`、`brew_with_numbers`、`purchase_with_numbers`) があり、`backend/brew_book/tests/support/mod.rs` の 8 経路のテストの台帳、`backend/brew_book/tests/schema.rs` の ADR-0006 の列とインデックスの照合、`backend/pbt/tests/prop_stats.rs` の `archived_at IS NULL` の検査、`frontend/pbt/tests/prop_records.rs` の切り替えの検査、アーカイブを使うテスト (`schema.rs`、`api_suite.rs`、`d1_binding.rs`、`test_queries.rs`、`test_records.rs`、`wrangler_records_api.rs`、`wrangler_purchases_brews_api.rs`、`wrangler_stats_api.rs`、`wrangler_export_api.rs`、`wrangler_account_api.rs`、`wrangler_records_scale.rs`、`wrangler_stats_scale.rs`、`support/mod.rs`、`support/seed.rs`) がある。
- 文書: `docs/prd/brewbook.md` の用語、目的、UC-7、スコープ、エラー応答の 409、FR-5、FR-9、FR-11、FR-12、FR-13、FR-14、FR-18、FR-19、FR-18 の 612 行と 616 行、決定の履歴の 92 行、関連資料の ADR-0006 の行。`docs/adr/0006-data-model-and-archive.md` のアーカイブの決定。`docs/design/` の `README.md`、`components/ListRow/README.md`、`components/Detail/README.md`、`components/Lists/README.md`、`components/Home/README.md`、`components/AppBar/README.md`、`components/Feedback/README.md`、`tokens.json` と各 `preview.html` のアーカイブの記述。

## 目的

アーカイブの概念をコードとデータベースから削除し、一覧、単件取得、統計、サジェスト、エクスポートの挙動をアーカイブ無しの前提に揃える。
アーカイブ済みだった記録は削除せず、通常の記録として扱う。

## 設計判断

- データベース: `backend/brew_book/migrations/0003_remove_archive.sql` を追加する。列を索引が参照しているため、先に 4 つのインデックスを `DROP INDEX` し、次に 4 テーブルの `archived_at` を `ALTER TABLE ... DROP COLUMN` で削除し、最後に `archived_at` を外した 4 つのインデックス (`idx_shops_user_created_at`、`idx_products_user_created_at`、`idx_purchases_user_purchased_on`、`idx_brews_user_brewed_at`) を作る。`DELETE` は書かず、既存の行を残す。
- 共有ライブラリ: `Archived`、`parse_include_archived`、`ListQuery::archived`、`FindQuery::archived`、`set_archived` と `*_set_archived` を削除し、`list`、`find_one`、`list_qualified`、`find_qualified` から `archived_at` の条件を消す。4 つの列の並びから `archived_at` を消す。`stats.rs` の集計から `archived_at` の条件を消す。
- API: `routes.rs` の 8 経路を削除し、`backend/brew_book/src/lib.rs` の振り分けからも削除する。`ListParams` から `include_archived` を削除する (`include_archived` を渡したリクエストは未知のパラメータとして無視する)。`require_product`、`require_shop`、`require_purchase` からアーカイブ済みの 409 を削除し、存在しないか他の利用者のものだけを 404 にする。`backend/brew_book/src/records/mod.rs` の `purchase_exists` から `Archived::Include` の引数を消す。`d1_check.rs` を `Archived` を使わない形に直す。`backend/brew_book/src/export.rs` の「アーカイブ済みも含む」の記述と条件を消す。`backend/brew_book_core/fuzz/fuzz_targets/parse_strings.rs` の `parse_include_archived` の呼び出しを消す (`mise run lint` の型検査が通るようにする)。
- Frontend: `list_view.rs` の切り替えを削除し、`records/list.rs` の切り替えの状態機械を削除し、`ListRow` から `archived` と `ArchivedBadge` を削除し、`archive_button` と各フォームのアーカイブの処理を削除する。`RecordsApi` の `set_*_archived` とモデルの `archived_at`、`is_archived` を削除する。i18n の 6 キーを削除し、`KEY_COUNT` と `ja` と `en` の表を詰める。
- テスト: `tests/support/seed.rs` の `archived_at` を引数に取る 9 つのヘルパーから `archived_at` の引数を消し、`tests/support/mod.rs` の 8 経路の台帳からアーカイブの行を消し、アーカイブを使うテストを更新する。`tests/schema.rs` は、0001、0002、0003 を順に適用した最終スキーマを検証する形にする (4 テーブルに `archived_at` が無いこと、4 つのインデックスが `archived_at` を含まないこと)。ADR-0006 との列の照合は、ADR-0018 が定める最終スキーマとの照合に置き換える。`prop_stats.rs` の `archived_at IS NULL` の検査を消し、`prop_records.rs` の切り替えの検査を消す。
- 文書: PRD の用語からアーカイブの定義を削除し、目的とスコープの「アーカイブ、アーカイブ解除」を削除する。UC-7 の行を削除し、以降の UC の番号は変えない (欠番にする)。エラー応答の 409 から「アーカイブ済みの参照」を削除する。FR-5 のアーカイブの 404 の行、FR-9 の「アーカイブ済みの商品または店を指定した新規登録は 409」、FR-11 の「アーカイブ済みの購入を指定した新規登録は 409」、FR-13 の「アーカイブ済みの記録の入力値も候補に含める」、FR-14 の「アーカイブ済みを含む」、FR-18 の「アーカイブ済みの抽出と購入を含めない」「アーカイブ済みの購入も指定できる」と 612 行の「アーカイブ済みの記録が含まれない」と 616 行の「集計の SQL は、アーカイブの条件と `user_id` の条件を含めて」、FR-19 の「アーカイブされていない商品」と「既定ではアーカイブ済みの商品を含めない」を削除する。FR-12 の見出しと本文を削除し、以降の FR の番号は変えない (欠番にする)。決定の履歴の 92 行の「アーカイブの除外規則」は所有者の承認の記録なので残し、アーカイブを削除した決定 (2026-10-03) を PRD に足す。関連資料の ADR-0006 の行に部分置き換えの注記を足し、ADR-0018 の行を足す。
- ADR: `docs/adr/0018-remove-archive.md` を追加する。ADR-0006 はデータモデル全体の ADR のため、置き換えはアーカイブの部分に限る。ADR-0006 の Status を `Partially superseded by ADR-0018 (2026-10-03)` に変え、本文の冒頭に「アーカイブ (論理削除) の決定だけを ADR-0018 が置き換える。データモデルの他の決定は残る」の注記を足す。表と本文の `archived_at` の記述は決定の履歴として残す。ADR-0018 がアーカイブの廃止と最終スキーマ (4 テーブルから `archived_at` を削除し、インデックスを作り直す) を定める。
- 設計の原本: `docs/design/` のアーカイブの記述 (上の「背景」に列挙した README、`tokens.json`、`preview.html`) を削除または更新する。
- `CHANGES.md` に `[CHANGE]` のエントリを足す (後方互換を壊す変更)。

採らなかった案は次のとおりである。

- データベースの `archived_at` 列を残して画面と API だけ削除する: 使われない列とインデックスが残り、スキーマと ADR-0006 が実装と食い違うため却下する (所有者の回答で却下)。
- アーカイブ済みの記録をマイグレーションで物理削除する: 所有者の回答は「記録は残す」であるため却下する。
- アーカイブを物理削除の機能に置き換える: 所有者の要望は削除機能の追加ではなくアーカイブの削除であり、新しい削除の API と画面は要求に無いため却下する。
- FR-12 の番号を詰める: FR-12 を参照する記述が本文の複数箇所にあり、参照の付け替えを誤る危険が大きいため却下する (欠番を残す)。
- ADR-0006 の Status を `Superseded` にして本文を書き換える: データモデルの他の決定 (4 つの記録の関係、物理削除はアカウント削除だけ) は有効なままなので、部分置き換えにする。本体の書き換えは履歴を消すため却下する。

追加の API 呼び出しと権限は無い。

## 完了条件

- `backend/brew_book/migrations/0003_remove_archive.sql` が 4 つのインデックスを削除し、4 テーブルの `archived_at` を削除し、`archived_at` を含まない 4 つのインデックスを作る。`DELETE` を含まない。
- スキーマのテストで、0001、0002、0003 を順に適用した最終スキーマの 4 テーブルに `archived_at` が無いことと、4 つのインデックスが `archived_at` を含まないことを検証する。
- `routes.rs` の台帳からアーカイブの 8 経路が消え、経路の照合のテスト (`backend/brew_book/tests/api_suite.rs` と `tests/support/mod.rs` の台帳) が通過する。
- 一覧、単件取得、更新の API の応答から `archived_at` が消え、`include_archived` を指定しても挙動が変わらない (API のテスト)。
- アーカイブ済みだった記録が一覧に含まれる (0002 の状態で `archived_at` を設定した行を入れ、0003 を適用した後に行が残り、一覧の API が返すテスト)。
- 統計、サジェスト、エクスポートのクエリから `archived_at` の条件と列が消える (`test_query.rs`、`test_stats.rs`、`prop_stats.rs`、`wrangler_export_api.rs` の更新)。
- `backend/brew_book_core/fuzz/fuzz_targets/parse_strings.rs` と `backend/brew_book/src/d1_check.rs` が `Archived` と `parse_include_archived` を参照しない (`mise run lint` の型検査が通過する)。
- Frontend からアーカイブの操作 (切り替え、ボタン、バッジ、API 呼び出し) が消え、モデルに `archived_at` と `is_archived` が無い (Frontend のテストと `prop_records.rs` の更新)。
- i18n の 6 キーが消え、`KEY_COUNT` と日本語と英語の表の要素数が一致する (`test_i18n.rs` の更新)。
- `tests/support/seed.rs` の `archived_at` を引数に取る 9 つのヘルパーから `archived_at` の引数が消え、アーカイブを使うテストが通過する。
- PRD、ADR-0006 (部分置き換えの注記)、新しい ADR-0018、`docs/design/` が上の設計判断のとおりに改訂され、`CHANGES.md` に `[CHANGE]` のエントリがある。
- E2E のスクリーンショット比較の差分を確認し、意図した差分は `docs/design/screenshots/` を更新し、意図しない差分は直す (比較の結果を issue に記録する)。
- `mise run check` が通過する。

## 関連

- 同じ購入と抽出のフォームを変える 0049 の後 (番号順) に処理する。
- 0051 はこの issue の後に着手する (一覧のツールバーと `include_archived` を置き換えるため)。
