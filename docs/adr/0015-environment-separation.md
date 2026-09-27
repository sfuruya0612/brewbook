# ADR-0015: 環境をローカル、staging、production に分ける

Created: 2026-09-27
Model: DeepSeek V4.1 Flash
Status: Accepted

## 背景

2026-09-27 時点の構成は、Worker `brewbook` (利用者向け) と `brewbook-admin` (管理者)、
D1 `brewbook`、R2 `brewbook-photos` の 1 組を本番として動かし、ローカル開発は `wrangler dev` の
`--var` で本番の `RP_ID` と `ORIGIN` を上書きしていた。
検証用の環境が無く、デプロイを伴う変更を本番で直接試すしかない。
所有者が、ローカル、develop (検証用)、production の環境分離を決めた。

## 決定

環境は 3 つとし、リモートのリソースは環境ごとに分ける (ADR-0002、0003、0005、0008 を環境の数だけ適用する)。

| 環境 | Worker (利用者向け / 管理者) | D1 | R2 |
| --- | --- | --- | --- |
| ローカル | miniflare (`wrangler dev`、名前は `brewbook-local` / `brewbook-admin-local`) | ローカル | ローカル |
| staging | `brewbook-staging` / `brewbook-admin-staging` | `brewbook-staging` | `brewbook-photos-staging` |
| production | `brewbook` / `brewbook-admin` | `brewbook` | `brewbook-photos` |

- `wrangler.toml` は 1 ファイルとし、トップレベルをローカル専用、`[env.staging]` と `[env.production]` を
  リモートの環境とする。vars とバインディング (D1、R2) は環境ごとに定義する (wrangler の非継承の仕様)。
  `assets`、`build`、`observability` はトップレベルで定義し、環境が継承する。
- トップレベルの名前は `brewbook-local`、`brewbook-admin-local` とする。
  誤って `wrangler deploy` (--env なし) を実行しても、本番 (`brewbook`) を更新しないためである。
  リモートの環境へは必ず `--env` を付ける。
- パスキーの `RP_ID` と `ORIGIN`、管理者の `APP_ORIGIN` は環境ごとの vars に持つ。
  ローカルは `localhost`、staging は `brewbook-staging.<アカウントのサブドメイン>.workers.dev`、
  production は `brewbook.<アカウントのサブドメイン>.workers.dev` とする。
- Secrets (`R2_ENDPOINT`、`R2_ACCESS_KEY_ID`、`R2_SECRET_ACCESS_KEY`) は環境ごとに設定し、
  R2 の API トークンも環境ごとに分ける。ローカルの `.dev.vars` は staging の値を置き、
  本番の資格情報をローカルの端末に置かない。
- R2 の CORS も環境ごとに持つ。`cors.json` は本番のバケット用で本番のオリジンだけを許可し、
  `cors.staging.json` は staging のバケット用でローカルの開発と staging のオリジンを許可する。
  ライフサイクル (`lifecycle.json`) は共通とする。
- mise のタスクは環境を名前に含める。
  `deploy-app-staging`、`deploy-app-production` (利用者向け)、
  `deploy-admin-staging`、`deploy-admin-production` (管理者)、
  `deploy-staging`、`deploy-production` (両方をまとめてキックする集約)、
  `db-migrate-staging`、`db-migrate-production`、`r2-setup-staging`、`r2-setup-production`。
  初回のリソースの作成も `d1-create-staging`、`r2-create-staging`
  (production は `d1-create-production`、`r2-create-production`) として持つ。
  `wrangler d1 create` が返す `database_id` の `wrangler.toml` への設定は自動化せず、README の手順で行う
  (ADR-0009: タスクの中に独自のロジックを書かない)。
  ローカルの `dev`、`dev-admin`、`db-migrate` は環境名を付けない。
- Cloudflare Access の保護は環境ごとの管理者 Worker (`brewbook-admin-staging` と `brewbook-admin`) に設定する。
  利用者向けの Worker には設定しない (ADR-0008)。
- production のリソース名と URL は変えない。既存の D1 (`brewbook`) と R2 (`brewbook-photos`) をそのまま使う。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| 環境ごとに `wrangler.toml` を分けて `--config` で切り替える | 設定の大半が共通で重複が多くなる。wrangler の環境 (`[env.*]`) で同じ分離ができる |
| トップレベルを本番のままにし、`--env` を任意にする | `--env` の付け忘れによる `wrangler deploy` が本番を更新する。事故を防ぐ名前の分離を選んだ |
| 検証用の環境を作らず本番で試す | 利用者のデータを壊す。D1 の Time Travel での復旧はできるが、検証のたびに本番を巻き戻す運用は取らない |
| Cloudflare の Preview URLs で検証する | D1 と R2 が本番と分離されない。URL が毎回変わり、パスキーの `RP_ID` と登録用リンクのオリジンを固定できない |
| ローカル専用の 3 つ目のリモート環境 (development) を作る | ローカルは miniflare で完結し、リモートの開発環境を必要とする作業が無い。増やすとリソースと Secret の管理だけが増える |

## 結果

- 本番の Worker 名、URL、D1、R2 は変わらない。既存のデプロイはそのまま動く。
- staging の初回の設定 (D1 と R2 の作成、`database_id` の設定、Secrets、CORS、Access) が必要になる。
  手順は README の「デプロイとデプロイ後の確認」に置く。
- ローカルの D1 は、バインディングの名前 (`brewbook-local`) が変わるため、以前のローカルの状態とは別になる。
  ローカルは `mise run db-migrate` で作り直せる。R2 のローカルは名前を変えていないため、以前の状態をそのまま使う。
- staging の D1、R2、Worker は無料枠の範囲で動かす想定である。課金が発生する構成 (Workers Paid など) は取らない。
- 環境を増やすときは本 ADR を改訂し、README の表と mise のタスクを合わせて更新する。
