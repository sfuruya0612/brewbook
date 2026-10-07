# mise run check の並列実行で frontend:test-same-origin の E2E が fonts.ready のタイムアウトで失敗する

Created: 2026-10-03
Model: DeepSeek V4.1 Flash
Completed: 2026-10-07

## 症状

`mise run check` を既定の並列実行で実行すると、`frontend:test-same-origin` の `wrangler_web_routes_registration_login_and_brew_save_ok` が次の出力で終了コード 1 になる。

```
[frontend:test-same-origin] fonts.ready must resolve: The Javascript code did not complete within the script timeout (see WebDriver::set_script_timeout())
[frontend:test-same-origin] test wrangler_web_routes_registration_login_and_brew_save_ok ... FAILED
[frontend:test-same-origin] test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 76.70s
```

2026-10-03 の `mise run check` (既定の並列度 8) で 1 回観測した (issue 0030 の実装の検証中)。この失敗で `backend:test-integration` が中断し、check は通過しなかった。

## 再現手順

1. リポジトリのルートで `mise run check` を実行する (既定の並列度 8)。
2. `frontend:test-same-origin` の `wrangler_web_routes_registration_login_and_brew_save_ok` が `fonts.ready must resolve: ... script timeout` で失敗することがある (2026-10-03 に 1 回発生。発生は確率的)。
3. 失敗したタスクを単独で実行すると通過する: `mise run frontend:test-same-origin`。

環境: macOS、chromedriver 154.0.8037.57 (`mise.toml`)、Chrome は手元の 154.0.8037.59。`mise run check` は `frontend:test-same-origin`、`frontend:test-web`、`backend:test-integration` などを並列に実行する (mise の既定の `jobs` は 8)。

## 原因

特定できていない。分かっている範囲は次のとおりである。

- 失敗は `backend/brew_book/tests/support/e2e.rs` の `wait_fonts` で起きる。`document.fonts.ready` の解決を `execute_async` (WebDriver の script timeout) で待っており、並列の負荷の下でフォントの読み込みが script timeout を超えている。ハーネスは `set_script_timeout` を呼んでおらず、値は chromedriver の既定である。
- `wait_theme` のような `STEP_TIMEOUT` (60 秒) の期限付きのポーリングではなく、1 回の `execute_async` で待つ形になっている。この形が負荷に弱いという仮説は未確認である。
- 関連: 0046 (`frontend:test-web` の ChromeDriver の失敗)、0030 (miniflare の接続断)。いずれも既定の並列実行でだけ起きる。

2026-10-07 に、2026-10-03 の失敗の記録とコードから原因を特定した。この失敗は確率的で、2026-10-07 06:59:03 から 08:21:51 の 0046 の close の検証の `mise run check` では `frontend:test-same-origin` は通過しており、再現していない。既定の並列度での再現には実行時間とマシンの負荷がかかるため、再現は試みず、失敗の署名とコードの経路から原因を特定して修正する判断をした。

特定した原因:

- `backend/brew_book/tests/support/e2e.rs` の `E2eBrowser::open` は WebDriver のセッションを作るが、修正前は `set_script_timeout` を呼んでいなかった。このため `execute_async` の script timeout は chromedriver の既定 (30 秒) のままであった。
- `wait_fonts` は `document.fonts.ready` を 1 回の `execute_async` で待つ (`wait_theme` のような `STEP_TIMEOUT` の期限付きのポーリングではない)。並列実行の負荷でフォントの読み込みが 30 秒を超えると、この呼び出しが script timeout で失敗し、2026-10-03 の `fonts.ready must resolve: ... script timeout` の署名になる。
- 同じ `execute_async` の待ちは `compare_images` (画像の比較) にもある。
- 0053 の close の記録: 「`E2eBrowser::open` は script timeout を設定していない (0052 の範囲)。この issue では `TestBrowser` に 60 秒を明示し、E2E 側の扱いは 0052 に委ねる。」
- `wait_fonts` が待つフォントは `frontend/index.html` の 23 行目から 28 行目で Google Fonts (`fonts.googleapis.com`、`fonts.gstatic.com`) から読む。`document.fonts.ready` はこの外部オリジンの取得が終わるまで解決しないため、読み込みの停滞は並列実行の負荷でも外部のネットワークでも起こり得る。60 秒という値はリポジトリの他の上限 (`STEP_TIMEOUT`、`SCRIPT_TIMEOUT`) と揃えたもので、停滞が続く場合に足りる保証はない。

## 修正方針

`backend/brew_book/tests/support/e2e.rs` の `E2eBrowser::open` で `driver.set_script_timeout(SCRIPT_TIMEOUT)` (60 秒、`backend/brew_book/tests/support/browser.rs` の共有の定数) を明示する。`wait_fonts` と `compare_images` の `execute_async` の上限が 60 秒になり、`TestBrowser` (0053 で 60 秒を明示済み) と揃う。

- 検討して採らなかった案: `wait_fonts` だけを `STEP_TIMEOUT` の期限付きのポーリングに変える。待ちの形は変わるが、`compare_images` の `execute_async` は直らず、上限が chromedriver の既定 (30 秒) のまま残る。
- 検討して採らなかった案: `STEP_TIMEOUT` を大きくする。画面の操作全般の待ちの上限が広がり、影響範囲が大きい。
- 60 秒で足りる保証はない (外部のフォントの読み込みの停滞)。再発した場合は、さらに広げるか、`document.fonts.status` の期限付きのポーリングにするか、フォントをリポジトリに含めるかを検討する。
- 新しい API と権限は要らない。

## 完了条件

- `mise run check` を既定の並列実行で 3 回連続して実行し、`frontend:test-same-origin` が失敗しない。
- 原因を特定して修正した場合、特定した原因と修正の内容を issue に記録する。
- `mise run check` が通過する。

## 解決方法

`backend/brew_book/tests/support/e2e.rs` の `E2eBrowser::open` で `driver.set_script_timeout(SCRIPT_TIMEOUT)` を明示的に呼び出し、`execute_async` の script timeout を chromedriver の既定値 (30 秒) から 60 秒に広げた。

- 変更: `E2eBrowser::open` のセッションの作成と窓の大きさの設定の後に `set_script_timeout(super::browser::SCRIPT_TIMEOUT)` (60 秒。`backend/brew_book/tests/support/browser.rs` の共有定数) を追加した。あわせて `get_timeouts()` でセッションの script timeout を読み戻し、60 秒であることを確かめる (設定が実行で検証でき、呼び出しが消えたときに気付ける。レビューの指摘)。`wait_fonts` と `compare_images` で `execute_async` の上限が 60 秒になり、`TestBrowser` (0053 で 60 秒を明示済み) と揃う。実装の変更は `backend/brew_book/tests/support/e2e.rs` だけである。
- 原因の記録: `E2eBrowser::open` が `set_script_timeout` を呼んでおらず、`wait_fonts` の `document.fonts.ready` を待つ 1 回の `execute_async` が chromedriver の script timeout の既定値 (30 秒) に達して失敗することを「## 原因」と「## 修正方針」に記録した。フォントは `frontend/index.html` で Google Fonts から読むため、読み込みの停滞は並列実行の負荷でも外部のネットワークでも起こり得ることも記録した。
- 再現確認: この失敗は 2026-10-03 に 1 回発生した確率的なもので、2026-10-07 06:59:03 から 08:21:51 の 0046 の close の検証の `mise run check` では `frontend:test-same-origin` は通過しており、再現していない。既定の並列度での再現には実行時間とマシンの負荷がかかるため、再現は試みず、失敗の署名とコードの経路から原因を特定して修正する判断をした。
- 完了条件の検証:
  - 所有者の指示 (2026-10-07) により、`mise run check` の再実行はしていない。代わりに `mise run frontend:test-same-origin` を 1 回実行し、通過した (2026-10-07 11:48:41 から 12:05:52。テスト `wrangler_web_routes_registration_login_and_brew_save_ok` は 23.23 秒で通過し、タスク全体は 1017.26 秒)。この実行は `mise.local.toml` の設定 (jobs = 2、CARGO_BUILD_JOBS = 4) で行い、この issue の失敗条件である並列負荷は再現しない。`mise run check` を 1 回実行したが (同日 08:48:15 から 10:47:14)、`frontend:test-same-origin` がビルドディレクトリのロックを待っている間に環境 (セッションの再起動) からの SIGTERM で停止したため、check は通過していない。既定の並列度の CI (jobs = 8) での通過は push 後に確認する (未検証)。
  - 原因と修正の内容は「## 原因」と「## 修正方針」に記録した。
  - `mise run check` の通過の行は、実行した 1 回が中断したため確認していない (所有者の指示により再実行はしていない)。実行できた部分 (fmt、formal、lint、backend:build、frontend:setup、frontend:test-web、frontend:build、frontend:lint、frontend:test、frontend:build-e2e) は通過し、失敗の署名は無い。
- 残る失敗の経路: フォントは Google Fonts から読むため、外部のネットワークの停滞が続けば 60 秒でも足りない可能性がある (未確認)。再発した場合は、さらに広げるか、`document.fonts.status` の期限付きのポーリングにするか、フォントをリポジトリに含めるかを検討する。
