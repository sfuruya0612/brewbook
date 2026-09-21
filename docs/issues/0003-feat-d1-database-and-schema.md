# D1 のバインディングと初期スキーマを入れる

Created: 2026-09-21
Model: deepseek-v4p1-flash
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
