# mise run check の並列実行で frontend:test-same-origin の E2E が fonts.ready のタイムアウトで失敗する

Created: 2026-10-03
Model: DeepSeek V4.1 Flash

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

## 完了条件

- `mise run check` を既定の並列実行で 3 回連続して実行し、`frontend:test-same-origin` が失敗しない。
- 原因を特定して修正した場合、特定した原因と修正の内容を issue に記録する。
- `mise run check` が通過する。
