# CI の frontend:test-integration が ChromeDriver と Chrome の版の不一致で失敗する

Created: 2026-09-29
Model: deepseek-v4p1-flash
Completed: 2026-09-29

## 症状

2026-09-29 の main (00698b1) の CI (run 36560608583) で、`frontend:test-integration` が次の出力で失敗した (ジョブは 9 分 54 秒で終了)。

```
[frontend:test-integration] SessionNotCreatedException (500): session not created: This version of ChromeDriver only supports Chrome version 154
[frontend:test-integration] Current browser version is 153.0.8010.52 with binary path /opt/google/chrome/chrome
```

`frontend:test-same-origin` (0017 の e2e) も同じ chromedriver を使うため、同じ失敗になる。

## 再現手順

1. main に push する。
2. `mise run check` の `frontend:test-integration` が `session not created` で失敗する。

環境: GitHub Actions の `ubuntu-latest`、chromedriver 154.0.8037.57 (`mise.toml`)、Chrome for Testing 154.0.8037.57 (ワークフロー)。runner image には Chrome 153.0.8010.52 が既定の場所 `/opt/google/chrome/chrome` に入っている。

## 原因

ワークフローは Chrome for Testing を `/usr/local/bin/google-chrome` にシンボリックリンクし、`CHROME_EXECUTABLE` で Flutter に指していた。
しかし chromedriver (`flutter drive` と `frontend:test-same-origin` が起動する) は PATH や `CHROME_EXECUTABLE` を見ず、既定の場所の Chrome (`/opt/google/chrome/chrome`) を起動する。
runner image の Chrome は 153 のため、chromedriver 154 が要求する版 (154) と一致せず、セッションを作れない。

## 完了条件

- CI の `frontend:test-integration` が `session not created` で失敗しない。
- chromedriver とブラウザの版が CI で一致する。
- `mise run check` が通過する。

## 解決方法

- ワークフローの Chrome for Testing の導入で、`/opt/google/chrome/chrome` を Chrome for Testing の実体へのシンボリックリンクに置き換えた。chromedriver の既定の探索先が Chrome for Testing になり、runner image に入っている Chrome の版に依存しなくなる。`CHROME_EXECUTABLE` と `/usr/local/bin/google-chrome` は Flutter の Chrome を使う処理 (`flutter test --platform chrome` など) のためにそのまま残す。
- 完了条件の確認: PR #1 の CI (run 36565577421、2026-09-29) で check が通過した (23 分 16 秒)。`frontend:test-integration` (326 秒) と `frontend:test-same-origin` (283 秒) を含み、`session not created` は再発していない。`mise run check` は手元でも通過した (issue 0032 の解決方法を参照)。

## 補足

- 版を上げるときは `mise.toml` の chromedriver とワークフローの Chrome for Testing の 2 か所を揃える (issue 0013)。
