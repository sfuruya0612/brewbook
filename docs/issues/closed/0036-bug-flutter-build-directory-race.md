# mise run check の並列実行で Flutter の build ディレクトリが競合し frontend:test-web が失敗する

Created: 2026-10-02
Model: deepseek-v4p1-flash
Completed: 2026-10-02

## 症状

`mise run check` を既定の並列実行で実行すると、`frontend:test-web` が次の出力で終了コード 1 になる。

```
[frontend:test-web] Flutter failed to copy file from "/Users/.../frontend/build/native_assets/macos/native_assets.json" to "/Users/.../frontend/build/unit_test_assets/NativeAssetsManifest.json". The file or directory could not be found.
[frontend:test-web] PathNotFoundException: Cannot copy file to '/Users/.../frontend/build/unit_test_assets/NativeAssetsManifest.json', path = '/Users/.../frontend/build/native_assets/macos/native_assets.json' (OS Error: No such file or directory, errno = 2)
[frontend:test-web] This can sometimes happen if the file was deleted or moved while the tool was running. Try running "flutter clean" and try again.
```

2026-10-02 の `mise run check` (既定の並列度 8) で 1 回観測した。同じ実行では `frontend:analyze` の analysis server と `frontend:build` の wasm dry run も -15 (SIGTERM) で終了している。

## 再現手順

1. リポジトリのルートで `mise run check` を実行する (既定の並列度 8)。
2. `frontend:test-web` が `frontend/build/native_assets/macos/native_assets.json` のコピーで失敗する (2026-10-02 に 1 回発生。発生は確率的)。
3. `mise run --jobs 1 check` では発生しない (2026-10-01 から 10-02 の実行で 2 回連続して通過)。

環境: macOS、Flutter 3.47.5 (`mise.toml`)。`mise run check` は `frontend:build`、`frontend:build-e2e`、`frontend:analyze`、`frontend:test`、`frontend:test-web` を並列に実行する (mise の既定の `jobs` は 8)。

## 原因

特定できていない。分かっている範囲は次のとおりである。

- 失敗は `frontend/build/native_assets/macos/native_assets.json` が読めないことによる (`PathNotFoundException`)。並列に走る別の Flutter のタスクが `frontend/build` を書き換えている間に `frontend:test-web` が読んだと考えられる (未確認)。
- `frontend:build`、`frontend:build-e2e`、`frontend:test`、`frontend:test-web` は、いずれも `frontend/build` (`native_assets`、`unit_test_assets`、`web`、`e2e-web`) を使う。`--jobs 1` ではこれらが直列になり、発生していない。
- 同じ実行で `frontend:analyze` の analysis server と `frontend:build` の wasm dry run が -15 (SIGTERM) で終了している。失敗した `frontend:test-web` に伴う mise の兄弟タスクの停止か、Flutter 自身の子プロセスの停止かは未確認である。

## 完了条件

- `mise run check` を既定の並列実行で 3 回連続して実行し、`frontend:test-web` が失敗しない。
- 原因を特定して修正した場合、特定した原因と修正の内容を issue に記録する。
- `mise run check` が通過する。

## 補足

- 0020 の検証 (2026-10-02) で見つけた。0020 の変更は frontend のタスクに触れていない。
- 関連: 0029 (frontend:test-integration の Chrome セッション作成の失敗)。どちらも既定の並列実行でだけ起きる。

## 解決方法

Flutter のビルドとタスクを 0045 で削除したため、この競合は再現しなくなった (2026-10-02 の所有者の決定)。

- 競合していた Flutter のタスク (`frontend:build`、`frontend:build-e2e`、`frontend:analyze`、`frontend:test`、`frontend:test-web`、`frontend:test-integration`) と `[tools]` の `flutter` を 0045 が消した。`frontend/build/` を共有する Flutter の処理自体が無くなり、`frontend:test-web` は Dioxus の wasm-bindgen-test (ブラウザのテスト) になった。
- 完了条件の 1 行目 (既定の並列で 3 回連続) は、対象の Flutter のタスクが無くなったため実施しない。代わりに、Flutter の無い状態で `mise run check` を既定の並列で実行し、通過することを確認した (2026-10-02 と 2026-10-03。約 23 分と約 27 分。ログの要約は 0045 の解決方法に記録した)。1 回目の実行では別の結合テスト (`wrangler_account_api` の R2 のカーソルの検査) が並列の負荷でタイムアウトしたが、単体では通過し、2 回目と 3 回目の実行では全て通過した。
- 原因の特定と修正は行っていない (対象のタスクが無くなったため)。
