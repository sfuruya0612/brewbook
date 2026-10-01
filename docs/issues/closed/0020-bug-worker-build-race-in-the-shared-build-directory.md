# worker-build が同じディレクトリで並行に走ると build/.tmp が消えて wrangler dev が起動しない

Created: 2026-09-23
Model: deepseek-v4p1-flash
Completed: 2026-10-02

## 症状

結合テストで `wrangler dev` が起動に失敗し、次の出力で終了コード 1 になる。

```
[custom build] Running: worker-build --release
[custom build] [INFO]: Checking for the Wasm target...
[custom build] [INFO]: Compiling to Wasm...
[custom build] Error: Failed to read /Users/user/apps/coffee/backend/brew_book/build/.tmp/package.json
[custom build]     No such file or directory (os error 2)
[ERROR] Process exited with non-zero status (1)
```

テストは `wrangler dev must start: "wrangler dev exited before ready with exit status: 1"` で失敗する (2026-09-23 の `mise run check` の `d1_binding` の `wrangler_d1_binding_inserts_and_selects_with_placeholders` で発生。リポジトリの 0019 とは別の事象)。

## 再現手順

1. リポジトリのルートで `mise run backend:test-integration` (または `mise run check`) を実行する。
2. `d1_binding` の 2 テスト (`wrangler_d1_binding_inserts_and_selects_with_placeholders` と `wrangler_d1_check_is_disabled_without_its_var`) が同じテストバイナリ内で並行に走り、それぞれが `wrangler dev` を起動する。`wrangler dev` は `backend/brew_book` で `worker-build --release` を実行するため、2 つのビルドが同じ `backend/brew_book/build/.tmp` を使う。
3. 片方のビルドが `build/.tmp` を作り直している間に、もう片方が `build/.tmp/package.json` を読むと、`No such file or directory` で失敗する (発生は確率的。負荷が高いときに起きやすい)。

## 原因

`backend/brew_book/tests/support/mod.rs` の `DevServer::start_with` は、テストごとに `wrangler dev` を起動し、その中で `worker-build --release` が同じ `backend/brew_book` の `build/` ディレクトリへ出力する。同じテストバイナリの並行テスト (および同時に走る別のテストバイナリ) はこのディレクトリを共有するため、`build/.tmp` の作成と読み取りが競合する。
`wrangler dev` は `main = "build/worker/shim.mjs"` を読む前に `[build] command = "worker-build --release"` を実行するため、ビルドの完了が起動の前提になっている。

## 完了条件

- 同じテストバイナリの並行テストで `wrangler dev` を起動しても、`build/.tmp` の競合でビルドが失敗しない (ビルドの出力先をテストごとに分ける、ビルドを 1 回にまとめる、ビルドを直列化するなどの方法で)。
- `mise run backend:test-integration` を 5 回連続で実行して、`d1_binding` が失敗しない。
- `mise run check` が通過する。

## 解決方法

- テストの `wrangler dev` にビルドさせないようにした。`backend/brew_book/tests/support/mod.rs` の `ensure_worker_built` が worker をプロセスごとに 1 回だけビルドし、`build/worker/shim.mjs` を使い回す。ビルドは `build/.worker-build.lock` のファイルロック (`flock`) でプロセス間でも直列化するため、`build/.tmp` の競合が起きない。
- `wrangler dev` には、`wrangler.toml` から `[build]` を外してパスを絶対パスにした一時の設定を `--config` で渡す。`mise.toml` の `backend:test-integration` と `frontend:test-same-origin` は `backend:build` に依存し、`BREWBOOK_SKIP_WORKER_BUILD=1` を設定して、タスクが先にビルドした worker を再利用する。check の中のテストの起動 (約 20 か所) ごとに走っていた wasm のリリースビルドは、タスクの事前ビルド 1 回になる。
- ビルドが起動を直列化しなくなったことで、並行に起動するテストの間で空きポートの確認と `wrangler dev` の bind が競合し、workerd が `Address already in use` で終了する事象が表面化した。`start_with_assets` はこのメッセージで失敗したときに別のポートで起動をやり直す (最大 3 回)。`vars` を組み立てるクロージャは `FnOnce` から `Fn` に変えた。
- 再現確認 (2026-10-02、macOS、M3 Pro、36 GB):
  - 修正前 (ビルドが起動を直列化する状態) の `cargo test -p brew_book --test d1_binding` は 3 回連続で通過した。
  - ビルドを外しただけの状態では、同じテストが 3 回連続で `Address already in use` により失敗した。再試行の追加後は 3 回連続で通過した。
  - `cargo test -p brew_book --test dev_server` (3 テストが並列) は 3 回連続で通過し、実行後の残存プロセスは 0 だった。
- 完了条件の確認:
  - `mise run backend:test-integration` を 5 回連続で実行し、全て通過した (671.02、400.39、413.83、396.30、397.02 秒)。`d1_binding` は 5 回とも成功した。各実行の直後の `workerd`、`wrangler dev`、esbuild の残存は 0 だった。
  - `mise run --jobs 1 check` が通過した (2254.64 秒)。既定の並列実行では `frontend:test-web` が Flutter の `build` ディレクトリの競合で失敗したが、0020 の変更は frontend のタスクに触れておらず、別の issue (0036) として登録した。
