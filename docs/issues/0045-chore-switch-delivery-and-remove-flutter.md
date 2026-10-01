# 配信を Dioxus のビルドに差し替え、Flutter 一式を削除する

Created: 2026-10-02
Model: deepseek-v4p1-flash
対応 ADR: ADR-0017、ADR-0005、ADR-0008、ADR-0009、ADR-0010、ADR-0012 (docs/adr/0012-load-testing-with-k6.md)
関連 PRD: スコープ、非機能要求 運用と性能、制約と前提
依存: 0044

## 背景

本 issue は、2026-10-01 の所有者の決定 (ADR-0017) から起票した。

`backend/brew_book/wrangler.toml` の `[assets]` は `directory = "../../frontend/build/web"` で Flutter のビルドを指し、`run_worker_first = ["/api/*"]` と `not_found_handling = "single-page-application"` を持つ (20 行目から 23 行目)。
`mise run dev` と `deploy-app-*` は `frontend:build` に依存する (111 行目、120 行目、129 行目)。
`load/k6/load.js` は `/main.dart.js` と `/flutter_bootstrap.js` を叩く。
README は Flutter のビルドとルーティングの記述を持つ (iOS の記述は ADR-0004、ADR-0005、ADR-0010 と `frontend/ios/` にある)。
`docs/design/README.md` は Flutter への落とし込みの表を持ち、`docs/design/components/*/README.md` にも Flutter の実装の指示がある (例: `Charts/README.md` の「fl_chart で描く」)。
CI は Flutter のセットアップと、`mise.toml` と `ci.yml` に Flutter を前提にしたコメントを持つ。

## 目的

本番とローカルの配信が Dioxus のビルドになり、Flutter の実装とツールとテストがリポジトリから消える。

## 設計判断

- `[assets]` の `directory` を Dioxus のビルドの成果物 (0037 で記録したパス) に変える。
  `run_worker_first` と `not_found_handling` は変えない (ADR-0005 の決定を維持する)。
- `mise.toml` の `dioxus:*` を `frontend:*` に改名し、Flutter のタスクを消す。
  最終的なタスクは `frontend:setup` (`cargo fetch` として新設する。Flutter の `flutter pub get` は消す)、`frontend:build`、`frontend:build-e2e`、`frontend:lint` (旧 `dioxus:lint`)、`frontend:test`、`frontend:test-web`、`frontend:test-same-origin` とし、`[tasks.test]` と `[tasks.check]` の `depends` をこの構成に合わせる (Flutter の `frontend:analyze`、`frontend:test-integration` は消す)。
- k6 の対象を `/`、JavaScript のローダー、WebAssembly の 3 つに変える。
  ファイル名にハッシュが付いてビルドのたびに変わる場合は、名前を固定する設定を使うか、k6 の前に一覧を取るかを実装の最初に決めて記録する。
- `frontend/ios/`、`frontend/lib/`、`frontend/test/`、`frontend/integration_test/`、`frontend/test_driver/`、`frontend/web/`、`frontend/README.md`、`frontend/.gitignore`、`pubspec.yaml`、`pubspec.lock`、`analysis_options.yaml`、`l10n.yaml`、`.metadata` を削除する (追跡されているファイルは `git rm` を使う)。
  追跡外の生成物 (`frontend/flutter_0*.log`、`frontend/.flutter-plugins-dependencies`) はあれば `rm` で削除する。
  `frontend/web/` の表示名 (`index.html` の `apple-mobile-web-app-title` と `manifest.json` の `name`、`short_name`) とアイコン (icons) は 0037 で Dioxus の静的アセットに移したものを使う (`apple-touch-icon-180.png` は iOS を対象から落とすため移さない)。
- `backend/brew_book/tests/support/mod.rs` の `DEFAULT_ASSETS_DIR` と一時設定の置換文字列、`backend/brew_book_admin/tests/support/mod.rs` のアセットのパス、`backend/brew_book/tests/wrangler_same_origin_api.rs` の Flutter の成果物名の検証を、Dioxus の成果物に合わせる。
- `backend/brew_book/wrangler.toml`、`mise.toml`、`.github/workflows/ci.yml` の Flutter と iOS を前提にしたコメントを更新し、ルートの `.gitignore` の Flutter の行 (`/frontend/build/`、`/frontend/.dart_tool/` など) を削除する。
- `mise.toml` の `[tools]` から `flutter` を外す。
  `chromedriver` は残す (`wasm-bindgen-test` と E2E が使う)。
- CI から Flutter のセットアップを外し、Chrome for Testing は残す (chromedriver と版を揃える)。
- README、`docs/design/README.md` (Flutter への落とし込みの表を Dioxus と Tailwind の対応表に変える)、`docs/design/components/*/README.md` (Flutter の実装の指示を Dioxus と Tailwind に変える)、`CHANGES.md` を更新する。
  原本 (`tokens.json`、`tokens.css`、プレビュー、スクリーンショット) は書き換えない。
- 採らない案: Flutter を残して両方を配信する (二重の保守になる)、`docs/design/` の原本を書き換える (原本はそのまま。実装が追随する)。

## 完了条件

- `mise run dev` が Dioxus のビルドを配信し、`http://localhost:8787/` で画面が表示され、`/register` などの経路を直接開くと `index.html` が 200 で返る (README の確認手順)。
- `mise run load` が Dioxus の静的アセットとログインのチャレンジ発行を叩き、閾値 (失敗率 1% 未満、p95 500 ms 未満) を満たす。
- `frontend/` に Flutter のファイルが残っていない (`git ls-files frontend` で確認する)。
- `backend/brew_book/tests/support/mod.rs` の `DEFAULT_ASSETS_DIR` と置換文字列、`backend/brew_book_admin/tests/support/mod.rs` のアセットのパス、`backend/brew_book/tests/wrangler_same_origin_api.rs` の検証が Dioxus の成果物を指し、`mise run backend:test-integration` が成功する。
- `backend/brew_book/wrangler.toml`、`mise.toml`、`.github/workflows/ci.yml` に Flutter と iOS を前提にしたコメントが残っておらず、ルートの `.gitignore` に Flutter の行が残っていない。
- `mise.toml` に Flutter のツールとタスクが残っていない。
- README、`docs/design/README.md`、`docs/design/components/*/README.md` の Flutter の記述が Dioxus の記述に置き換わっている。
- ADR-0005 の Flutter のルーティングとビルドと iOS 向けの配信 (`/.well-known/*`、`apple-app-site-association`、Cookie と Bearer トークン)、ADR-0008 の管理者画面の Flutter の記述、ADR-0009 の Flutter の版の固定と Flutter を前提にした記述 (背景とタスクの規約)、ADR-0010 の Flutter と iOS の名前と iOS の Associated Domains の記述 (名前空間への `brew_book_frontend` の追加を含む)、ADR-0012 の Flutter の成果物の記述を、Dioxus と Web だけの対象に合わせて改訂する。
- `CHANGES.md` の `## develop` に移行の変更が追記されている。
- `mise run check` が通過する (Flutter の無い状態で、全ての自動テストが CI と同じ入口で動く)。

## 関連

- 0037 から 0044 が Dioxus の実装を作る。
- 0036 (Flutter の並列実行で build ディレクトリが競合する) は、Flutter のタスクを消すこの issue で発生しなくなる。close するかは所有者に確認する。
- この issue が移行の最後である。
