# worker-build が同じディレクトリで並行に走ると build/.tmp が消えて wrangler dev が起動しない

Created: 2026-09-23
Model: deepseek-v4p1-flash

## 症状

結合テストで `wrangler dev` が起動に失敗し、次の出力で終了コード 1 になる。

```
[custom build] Running: worker-build --release
[custom build] [INFO]: 🎯  Checking for the Wasm target...
[custom build] [INFO]: 🌀  Compiling to Wasm...
[custom build] Error: Failed to read /Users/user/apps/coffee/backend/coffee_log/build/.tmp/package.json
[custom build]     No such file or directory (os error 2)
✘ [ERROR] Process exited with non-zero status (1)
```

テストは `wrangler dev must start: "wrangler dev exited before ready with exit status: 1"` で失敗する (2026-09-23 の `mise run check` の `d1_binding` の `wrangler_d1_binding_inserts_and_selects_with_placeholders` で発生。リポジトリの 0019 とは別の事象)。

## 再現手順

1. リポジトリのルートで `mise run backend:test-integration` (または `mise run check`) を実行する。
2. `d1_binding` の 2 テスト (`wrangler_d1_binding_inserts_and_selects_with_placeholders` と `wrangler_d1_check_is_disabled_without_its_var`) が同じテストバイナリ内で並行に走り、それぞれが `wrangler dev` を起動する。`wrangler dev` は `backend/coffee_log` で `worker-build --release` を実行するため、2 つのビルドが同じ `backend/coffee_log/build/.tmp` を使う。
3. 片方のビルドが `build/.tmp` を作り直している間に、もう片方が `build/.tmp/package.json` を読むと、`No such file or directory` で失敗する (発生は確率的。負荷が高いときに起きやすい)。

## 原因

`backend/coffee_log/tests/support/mod.rs` の `DevServer::start_with` は、テストごとに `wrangler dev` を起動し、その中で `worker-build --release` が同じ `backend/coffee_log` の `build/` ディレクトリへ出力する。同じテストバイナリの並行テスト (および同時に走る別のテストバイナリ) はこのディレクトリを共有するため、`build/.tmp` の作成と読み取りが競合する。
`wrangler dev` は `main = "build/worker/shim.mjs"` を読む前に `[build] command = "worker-build --release"` を実行するため、ビルドの完了が起動の前提になっている。

## 完了条件

- 同じテストバイナリの並行テストで `wrangler dev` を起動しても、`build/.tmp` の競合でビルドが失敗しない (ビルドの出力先をテストごとに分ける、ビルドを 1 回にまとめる、ビルドを直列化するなどの方法で)。
- `mise run backend:test-integration` を 5 回連続で実行して、`d1_binding` が失敗しない。
- `mise run check` が通過する。
