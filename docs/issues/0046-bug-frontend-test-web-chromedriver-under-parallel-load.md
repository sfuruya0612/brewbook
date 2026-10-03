# mise run test の並列実行で frontend:test-web の ChromeDriver が失敗する

Created: 2026-10-03
Model: DeepSeek V4.1 Flash

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

## 完了条件

- `mise run test` を既定の並列実行で 3 回連続して実行し、`frontend:test-web` が失敗しない。
- 原因を特定して修正した場合、特定した原因と修正の内容を issue に記録する。
- `mise run check` が通過する。
