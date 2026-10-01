# 結合テストの DevServer が workerd のプロセスを残す

Created: 2026-09-23
Model: deepseek-v4p1-flash
Completed: 2026-10-01

## 症状

`wrangler dev` を起動する結合テスト (`mise run backend:test-integration`、`mise run check`) の実行後、`workerd serve --binary` のプロセスが残る。
2026-09-23 の観測では、1 回の `mise run check` の後に 63 個、過去の実行を含めると 477 個が残っていた。
残ったプロセスが多い状態では、`wrangler d1 migrations apply` が `✘ [ERROR] connect EADDRNOTAVAIL 127.0.0.1:<ポート> - Local (0.0.0.0:0)` で終了コード 1 になり、`backend:test-integration` の `d1_binding` が失敗する。
残っているプロセスを全て終了させると、同じテストは通る。

## 再現手順

1. リポジトリのルートで `mise run backend:test-integration` (または `mise run check`) を実行する。
2. `ps -eo pid,command | grep 'workerd serve --binary' | grep -v grep | wc -l` で残っている workerd の数を数える。起動したテストサーバーの数だけ残る。
3. 残った workerd が多数ある状態で `cd backend/brew_book && wrangler d1 migrations apply DB --local --persist-to /tmp/probe` を実行すると、`fetch failed` (wrangler のログには `connect EADDRNOTAVAIL 127.0.0.1:<ポート>`) で終了コード 1 になる。残っているプロセスを全て終了させると、同じコマンドは成功する。
4. `mise run check` を繰り返すと、`backend:test-integration` の `d1_binding` (`wrangler_d1_binding_inserts_and_selects_with_placeholders`、`wrangler_d1_check_is_disabled_without_its_var`) が `wrangler dev must start: "wrangler d1 migrations apply failed with exit status: 1"` で失敗する。

## 原因

`backend/brew_book/tests/support/mod.rs` の `DevServer::stop` は `child.kill()` で `wrangler` のプロセスだけを SIGKILL する。`wrangler` は SIGKILL では後始末ができず、その子の `workerd` が孤児として残る。
残った `workerd` がローカルのポートを使い続けるため、新しい `wrangler` の接続が `EADDRNOTAVAIL` になると考えられる。この因果は、477 個残った状態で失敗し、全て終了させた後に成功した観測に基づく仮説である。

## 完了条件

- `mise run backend:test-integration` の実行直後に `ps -eo pid,command | grep 'workerd serve --binary' | grep -v grep` が何も出さない (`DevServer::stop` の修正、またはテスト後の確認の追加による)。
- `mise run check` を続けて 2 回実行しても、`backend:test-integration` が失敗しない。
- `mise run check` が通過する。

## 解決方法

- `backend/brew_book/tests/support/mod.rs` と `backend/brew_book_admin/tests/support/mod.rs` の `DevServer` を、`CommandExt::process_group(0)` で新しいプロセスグループとして起動するようにした。`stop` はプロセスグループへ SIGTERM を送り、`STOP_TIMEOUT` (5 秒) の間だけ子の終了を待ち、残っていれば SIGKILL でグループごと止める。`wrangler` (node) の子の `workerd` と esbuild も同じプロセスグループに属するため、孫が孤児として残らない。
- 停止の後にプロセスグループが空になったことを確認する `process_group_exists` を追加し、回帰テスト `wrangler_dev_server_stop_leaves_no_process_in_its_process_group` で検証する。
- 再現確認 (2026-10-01、macOS、M3 Pro、36 GB): 修正前は `cargo test -p brew_book --test dev_server` の実行後に `workerd` 2 個、esbuild 2 個、`wrangler dev` (node) 1 個が残った。修正後は同じテストの実行後に 0 個になった。
- 完了条件の確認:
  - `mise run backend:test-integration` を実行し、直後に `workerd`、`wrangler dev`、esbuild の残存が 0 であることを確認した (558.06 秒、2026-10-01)。
  - `mise run --jobs 1 check` を 2 回連続で実行し、どちらも失敗しなかった (1 回目 2735.79 秒、2 回目 2561.62 秒)。各実行の直後の残存プロセスは 0 だった。
  - `mise run check` が通過した。
