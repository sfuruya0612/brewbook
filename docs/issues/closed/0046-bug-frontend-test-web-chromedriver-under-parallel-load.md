# mise run test の並列実行で frontend:test-web の ChromeDriver が失敗する

Created: 2026-10-03
Model: DeepSeek V4.1 Flash
Completed: 2026-10-07

## 症状

`mise run test` を既定の並列実行で実行すると、`frontend:test-web` が次のいずれかで終了コード 1 になる。

1 回目の実行 (2026-10-03):

```
[frontend:test-web] driver status: signal: 9 (SIGKILL)
[frontend:test-web] Error: driver failed to bind port during startup
[frontend:test-web] error: test failed, to rerun pass `-p brew_book_frontend --test test_photo_web`
```

2 回目の実行 (2026-10-03):

```
[frontend:test-web] ChromeDriver was started successfully on port 63520.
[frontend:test-web] [1790998089.559][SEVERE]: Timed out receiving message from renderer: 299.967
[frontend:test-web] Error: webdriver POST /session/2d293d88a13f463bc69471676629b69c/url timed out after 300.0s
[frontend:test-web] error: test failed, to rerun pass `-p brew_book_frontend --test test_stats_web`
```

どちらも 2 回連続で発生した (2026-10-03 の `mise run test` の 1 回目と 2 回目)。同じ実行では `backend:test-integration` の `wrangler_account_api` の想定規模テストには到達していない (`frontend:test-web` の失敗で mise が兄弟タスクを止めるため)。

## 再現手順

1. リポジトリのルートで `mise run test` を実行する (既定の並列度 8)。
2. `frontend:test-web` が ChromeDriver の起動の失敗 (ポートの bind) または webdriver の応答のタイムアウト (300 秒) で失敗する (2026-10-03 に 2 回連続で発生。発生は確率的)。
3. 失敗したタスクを単独で実行すると通過する: `mise run frontend:test-web`。

環境: macOS、chromedriver 154.0.8037.57 (`mise.toml`)、Chrome は手元の 154.0.8037.59。`mise run test` は `backend:test`、`backend:test-integration`、`frontend:test`、`frontend:test-web`、`frontend:test-same-origin` を並列に実行する (mise の既定の `jobs` は 8)。

## 原因

特定できていない。分かっている範囲は次のとおりである。

- 1 回目の失敗は ChromeDriver のプロセスが SIGKILL され、ポートの bind に失敗している。2 回目は ChromeDriver は起動したが、Chrome の renderer が 300 秒応答せず webdriver のコマンドがタイムアウトしている。
- どちらも `mise run test` の既定の並列実行でだけ起き、`frontend:test-web` の単独実行では起きていない。
- 閉じた 0029 (Flutter の統合テストの Chrome セッション作成の失敗) と同じ、並列実行の資源競合の疑いがある (未確認)。0029 は Flutter のタスクを消したため close したが、Chrome を使うテストは Dioxus の wasm-bindgen-test (`frontend:test-web`) として残っている。
- 同じ実行で `frontend:test-same-origin` も ChromeDriver と Chrome を使う。Chrome のプロセスと ChromeDriver のポートの競合は未確認である。

2026-10-06 に `mise.local.toml` を外した既定の並列度 (jobs = 8) で `mise run test` を実行し、1 回目で再現した。失敗したテストは `test_records_lists_web` (frontend:test-web の Chrome でテストを実行するバイナリの 4 番目。前の 3 つは通過した) で、署名は `driver status: signal: 9 (SIGKILL)` と `Error: driver failed to bind port during startup` である (1 回目の失敗と同じ)。

特定した原因:

- wasm-bindgen-test-runner 0.2.129 は、chromedriver の起動 (ポートの bind) を待つ上限を既定で 5 秒とする (`crates/cli/src/wasm_bindgen_test_runner.rs` の `driver: timeout_from_env("WASM_BINDGEN_TEST_DRIVER_TIMEOUT", 5)`)。期限までに bind しないと、ランナーが chromedriver を SIGKILL して `driver failed to bind port during startup` で失敗する (`crates/cli/src/wasm_bindgen_test_runner/headless.rs` の `BackgroundChild::drop` が `self.child.kill()` の後に `driver status: ...` を出す。ログの SIGKILL はランナー自身の後始末である)。
- ログに `Failed to start driver, trying again ...` と `Driver has not bound port after 10s, restarting ...` が無いことは、chromedriver が終了したのではなく、生存のまま 5 秒の期限に達したことを示す。起動の再試行は 10 秒ごとのため、上限の既定 5 秒では再試行も働かない。
- 単独の計測では chromedriver の bind は 0.04 秒から 0.12 秒である。並列実行では、この 5 秒を超える起動の遅れが起きた (5 秒は単独の計測の 40 倍以上に当たる)。

2 回目の失敗 (renderer の 300 秒のタイムアウト) は 2026-10-06 の再現では発生していない。仮説: マシンの資源の飽和で Chrome の renderer が応答しなくなった (未確認)。ページロードの待ちの上限は、ランナーが `max(300 秒, WASM_BINDGEN_TEST_TIMEOUT)` とする。

2026-10-06 の 3 連続実行の検証の 2 回目に、driver の起動の上限の修正で driver は起動するようになった後、別の上限に当たった。`test_passkey_web` で `Error: webdriver POST /session timed out after 60.0s` が出た (同じログの driver の stdout に `ChromeDriver was started successfully on port 61635.` があり、driver の起動は成功している)。

- ランナーは Chrome のセッションの作成 (Chrome の起動) を待つ上限を `max(60 秒, WASM_BINDGEN_TEST_TIMEOUT)` とする (`crates/cli/src/wasm_bindgen_test_runner.rs` の `startup: Duration::from_secs(60).max(test)`)。既定は 60 秒で、環境変数で直接は変えられない。
- 単独の計測ではセッションの作成は 0.57 秒から 0.96 秒である。失敗の時点のマシンは空きメモリが約 100 MB (16 KB のページで 6,637) で、DataGrip (926 MB)、Slack (557 MB)、mds_stores (2.5 GB)、利用者の Chrome (49 プロセス) が稼働していた。メモリの圧迫の下で Chrome の起動が 60 秒を超えた。

## 修正方針

`mise.toml` の `frontend:test-web` のタスクで `WASM_BINDGEN_TEST_DRIVER_TIMEOUT=60` を明示する。ランナーが chromedriver の起動を待つ上限が 60 秒になり、10 秒ごとの再起動 (最大 6 回) が働くようになる。60 秒はリポジトリの他の待ちの上限 (`STEP_TIMEOUT`、`SCRIPT_TIMEOUT`) と揃える。

並列実行での起動の遅れの上限は未計測である (5 秒を超えたことは確認している)。ランナーは bind しないまま 10 秒たつと chromedriver を起動し直すため、1 つの driver プロセスに bind まで与えられる時間は 10 秒で、`WASM_BINDGEN_TEST_DRIVER_TIMEOUT` の値は試行の合計の上限に当たる。

2026-10-06 の検証の 2 回目で、driver の起動の後に Chrome のセッションの作成の上限 (60 秒) でも失敗したため、`WASM_BINDGEN_TEST_TIMEOUT=300` も明示する。セッションの作成の上限が 300 秒になり、ページロードの上限 (300 秒) と揃う。テスト自体の待ちも 300 秒になるが、正常時は出力の `test result: ` を検知した時点で終わるため影響しない (ハングしたテストの検知が遅くなるだけである)。

- 検討して採らなかった案: Chrome を使う 3 タスク (`backend:test-integration`、`frontend:test-web`、`frontend:test-same-origin`) を mise の `wait_for` で直列化する。資源の競合は減るが、失敗の直接の原因 (ランナーの 5 秒の上限) を直さず、`mise run check` の所要時間も延びる。renderer のタイムアウトが再現した場合は改めて検討する。
- 検討して採らなかった案: ページロードの上限 (300 秒) を延ばすために `WASM_BINDGEN_TEST_TIMEOUT` を 300 秒より大きくする。2 回目の失敗 (renderer のタイムアウト) は再現しておらず、テスト自体の待ちがさらに延びるため採らない。
- 新しい API と権限は要らない。

## 完了条件

- `mise run test` を既定の並列実行で 3 回連続して実行し、`frontend:test-web` が失敗しない。
- 原因を特定して修正した場合、特定した原因と修正の内容を issue に記録する。
- `mise run check` が通過する。

## 解決方法

`mise.toml` の `frontend:test-web` のタスクで `WASM_BINDGEN_TEST_DRIVER_TIMEOUT=60` と `WASM_BINDGEN_TEST_TIMEOUT=300` を明示し、wasm-bindgen-test-runner が chromedriver と Chrome の起動を待つ上限を広げた。

- 変更: `mise.toml` の `[tasks."frontend:test-web"]` の run のスクリプトに `export WASM_BINDGEN_TEST_DRIVER_TIMEOUT=60` と `export WASM_BINDGEN_TEST_TIMEOUT=300` を理由のコメントとともに追加した。実装の変更はこのファイルだけである。
- 原因の記録: (1) driver の起動を待つ上限 (既定 5 秒) の期限切れで、ランナーが chromedriver を SIGKILL して `driver failed to bind port during startup` で失敗すること。(2) driver の起動後に、Chrome のセッションの作成を待つ上限 (60 秒) の期限切れで、`webdriver POST /session timed out` で失敗すること。どちらも「## 原因」と「## 修正方針」に記録した。
- 再現確認: 修正前に 2026-10-06 の既定の並列度 (jobs = 8) の `mise run test` で再現した (`test_records_lists_web` が `driver failed to bind port during startup` で失敗)。修正後は、2026-10-07 の既定の並列度 (jobs = 8) の `mise run test` で `frontend:test-web` が通過し (114.15 秒、Chrome でテストを実行するバイナリすべて)、2026-10-07 の `mise run check` (jobs = 2) でも `frontend:test-web` は失敗しなかった。
- 完了条件の検証:
  - 所有者の指示 (2026-10-07) により、「`mise run test` を既定の並列実行で 3 回連続して実行し、`frontend:test-web` が失敗しない」の行は、`mise.local.toml` の設定 (jobs = 2、CARGO_BUILD_JOBS = 4) のまま `mise run check` を 1 回実行して通過したことで確認した (2026-10-07 06:59:03 から 08:21:51。`frontend:test-web` は 37.30 秒で通過し、ほかのタスクも失敗しなかった)。この実行は jobs = 2 のため、この issue の失敗条件である並列負荷を再現しない。既定の並列度 (jobs = 8) での 3 回連続の検証は、マシンの負荷 (Sophos のスキャンが 18 時間 100% CPU で稼働し、ほかの開発サーバーとデスクトップのアプリも動いていた) で 1 回の実行に 1 時間から 2 時間かかり、実行がセッションの再起動に巻き込まれて中断したため、行っていない (0053 と同じ読み替え)。既定の並列度の CI (jobs = 8) での通過は push 後に確認する (未検証)。
  - 原因と修正の内容は「## 原因」と「## 修正方針」に記録した。
  - `mise run check` が通過した (2026-10-07)。検査のログには、wasm のバイナリごとに `Set WASM_BINDGEN_TEST_TIMEOUT to 300 seconds...` と `Set WASM_BINDGEN_TEST_DRIVER_TIMEOUT to 60 seconds...` が出ており、環境変数がランナーに届いている。失敗の署名 (`driver failed to bind port during startup`、`webdriver ... timed out`、`test result: FAILED`) はログに無い。
- 残る失敗の経路: 2 回目の失敗 (renderer の 300 秒のタイムアウト) は未修正で、再発は未確認である。ページロードの上限は `max(300 秒, WASM_BINDGEN_TEST_TIMEOUT)` のままである。再発した場合は、`WASM_BINDGEN_TEST_TIMEOUT` を 300 秒より大きくしてページロードの上限を延ばすことを検討する (テスト自体の待ちも延びる)。
