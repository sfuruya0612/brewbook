# D1 のバインディングと初期スキーマを入れる

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-22
対応 ADR: ADR-0002 (docs/adr/0002-database-cloudflare-d1.md)、ADR-0006 (docs/adr/0006-data-model-and-archive.md)
関連 PRD: 制約と前提 (Database は D1、ローカルは wrangler のローカル D1)、性能 (カーソル方式、想定規模)、可用性 (Time Travel)
依存: 0001, 0002

## 背景

ADR-0002 は、データベースに Cloudflare D1 を使い、本番もローカルも同じスキーマとコードで動かすと決めた。
マイグレーションは wrangler の D1 マイグレーションで管理し、D1 データベースは 1 つとし、利用者向けの Worker と管理者 Worker が同じデータベースをバインディングで参照する。
SQL は ORM を使わず、プレースホルダ付きの SQL を D1 の API に渡す。
主キーはサーバーで生成する UUID v4 の文字列、日時は ISO 8601 の UTC の文字列、日付はタイムゾーンを持たない `YYYY-MM-DD` の文字列で保存する。
ADR-0006 は、users、registration_tokens、passkey_credentials、webauthn_challenges、sessions、shops、products、flavor_tags、product_flavor_tags、purchases、brews の 11 テーブル、`archived_at` による論理削除、全クエリに `user_id` を含める規則を決めた。
現状は D1 のバインディングもスキーマも無いため、記録を保存できない。

## 目的

Worker から D1 にプレースホルダ付きの SQL でアクセスでき、11 テーブルのスキーマをマイグレーションで再現できるようにする。

## 設計判断

- `backend/coffee_log/wrangler.toml` に D1 のバインディングを `DB` として定義し、データベース名は `coffee-log` とする。
  マイグレーションは `backend/coffee_log/migrations/` に置き、`wrangler d1 migrations` で適用する。
  管理者 Worker は同じ `database_id` を参照し、マイグレーションを持たない (ADR-0002。管理者 Worker の設定は 0018 が作る)。
- ローカルの適用と本番の適用を分ける。
  `mise run db-migrate` は `--local`、`mise run db-migrate-remote` は `--remote` で適用する。
  本番への適用はデプロイ手順の中でのみ実行し、`mise run check` には含めない。
- 初期スキーマは 1 つのマイグレーションにまとめ、ADR-0006 の 11 テーブルの全列を入れる。
  - 主キーは TEXT の UUID v4、日時は TEXT の ISO 8601 UTC、日付は TEXT の `YYYY-MM-DD`。
  - 外部キー制約を定義する (D1 は外部キー制約を有効にする)。物理削除はアカウント削除のときだけ行い、子から順に消す (ADR-0006)。
  - `shops`、`products`、`purchases`、`brews` に `created_at`、`updated_at`、`archived_at` を持たせる。
  - `flavor_tags` は利用者ごとに名前で一意にする UNIQUE 制約を持つ。`product_flavor_tags` も `user_id` を持つ。
  - `passkey_credentials.credential_id` に全利用者で一意の UNIQUE 制約を付け、一覧、サジェスト、統計に必要な複合インデックス (利用者 ID、`archived_at`、並び順のキー) を張る。
    この 2 つは ADR-0006 が定めた列に対する本 issue の設計判断であり、ログイン (0005) が credential ID で引けるようにするために付ける。
- 乱数は Web Crypto の `crypto.getRandomValues` を `js_sys` 経由で呼ぶ。
  worker 0.8.6 は `js_sys` を re-export し、`worker::crypto` に乱数の API が無いことは docs.rs で確認した。
  UUID v4 は 16 バイトの乱数からバージョンと variant のビットを立てる純粋な関数として `coffee_log_core` に置き、`uuid` クレートを追加しない。
- 日時はミリ秒 3 桁と末尾 `Z` を持つ固定長の ISO 8601 UTC 文字列にし、辞書順の比較が時刻順と一致するようにする。
  整形は epoch ミリ秒から行う純粋な関数として `coffee_log_core` に置き、PBT で単調性と往復を検証する。
  現在時刻は `worker::Date::now().as_millis()` で取り、純粋な関数には引数で渡す。
  `chrono` と `time` は直接依存にしない (worker クレートが chrono を推移的に持つが、整形と比較だけなら自前で足りる)。
- カーソルは並び順のキー (日時または日付と ID) を JSON にして base64url で符号化する純粋な関数にする。
  復号できない値は 400、ページサイズは既定 50、最大 200、200 を超える指定は 400 とする (PRD の性能)。
  base64url は自前で実装し、クレートを追加しない。
- SQL の組み立ては `coffee_log_core` の純粋な関数に置き、値は必ずプレースホルダで渡す。
  利用者 ID と `archived_at` の条件は、SQL を組み立てる 1 つのモジュールが必ず付ける (ADR-0006)。
  本 issue では全クエリの共通部分 (利用者 ID と `archived_at` の条件、カーソルと limit の組み立て) を用意し、個別のクエリは 0006 以降が追加する。
- D1 の batch は複数の文を 1 トランザクションで実行する場合だけ使い、アカウント削除 (0012) で使う。
- バックアップは D1 の Time Travel (Point-in-Time Recovery) に依存し、独自のバックアップは作らない (ADR-0002)。
- D1 の復旧は Time Travel のコマンド (ブックマークの取得と復元) を使い、手順と実行の条件 (所有者の承認) を README に書く。復旧の訓練は行わない。
- 採らない案: ORM とクエリビルダ (D1 のバインディングに未対応。ADR-0002)、Durable Objects (ADR-0002)、オフセットページング (ADR-0002)、`uuid` クレート (16 バイトの整形だけなので足りる)、`chrono` (整形と比較だけなので足りる)、`base64` クレート (UUID と日付だけを扱うため足りる)。

## 完了条件

- `mise run db-migrate` がローカルの D1 に初期マイグレーションを適用し、`wrangler d1 execute` で 11 テーブルとインデックスの存在を確認できる。
- マイグレーションの SQL が、ADR-0006 が定めた 11 テーブルの列と、本 issue の設計判断の UNIQUE 制約とインデックスの一覧に一致することを、一覧を写したテストデータとの照合テストで確認できる。
- D1 のバインディングを通したプレースホルダ付きの INSERT と SELECT が `wrangler dev` に対する結合テストで成功する。
- UUID v4 の生成、時刻の整形、カーソルの符号化の単体テストと PBT が通る (往復、境界値、無効入力の拒否)。
- ページサイズの既定と上限の検証が単体テストで通る (0 以下、整数でない値、200 を超える値、復号できないカーソルは 400)。
- 一覧のクエリを組み立てる関数が、利用者 ID と `archived_at` の条件を必ず含むことを、生成した SQL の単体テストで確認できる。
- 想定規模 (利用者 20 人、1 利用者あたり店 100 件、商品 1,000 件、購入 3,000 件、抽出 30,000 件) の行数から D1 の 1 データベースの容量の見積もりを issue 本文に追記し、上限に収まることを確認する。
- Time Travel の復旧手順 (ブックマークの取得と復元のコマンド、実行の条件) が README にある。
- `mise run check` が通過する。

## 関連

- 0005 が認証のテーブルを使い、0006 以降が記録のクエリを追加する。
- 0012 がアカウント削除の batch を使う。
- 0018 が同じ D1 データベースを管理者 Worker から参照する。

## 容量の見積もり

想定規模 (利用者 20 人、1 利用者あたり店 100 件、商品 1,000 件、購入 3,000 件、抽出 30,000 件) の行数から、D1 の 1 データベースの容量を次の前提で見積もる。

- UUID 36 バイト、ISO 8601 UTC 24 バイト、日付 10 バイト、SHA-256 のハッシュ 64 バイト、日本語の自由記述は 1 文字 3 バイトとして平均を置く。
- SQLite の 1 行あたりのヘッダと rowid に 10 バイトを足し、複合インデックスの 1 行を 130 バイト、UNIQUE 制約のインデックスの 1 行を 70 バイトとする。

| テーブル | 行数 (上限) | 1 行 | 合計 |
| --- | ---: | ---: | ---: |
| users | 20 | 100 B | 2 KB |
| registration_tokens | 40 | 200 B | 8 KB |
| passkey_credentials | 100 | 370 B | 37 KB |
| webauthn_challenges | 1,000 | 160 B | 160 KB |
| sessions | 1,000 | 200 B | 200 KB |
| shops | 2,000 | 200 B | 400 KB |
| products | 20,000 | 210 B | 4.2 MB |
| flavor_tags | 20,000 | 100 B | 2.0 MB |
| product_flavor_tags | 100,000 | 120 B | 12.0 MB |
| purchases | 60,000 | 320 B | 19.2 MB |
| brews | 600,000 | 320 B | 192.0 MB |
| 複合インデックス (4 本) | 682,000 | 130 B | 88.7 MB |
| UNIQUE 制約のインデックス (2 本) | 20,100 | 70 B | 1.4 MB |
| PRIMARY KEY の自動作成インデックス (11 本) | 804,160 | 70 B | 56.3 MB |
| 合計 | | | 約 376 MB |

- 抽出 (brews) が全体の約半分を占め、次に購入 (purchases)、タグの対応 (product_flavor_tags) が続く。
- D1 の 1 データベースの容量上限は Workers Free プランで 500 MB、Workers Paid プランで 10 GB である (Cloudflare の D1 のドキュメントの記載。この環境では上限の値を再確認していない)。見積もりは約 376 MB で、Free プランの上限の約 7.5 割に収まる。
- 見積もりが最も動くのは brews の notes である。notes の平均が 60 バイトの前提で約 376 MB、平均 300 バイトなら約 520 MB になり、Free プランの上限を超える。Free プランで使う場合は、記録が増えた時点で notes の平均を実測し、上限との余裕を確認する。

## 解決方法

`backend/coffee_log/wrangler.toml` に D1 のバインディングを定義し、`migrations/0001_initial_schema.sql` に 11 テーブルの初期スキーマを置き、`coffee_log_core` に一覧の共通部分 (UUID、日時、カーソル、クエリの組み立て) を追加した。

- `backend/coffee_log/wrangler.toml` に `[[d1_databases]]` を `DB` として追加した (`database_name = "coffee-log"`、`database_id` は本番データベースを作成できないための UUID のプレースホルダで、`wrangler d1 create coffee-log` の値に差し替える手順をコメントと README に書いた)。
- `backend/coffee_log/migrations/0001_initial_schema.sql` に ADR-0006 の 11 テーブル (users、registration_tokens、passkey_credentials、webauthn_challenges、sessions、shops、products、flavor_tags、product_flavor_tags、purchases、brews) の全列、外部キー 15 本、UNIQUE 制約 (`flavor_tags` の `(user_id, name)`、`passkey_credentials.credential_id`)、4 つの複合インデックスを入れた。`product_flavor_tags` は `PRIMARY KEY (product_id, tag_id)`、`purchases.price_currency` は `NOT NULL DEFAULT 'JPY'` とした。
- `coffee_log_core` に `ids` (UUID v4 の生成)、`datetime` (epoch ミリ秒と ISO 8601 UTC の往復。0000-01-01 から 9999-12-31)、`cursor` (カーソルの符号化と復号、ページサイズの検証)、`query` (利用者 ID と `archived_at` の条件を付ける一覧の SQL の組み立て) を追加した。UUID は 16 バイトの乱数から作り、乱数は `backend/coffee_log/src/random.rs` が `globalThis.crypto.getRandomValues` を `Reflect` 経由で呼ぶ (js-sys 0.3.105 には `crypto` の型が無いため)。カーソルの base64url は `coffee_log_core::base64url` に任せ、実装を 1 つにした。
- `backend/coffee_log/src/d1_check.rs` は、本番の vars に無い `D1_CHECK` の var が `true` のときだけ有効な `/api/__d1_check` を台帳外の経路として処理する。結合テストは `wrangler dev --var D1_CHECK:true` でこれを有効化し、プレースホルダ付きの INSERT と SELECT を確認する (`tests/d1_binding.rs`)。
- `tests/schema.rs` はマイグレーションの SQL を解析し、ADR-0006 の列、主キー、UNIQUE 制約、外部キー、複合インデックス (並び順の `DESC` を含む) を、テストデータとして写した一覧と突き合わせる。
- `mise.toml` に `db-migrate` と `db-migrate-remote` を追加し、`backend:test-integration` に `--test d1_binding` を足した。`README.md` にリポジトリの概要、開発の入口、D1 の Time Travel の復旧手順 (ブックマークの取得と復元、実行の条件) を書いた。

完了条件の検証:

- `mise run db-migrate` がローカルの D1 に初期マイグレーションを適用した (wrangler の出力は「16 commands executed successfully」。SQL ファイルの文は 15 で、差分は wrangler が内部の管理テーブルを作る分)。再実行で「No migrations to apply!」になることを確認した。`wrangler d1 execute` で 11 テーブルと 4 インデックスを確認した。
- マイグレーションの SQL と ADR-0006 の列の照合を `tests/schema.rs` の 6 テストで確認した (列、主キー、UNIQUE、外部キー、複合インデックスの向き)。
- プレースホルダ付きの INSERT と SELECT が `wrangler dev` の結合テスト (`tests/d1_binding.rs` の 2 テスト) で成功した。値に SQL 断片を入れて送り、`?` 束縛のまま往復すること、乱数が毎回変わること (UUID が変わらない壊れ方を検出する) も確認した。
- UUID v4、時刻の整形、カーソルの符号化の単体テストと PBT が通った (test_ids 3、test_datetime 9、test_cursor 9、prop_ids 2、prop_datetime 3、prop_cursor 2)。境界値は 1970-01-01、0000-01-01、9999-12-31、うるう年、負の epoch、範囲外、非固定長である。
- ページサイズの既定 50 と上限 200 の検証、0 以下、整数でない値、200 超、復号できないカーソル、カーソルの種類の不一致がすべて 400 になることを `tests/test_cursor.rs` の 9 テストで確認した。
- 一覧のクエリが既定で `WHERE user_id = ?` と `archived_at IS NULL` を必ず含むことを `tests/test_query.rs` の 6 テストで確認した。
- 容量の見積もりを issue の「## 容量の見積もり」に追記した (約 376 MB。PRIMARY KEY の自動作成インデックスを含み、Free プランの上限 500 MB の約 7.5 割に収まる。notes の平均が 300 バイトなら上限を超えるため、実測での確認が必要)。
- Time Travel の復旧手順を README に書いた。
- `mise run check` が通過した (本 issue のテストは 63、0004 を含む統合後の作業ツリーでは 154 テスト)。

方針からの乖離 (方式は変えていない):

- 乱数は `worker::js_sys::crypto` ではなく `globalThis.crypto.getRandomValues` を `Reflect` 経由で呼ぶ (js-sys 0.3.105 に `crypto` の型が無い)。
- D1 の結合テストは台帳に非 PRD の経路を足さず、`D1_CHECK` の var が `true` のときだけ有効な台帳外の経路 (`/api/__d1_check`) で行う。既定は 404 になることをテストで確認する。この経路を残すか削除するかは、0006 以降の結合テストの方針と併せて判断する。
- 複合インデックスの並び順のキーは `DESC` を付ける (`ORDER BY <キー> DESC, id` にインデックスだけで一致させるため)。照合テストは向きも比較する。
- `product_flavor_tags` の `PRIMARY KEY (product_id, tag_id)`、`purchases.price_currency` の `NOT NULL DEFAULT 'JPY'`、`passkey_credentials.credential_id` の列定義の UNIQUE は、ADR-0006 の列に対する追加の制約。
- `query::Archived` の既定は `archived_at IS NULL` を必ず付け、`include_archived` を明示したときだけ外す (完了条件 6 の「必ず含む」は既定の経路を指す)。`datetime` は 0000-01-01 から 9999-12-31 の固定長 24 文字とした。インデックス名は `idx_<table>_user_archived_<key>` とした。
- `wrangler.toml` の `database_id` は UUID のプレースホルダで、本番データベースの作成と差し替えは所有者の作業になる。

却下した指摘:

- 照合テストに列の型、`NOT NULL`、`DEFAULT` の比較を足す案は、列名、主キー、UNIQUE、外部キー、索引の一致で完了条件の照合を満たしており、型の不一致は 0006 以降の結合テスト (クエリの実行) で検出されるため、対応しない。
- 適用結果 (テーブルと索引の実在) の自動検証と外部キー違反の負のテストは、完了条件が手動確認と SQL の照合を求めているため、本 issue では追加しない。0006 以降の結合テストに委ねる。

スコープ外 (報告のみ):

- 本番 D1 の作成と `database_id` の差し替えは、Cloudflare の認証情報と本番操作の権限が無いため本 issue ではできない。0017 のデプロイ前に所有者が `wrangler d1 create coffee-log` を実行する。
