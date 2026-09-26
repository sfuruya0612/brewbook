# mise run check の並列実行で frontend:test-integration の Chrome セッション作成が失敗する

Created: 2026-09-26
Model: deepseek-v4p1-flash

## 症状

`mise run check` を既定の並列実行で実行すると、`frontend:test-integration` が次の出力で終了コード 1 になる。

```
[frontend:test-integration] Oops; flutter has exited unexpectedly: "SessionNotCreatedException (500): session not created: DevToolsActivePort file doesn't exist".
...
[frontend:test-integration] ERROR task failed
```

2026-09-26 に 2 回連続で発生した (issue 0022 の実装時)。
同じ日に issue 0021 で実行した既定の並列実行の `mise run check` は 1222 秒で通過しており、常に失敗するわけではない。
失敗した実行では、Flutter の Web ビルド 2 つ (`frontend:build`、`frontend:build-e2e`) と Rust のテストのコンパイルが同時に走っていた。
失敗した実行では、`flutter drive` の起動時の `flutter doctor` の応答が 90 秒から 115 秒に伸びていた (通過した実行では 4.3 秒)。

## 再現手順

1. リポジトリのルートで `mise run check` を実行する。
2. `frontend:test-integration` が `SessionNotCreatedException (500): session not created: DevToolsActivePort file doesn't exist` で失敗する (2026-09-26 に 2 回連続で発生。発生は確率的)。
3. 失敗したタスクを単独で実行すると通過する: `mise run frontend:test-integration` (exit 0)。
4. `mise run check` を直列実行すると通過する: `mise run --jobs 1 check` (exit 0、1249 秒)。

環境: macOS、Google Chrome 154.0.8037.58 (アプリケーションとしてインストール済み)、chromedriver 154.0.8037.57 (`mise.toml`)。Chrome は起動時に DevToolsActivePort を作れていない。

## 原因

特定できていない。候補は次の 2 つで、いずれも未確認である。

- 並列実行の資源競合: 同じ `mise run check` の中で Flutter のビルド 2 つと Rust のコンパイルが同時に走り、Chrome の起動が時間内に完了しなかった。
- Chrome と chromedriver の版のずれ: 手元の Chrome は 154.0.8037.58、chromedriver は 154.0.8037.57 である。CI は Chrome for Testing の同じ版を入れて揃えている (`.github/workflows/ci.yml`)。

## 完了条件

- `mise run check` を既定の並列実行で 3 回連続して実行し、`frontend:test-integration` が失敗しない。
- 原因を特定して修正した場合、特定した原因と修正の内容を「## 解決方法」に書く。
- `mise run check` が通過する。

## 補足

- 上の完了条件にある「## 解決方法」への言及は文中の参照であり、見出しではない。この issue は未実装で、close の成果物を持たない。
- 完了条件の 2 行目は条件付きで、そのままでは close の判定に使えない。実装の回で原因を特定して修正した場合の記録の指示であり、close の判定は 1 行目と 3 行目で行う。
