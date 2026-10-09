# ADR-0021: main への push を契機に Workers Builds で本番へ自動デプロイする

Created: 2026-10-09
Model: DeepSeek V4.1 Flash
Status: Accepted

## 背景

2026-10-09 時点の本番のデプロイは、所有者が手元で `mise run deploy-production` を実行する手動の手順である (README)。
Frontend のビルドと Rust のビルドと `wrangler deploy` を手元で実行し、スキーマ変更を含むリリースでは
先に `mise run db-migrate-production` を実行する必要がある。
デプロイの手順が人の記憶に依存し、マイグレーションの適用を忘れると本番のコードとスキーマがずれる。
所有者が、main への push を契機に本番へ自動でデプロイすることを決めた。

## 決定

- 本番 (利用者向け `brewbook` と管理者 `brewbook-admin`) のデプロイは、
  Cloudflare Workers Builds (Cloudflare の GitHub 連携) で行う。
  main への push を契機に Build command でビルドし、Deploy command でデプロイする。
- 接続は Worker ごとに行う。Root directory は利用者向けが `backend/brew_book`、管理者が `backend/brew_book_admin`。
  Build command と Deploy command の内容は README の「main への push からの自動デプロイ」に置く。
- ビルド環境 (Ubuntu 24.04) には Rust と mise が入っていないため、Build command で mise を入れ、
  `mise.toml` の版でデプロイに要るツール (Rust、worker-build、Node.js、wrangler、Dioxus CLI) だけを入れる (ADR-0009)。
  Node.js は wrangler (npm のパッケージ) の install dependency で、`mise.toml` に無いと wrangler を入れられない
  (2026-10-09 に mise 2026.10.4 で確認)。
  k6、Java、chromedriver などデプロイに不要なツールは入れない。
- Workers Builds は `wrangler.toml` の `[build]` (Custom Builds) を Build command として使わない
  (2026-10-09 に Cloudflare のドキュメントで確認)。
  `worker-build --release` は Deploy command の `wrangler deploy` が通常どおり実行するため、
  Build command では実行しない (ビルドの二重実行を避ける)。
- 本番の D1 へのマイグレーションの適用は、利用者向けの Deploy command がデプロイの直前に実行する。
  適用済みのマイグレーションは飛ばすため、毎回の実行で問題はない。
  手元からのデプロイ (`mise run deploy-production`) は切り分けと緊急のリリースのために残し、
  その場合は従来どおり `mise run db-migrate-production` を先に実行する。
- Workers Builds 用の API トークンは所有者が作り、Workers Scripts の Edit、D1 の Edit、
  Account Settings の Read を与える。
  自動で発行されるトークンには D1 の権限が無い (2026-10-09 に Cloudflare のドキュメントで確認)。
- main を保護し、CI (`check`) を必須の status check にすることを推奨する。
  Workers Builds は push と同時にデプロイするため、検証を通ったコミットだけを main に入れる。
- ビルドの上限は 20 分である。Workers Builds は Rust のビルドキャッシュを持たず、毎回コールドビルドになる。
  所要が上限を超える場合は、GitHub Actions から `mise` のタスクを実行する方式に切り替える。
- staging のデプロイは手動のままとする。develop ブランチが無く、必要になったときに追加する。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| GitHub Actions から wrangler でデプロイする | 既存の CI のキャッシュと mise のタスクをそのまま使えるが、所有者が Cloudflare のダッシュボードでビルドの履歴と版の管理を一箇所にすることを選んだ |
| 手動のデプロイを続ける | デプロイとマイグレーションの適用が人の記憶に依存する |
| マイグレーションを手動のままにする | 自動デプロイとスキーマ変更の順序が人の手順に依存し、適用忘れの事故が起きる |
| staging も develop ブランチで自動デプロイする | develop ブランチが無く、staging を使う頻度が低い |

## 結果

- main への push で本番が更新され、マイグレーションもデプロイの直前に適用される。
  デプロイの履歴と版は Cloudflare のダッシュボードで確認できる。
- ビルドは Cloudflare のビルド環境で毎回コールドビルドになり、20 分の上限に近くなる可能性がある。
  上限を超える場合は GitHub Actions に切り替える。
- Workers Builds のビルド時間はアカウントの枠 (Free プランは月 3,000 分) を消費する。
- 管理者のデプロイは利用者向けのマイグレーションの適用と並行するため、スキーマ変更を含むリリースでは、
  管理者の新しいコードが古いスキーマで動く短い期間が生じ得る。
- デプロイの設定は Cloudflare のダッシュボードにあり、リポジトリのファイルには無い。
  手順は README に置き、変更するときは README と本 ADR を更新する。
