# 過去の入力値のサジェスト API を実装する

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-23
対応 ADR: ADR-0006 (docs/adr/0006-data-model-and-archive.md)
関連 PRD: FR-5、FR-13、成功指標 (データ分離)
依存: 0001, 0002, 0003, 0005, 0006, 0007

## 背景

ADR-0006 は、サジェスト (PRD の FR-13) を products、purchases、brews の該当列から `user_id` で絞って DISTINCT を取ると決めた。
並び順は `updated_at` を使う。
FR-13 は、前後の空白を除いた値の前方一致 (大文字と小文字を区別しない)、重複の除去、最大 20 件、アーカイブ済みの包含、他の利用者の除外、候補に無い値の入力を許すことを求める。
0006 と 0007 で記録の API ができ、自由記述の項目は保存時に前後の空白を除く規則にした。本 issue がサジェストを追加する。

## 目的

利用者が自由記述の項目を入力するとき、同じ利用者が過去に入力した値が候補として表示されるようにする。

## 設計判断

- 経路は `GET /api/suggestions/<項目>` とし、`q` (入力中の文字列) を省略可能なクエリパラメータで受け取る。
- 項目の名前は 8 つだけを受け付け、それ以外は 400 を返す (PRD の 404 の定義は記録とトークンに限るため、入力の形式の違反として 400 にする)。
  | 項目 | テーブル |
  | --- | --- |
  | `producer`、`origin`、`region`、`process`、`variety` | products |
  | `roast` | purchases |
  | `method`、`grind_setting` | brews |
- `q` を前後の空白を除いた値とし、列の値を `lower()` したものの前方一致で絞る。
  SQL は `lower(列) LIKE lower(?) || '%' ESCAPE '\'` とし、`q` の中の `%` と `_` と `\` をエスケープする。
  SQLite の `lower()` は ASCII だけを変換するが、日本語の値には大文字小文字の区別が無いため問題にならない。
- 重複を除いた値ごとに `updated_at` の最大を取り、その降順、同じときは値の昇順 (Unicode コードポイント) で並べ、先頭の 20 件を返す。
  SQLite の TEXT の比較は UTF-8 のバイト順で、これは Unicode コードポイントの順と一致する。
- アーカイブ済みの記録の値も候補に含める (FR-13)。`archived_at` の条件を付けない。
  他の利用者の値は含めない (`user_id` の条件を付ける)。
- 候補の選択は任意であり、候補に無い値も入力できる。API は候補を返すだけで、入力を制限しない。
- 採らない案: FTS5 を使う (前方一致と 20 件の並び替えには LIKE で足り、仮想テーブルと同期の管理が増える)、クライアントが全値を取得して絞る (記録が増えると転送量が増え、FR-13 の順序をクライアントが再現できない)、候補専用のテーブルを作る (記録の更新と二重管理になる。ADR-0006 は記録の列から取ると決めた)、大文字と小文字を区別する (FR-13 に反する)。

## 完了条件

- FR-13 の受け入れ基準を満たす。
  前後の空白を除いた値の前方一致 (大文字小文字を区別しない)、重複の除去と最大 20 件、`updated_at` の降順と値の Unicode コードポイントの昇順、`q` が空のときの全値の返却、アーカイブ済みの包含、他の利用者の除外、候補に無い値の入力を妨げないことを自動テストで確認する。
- 項目名が上の 8 つ以外の場合は 400 を返す。
- `%` と `_` を含む `q` が文字として扱われる (ワイルドカードにならない)。
- 経路の台帳 (0001) に本 issue の経路を追加し、正常系と未認証 401 のテストを揃え、`q` の型が不正な場合の 400 のテストを追加する。
- `mise run check` が通過する。

## 関連

- 0006 と 0007 が記録の API と自由記述の項目を作る。
- 0014 が入力中の候補の表示を作る。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. 応答は `{"values": ["...", ...]}` とした。方針は応答の形を定めておらず、既存の一覧の応答 (`<資源名>: [...]`) の形に合わせたためである。
2. 経路のパラメータ名を `:field` とし、`lib.rs` の `route_handler` と `authenticated_route` が `field` も渡すようにした (既存の `id` と同じ扱い)。
3. 「`q` の型が不正」は、クエリパラメータの配列 (`?q=a&q=b`) と解釈して 400 を返す。クエリ文字列には型が無いため、1 つの文字列でない指定を型の違反とした。
4. サジェストの SQL は空文字の値 (`""`) を除外しない。方針が示す SQL のままで、FR-13 の受け入れ基準にも空文字の扱いは無いためである (候補の表示 (0014) で問題になる場合は、別 issue で扱う)。
5. `has_input` の説明を「JSON の本体を読む経路」から「JSON の本体かクエリパラメータを読む経路」に直した。サジェストは `q` を読むためである。
6. テスト用の下ごしらえ (`support/seed.rs`) に自由記述の列を入れる `product_with_texts`、`purchase_with_roast`、`brew_with_texts` を追加し、既存の `product`、`purchase`、`brew` はそれらに委譲する形にした (既存の呼び出しは変更なし)。`mise.toml` の `backend:test-integration` に新しい結合テストを追加した。
7. 乖離 5 の定義の変更に合わせて、一覧の 4 経路 (shops、products、purchases、brews) の `has_input` を false から true にし、`backend/coffee_log/tests/support/mod.rs` の `SUITE` に `KIND_INVALID_INPUT_400` を追加した (4 経路には既に 400 のテストがある)。

## 解決方法

`GET /api/suggestions/:field` を追加した。SQL の組み立ては `coffee_log_core::query`、経路の処理は `coffee_log::records::suggestions` に置いた。

- `backend/coffee_log_core/src/query.rs` に `parse_suggestion_field`、`SuggestionItem`、`SuggestionFieldError`、`SUGGESTION_LIMIT`、`suggestions` と、非公開の `escape_like` を追加した。8 つの項目 (producer、origin、region、process、variety、roast、method、grind_setting) の列とテーブルを対応させ、`lower(列) LIKE lower(?) || '%' ESCAPE '\'` の形で前方一致を組み立てる。`q` の `%`、`_`、`\` は `escape_like` でエスケープする。重複を除いた値ごとに `MAX(updated_at)` を取り、その降順、同じときは値の昇順で並べ、`SUGGESTION_LIMIT` (20 件) に限る。`user_id` の条件を付け、`archived_at` の条件は付けない (FR-13)。
- `backend/coffee_log/src/records/suggestions.rs` を新設した (`q` の読取、項目名の検証、応答の組み立て)。
- `backend/coffee_log_core/src/routes.rs` に `suggestions_list` (`GET /api/suggestions/:field`、認証必須、入力あり) を追加し、`backend/coffee_log/src/lib.rs` で `field` を渡すようにした。`has_input` の説明を「JSON の本体かクエリパラメータを読む経路」に直し、同じ定義に合わせて一覧の 4 経路 (shops、products、purchases、brews) の `has_input` を true にした (`support/mod.rs` の `SUITE` に `KIND_INVALID_INPUT_400` を追加。4 経路には既に 400 のテストがある)。`backend/coffee_log/src/records/mod.rs` にはモジュールの説明と `pub mod suggestions` を追加した。
- `backend/coffee_log/tests/wrangler_suggestions_api.rs` (12 テスト) を追加した。`support/mod.rs` の `SUITE` に種別を追加し、`support/seed.rs` に `product_with_texts`、`purchase_with_roast`、`brew_with_texts` を追加した。`backend/coffee_log_core/tests/test_query.rs` に 4 テスト (mod suggestions)、`backend/pbt/tests/prop_query.rs` に 2 テストを追加した。
- `mise.toml` の `backend:test-integration` に新しい結合テストを追加した。

完了条件の検証:

- FR-13: `wrangler_suggestions_list_returns_every_value_of_the_field` (全値、重複の除去、アーカイブ済みの包含、`updated_at` の最大による並び)、`wrangler_suggestions_filters_by_the_trimmed_prefix` (前後の空白の除去と大文字小文字の区別なし)、`wrangler_suggestions_dedupes_the_values`、`wrangler_suggestions_orders_by_the_latest_update_and_then_by_the_value` (同じ `updated_at` のときの値の昇順、20 件の上限)、`wrangler_suggestions_returns_only_own_records` (他の利用者の除外)、`wrangler_suggestions_do_not_restrict_the_input` (候補に無い値の登録と反映) が確認した。SQL の形は `test_query.rs::suggestions::a_suggestion_query_groups_the_values_and_orders_them` が確認した。
- 項目名の検証: `wrangler_suggestions_invalid_field_400` (大文字表記と未定義名は 400、項目名の無い経路は 404)、`test_query.rs::suggestions::the_eight_field_names_are_accepted_and_the_others_are_rejected` が確認した。
- ワイルドカード: `wrangler_suggestions_treats_the_wildcards_as_characters` (`100%`、`%`、`a_b`、`_`、`C:\`、`\`)、`test_query.rs::suggestions::the_pattern_escapes_the_wildcards_and_the_escape_character` (正確な出力を 1 例)、`prop_query.rs` の `the_like_pattern_restores_the_given_value` と `the_like_pattern_has_no_unescaped_wildcard` が確認した。任意入力の復元は PBT が担う。
- 空白だけの自由記述は空文字として保存され (0006 の規則)、`wrangler_suggestions_include_an_empty_value` が空文字も候補に含まれることを確認した (乖離 4)。
- 台帳とスイート: `api_suite.rs::the_ledger_and_the_suite_match` が `suggestions_list` の種別 (正常系、未認証 401、入力不正 400) を照合し、`wrangler_suggestions_list_ok`、`wrangler_suggestions_unauthenticated_401`、`wrangler_suggestions_invalid_q_400` (q を 2 回指定した配列) が確認した。一覧の 4 経路も `KIND_INVALID_INPUT_400` を要求されるようになり、既存の `wrangler_shops_list_invalid_input_400`、`wrangler_products_list_invalid_input_400`、`wrangler_purchases_list_invalid_input_400`、`wrangler_brews_list_invalid_input_400` が照合を通した。
- `mise run check`: exit 0 (ベースラインからの新たな失敗は無し)。

方針からの乖離 (方式は変えていない): issue の「## 実装詳細の乖離」1 から 6 にある (応答の形、`:field` の名前、`q` の型の解釈、空文字の除外をしないこと、`has_input` の説明、テスト用の下ごしらえと `mise.toml`)。
