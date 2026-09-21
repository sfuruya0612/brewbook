# Rust ワークスペースと Worker のルーティング基盤を作る

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-22
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

## 解決方法

`backend/` に 3 クレートと PBT のクレートを持つ Cargo ワークスペースを作り、利用者向けの Worker が Router で `/api/*` を処理してリクエストごとに 1 行の JSON ログを出すようにした。

- `backend/Cargo.toml` で `coffee_log`、`coffee_log_admin`、`coffee_log_core`、`pbt` をメンバーにした。`coffee_log` と `coffee_log_admin` の `[lib] crate-type` は `["cdylib", "rlib"]` とした (`rlib` はネイティブの結合テストが crate をリンクするために要る)。
- `coffee_log_core` にエラー応答 (`error.rs` の `ErrorCode`、`ErrorEnvelope`、`envelope`、`body`) と経路の台帳 (`routes.rs` の `Route`、`ROUTES`、`pattern_matches`、`match_route`、`matched`、`test_requirements`) を置き、`worker` クレートに依存させていない。
- `coffee_log` の `src/lib.rs` で台帳から Router を組み立て、台帳に一致しないリクエスト (未実装の `/api/*` を含む) に `{"error": {"code": "not_found", "message": "route not found"}}` の 404 を返す。台帳に載っていてハンドラが未実装の経路は `handle_not_implemented` が 404 を返す。
  `src/logging.rs` の `RequestLog` は経路名、メソッド、ステータス、処理時間だけを持ち、リクエスト本文とクエリの値を持たない。処理時間はリクエストの受信から応答の生成までを `Date::now().as_millis()` で測る。
- `coffee_log_admin` は `coffee_log_core` と `worker` だけを使う最小の Worker とし、0018 が実装するまでの間は全てのリクエストに 404 を返す。
- `pbt/tests/prop_routes.rs` が `pattern_matches` の一致と不一致を PBT で検証する。
- 結合テストは `coffee_log/tests/support/mod.rs` の `DevServer` が `wrangler dev` を空きポートで起動し (`--persist-to` に一時ディレクトリを指定)、`wrangler d1 migrations apply` を実行し (D1 のバインディングは 0003 が追加するため、0001 の時点では設定が無くスキップする)、HTTP が応答するまで待ち、`Drop` で停止して一時ディレクトリを消す。`tests/dev_server.rs` が POST `/api/not-implemented` の 404 と、`wrangler dev` の出力のうち POST のリクエストのログ 1 行を JSON として検証する。
- `tests/support/mod.rs` の `SUITE` (経路名とテスト種別) と `covers` が台帳との照合を担い、`covers` は `test_requirements` の 3 フィールドから必要な種別を導く。`tests/api_suite.rs` が照合の検査自体をサンプルの台帳とスイートで検証する (種別の欠落、経路の不一致、完全なスイートの受理)。`build_router_from` は 5 つのメソッドを含むサンプルの台帳の登録がパニックしないことを確認するテストで実行する。
- `mise.toml` の `[tools]` に `"cargo:worker-build" = "0.8.6"` を追加した。`coffee_log/wrangler.toml` の `[build]` が `worker-build --release` を実行し、`main` は `build/worker/shim.mjs` を指す。
- `.gitignore` を新設し、`backend/target/`、`backend/coffee_log/build/`、`backend/coffee_log/.wrangler/` を除外した (0002 が frontend 分を追加する)。

完了条件の検証:

- `cargo build --target wasm32-unknown-unknown -p coffee_log -p coffee_log_admin` の成功と、`target/wasm32-unknown-unknown/debug/coffee_log.wasm` と `coffee_log_admin.wasm` の生成を確認した。
- `wrangler dev` が `POST /api/not-implemented` に `application/json` の 404 と `{"error": {"code": "not_found", "message": "route not found"}}` を返すことを `tests/dev_server.rs::dev_server_returns_json_404_and_logs_the_request` で確認した。
- 同じテストが、POST のリクエストのログ行が 1 行だけで、JSON として `event`、`route`、`method`、`status`、`duration_ms` の 5 フィールドを持つことを確認した。
- 台帳とスイートの照合は `tests/api_suite.rs::the_ledger_and_the_suite_match`、`the_checker_accepts_a_complete_suite`、`the_checker_detects_a_missing_test_kind`、`the_checker_detects_a_route_present_on_one_side_only` が担う。未認証 401 と入力不正 400 の要否は `coffee_log_core/tests/test_routes.rs::test_requirements_follow_the_auth_and_input_flags` が認証と入力の 4 通りの組み合わせで確認した。
- `cargo test -p coffee_log_core` の 10 テスト (`test_error.rs` 5、`test_routes.rs` 5) が通った。
- ハーネスの起動、マイグレーションの適用のコードパス (0001 ではスキップ)、停止、404 の確認が 1 つのテスト実行で完了することを確認した。
- `coffee_log/tests/test_logging.rs` の 2 テストが `RequestLog` のフィールドが 5 つだけであることを確認した。
- `cargo clippy --workspace --all-targets -- -D warnings` と `cargo fmt --all --check` が通った。
- `mise.toml` に版固定の `"cargo:worker-build" = "0.8.6"` があることを確認した。
- `cargo test --workspace` は 21 テストが通り、失敗は無い (api_suite 5、dev_server 1、test_logging 2、test_error 5、test_routes 5、prop_routes 3)。

方針からの乖離:

- `ErrorCode` に `Internal` (500) を追加した。PRD の表は受け入れ基準の 6 コードだけを定めるため、ルーティング以外の Worker の失敗を PRD の形式で返すには 500 の code が要る。
- PBT の実行に `proptest` を `pbt` の dev-dependency に追加した (グローバルのテスト規約が PBT を求めるため)。統合テストの HTTP 呼び出しの `reqwest` (blocking) は issue の設計判断どおり。
- 完了条件 6 の「マイグレーションの適用」は、D1 のバインディングを追加する 0003 まで設定が無いため、0001 ではハーネスが適用のコードパスを持ち、構成が無ければスキップする。実際の適用 (バインディング名 `DB`、`--persist-to` の共有) の確認は 0003 が行う。
- 結合テストは `wrangler dev` の子プロセスから `OPENCODE`、`AGENT`、`AI_AGENT`、`CLAUDECODE` を除いて起動する。AI エージェントを検出した wrangler はログを標準出力に出さず Local Explorer に送るため、テストがログを読めない。

未検証の範囲 (経路を追加する issue が確認する):

- 台帳が空のため、Router の実際のディスパッチと `handle_not_implemented` の 404 は 0005 以降が最初に実行する。`pattern_matches` と Router (matchit) の照合の一致も経路の追加時に統合テストで確認する。
- `SUITE` の宣言は実際のテスト関数と機械的には結び付いていない。経路を追加する issue は、台帳への経路の追加とスイートへの種別の追加、実際のテストの追加をセットで行い、レビューで確認する。
