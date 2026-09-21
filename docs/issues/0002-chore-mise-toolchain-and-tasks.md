# mise のタスクと GitHub Actions の CI を整備する

Created: 2026-09-21
Model: deepseek-v4p1-flash
対応 ADR: ADR-0009 (docs/adr/0009-mise-toolchain-and-tasks.md)、ADR-0007 (docs/adr/0007-frontend-flutter-web-first.md)
関連 PRD: 運用 (デプロイ手順は mise のタスク、CI は全自動テスト)、制約と前提 (CI は同じ mise.toml を使う、成果物の規約)
依存: 0001

## 背景

ADR-0009 は、ツールの版と、ビルド、デプロイ、テスト、ローカル開発の起動、D1 のマイグレーション、デプロイ後の確認のコマンドラインを `mise.toml` に集約すると決めた。
`mise.toml` には `[tools]` があり Flutter 3.47.5、Rust 1.98.1 (wasm32-unknown-unknown ターゲット付き)、wrangler 4.135.0 を固定しているが、`[tasks]` が無い。
CI のワークフローもリポジトリに無い。
PRD は CI が Backend と Frontend の全自動テストを実行し、全て通過するまでマージしないことを前提にする (所有者の共通規約)。
0001 で `backend/` の Rust ワークスペースができ、直接 `cargo` を実行して確認できる状態になった。

## 目的

手元と CI が同じ `mise.toml` と同じタスクでビルド、テスト、デプロイを実行できるようにする。

## 設計判断

- タスクは全体のタスクと `backend:*`、`frontend:*` の階層にし、`check` が fmt、lint、test を `depends` で束ねる形にする (既存リポジトリの `mise run check` と同じ入口にする)。
- 本 issue が定義するタスクは `setup`、`fmt`、`lint`、`test`、`backend:build`、`backend:test`、`backend:test-integration`、`check` とする。
  `backend:test` はネイティブターゲットの単体テストと PBT、`backend:test-integration` は `wrangler dev` を起動する結合テストとする。
  残りは対象ができた issue が追加する。
  `db-migrate` と `db-migrate-remote` は 0003、`r2-setup` は 0009、`frontend:*` は 0013 (0015 と 0016 がテストを追加)、`dev` と `deploy` は 0017、`deploy-admin` と `verify-deploy` は 0018 が追加する。
- `check` は CI と手元の最終確認の共通の入口とし、`mise run check` だけで静的検査と全ての自動テスト (結合テストと Flutter のテストを含む) が走るようにする。
  `db-migrate-remote`、`r2-setup`、`deploy`、`deploy-admin`、`verify-deploy` は本番環境に触るため `check` に含めない。
  Flutter 側のタスクが無い間は Backend の検査だけになる。
- タスクの中に独自のロジックを書かない (ADR-0009)。
  `run` は標準のコマンドの組み合わせにし、`depends` と `dir` だけを使い、条件分岐やループを書かない。
- CI は GitHub Actions とし、`.github/workflows/ci.yml` に置く。
  `jdx/mise-action` で mise を入れ、`mise run check` を実行する。
  テストが使うブラウザはワークフローで版を固定して入れる例外とし、理由を 0013 に記録する (ADR-0009 の「CI も同じ mise.toml を使う」はツールチェーン (Flutter、Rust、wrangler) とタスクの共有を指す)。
  所有者のリポジトリは GitHub で運用されているため GitHub Actions を選ぶ。
  リポジトリに remote がまだ無いため、ワークフローの実行確認は remote の設定後になる。
  採らない案: ワークフローにコマンドを直接書く (手元と二重管理になり ADR-0009 に反する)、CircleCI (GitHub の運用に合わせる)。
- action の版はタグで固定する (版を固定する方針は ADR-0009 と同じ)。
- `.gitignore` をリポジトリのルートに置き、`backend/target/`、`frontend/build/`、`frontend/.dart_tool/`、`.wrangler/`、`.dev.vars` を無視する。
  `Cargo.lock` はコミットする (アプリケーションであり、手元と CI で同じ依存の解決を使うため)。
- 成果物の規約 (絵文字を使わない、コメントは日本語、ログとエラーメッセージは英語) のうち、絵文字は `mise run lint` に含める検査で検出する。
  `[tools]` に ripgrep を追加し、リポジトリの追跡対象のファイルに絵文字が無いことを検査する。
  コメントの言語とログの言語は、機械的な判定が難しいためレビューのチェック項目とする。
- iOS のビルドはタスクにも CI にも入れない (ADR-0007)。
  Android のビルドも対象外とする (PRD のやらないこと)。

## 完了条件

- `mise.toml` の `[tasks]` に本 issue のタスクがあり、`mise tasks ls` に現れる。
- `mise run backend:build`、`mise run backend:test`、`mise run backend:test-integration`、`mise run fmt`、`mise run lint`、`mise run test`、`mise run check` が成功する。
- `mise run check` に本番環境に触るタスク (`db-migrate-remote`、`r2-setup`、`deploy`、`deploy-admin`、`verify-deploy`) が含まれない。
- `.github/workflows/ci.yml` が `jdx/mise-action` で mise を入れ、`mise run check` を実行する。action の版がタグで固定されている。
- `.gitignore` が追加され、ビルド生成物が `git status` に現れない。
- `mise run lint` に絵文字の検査が含まれ、絵文字を故意に含むファイルで失敗する。
- wrangler が mise 以外の経路 (リポジトリの `package.json`、全体インストール) で管理されていない。
- `mise.toml` を変更せずに `mise run check` が成功する (タスクが版を二重に持っていない)。

## 関連

- 0001 が Rust ワークスペースを作る。
- 0003 が `db-migrate` と `db-migrate-remote` を追加する。
- 0009 が `r2-setup` を追加する。
- 0013 が `frontend:*` を追加する。
- 0017 が `dev` と `deploy` を追加する。
- 0018 が `deploy-admin` と `verify-deploy` を追加する。
