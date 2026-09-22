# mise のタスクと GitHub Actions の CI を整備する

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-22
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

## 解決方法

`mise.toml` に 8 つのタスクを追加して `check` を静的検査と全自動テストの共通の入口にし、GitHub Actions の CI を追加し、`.gitignore` を frontend と wrangler の生成物まで広げた。

- `mise.toml` の `[tools]` に `ripgrep = "15.2.0"` を追加した。 `[tasks]` に `setup`、`fmt`、`lint`、`test`、`check`、`backend:build`、`backend:test`、`backend:test-integration` を追加した。 `check` は `fmt`、`lint`、`test` に依存し、 `test` は `backend:test` と `backend:test-integration` に依存する。
- `backend:test` は `cargo test --workspace -- --skip wrangler_` の 1 コマンドとした。 テストの対象は自動で検出され、 `wrangler dev` を起動するテストは名前に `wrangler_` を付ける規約で除外する。 `backend:test-integration` は `cargo test -p coffee_log --test dev_server` でそのテストを実行する。 これに合わせて `dev_server` のテスト名を `wrangler_dev_server_returns_json_404_and_logs_the_request` に変更した。
- `lint` は clippy (`cargo clippy --manifest-path backend/Cargo.toml --workspace --all-targets -- -D warnings`)、 `rg --version`、絵文字の検査 (`! rg --hidden -g '!.git' --pcre2 '(\p{Emoji_Presentation}|\p{Emoji}\x{FE0F})'`) の 3 コマンドとした。 絵文字の検査は `.gitignore` に従って走査し、 U+FE0F (絵文字表示セレクタ) で絵文字表示になる文字 (U+2764 と U+FE0F の組み合わせなど) も検出する。
- `.github/workflows/ci.yml` は `actions/checkout@v7.0.1`、 `jdx/mise-action@v4.3.0`、 `run: mise run check` の 3 ステップとし、 `permissions: contents: read` と `timeout-minutes: 30` を付けた。 リポジトリに remote が無いため、ワークフローの実行確認は未実施。
- `.gitignore` に `/frontend/build/`、 `/frontend/.dart_tool/`、 `.wrangler/`、 `.dev.vars` を追加した。 既存の `/backend/coffee_log/.wrangler/` はアンカー無しの `.wrangler/` に置き換え、 backend 以外の wrangler の状態も除外する。

完了条件の検証:

- `mise tasks ls` に 8 タスクが現れることを確認した。
- `mise run backend:build`、 `mise run backend:test`、 `mise run backend:test-integration`、 `mise run fmt`、 `mise run lint`、 `mise run test`、 `mise run check` が worktree で成功し、 統合後の作業ツリーでも `mise run check` が終了コード 0 で 21 テストが通ることを確認した。
- `backend:test` は 20 テスト (21 から `wrangler dev` を起動する 1 件を除いた数) を実行し、 `wrangler` を起動しないことを確認した (1 filtered out)。
- `check` の依存の閉包は `fmt`、 `lint`、 `test`、 `backend:test`、 `backend:test-integration` だけで、 本番環境に触るタスクは含まれない (そもそも定義が無い)。
- `.github/workflows/ci.yml` が `jdx/mise-action` で mise を入れ、 `mise run check` を実行する。 action の版は完全なタグで固定し、 `git ls-remote` で実在を確認した。
- `.gitignore` の追加後、 `mise run backend:build` と `backend:test-integration` の実行後もビルド生成物が `git status --porcelain` に現れないことを確認した。
- `mise run lint` の絵文字の検査が、 U+1F600、 U+2764 U+FE0F、 U+0031 U+FE0F U+20E3 をそれぞれ含む一時ファイルで終了コード 1 になり、 削除すると終了コード 0 になることを確認した。
- リポジトリは wrangler を `mise.toml` の `[tools]` だけで管理する。 追跡下に `package.json` は無く、 CI は mise-action で入れ、 タスクに版の指定は無い。 ホストには Homebrew の wrangler が元から入っているが (`whence -a wrangler` の 2 件目)、 リポジトリはそれに依存せず、 テストが使うのは PATH で先に解決される mise の 4.135.0 である。
- タスクのコマンドは `cargo`、 `worker-build`、 `rg` の素の呼び出しだけで版を持たない (`wrangler` は結合テストのハーネスが PATH から起動する)。 `mise.toml` を変更せずに `mise run check` が成功する (版は `[tools]` の 1 か所)。

方針からの乖離 (方式は変えていない):

- `fmt` は整形の検査 (`cargo fmt --all --check`) とした。 `check` が `fmt` に依存するため、 未整形を検出して失敗させる必要がある。
- `lint` に clippy と絵文字の検査をまとめ、 `rg --version` を先に実行するようにした。 方針が本 issue のタスクを 8 つに固定しているため。
- `setup` は `cargo fetch` とした。 frontend 側の依存は 0013 が追加する。
- CI ワークフローに `permissions: contents: read` と `timeout-minutes: 30` を付けた (検査に必要な最小権限と、 外部プロセスを起動するテストのハングで runner を占有しないため)。
- `backend:test` の除外は、 テスト名の列挙ではなく `wrangler_` プレフィックスの名前規約にした。 テストを追加する issue が除外の追加を忘れても、 命名規約に沿えば自動で除外される。
- 絵文字の検査の走査対象は、 設計判断の文言の「追跡対象のファイル」ではなく ripgrep の `.gitignore` の解釈に従う。 追跡下でも `.gitignore` に一致するファイルは対象外になり、 未追跡でも無視されていないファイルは対象になる。 現時点で追跡下かつ無視されるファイルは無い。
- 絵文字の検査の限界は 2 つある。 `!` が終了状態を反転するため ripgrep の検索中のエラー (終了コード 2) も成功になる (ripgrep が見つからない場合は先頭の `rg --version` で失敗する)。 また、 検出するのは `Emoji_Presentation` と、 `Emoji` の直後に U+FE0F が続く文字だけで、 U+FE0F を伴わないテキスト表示の記号 (U+00A9 など) は検出しない (誤検出を避けるため)。

スコープ外 (報告のみ):

- 絵文字の検査が ripgrep の検索中のエラーも成功にする点は、 ADR-0009 がタスク内の条件分岐を禁じるため、 タスク内では解消しない。 恒久的に塞ぐには Rust の検査コードが必要になる。
