# GitHub Actions の check が 1 時間のタイムアウトになる

Created: 2026-09-29
Model: deepseek-v4p1-flash
Completed: 2026-09-29

## 症状

2026-09-27 の main (3104ca8) の CI (run 36317505696) で、check が 1h0m24s で次によりキャンセルされた。

```
The job has exceeded the maximum execution time of 1h0m0s
```

ログ (`gh run view --job=108614895357 --log`) では、`mise run check` の開始直後に `lint` が失敗し、その後に `frontend:analyze` の `Terminated` と 3 つの `Waiting for another flutter command to release the startup lock...` を最後に、キャンセル (13:00) まで何も出力されなくなった。

```
[lint] error: 'cargo-clippy' is not installed for the toolchain '1.98.1-x86_64-unknown-linux-gnu'.
[lint] help: run `rustup component add --toolchain 1.98.1-x86_64-unknown-linux-gnu clippy` to install it
[frontend:analyze] Terminated
[frontend:test-web] Waiting for another flutter command to release the startup lock...
```

キャンセル時の後始末でも mise のプロセスが残っており (`Terminate orphan process: pid (6683) (mise)`)、`mise run check` は終了していなかった。

## 再現手順

1. main に push する。
2. check の `mise run check` が、`lint` の clippy のエラーで失敗した後も終了せず、ジョブのタイムアウト (60 分) まで戻らない。

環境: GitHub Actions の `ubuntu-latest`、mise 2026.9.15 (`jdx/mise-action` が入れる版)、Flutter 3.47.5、Rust 1.98.1 (`mise.toml`)。

## 原因

2 つある。

1. runner の rustup のプロファイルにより、mise が入れる 1.98.1 に clippy と rustfmt が入らない。
   GitHub Actions の runner image は rustup を minimal プロファイルで入れ、clippy と rustfmt は stable ツールチェーンにだけ追加する (`images/ubuntu/scripts/build/install-rust.sh`)。
   mise は `profile` の指定が無いと runner の rustup のプロファイルを引き継ぐため (mise の Rust のツールオプション)、mise が入れる 1.98.1 には clippy が入らず、`lint` が数ミリ秒で失敗する。
   ログの `rust@1.98.1 ... downloading 4 components` は minimal 相当である。
2. mise はタスクが失敗すると、並列の兄弟タスクを SIGTERM して速やかに終了する。
   ただしプロセスグループでの後始末は、mise 自身がセッションリーダーのときは行わない (mise の実装の `should_use_pgroup`)。
   GitHub Actions の runner 配下ではこの条件に当たり、SIGTERM が Flutter の子プロセスまで届かない。
   flutter のコマンドは初回の起動時に起動ランチャー (`bin/internal/shared.sh`) が startup lock を取り、Dart SDK の取得とツールのビルドを行うため、ロックを持ったままのプロセスとロック待ちのプロセスが残る。
   mise は残ったプロセスの出力 (パイプ) が閉じるまで待つため、ロックの待ちが解けるまで、または残ったプロセスが終わるまで終了しない。
   2026-09-29 に Docker (ubuntu 24.04、mise 2026.9.15) で、`setsid` で mise をセッションリーダーにしたときに同じ挙動 (兄弟の失敗後も Flutter の起動ランチャー相当のロックの保持と待ちが残り、mise は残りが終わるまで終了しない) を再現した。

## 完了条件

- `mise run check` が通過する。
- CI の check で、lint が clippy の不在で失敗しない。
- タスクが失敗しても、CI の check が 1 時間のタイムアウトまで戻らない (ハングの歯止めはジョブの `timeout-minutes` のままとする)。

## 解決方法

- `mise.toml` の Rust に `profile = "minimal"` と `components = ["clippy", "rustfmt"]` を明示した (mise の Rust のツールオプション)。runner の rustup のプロファイルに依存せず、手元と CI のどちらでも clippy と rustfmt が入る。ADR-0009 の「ターゲットは `mise.toml` の Rust の設定で入れる」と同じ考え方で、ツールチェーンの構成を `mise.toml` に集約する。
- `mise.toml` の Flutter を使うタスク (`frontend:build`、`frontend:build-e2e`、`frontend:analyze`、`frontend:test`、`frontend:test-web`、`frontend:test-integration`) に `depends = ["frontend:setup"]` を追加し、初回の Flutter のツールのビルド (startup lock を取って行う処理) を並列の前に直列で済ませるようにした。失敗で兄弟タスクが止まっても、startup lock の待ちが残りにくくする。
- 再現確認 (2026-09-29、macOS、Docker の ubuntu 24.04): 修正前は mise 2026.9.15 を `setsid` で起動すると、兄弟タスクの失敗後もロック保持と待ちが残り、mise が終了しなかった。`mise.toml` の修正後は、手元の `mise run check` が通過した。
- 完了条件の確認:
  - `mise run check` が終了コード 0 で通過した (1881 秒、2026-09-29)。`mise run -n check` で、Flutter を使うタスクが `frontend:setup` の後に並ぶことを確認した。
  - CI (2026-09-29 の run 36560608583): lint が clippy の不在で失敗しないことと、`frontend:test-integration` の失敗でタスクが止まっても 1 時間のタイムアウトまで戻らないこと (ジョブは 9 分 54 秒で終了) を確認した。この run 自体は別のバグ (0033、chromedriver と Chrome の版の不一致) で失敗した。
  - 修正後の CI は run 36565577421 (2026-09-29、PR #1) で check 全体が通過した (23 分 16 秒、`backend:test-integration` と 0017 の e2e を含む)。

## 補足

- 原因 2 は mise 側の後始末の限界であり、リポジトリ側では直せない。ジョブの `timeout-minutes: 60` を最終的な歯止めとして残す。
- 完了条件の 3 行目は、`mise run check` の失敗時にハングしないことを指す。検査の内容によっては失敗そのものは起きるため、1 時間戻らないことを条件とする。
