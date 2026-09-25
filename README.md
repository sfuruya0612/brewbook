# coffee-log

コーヒーの購入と抽出を記録するアプリ。
Frontend は Flutter、Backend は Rust の Cloudflare Worker、データベースは Cloudflare D1、写真は Cloudflare R2 を使う。
全体の要求は `docs/pdr/coffee-log.md`、設計の決定は `docs/adr/` にある。

## 開発の前提

- mise を入れ、リポジトリのルートで `mise install` を実行して `mise.toml` のツール (Flutter、Rust、wrangler、worker-build) を入れる。
- コマンドラインは `mise.toml` のタスクに集約する (ADR-0009)。静的検査と全ての自動テストは `mise run check` で実行する。

## ローカル開発

- ローカルの D1 にマイグレーションを適用する: `mise run db-migrate`
- 画面と API を同じオリジンから配信する: `mise run dev`
  タスクは `flutter build web` を実行してから `wrangler dev` を起動する。表示された `http://localhost:8787/` を開く。
- Flutter の開発サーバー (ホットリロード) は使わない。
  別オリジンになり、CORS を許可しない決定と `Origin` の検証に反するためである (ADR-0005)。
- 写真のアップロードを使うときは、R2 の値を `backend/coffee_log/.dev.vars` (git 管理外) に置く。
  必要な名前は `R2_ENDPOINT`、`R2_ACCESS_KEY_ID`、`R2_SECRET_ACCESS_KEY` である。

## データベース (D1)

- スキーマは wrangler の D1 マイグレーションで管理し、`backend/coffee_log/migrations/` に置く (ADR-0002)。
  利用者向けの Worker と管理者 Worker は 1 つのデータベースをバインディングで共有する。
- ローカルの D1 にマイグレーションを適用する: `mise run db-migrate`
- 本番の D1 にマイグレーションを適用する: `mise run db-migrate-remote`
  本番への適用はデプロイ手順の中でのみ実行する (`mise run check` には含めない)。
- 本番の `database_id` は、データベースを作るときに
  `wrangler d1 create coffee-log --config backend/coffee_log/wrangler.toml` が返す値へ
  `backend/coffee_log/wrangler.toml` を書き換えて設定する。

## デプロイとデプロイ後の確認

画面と API は 1 つの Worker (`coffee-log`) が `https://coffee-log.<アカウントのサブドメイン>.workers.dev` で配信する (ADR-0005)。
`mise run deploy` が Flutter のビルドを先に実行してから `wrangler deploy` するため、画面と API は 1 回のデプロイで更新される。
デプロイのタスクは `mise run check` に含めない (0002 の規則)。

デプロイは次の順で行う。

1. 初回だけ、アカウントに合わせて値を設定する。
   - `wrangler login` でアカウントにログインする。
   - D1 のデータベースを作り、`database_id` を `backend/coffee_log/wrangler.toml` に設定する (上の「データベース (D1)」)。
   - workers.dev のサブドメインを確認し、`backend/coffee_log/wrangler.toml` の `[vars]` の `RP_ID` と `ORIGIN` を
     `coffee-log.<サブドメイン>.workers.dev` と `https://coffee-log.<サブドメイン>.workers.dev` に置き換える。
     この 2 つはパスキーの Relying Party ID と、状態を変更する API が検証する同一オリジンになる (ADR-0004、ADR-0005)。
   - `backend/coffee_log/cors.json` の `https://coffee-log.example.workers.dev` を同じオリジンに置き換える。
     写真はブラウザから R2 へ直接 PUT するため、R2 の CORS でアプリのオリジンからの PUT だけを許可する (ADR-0003)。
   - R2 の API トークンを Secret に置く。
     名前は `R2_ENDPOINT`、`R2_ACCESS_KEY_ID`、`R2_SECRET_ACCESS_KEY` で、値はリポジトリに含めない (PRD のセキュリティ)。
   - R2 の CORS とライフサイクルを適用する: `mise run r2-setup`
     CORS のオリジンやライフサイクルを変えたときは、毎回このタスクを実行する。
2. スキーマ変更を含むリリースでは、デプロイの前に本番の D1 へマイグレーションを適用する: `mise run db-migrate-remote`
   本番への適用は意識して実行するため、`mise run deploy` の `depends` には含めない。
3. デプロイする: `mise run deploy`
4. デプロイ後の確認を行う。
   - `https://coffee-log.<サブドメイン>.workers.dev/` を開くと画面が表示される。
   - Flutter のルーティングのパス (`/register` など) を直接開くと `index.html` が 200 で返り、画面が表示される。
   - 登録用リンクからパスキーを登録し、ログインして抽出を保存できる。
   - 購入の画面から写真をアップロードできる (ブラウザから R2 へ直接 PUT する)。
   - 応答時間の p95 を集計し、成功指標を満たしていることを確認する (下の「応答時間の p95 の集計」)。

### 応答時間の p95 の集計

リリース後に、Workers Logs の保持期間の全量で応答時間の p95 を集計し、次の成功指標を満たすことを確認する (PRD の性能)。
集計はリリースごとに所有者が行う。保持期間は Workers Free プランで 3 日、Workers Paid プランで 7 日である
(2026-09-21 に Cloudflare のドキュメントで確認)。

| 対象 | p95 |
| --- | --- |
| 店、商品、購入、抽出の一覧と単件の取得 | 200 ms 以内 |
| 統計 (FR-18) の 4 経路 | 500 ms 以内 |

Worker はリクエスト 1 件ごとに、経路名と処理時間を JSON の行で Workers Logs に出す (`backend/coffee_log/src/logging.rs`)。
ダッシュボードの Query Builder で集計する。

1. Cloudflare ダッシュボードの Workers & Pages で `coffee-log` を選ぶ。
2. Observability の Overview の Query Builder を開く。
3. Visualization で `P95` を選び、フィールドに `duration_ms` を指定する。
4. Filter で `event` が `request` の行に絞り、Group By に `route` を指定する。
   経路の名前は `backend/coffee_log_core/src/routes.rs` の台帳と同じで、`mise run dev` のログと突き合わせられる。
5. 期間に保持期間の全量 (Free 3 日、Paid 7 日) を指定する。
6. Run で実行し、一覧と単件の経路が 200 ms 以内、統計の 4 経路 (`stats_brews`、`stats_purchases`、
   `stats_brew_ratings`、`purchases_rating_history`) が 500 ms 以内であることを確認する。
   写真の取得 (`purchases_photo_get`) は対象外である。

同じクエリは Workers Observability の REST API からも実行できる。集計の結果はリリースの記録として残す。

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
