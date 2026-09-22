# coffee-log

コーヒーの購入と抽出を記録するアプリ。
Frontend は Flutter、Backend は Rust の Cloudflare Worker、データベースは Cloudflare D1、写真は Cloudflare R2 を使う。
全体の要求は `docs/pdr/coffee-log.md`、設計の決定は `docs/adr/` にある。

## 開発の前提

- mise を入れ、リポジトリのルートで `mise install` を実行して `mise.toml` のツール (Flutter、Rust、wrangler、worker-build) を入れる。
- コマンドラインは `mise.toml` のタスクに集約する (ADR-0009)。静的検査と全ての自動テストは `mise run check` で実行する。

## データベース (D1)

- スキーマは wrangler の D1 マイグレーションで管理し、`backend/coffee_log/migrations/` に置く (ADR-0002)。
  利用者向けの Worker と管理者 Worker は 1 つのデータベースをバインディングで共有する。
- ローカルの D1 にマイグレーションを適用する: `mise run db-migrate`
- 本番の D1 にマイグレーションを適用する: `mise run db-migrate-remote`
  本番への適用はデプロイ手順の中でのみ実行する (`mise run check` には含めない)。
- 本番の `database_id` は、データベースを作るときに
  `wrangler d1 create coffee-log --config backend/coffee_log/wrangler.toml` が返す値へ
  `backend/coffee_log/wrangler.toml` を書き換えて設定する。

## D1 の復旧 (Time Travel)

バックアップは D1 の Time Travel (Point-in-Time Recovery) に依存し、独自のバックアップは作らない (ADR-0002)。
復旧できる期間は Workers Free プランで 7 日、Workers Paid プランで 30 日である (2026-09-21 に Cloudflare のドキュメントで確認)。
復旧の訓練は行わない。

### ブックマークの取得

復旧のときに使うブックマークを取得する。コマンドは本番のデータベースだけを対象にする。

```sh
wrangler d1 time-travel info coffee-log --config backend/coffee_log/wrangler.toml
```

- 出力のブックマークを控える。`--json` を付けると JSON で出力する。
- `--timestamp <RFC3339 または Unix 秒>` を付けると、その時点のブックマークを取得できる。
- 復旧の前に、必ず現在のブックマークを控える。直前の状態に戻すときに使う。

### 復元

```sh
wrangler d1 time-travel restore coffee-log --bookmark <ブックマーク> --config backend/coffee_log/wrangler.toml
```

- `--bookmark` の代わりに `--timestamp` でも時点を指定できる。
- 復元は本番のデータベースを過去の時点に巻き戻し、その時点より後に書かれた行は失われる。

### 実行の条件

- 復元の実行は所有者の承認を得たときだけ行う。
- 実行の前に現在のブックマークを取得して控える。巻き戻しを取り消すときに、そのブックマークで復元する。
- 復元の後、`/api` の応答と主要な画面の動作を確認し、結果を所有者に報告する。
- マイグレーションは復元の対象に含まれる。復元した時点より後に適用したマイグレーションは未適用の状態に戻るため、必要なら再度 `mise run db-migrate-remote` で適用する。
- ローカルの D1 は対象外とする。ローカルの状態は `mise run db-migrate` で作り直せる。
