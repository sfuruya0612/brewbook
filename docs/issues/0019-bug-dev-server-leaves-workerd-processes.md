# 結合テストの DevServer が workerd のプロセスを残す

Created: 2026-09-23
Model: deepseek-v4p1-flash

## 症状

`wrangler dev` を起動する結合テスト (`mise run backend:test-integration`、`mise run check`) の実行後、`workerd serve --binary` のプロセスが残る。
2026-09-23 の観測では、1 回の `mise run check` の後に 63 個、過去の実行を含めると 477 個が残っていた。
残ったプロセスが多い状態では、`wrangler d1 migrations apply` が `✘ [ERROR] connect EADDRNOTAVAIL 127.0.0.1:<ポート> - Local (0.0.0.0:0)` で終了コード 1 になり、`backend:test-integration` の `d1_binding` が失敗する。
残っているプロセスを全て終了させると、同じテストは通る。

## 再現手順

1. リポジトリのルートで `mise run backend:test-integration` (または `mise run check`) を実行する。
2. `ps -eo pid,command | grep 'workerd serve --binary' | grep -v grep | wc -l` で残っている workerd の数を数える。起動したテストサーバーの数だけ残る。
3. 残った workerd が多数ある状態で `cd backend/coffee_log && wrangler d1 migrations apply DB --local --persist-to /tmp/probe` を実行すると、`fetch failed` (wrangler のログには `connect EADDRNOTAVAIL 127.0.0.1:<ポート>`) で終了コード 1 になる。残っているプロセスを全て終了させると、同じコマンドは成功する。
4. `mise run check` を繰り返すと、`backend:test-integration` の `d1_binding` (`wrangler_d1_binding_inserts_and_selects_with_placeholders`、`wrangler_d1_check_is_disabled_without_its_var`) が `wrangler dev must start: "wrangler d1 migrations apply failed with exit status: 1"` で失敗する。

## 原因

`backend/coffee_log/tests/support/mod.rs` の `DevServer::stop` は `child.kill()` で `wrangler` のプロセスだけを SIGKILL する。`wrangler` は SIGKILL では後始末ができず、その子の `workerd` が孤児として残る。
残った `workerd` がローカルのポートを使い続けるため、新しい `wrangler` の接続が `EADDRNOTAVAIL` になると考えられる。この因果は、477 個残った状態で失敗し、全て終了させた後に成功した観測に基づく仮説である。

## 完了条件

- `mise run backend:test-integration` の実行直後に `ps -eo pid,command | grep 'workerd serve --binary' | grep -v grep` が何も出さない (`DevServer::stop` の修正、またはテスト後の確認の追加による)。
- `mise run check` を続けて 2 回実行しても、`backend:test-integration` が失敗しない。
- `mise run check` が通過する。
