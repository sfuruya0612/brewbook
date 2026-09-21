# ADR-0009: ツールチェーンのバージョン管理とコマンドラインを mise で管理する

Created: 2026-09-21
Model: Claude Fable 5.1
Status: Accepted

## 背景

Backend は Rust (ADR-0001)、Frontend は Flutter (ADR-0007) で書き、デプロイとローカル開発には wrangler を使う (ADR-0002、ADR-0005)。
`shiguredo_s3` の MSRV は 1.93 で (ADR-0003)、所有者の手元に元から入っていた Rust 1.76 では満たせなかった。
2026-09-21 に所有者が mise で Rust の最新版 (rustc 1.98.1) を入れ、リポジトリのルートに `mise.toml` を置いた。
同日、所有者は次を決めた。

- ツールのバージョン管理と、ビルド、デプロイ、テストなどのコマンドラインは、今後 mise のタスクで管理する。
- Flutter と Rust のバージョンは `latest` ではなく版を固定する。
- wrangler も mise で管理する (同日に所有者が追加で決定)。

同日に所有者が `mise.toml` を書き換え、Flutter を 3.47.5、Rust を 1.98.1、wrangler を 4.135.0 に固定した (「## 結果」)。

## 決定

- ツールのバージョンはリポジトリのルートの `mise.toml` の `[tools]` で管理する。
  Flutter、Rust、wrangler (npm パッケージ) は具体的な版で固定し、`latest` を使わない。
  2026-09-21 時点の版は Flutter 3.47.5、Rust 1.98.1、wrangler 4.135.0 とする。
  Rust の版は `shiguredo_s3` の MSRV (1.93) 以上とする。
- Rust の wasm32-unknown-unknown ターゲットは、`mise.toml` の Rust の設定 (`targets`) で入れる。
  mise は Rust を rustup 経由で入れ、`targets` にターゲットの配列を書けることを 2026-09-21 に mise のドキュメント (https://mise.jdx.dev/lang/rust.html) で確認した。
  手元で `rustup target add` を実行する手順は残さない。
- ビルド、デプロイ、テスト、ローカル開発の起動、D1 のマイグレーション、デプロイ後の確認 (ADR-0008) のコマンドラインは、`mise.toml` の `[tasks]` に定義する。
  リポジトリに別のシェルスクリプトや Makefile を置かず、README とドキュメントはタスク名で手順を示す。
- CI は同じ `mise.toml` で mise を使ってツールを入れ、同じタスクを実行する。
  CI と手元でコマンドラインを二重に持たない。
- タスクの内容は Flutter と Rust の標準のコマンド (`flutter build web`、`cargo test`、`wrangler deploy` など) の組み合わせに留め、タスクの中に独自のロジックを書かない。
  ロジックが必要になる場合は Rust か Dart のコードにして、タスクからはそれを呼ぶだけにする。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| rustup の `rust-toolchain.toml` と Flutter の FVM で別々に版を固定する | ツールごとに管理の方法が分かれる。所有者が mise で統一することを決めた |
| Makefile やシェルスクリプトでコマンドラインを持つ | 所有者が mise のタスクで管理することを決めた。mise のタスクなら版の管理と同じファイルに置ける |
| 版を `latest` のままにする | 手元と CI で版がずれ、再現できないビルドになる。所有者が版の固定を決めた |
| Rust だけを `rust-toolchain.toml` でも固定する | mise と rustup の 2 か所に版を持つことになる。mise の設定だけにする |

## 結果

- 手元と CI の Flutter、Rust、wrangler の版が `mise.toml` の 1 か所で決まる。
  wrangler をリポジトリの `package.json` や全体インストールで別に持たない。
- 版を上げるときは `mise.toml` の変更をコミットし、CI で全テストが通ることを確認してからマージする (PRD の「制約と前提」)。
- 開発者は mise を入れていることが前提になる。
  mise を入れずに標準のコマンドを直接叩いても動くが、版の保証は無い。
- 2026-09-21 の書き換え後の `mise.toml` は、Flutter 3.47.5 (同日時点の stable の最新。2026-09-18 公開)、Rust 1.98.1 (`targets = ["wasm32-unknown-unknown"]` 付き)、wrangler 4.135.0 を固定しており、本 ADR の決定と一致する。
  筆者が同日に `mise ls --current` と `rustup target list --installed` で、3 つの版と wasm32 ターゲットが `mise.toml` から解決されることを確認した。
- 本 ADR は版を上げる時期を定めない。
