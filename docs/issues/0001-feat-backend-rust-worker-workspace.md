# Rust ワークスペースと Worker のルーティング基盤を作る

Created: 2026-09-21
Model: deepseek-v4p1-flash
対応 ADR: ADR-0001 (docs/adr/0001-backend-rust-on-cloudflare-workers.md)、ADR-0002 (docs/adr/0002-database-cloudflare-d1.md)、ADR-0009 (docs/adr/0009-mise-toolchain-and-tasks.md)
関連 PRD: 制約と前提 (Backend の 3 クレート構成、成果物の規約)、成功指標 (API の応答時間のログ、エンドポイントのテスト網羅)
依存: なし

## 背景

ADR-0001 は、Backend を Rust で書き、workers-rs で wasm32-unknown-unknown 向けにビルドして Cloudflare Workers にデプロイすると決めた。
リポジトリには `mise.toml` と `docs/` しかなく、Cargo のワークスペースも Worker のコードも無い。
ADR-0001 は、利用者向けの Worker (`coffee_log`)、管理者 Worker (`coffee_log_admin`、ADR-0008)、両方で使う共有ライブラリ (`coffee_log_core`) の 3 クレートに分けることも決めた。
管理者 Worker の実装は 0018 で行うが、ワークスペースと 3 つ目のクレートの枠は本 issue で作る。
ADR-0002 は、SQL の組み立てと結果の変換を、D1 のバインディングに触れるコードから分離し、ネイティブターゲットでテストできるようにすることを求める。
PRD の成功指標は、Worker がリクエストごとに経路名と処理時間を英語のログに出し、リリース後に Workers Logs で p95 を集計することを前提にする。
同じ成功指標は、Router の定義から生成したエンドポイント一覧と、テストが付けた識別子を CI で照合することも求める。
現状は Worker が存在しないため、これらの前提を満たす場所が無い。

## 目的

3 クレートのワークスペースを作り、利用者向けの Worker が Router で `/api/*` を処理し、経路名と処理時間を英語でログに出せる状態にする。

## 設計判断

- ディレクトリは `backend/` に置く (PRD の「Frontend と Backend を同じリポジトリの別ディレクトリ」に従う)。
  `backend/Cargo.toml` をワークスペースとし、`backend/coffee_log/`、`backend/coffee_log_admin/`、`backend/coffee_log_core/` をメンバーにする。
- 依存は `worker = { version = "0.8.6", features = ["d1"] }`、`serde`、`serde_json` の 3 つに絞る。
  R2 の `Bucket` は feature を要さない (worker 0.8.6 の feature が `d1`、`queue`、`http` だけであることを docs.rs の worker 0.8.6 のページで確認した)。
  wasm のビルドに `worker-build` を使うため、`mise.toml` の `[tools]` に `"cargo:worker-build"` を追加する。
  版は crates.io の worker-build の最新 (2026-09-21 時点で 0.8.6) に固定し、mise の cargo バックエンドで入れる。
- HTTP のルーティングは `worker::Router` を使う。
  axum と `worker` の `http` feature は使わない (ADR-0001)。
- 経路は 1 つの台帳 (経路名、メソッド、パターン、認証の要否、入力の有無) で定義し、Router はその台帳から組み立てる。
  経路名はログとテストの識別子に使う。
  成功指標の照合は、この台帳を入力にしたデータ駆動の結合テストで行い、認証の要否に応じた種別 (正常系、未認証 401、入力不正 400) が全経路に揃っていることを最後に検査する。
  台帳とテストスイートの枠を本 issue で作り、経路とテストは 0005 以降が追加する。
- エラー応答は `{"error": {"code": "<snake_case>", "message": "<英語>"}}` の JSON に統一し、ステータスコードは PRD の「エラー応答の規約」に従う (400、401、403、404、409、410)。
  応答の組み立てのうち純粋な部分は `coffee_log_core` に置き、境界値を単体テストする。
- ログは `worker::console_log!` で 1 行の JSON (`event`、`route`、`method`、`status`、`duration_ms`) を英語で出す。
  利用者の記録の内容 (商品名、感想など) は出さない (PRD の運用)。
  処理時間の計測はリクエストの受信から応答の生成までとし、Workers Logs で `duration_ms` の p95 を集計できる形にする。
- `coffee_log_core` は `worker` クレートに依存させない。
  wasm に依存しないロジック (入力検証、エラー変換、SQL の組み立て) を置き、ネイティブターゲットでテストする。
- テストはグローバルの規約に従う。
  単体テストは `tests/test_<module>.rs`、PBT は `pbt/tests/prop_<module>.rs` に置き、PBT は `pbt/` をワークスペースのメンバークレートにする。
- バインディングに触る部分は `wrangler dev` に対する結合テストで検証する (ADR-0001)。
  結合テストはネイティブの Rust テストとし、`backend/coffee_log/tests/` に置く。
  テストハーネスが `wrangler dev` を子プロセスとして起動し (ローカルの状態ディレクトリを指定する)、マイグレーションを適用し、応答を待ってからテストを実行し、終了時に停止する。
  この起動と停止のロジックは Rust のコードに置く (ADR-0009 の「ロジックは Rust か Dart のコードにする」に従う)。
  HTTP 呼び出しのために dev-dependency に `reqwest` (blocking) を追加する (依存を追加する理由は issue に記録する規約に従い、ここに記す)。
- 本 issue では `mise.toml` のタスクを追加しない (ADR-0009 のタスクは 0002 が追加する)。
  確認は `cargo` と `wrangler` を直接実行して行う。
- 採らない案: axum (依存が増える。Router で足りる。ADR-0001)、`http` feature (axum 前提の機能で不要)、単一クレート (管理者 Worker との分離が ADR-0008 と衝突する)、経路ごとの手書きの Router 登録 (台帳と二重管理になり、成功指標の照合ができない)。

## 完了条件

- `backend/` に 3 クレートのワークスペースがあり、`cargo build --target wasm32-unknown-unknown -p coffee_log -p coffee_log_admin` が成功する。
- `coffee_log` の Worker が `wrangler dev` で起動し、未実装の `/api/*` の経路に PRD の形式の 404 の JSON を返す。
- リクエスト 1 件ごとに `event`、`route`、`method`、`status`、`duration_ms` を含む 1 行の JSON ログが英語で出る (wrangler dev の出力で確認する)。
- `/api` の経路の台帳がコードにあり、結合テストのスイートが台帳を列挙して未認証 401 と入力不正 400 の要否を判定できる。
- `coffee_log_core` のエラー変換の単体テストが `cargo test -p coffee_log_core` で通る。
- 結合テストのハーネスが `wrangler dev` の起動とマイグレーションの適用と停止を行い、存在しない経路の 404 の確認までを 1 つのテスト実行で完了する。
- ログに記録の内容を載せられないことを、ログ出力の関数が受け取る型にリクエスト本文とクエリの値を含めない設計 (経路のメタデータだけ) として単体テストで確認する。
  記録の内容を含むリクエストを実行してログに現れないことを確認する結合テストは、記録を扱う最初の経路を作る 0006 が持つ。
- `cargo clippy --all-targets -- -D warnings` と `cargo fmt --check` が通る。
- `mise.toml` の `[tools]` に `"cargo:worker-build"` が版固定で追加されている。

## 関連

- 0002 が `mise.toml` のタスクと CI を追加する。
- 0003 が D1 のバインディングと初期スキーマを追加する。
- 0018 が `coffee_log_admin` を実装する。
