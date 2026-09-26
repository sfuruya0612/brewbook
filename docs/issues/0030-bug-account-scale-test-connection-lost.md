# 想定規模のアカウント削除テストが miniflare の接続断でまれに失敗する

Created: 2026-09-26
Model: deepseek-v4p1-flash

## 症状

`mise run backend:test-integration` (または `mise run check`) の `wrangler_account_api` のテスト `wrangler_account_delete_removes_the_assumed_scale_of_the_r2_objects_by_cursor` が、次の出力で終了コード 101 になる。

```
thread 'wrangler_account_delete_removes_the_assumed_scale_of_the_r2_objects_by_cursor' panicked at brew_book/tests/support/http.rs:127:37:
the response must be JSON but was Error: Network connection lost.
    at async Object.fetch (file:///Users/.../miniflare/dist/src/workers/core/entry.worker.js:5216:22): expected value at line 1 column 1
```

2026-09-26 に観測した発生率は、単独実行 5 回で 2 回成功、3 回失敗である (失敗時の所要は 18 秒から 47 秒、成功時は 73 秒から 81 秒)。
失敗はテストの準備 (`put_objects` の `users/` 3,000 件の先頭の `count=1000` の呼び出し) で起き、削除の処理には達していない。
同じ日に、アプリ名の変更 (0023) の前のコミット (`8926491`) の worktree でも同じ失敗を確認しており、0023 の変更とは無関係である。

## 再現手順

1. リポジトリのルートで `mise run backend:test-integration` (または `mise run check`) を実行する。
2. `wrangler_account_api` の `wrangler_account_delete_removes_the_assumed_scale_of_the_r2_objects_by_cursor` が `Network connection lost.` で失敗することがある (2026-09-26 の観測で 5 回中 3 回)。
3. 単独実行でも再現する: `cd backend && cargo test -p brew_book --test wrangler_account_api wrangler_account_delete_removes_the_assumed_scale_of_the_r2_objects_by_cursor -- --exact`。
4. 繰り返すと成功する (2026-09-26 のリトライで 1 回目成功、2 回目失敗、3 回目成功)。

環境: macOS、wrangler 4.135.0、miniflare 5.20260918.0-alpha (wrangler に同梱)、ローカルの `wrangler dev` と R2 のバインディング。

## 原因

特定できていない。分かっている範囲は次のとおりである。

- 失敗はテスト専用の経路 (`/api/__r2_check`) の `put` で、1 リクエストで 1,000 件の R2 オブジェクトを置く処理の途中で起きる (`backend/brew_book/src/r2_check.rs`)。miniflare のプロキシがユーザー Worker への fetch で接続断を返している (miniflare の `entry.worker.js` の 500 応答)。
- workerd のクラッシュレポートは残っていない。失敗した実行の dev サーバーのログはテストの出力に現れない。
- 成功と失敗の分かれ目は未確認である (成功時は合計 6,000 件の用意と削除に 73 秒から 81 秒、失敗時は 18 秒から 47 秒で接続断)。

## 完了条件

- `mise run backend:test-integration` を 5 回連続して実行し、`wrangler_account_api` が失敗しない。
- 原因を特定して修正した場合、特定した原因と修正の内容を issue に記録する。
- `mise run check` が通過する。
