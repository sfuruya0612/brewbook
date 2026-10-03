# 想定規模のアカウント削除テストが miniflare の接続断でまれに失敗する

Created: 2026-09-26
Model: deepseek-v4p1-flash
Completed: 2026-10-03

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

### 原因の追記 (2026-10-03)

失敗は、miniflare の ProxyWorker とユーザー Worker の間の接続が失われることで起きる。並列の負荷の下で起きやすいが、2026-09-26 の記録では単独実行でも起きている。アプリのコードは失敗していない。

- 失敗時の wrangler のログ (2026-09-26 の `~/Library/Preferences/.wrangler/logs/wrangler-2026-09-26_10-07-26_560.log`。同日の `wrangler-2026-09-26_10-03-27_690.log` と `wrangler-2026-09-26_09-58-17_525.log` も同じ) では、ユーザー Worker が同じリクエストを 200 で処理している (`{"event":"request","route":"r2_check","method":"POST","status":200,"duration_ms":5069}`)。その 48 ミリ秒後に ProxyWorker が `Error inside ProxyWorker (the affected request failed; the dev server continues): POST http://127.0.0.1:61692/api/__r2_check (failed after 1 attempt): Network connection lost.` を出している。リクエストはユーザー Worker に届き、応答の経路で接続が失われている。
- miniflare 5.20260918.0-alpha の loopback サーバーのソースのコメントは、接続の再利用と idle の接続の閉鎖の競合が Worker の中で "Network connection lost" として現れることを説明している。Cloudflare のドキュメント (Errors and exceptions) は "Network connection lost" に対して fetch の再試行を求めている。
- 2026-10-03 の再現の試行では、単独実行 3 回、負荷 (frontend:test-web と CPU の負荷) あり 5 回、テストバイナリ全体の実行 6 回のいずれでも失敗しなかった。発生の確率は下がっている。一方で、2026-10-02 から 10-03 の移行の `mise run check` でも同じテストの失敗 (タイムアウト) が記録されており、無くなってはいない。

## 修正方針

- `backend/brew_book/tests/wrangler_account_api.rs` の `put_objects` に、失敗した範囲の再試行を入れる (最大 3 回、1 秒の間隔)。R2 の put は同じ鍵への上書きなので、同じ範囲をやり直しても結果は変わらない。
  - 再試行の対象は、送信の失敗 (reqwest のエラー) と、200 以外の応答 (ProxyWorker が返す 500 を含む) とする。
    - レビューの指摘により、やり直すのは送信の失敗と 5xx の応答に限る。4xx は入力の誤りなのでやり直さず、すぐに失敗させる。
  - 想定規模の下ごしらえのクライアントのタイムアウトを 120 秒にする。既定の 30 秒では、並列の負荷の下で 1,000 件の put が 30 秒を超えることがある (2026-10-02 から 10-03 の移行の check の失敗はタイムアウトとして記録されている)。
- 再試行のために、`backend/brew_book/tests/support/http.rs` の `ApiClient` に `try_post_json` (`Result<Response, reqwest::Error>` を返す) を追加し、既存の `post_json` は `try_post_json` を呼んで `expect` する形にする (既存の呼び出しの挙動は変えない)。
- 削除のリクエスト (`DELETE /api/account`) には再試行を入れない。削除の成功の後にセッションが消えるため、再試行は 401 になり安全にやり直せない。失敗は下ごしらえの put でだけ観測している。
- 新しい API と権限は不要である。

検討して採らなかった案:

- `MAX_COUNT` (1,000) を小さくする: 1 リクエストの時間は短くなるが、リクエストの回数と接続を使う回数が増える。原因は接続の再利用の競合であり、件数を減らしても無くならない。
- wrangler の版を上げる: 接続の切断は miniflare の既知の問題 (GitHub の cloudflare/workers-sdk #15203 など) で、4.135.0 より新しい版で直っている保証が無い。ツールの版の変更は別の判断 (ADR-0009) が要る。
- mise の並列度を下げる: check と CI の実行の形を変えずに、テストの下ごしらえを堅牢にする。

## 完了条件

- `mise run backend:test-integration` を 5 回連続して実行し、`wrangler_account_api` が失敗しない。
- 原因を特定して修正した場合、特定した原因と修正の内容を issue に記録する。
- `mise run check` が通過する。

## 解決方法

- `backend/brew_book/tests/support/http.rs` の `ApiClient` に `try_post_json` と `try_send` を追加した。送信の失敗 (reqwest のエラー) を呼び出し側で扱えるようにするためである。既存の `post_json` と `send` はこれらを呼んで `expect` する形にし、既存の呼び出しの挙動は変えていない。
- `backend/brew_book/tests/wrangler_account_api.rs` の想定規模の下ごしらえを直した。
  - `put_objects` は `ApiClient::with_timeout` で待ち時間を 120 秒にした (`PUT_TIMEOUT`)。既定の 30 秒では、並列の負荷の下で 1,000 件の put が 30 秒を超えることがある。
  - 1 つの範囲の put は `put_range` が送り、一時的な失敗 (送信の失敗と 5xx の応答) は 1 秒の間隔で最大 3 回やり直す (`PUT_RETRIES`)。4xx は入力の誤りなのでやり直さず、すぐに失敗させる。R2 の put は同じ鍵への上書きなので、同じ範囲をやり直しても結果は変わらない。最後まで成功しなければ、最後の失敗の内容を添えて失敗する。
  - 再試行の経路は、応答を差し替えられる小さな HTTP サーバー (`spawn_fake_server`) を使うテスト 2 件で検査する。`put_range_retries_the_same_range_after_a_failure` は 5xx の後に同じ本文で再送することを、`put_range_fails_after_the_retry_limit` は 4 回で打ち切って最後の失敗を添えることを検査する。
- 原因: miniflare の ProxyWorker とユーザー Worker の間の接続が失われる。並列の負荷の下で起きやすいが、2026-09-26 の記録では単独実行でも起きている。失敗時の wrangler のログでは、ユーザー Worker が同じリクエストを 200 で処理した 48 ミリ秒後に ProxyWorker が "Network connection lost" を出している。アプリのコードは失敗していない。Cloudflare のドキュメントは "Network connection lost" に対して fetch の再試行を求めている。
- 再現確認: 修正前の 2026-10-03 の試行 (単独実行 3 回、負荷 (frontend:test-web と CPU の負荷) あり 5 回、テストバイナリ全体の実行 6 回) では失敗しなかった。発生の確率は下がっている。原因は 2026-09-26 の失敗時の wrangler のログから特定した。修正後は `mise run backend:test-integration` を 5 回連続で実行し、失敗しなかった。
- 残るリスク: 削除の応答 (`DELETE /api/account`) と `count_objects` の経路には再試行を入れていない。削除は成功の後にセッションが消えて 401 になるため、安全にやり直せない。同じ接続断が起きた場合は、これまでどおり失敗する。
- 完了条件の確認:
  - `mise run backend:test-integration` を 5 回連続で実行し、全て通過した (851.16、371.84、372.70、371.84、379.67 秒)。`wrangler_account_api` は 5 回とも失敗しなかった。
  - `mise run check` が通過した (`--jobs 1`、1840.83 秒、タスクの失敗 0)。既定の並列実行では `frontend:test-web` が ChromeDriver のセッション作成のタイムアウトで失敗したが、0030 の変更は frontend のタスクに触れておらず、別の issue (0046) として登録した。
