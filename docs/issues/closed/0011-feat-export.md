# 全記録のエクスポートを実装する

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-24
対応 ADR: ADR-0006 (docs/adr/0006-data-model-and-archive.md)、ADR-0003 (docs/adr/0003-photo-storage-r2-presigned-upload.md)
関連 PRD: FR-5、FR-14、成功指標 (エクスポートの完全性)
依存: 0001, 0002, 0003, 0005, 0006, 0007

## 背景

ADR-0006 は、利用者データのテーブルを shops、products、flavor_tags、product_flavor_tags、purchases、brews の 6 つと定め、エクスポート (PRD の FR-14) の対象にした。
エクスポートには 6 テーブルのアーカイブ済みを含む全行と全列を含め、パスキー、セッション、チャレンジ、登録用トークンは含めない。
ADR-0003 は、エクスポートに写真の実体を含めず、`photo_key` と写真取得 API のパスだけを含めると決めた。
成功指標は、エクスポートした JSON から 6 テーブルの全行と全列を復元できることを、テスト専用の復元処理と比較テストで検証することを求める。
0006 と 0007 で記録の API ができた。本 issue がエクスポートを追加する。

## 目的

利用者が、自分の全記録を 1 つの JSON ファイルとしてダウンロードできるようにする。

## 設計判断

- 経路は `GET /api/export` とし、応答は JSON のダウンロード (`Content-Disposition: attachment`) にする。
- 形はトップレベルに 6 テーブルの名前を持つ配列を並べたオブジェクトとする。
  `{"shops": [...], "products": [...], "flavor_tags": [...], "product_flavor_tags": [...], "purchases": [...], "brews": [...]}`
- 各行はテーブルの全列を含め、アーカイブ済みも含める。
- 購入の行には `photo_key` と、写真取得 API のパス (`/api/purchases/<購入 ID>/photo`) を `photo_path` として含める。
  写真の実体と署名付き GET URL は含めない (ADR-0003)。
- 追加のメタデータ (エクスポート日時など) は入れない (復元テストの比較対象をテーブルの行と列だけにするため)。
  採らない案: テーブルごとに別のファイルにする (ダウンロードと復元の手順が増え、1 ファイルで完結する利点を失う)、メタデータを入れる (比較の対象が増え、復元の意味が薄れる)。
- 復元処理はテスト専用とし、本番のコードには置かない。
  復元では `user_id` と `photo_key` に含まれる利用者 ID を取り込み先の利用者に付け替え、6 テーブルの全行と全列を比較する。
  写真は付け替え後の `photo_key` と件数の一致で検証する。
- 出力は利用者の全記録をメモリに載せるため、想定規模 (抽出 30,000 件) でも Worker のメモリに収まることを確認し、結果を issue 本文に追記する。
- 採らない案: ストリーミングで少しずつ返す (Worker のメモリは抑えられるが、JSON の組み立てとテストの比較が複雑になる。想定規模ではメモリに収まる見込みである)。

## 完了条件

- FR-14 の受け入れ基準を満たす。
  6 テーブルのアーカイブ済みを含む全行と全列、パスキーとセッションとチャレンジと登録用トークンの除外、`photo_key` と写真取得 API のパス、写真の実体の除外を自動テストで確認する。
- 成功指標の測定方法を満たす。
  エクスポートをテスト専用の復元処理で別の利用者に取り込み、`user_id` と `photo_key` の利用者 ID を付け替えた上で、6 テーブルの全行と全列が元の記録と一致し、写真の件数が一致する自動テストが通る。
- 想定規模の記録を持つ利用者のエクスポートが Worker のメモリの範囲で成功することを確認し、結果を issue 本文に追記する。
- 他の利用者の記録がエクスポートに含まれない。
- 経路の台帳 (0001) に本 issue の経路を追加し、正常系と未認証 401 のテストが揃っている。
- `mise run check` が通過する。

## 関連

- 0006 と 0007 が 6 テーブルの API を作る。
- 0012 がアカウント削除を作る。
- 0016 がエクスポートのダウンロードの画面を作る。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. ダウンロードのファイル名を `coffee-log-export.json` とした (方針に指定が無いため)。`Content-Disposition: attachment` は方針どおりである。
2. `photo_path` は写真の有無に関わらず全ての購入の行に含める (方針の「購入の行には含める」を条件分岐なしで実装した)。
3. 行の並び順を主キーの昇順 (`id ASC`、`product_flavor_tags` は `product_id ASC, tag_id ASC`) に固定した (再現性のある出力とテストの比較のため。方針に指定は無い)。6 テーブルの SELECT は 1 つの `batch` (1 トランザクション) で実行する (別々に読むと、親の無い抽出を含む復元できない JSON になり得るため)。
4. タグ名に接頭辞を付けるテスト用の記録を使い、全列を非 NULL と NULL の両方で通す (エクスポートの列落ちを比較で検出するため)。
5. 復元は行の ID を保つため、取り込み元の行を子から順に削除してから親から順に挿入し、比較はエクスポート前の DB のスナップショットを基準にする。
6. テスト支援として `DevServer::query_rows` と `Seed::raw` を追加した (既存の `query_int` と同じ `wrangler d1 execute --json` を使う)。`mise.toml` の `backend:test-integration` に新しい結合テストを追加した。

## 想定規模のメモリの確認

2026-09-23 に抽出 30,000 件 (店 100 件、商品 1,000 件、購入 3,000 件、抽出 30,000 件、合計 34,100 行) を持つ利用者の
`GET /api/export` が成功することを想定規模の結合テスト (`wrangler_records_scale`) で確認した (`wrangler dev`)。

- 応答の JSON: 13,340,270 バイト (34,100 行)
- 処理時間 (3 回): min 574 ms、中央値 588 ms、最大 622 ms

Worker がメモリに載せるのは行の構造体と JSON の文字列で、見積もりは JSON の 2 倍強 (30 MB 程度) になり、分離環境のメモリ上限 (128 MB) に収まる。

## 解決方法

`GET /api/export` を追加した。6 テーブルのアーカイブ済みを含む全行と全列を 1 つの JSON として返し、購入の行には `photo_key` と `photo_path` を含める (写真の実体と署名付き URL は含めない)。

- `backend/coffee_log/src/export.rs` を新設した (ハンドラと行の型)。6 テーブルの SELECT を 1 つの `batch` (1 トランザクション) で実行して単一の時点の記録を読み (`take_rows`)、`PurchaseExportRow` に `photo_path` を付ける。追加のメタデータは持たない。
- `backend/coffee_log_core/src/query.rs` に `export_rows` (アーカイブ条件と LIMIT を付けない `SELECT ... WHERE user_id = ? ORDER BY ...`) と `PRODUCT_FLAVOR_TAG_COLUMNS` を追加した。
- `backend/coffee_log/src/respond.rs` に `json_attachment` (`Content-Disposition: attachment; filename="coffee-log-export.json"`) を追加した。
- `backend/coffee_log_core/src/routes.rs` に `export_get` (`GET /api/export`、認証必須、入力なし) を追加し、`backend/coffee_log/src/lib.rs` で振り分けるようにした。`support/mod.rs` の `SUITE` も同期した。
- `backend/coffee_log/tests/wrangler_export_api.rs` (5 テスト) を追加した。`support/seed.rs` に `Seed::raw`、`support/mod.rs` に `DevServer::query_rows` を追加した。`wrangler_records_scale.rs` にエクスポートの計測を追加し、`backend/coffee_log_core/tests/test_query.rs` に `export_rows` の単体テストと条件検査 (`Kind::Export`) を追加した。
- `mise.toml` の `backend:test-integration` に新しい結合テストを追加した。

完了条件の検証:

- FR-14 の受け入れ基準: `wrangler_export_ok` がトップレベルのキーが 6 テーブル名と一致すること、各テーブルの行数、各行のキー集合が列と一致すること (購入は列に `photo_path` を加えた集合)、応答の隣接する行の主キーが昇順であること、全行の `user_id` が呼び出し元であること、アーカイブ済み行の存在、`photo_key` と `photo_path` の一致、`X-Amz` と `Signature` と `photo_url` と `photo_data` と `base64` を含まないこと、`photo_path` にクエリ文字列が無いことを確認した。写真の実体の除外は、`photo_key` の位置に印を付けたオブジェクトを R2 に置き (`put_r2_object`)、その印と署名 URL の断片 (`X-Amz`、`Signature`) が本文に無いことで確認した (実装は `photo_key` と `photo_path` だけを載せ、応答の組み立てで R2 を読まない)。`wrangler_export_covers_every_column_of_the_schema` が、エクスポートの列の一覧を `pragma_table_info` の列と照合し、列の追加漏れを検出できるようにした。`wrangler_export_returns_only_own_records` が、他の利用者の行 ID が本文に含まれず、パスキーとセッションとチャレンジと登録用トークンの生の値とハッシュが本文に含まれないことを確認した。
- 成功指標の測定方法: `wrangler_export_round_trip_restores_every_row_and_column` が、テスト専用の復元処理 (`restore_statements`) で `user_id` と `photo_key` の利用者 ID を付け替えて別の利用者に取り込み、6 テーブルの全行と全列が元の記録と一致し、`photo_key` の件数が一致することを確認した (店名の `O'Brien` のように引用符を含む値も往復し、復元の SQL リテラルのエスケープを検査する)。`assert_export_matches_source` がエクスポートの JSON と DB の元の行を全列で比較する。
- 想定規模: `wrangler_records_scale.rs` がエクスポートの成功と応答のサイズを確認した (数値は issue の「## 想定規模のメモリの確認」にある)。
- 他の利用者の除外: `wrangler_export_returns_only_own_records` が確認した (記録が無い利用者では 6 テーブルが空配列になることも確認する)。
- 台帳とスイート: `api_suite.rs::the_ledger_and_the_suite_match` が `export_get` の種別 (正常系、未認証 401) を照合し、`wrangler_export_ok` と `wrangler_export_unauthenticated_401` が確認した。
- `mise run check`: exit 0 (ベースラインからの新たな失敗は無し)。

方針からの乖離 (方式は変えていない): issue の「## 実装詳細の乖離」1 から 6 にある (ファイル名、`photo_path` を全ての購入に含めること、並び順の固定、テスト用の記録、復元の手順、テスト支援と `mise.toml`)。
