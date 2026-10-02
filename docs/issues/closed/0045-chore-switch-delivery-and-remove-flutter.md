# 配信を Dioxus のビルドに差し替え、Flutter 一式を削除する

Created: 2026-10-02
Model: deepseek-v4p1-flash
Completed: 2026-10-02
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
- 2026-10-02 の所有者の決定: 0036 はこの issue で close する。close の記録には、Flutter のタスクを消した後に並列の `mise run check` で競合が再現しないことを書く。
- この issue が移行の最後である。

## 解決方法

配信を Dioxus のビルドに差し替え、Flutter 一式を削除した (ADR-0017、ADR-0005、ADR-0008、ADR-0009、ADR-0010、ADR-0012)。

- `backend/brew_book/wrangler.toml` の `[assets]` の `directory` を Dioxus のビルドの成果物 (`../../frontend/target/dx/brew_book_frontend/release/web/public`) に変えた (`run_worker_first` と `not_found_handling` は変えない)。
- `mise.toml` の `dioxus:*` を `frontend:*` に改名し、`frontend:setup` (`cargo fetch`) を新設した。最終的なタスクは `frontend:setup`、`frontend:build`、`frontend:build-e2e`、`frontend:lint`、`frontend:test`、`frontend:test-web`、`frontend:test-same-origin` で、`[tasks.test]` と `[tasks.check]` の `depends` をこの構成に合わせた。Flutter のタスク (`frontend:analyze`、`frontend:test-integration`、`frontend:build` の Flutter 版など) と `[tools]` の `flutter` を消した (`chromedriver` は残した)。
- `load/k6/load.js` の対象を `/`、JavaScript のローダー、WebAssembly の 3 つに変えた。ファイル名には `dx` がハッシュを付けるため、`setup` で `index.html` とローダーを 1 回読んで実際のパスを解決する方式にした (ADR-0012 の改訂に記録した)。
- `frontend/` の Flutter のファイルを削除した (`ios/`、`lib/`、`test/`、`integration_test/`、`test_driver/`、`web/`、`README.md`、`.gitignore`、`pubspec.yaml`、`pubspec.lock`、`analysis_options.yaml`、`l10n.yaml`、`.metadata`。206 ファイル、約 24,900 行)。追跡外の生成物も消した。`frontend/public/` の静的アセット (0037 が写したもの) はそのまま使う。
- `backend/brew_book/tests/support/mod.rs` の `DEFAULT_ASSETS_DIR` と一時設定の置換文字列を Dioxus の成果物に合わせ、`backend/brew_book/tests/wrangler_same_origin_api.rs` の Flutter の成果物名 (`flutter_bootstrap.js`、`main.dart.js`) の検証を Dioxus の成果物 (JavaScript のローダーを `index.html` から解決する) に変えた。`backend/brew_book_admin/tests/support/mod.rs` のアセットのパスも合わせた。
- `.github/workflows/ci.yml` から Flutter のセットアップを外した (Chrome for Testing と chromedriver の版の固定は残した)。`.gitignore` から Flutter の行 (`/frontend/build/`、`/frontend/.dart_tool/` など) を消した。
- `README.md`、`docs/design/README.md`、`docs/design/INDEX.md`、`docs/design/assets/AppIcon/README.md`、`docs/design/components/*/README.md` の Flutter の記述を Dioxus と Tailwind の記述に置き換えた (原本の `tokens.json`、`tokens.css`、プレビュー、スクリーンショットは書き換えていない)。
- `docs/adr/0005-same-origin-deployment.md`、`docs/adr/0008-admin-worker-cloudflare-access.md`、`docs/adr/0009-mise-toolchain-and-tasks.md`、`docs/adr/0010-app-name-and-namespace-brewbook.md`、`docs/adr/0012-load-testing-with-k6.md`、`docs/adr/0014-google-fonts-for-the-design.md` を改訂した (Flutter と iOS の記述を Dioxus と Web だけの対象に合わせた。「改訂: 2026-10-02」を追記)。
- `frontend/tests/test_i18n.rs` から ARB との照合の検査を削った (原本の ARB を 0045 で消すため。翻訳の表とキーの列挙の検査は残した)。`frontend/tests/` と `frontend/src/`、`backend/brew_book/tests/` のタスク名のコメントを `frontend:*` に直し、削除した Flutter のファイルを指すコメントを「移行前の Flutter 版 (0045 で削除)」と分かるように直した (`frontend/Cargo.toml` と `frontend/pbt/Cargo.toml`、`backend/brew_book/tests/wrangler_same_origin_e2e.rs` のタスク名のコメントを含む)。
- 0036 (Flutter の並列実行で build ディレクトリが競合する) を close した (2026-10-02 の所有者の決定。Flutter のタスクを消したため再現しない)。

完了条件の検証:

- `mise run dev` が Dioxus のビルドを配信し、`http://localhost:8787/` で画面が表示され、`/register` などの経路を直接開くと `index.html` が 200 で返る: 作業ツリーで `mise run dev` を起動し、15 秒で `http://localhost:8787/` が 200 (text/html) を返し、Dioxus の JavaScript のローダー (`assets/brew_book_frontend-*.js`) を参照することを確認した。`/register` を直接開いても 200 で同じ HTML が返ることを確認した。
- `mise run load` が Dioxus の静的アセットとログインのチャレンジ発行を叩き、閾値 (失敗率 1% 未満、p95 500 ms 未満) を満たす: `mise run dev` を起動した状態で `mise run load` を実行し、14,602 リクエストで失敗率 0.00% (閾値は 1% 未満)、p95 432 ms (閾値は 500 ms 未満) で成功した (50 VU で 60 秒)。
- `frontend/` に Flutter のファイルが残っていない (`git ls-files frontend` で確認する): 削除をステージした後に `git ls-files frontend` を実行し、Dart のファイル、`pubspec`、`analysis_options.yaml`、`l10n.yaml`、`.metadata`、`ios/`、`web/`、`integration_test/`、`test_driver/` が無いこと (121 件は Rust のクレートとテスト、静的アセット、`Dioxus.toml`、`Cargo.toml`、`Cargo.lock`、`index.html`、`tailwind.css`、`webdriver.json`) を確認した。作業ツリーの `frontend/` にも Flutter のファイルは無い。
- `backend/brew_book/tests/support/mod.rs` の `DEFAULT_ASSETS_DIR` と置換文字列、`backend/brew_book_admin/tests/support/mod.rs` のアセットのパス、`backend/brew_book/tests/wrangler_same_origin_api.rs` の検証が Dioxus の成果物を指し、`mise run backend:test-integration` が成功する: 作業ツリーで `mise run backend:test-integration` が成功した (`mise run check` の中で実行。1392 秒)。配信する成果物のディレクトリを E2E のビルドと共有するため、`backend:test-integration` の依存を `frontend:build-e2e` に変えた (E2E のビルドが出力を消してから作るため、実行中の削除と競合させない。0045 のレビューの指摘)。
- `backend/brew_book/wrangler.toml`、`mise.toml`、`.github/workflows/ci.yml` に Flutter と iOS を前提にしたコメントが残っておらず、ルートの `.gitignore` に Flutter の行が残っていない: `rg -i flutter .gitignore mise.toml .github/workflows/ci.yml backend/brew_book/wrangler.toml` で該当が無いことを確認した (ADR と docs の改訂の記述は除く)。
- `mise.toml` に Flutter のツールとタスクが残っていない: `[tools]` に `flutter` が無く、`mise tasks ls` に Flutter のタスクが無いことを確認した。
- README、`docs/design/README.md`、`docs/design/components/*/README.md` の Flutter の記述が Dioxus の記述に置き換わっている: 上記のとおり。
- ADR-0005、ADR-0008、ADR-0009、ADR-0010、ADR-0012 の Flutter の記述の改訂: 上記のとおり (ADR-0014 は 0039 が改訂済みで、0045 が残りの記述を合わせた)。
- `CHANGES.md` の `## develop` に移行の変更が追記されている: 0045 のエントリを `## develop` の `### misc` に追記した (0037 から 0044 のエントリは移行の各 issue が追記済み)。
- `mise run check` が通過する (Flutter の無い状態で、全ての自動テストが CI と同じ入口で動く): 作業ツリーで `mise run check` を既定の並列で実行し、通過した (2026-10-02、約 23 分)。内訳: fmt、lint (clippy と絵文字の検査)、formal (TLA+)、frontend:build、frontend:lint、frontend:test (native 230 件)、frontend:test-web (40 件)、frontend:test-same-origin (E2E)、backend:test (359 件)、backend:test-integration。1 回目の実行では `wrangler_account_api` の R2 のカーソルの検査が並列の負荷でタイムアウトした (単体では通過。0036 の解決方法にも記録した) が、2 回目で全て通過した。

方針を保った実装詳細の乖離:

- `frontend:setup` は `cargo fetch` とした (Flutter の `flutter pub get` の代わり。ビルドとテストは cargo が必要な依存を自動で取得するため、CI と手元で依存を先に揃えるためだけに置く)。
- `frontend:build-e2e` は `frontend:build` に依存させ、出力を消してから作る (mise は依存の無いタスクを並列に実行するため、配信のビルドと混ざらないようにする)。
- k6 のハッシュ付きのファイル名は `setup` で解決する方式にした (名前を固定する設定を使う案と比較し、`dx` の出力を変えずに済むため)。
- `frontend/tests/test_i18n.rs` の ARB との照合は、原本を消すため削った (キーの列挙と 2 つの表の網羅の検査は残した)。

レビューの指摘を受けて変えたもの (方式は変えていない):

- `backend/brew_book_admin/tests/support/mod.rs` のアセットのパスを直した (クレートのルートから 2 段上る。1 段では `backend/frontend/target/...` を指しており、Dioxus の成果物ではなかった)。
- Flutter の削除をステージし、`git ls-files frontend` で Flutter のファイルが無いことを確認した (削除は `rm` で行っていたため、ステージ前の `git ls-files` は削除前の一覧を返していた)。
- `backend:test-integration` の依存を `frontend:build-e2e` に変えた (配信する成果物のディレクトリを E2E のビルドと共有するため、実行中の `rm -rf` と競合しないようにする)。
- `load/k6/load.js` の `setup` で、`/` とローダーの状態コードと本文の有無を先に検査するようにした (サーバー未起動のときに原因の分かるエラーにする)。
- 削除した Flutter のファイルを指すコメントを「移行前の Flutter 版 (0045 で削除)」と分かるように直し、`frontend/src/lib.rs` にその旨の注記を足した (動作には影響しない)。

レビューの指摘の反映後、`mise run check` を既定の並列でもう一度実行し、通過した (2026-10-03、約 27 分)。完了条件の「`mise run check` が通過する」はこの実行で満たした (上記の 1 回目と 2 回目は反映前の実行である)。
