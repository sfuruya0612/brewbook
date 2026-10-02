# Frontend の Rust ツールチェーンと Dioxus のビルドのタスクを追加する

Created: 2026-10-02
Model: deepseek-v4p1-flash
Completed: 2026-10-02
対応 ADR: ADR-0017 (docs/adr/0017-frontend-rust-dioxus-web-only.md)、ADR-0009 (docs/adr/0009-mise-toolchain-and-tasks.md)
関連 PRD: 制約と前提 (ツールの版の固定とタスク)、非機能要求 運用 (CI)
依存: なし

## 背景

本 issue は、2026-10-01 の所有者の決定 (iOS 対応を要件から落とし、Frontend を Rust の Dioxus に置き換える。ADR-0017) から起票した。

ADR-0017 は Frontend を Flutter から Rust (Dioxus) に置き換えると決めた。
`mise.toml` の `[tools]` は `flutter = "3.47.5"` (2 行目) と `chromedriver = "154.0.8037.57"` (15 行目) を持ち、`frontend:*` の 8 タスク (`frontend:setup`、`frontend:build`、`frontend:build-e2e`、`frontend:analyze`、`frontend:test`、`frontend:test-web`、`frontend:test-integration`、`frontend:test-same-origin`) は Flutter のコマンドを実行する (211 行目から 278 行目)。
`[tasks.check]` は `frontend:build` と `frontend:analyze` を実行する (61 行目から 63 行目)。
CI (`.github/workflows/ci.yml`) は mise でツールを入れ、Chrome for Testing を固定版で入れる (31 行目から 41 行目)。
`mise run check` は手元で 27 分前後かかる (同ファイルの 17 行目のコメント)。

Dioxus のアプリをビルドするには Dioxus CLI (`dx`) が要り、Tailwind CSS を使うには Tailwind CLI が要る。
`mise registry` の一覧に dioxus と Tailwind CLI の項目があるかは未確認である (仮説。実装の最初に確認する)。

## 目的

Dioxus の Frontend をビルド、検査、テストするためのツールと mise のタスクが揃い、最小の Dioxus のアプリが CI でも同じ手順でビルドできるようにする。

## 設計判断

- Dioxus CLI は、`github:DioxusLabs/dioxus` のリリースのバイナリ (`dx-aarch64-apple-darwin.tar.gz` など) を `[tools]` に固定する案と、`cargo:dioxus-cli` でソースから入れる案を実装の最初に試し、選んだ方と根拠 (手元のビルドの所要と、CI での所要の計測値) を issue に記録する。
  ソースからのビルドは 10 分以上かかる見込みである (仮説)。
- `mise registry` で引けるか (experimental の有効化が要るか) は実装の最初に確認する。
- Tailwind CLI は、`dx` がプロジェクトのルートの `tailwind.css` を検出して Tailwind CLI をダウンロードして起動する (Dioxus 0.7 のリリースノート: https://dioxuslabs.com/blog/release-070/ と https://dioxuslabs.com/learn/0.7/essentials/ui/hotreload)。
  ADR-0009 の「ツールの版は `[tools]` だけが持つ」に合わせるため、mise で Tailwind CLI の版を固定して `dx` に渡す方法 (環境変数または PATH) を実装の最初に確認する。
  渡せない場合は `dx` の自動取得を許容し、その理由と取得される版を issue に記録する (ADR-0009 の例外として記録する)。
- `frontend/` に Cargo のワークスペースと `brew_book_frontend` のクレートを作り、`dx build` が通る最小のアプリ (空の画面) を置く。
  Flutter のファイルとは同居し、Flutter の削除は 0045 で行う。
- `frontend/web/` の静的アセット (favicon、`icons/`、`manifest.json` と、`index.html` の `<title>` と表示名、`manifest.json` のテーマ色) を Dioxus の静的アセットに移して参照させる。
  `index.html` の `apple-touch-icon` の参照は外す (`apple-touch-icon-180.png` は iOS を対象から落とすため移さない)。
- タスクは移行の間だけ `dioxus:*` の名前で追加し、Flutter の `frontend:*` は触らない。
  先に `frontend:*` の名前を奪うと、`frontend:build` に依存する `dev`、`deploy-app-*`、`backend:test-integration`、`check` が同時に壊れ、`dioxus:test` の改名先の `frontend:test` が既存のタスクと衝突するためである。
  追加するタスクは次の 5 つとする。
  - `dioxus:build`: `dx build --release --platform web`。
  - `dioxus:lint`: `cargo clippy` と `cargo fmt --check` を `frontend/Cargo.toml` に対して実行する。
  - `dioxus:test`: native の `cargo test` (単体と PBT) を `frontend/Cargo.toml` に対して実行する。
  - `dioxus:test-web`: `wasm-bindgen-test` を headless の Chrome (mise の chromedriver) で実行する。
  - `dioxus:build-e2e`: テスト用の feature を有効にした Web ビルド (0044 が使う)。
- `dioxus:build` と `dioxus:lint` を `[tasks.check]` に、`dioxus:test` と `dioxus:test-web` を `[tasks.test]` に参加させる。
  移行の間も Dioxus のビルドとテストが CI (`mise run check`) で実行されるようにするためである (0044 の E2E は `frontend:test-same-origin` 経由)。
- `.gitignore` に `frontend/target/` と dx の出力を足す (Flutter の行は 0045 で消す)。
- 成果物のディレクトリは実装の最初に確認して issue に記録する (Dioxus 0.7 の既定は `target/dx/<アプリ名>/release/web/public` の見込み。仮説)。
- CI に Rust のビルドのキャッシュ (`actions/cache` で `~/.cargo/registry`、`~/.cargo/git`、`backend/target`、`frontend/target`) を追加する。
  action の版はコミットハッシュで固定する (ADR-0011)。
- ローカル開発は Flutter のときと同じ「ビルドしてから `wrangler dev`」を維持する。
  `dx serve` の開発サーバーは別オリジンになり、CORS を許可しない決定と `Origin` の検証に反する (ADR-0005)。
- 採らない案: Trunk を入れる (ADR-0017 で不採用)、`dx serve` をローカル開発に使う (別オリジン。ADR-0005)、Tailwind を使わず手書きの CSS にする (ADR-0017 で Tailwind を使うと決めた)。

## 完了条件

- `mise.toml` の `[tools]` に Dioxus CLI の版が固定され、`mise install` の後に `dx --version` がその版を返す。
- Tailwind CLI の入手方法 (mise で固定する、または `dx` の自動取得を許容する) と、その理由と版が issue に記録されている。
- `mise run dioxus:build` が成功し、`index.html`、JavaScript のローダー、WebAssembly、Tailwind の出力を含む成果物のディレクトリを作る。
- 成果物のディレクトリのパスが issue に記録されている。
- 成果物のディレクトリの WebAssembly (`*.wasm`) と JavaScript (`*.js`) の合計のサイズ (gzip 後。Tailwind の CSS は含めない) を計測し、値を issue に記録する (ADR-0017 の結果)。
- `frontend/web/` の favicon、icons、`manifest.json` が Dioxus の静的アセットとして配信され、`index.html` から参照されている (`apple-touch-icon` の参照は外す)。
- `.gitignore` に `frontend/target/` と dx の出力が入り、ビルドの生成物が `git status` に現れない。
- `dioxus:build`、`dioxus:lint`、`dioxus:test`、`dioxus:test-web` が `[tasks.check]` または `[tasks.test]` に参加し、CI の `mise run check` で実行される。
- `mise run dioxus:lint`、`mise run dioxus:test`、`mise run dioxus:test-web` が成功する。
- CI に Rust のビルドのキャッシュが入り、2 回目以降の CI のログにキャッシュのヒット (`Cache restored from key: ...`) が出る (ログを issue に記録する)。
- 既存の Flutter のタスクとテストは動いたままで、`mise run check` が通過する。
- この issue では Flutter のタスクとツールを消さない (0045 で消す)。

## 関連

- 0038 以降が `dioxus:*` のタスクを使う。
- 0045 が `dioxus:*` を `frontend:*` に改名し、Flutter のタスクとツールを消す。

## 解決方法

Frontend の Rust (Dioxus) のツールチェーンとビルドとテストのタスクを追加した (ADR-0017)。

- `mise.toml` の `[tools]` に `"github:DioxusLabs/dioxus" = "0.7.10"` と `"github:wasm-bindgen/wasm-bindgen" = "0.2.129"` を追加した。
  Dioxus CLI は GitHub リリースのバイナリで入れる (2026-10-02 の手元の計測で取得 4.1 秒 / 30.7 MB。`cargo:dioxus-cli` のソースからのビルドは 10 分以上かかる見込みで、CI もダウンロードだけで済むため不採用)。`mise registry` に dioxus の項目が無いことは `mise ls dioxus` が "not found in mise tool registry" を返すことで確認した。CI での所要の計測値は、CI の実行が所有者の指示で移行の完了後になったため未計測である。
  wasm-bindgen は `dioxus:test-web` のランナー (`wasm-bindgen-test-runner`) に必要で、版は `frontend/Cargo.lock` の wasm-bindgen と揃えた。
- Tailwind CLI は `dx` の自動取得を許容した (ADR-0009 の例外。mise で Tailwind CLI だけを `dx` に渡す方法が無く、`NO_DOWNLOADS=1` は 4 ツールすべてを PATH に要求するため)。取得される版は tailwindcss v4.1.5 である (成果物の `tailwind.css` の先頭で確認)。理由と版は `mise.toml` のコメントに記録した。
- `frontend/` に Cargo のワークスペースと `brew_book_frontend` のクレートを追加した (`Cargo.toml`、`Cargo.lock`、`Dioxus.toml`、`src/main.rs`、`tests/test_build.rs`、`tests/test_web.rs`、`index.html`、`tailwind.css`)。
- `frontend/web/` の favicon、icons、`manifest.json` を `frontend/public/` に写した。`index.html` からは favicon と `manifest.json` を参照させ、icons は `manifest.json` の `icons[].src` から参照させた (`apple-touch-icon` の参照は外した。`apple-touch-icon-180.png` は iOS を対象から落とすため移していない)。
  PNG 5 ファイルは元とバイト単位で一致する。`manifest.json` は `description` を「brewbook の Frontend (ADR-0007)」から「brewbook の Frontend (ADR-0017)」に更新した (1 行の差分)。
  `index.html` の `<title>` と表示名 (`apple-mobile-web-app-title`) は、`Dioxus.toml` の `[web.app] title = "brewbook"` と `frontend/index.html` で引き継いだ。
- `mise.toml` に `dioxus:build`、`dioxus:build-e2e`、`dioxus:lint`、`dioxus:test`、`dioxus:test-web` の 5 タスクを追加し、`[tasks.check]` に `dioxus:build` と `dioxus:lint`、`[tasks.test]` に `dioxus:test` と `dioxus:test-web` を参加させた。Flutter の `frontend:*` の 8 タスクと `flutter` の `[tools]` は変更していない (0045 が消す)。
  `dioxus:build` は `set -e` の下でビルドの後に成果物を検査する。`index.html`、`tailwind.css`、`favicon.png`、`manifest.json`、icons の 4 ファイルは `test -s` (空のファイルも弾く)、ハッシュ付きの名前の `assets/*.wasm` と `assets/*.js` は `ls` で存在を確認する。
  `dioxus:test-web` は chromedriver の存在を先に確認し、`--workspace` を付けて wasm の全テストを実行する (テストのファイルを列挙しない)。
- `.gitignore` に `/frontend/target/` と `/frontend/public/tailwind.css` を追加した。
- `.github/workflows/ci.yml` に `actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9` (v6.1.0。コミットハッシュで固定。ADR-0011) を追加した。`path` は `~/.cargo/registry`、`~/.cargo/git`、`backend/target`、`frontend/target`、`key` は `${{ runner.os }}-cargo-${{ hashFiles('backend/Cargo.lock', 'frontend/Cargo.lock', 'mise.toml') }}`、`restore-keys` は `${{ runner.os }}-cargo-` とした。

完了条件の検証:

- Dioxus CLI の固定と `dx --version`: `mise.toml` の `[tools]` の `github:DioxusLabs/dioxus` が 0.7.10 を返すことを `mise exec -- dx --version` で確認した (このマシンでは Homebrew の deno の `dx` が PATH で先に解決されるため、タスクは `mise exec -- dx` を使う)。
- Tailwind CLI の入手方法と理由と版: 上記のとおり (`dx` の自動取得、tailwindcss v4.1.5)。
- `mise run dioxus:build`: 作業ツリーで成功。成果物のディレクトリは `frontend/target/dx/brew_book_frontend/release/web/public` で、`index.html`、`tailwind.css` (7,453 B)、`assets/brew_book_frontend_bg-dxh7a1b53fc6f58bfd.wasm`、`assets/brew_book_frontend-dxha0ca594af150c27f.js`、`favicon.png`、`icons/` の 4 ファイル、`manifest.json` の 10 ファイルを含む。`dioxus:build` のタスクが成果物の存在を検査する。
- 成果物のディレクトリ: 上記のパス。
- gzip 後のサイズ: 作業ツリーの成果物で wasm 464,504 B + js 13,606 B = 478,110 B (Tailwind の CSS は含めない)。生サイズは wasm 1,914,406 B、js 56,684 B。
- 静的アセット: `frontend/public/` の favicon、icons 4 種、`manifest.json` が成果物に含まれる。`index.html` は favicon と `manifest.json` と `tailwind.css` をタグで参照し、icons は `manifest.json` の `icons[].src` から参照する。`apple-touch-icon` の参照は無い。`tests/test_build.rs` の `index_html_references_the_static_assets` (コメントを除いて属性の組で検査し、`apple-touch-icon` の語を含む参照が無いことも検査する)、`public_dir_has_the_static_assets`、`manifest_has_the_name_and_theme_color` (JSON として解析し、`name`、`short_name`、`theme_color`、`background_color` の値と `icons[].src` の実在を検査する) で確認した。
- `.gitignore`: 追加済み。`git status --porcelain` にビルドの生成物は現れない (`git check-ignore -v` で `/frontend/target/` と `/frontend/public/tailwind.css` の一致を確認)。
- タスクの参加: `[tasks.check]` と `[tasks.test]` に追加済み。CI の実行は最終確認に委ねる。
- `mise run dioxus:lint`、`mise run dioxus:test`、`mise run dioxus:test-web`: 作業ツリーで成功 (lint は clippy と fmt とも警告なし、test は 4 件、test-web は headless Chrome で 1 件)。
- CI のキャッシュ: 追加済み。キャッシュのヒットログは CI の実行後に確認する (この issue では未確認。所有者の指示で CI は移行の完了後に実行する)。
- 既存の Flutter のタスクとテスト、`mise run check`: Flutter のタスクと `[tools]` は未変更 (`mise tasks ls` で `frontend:*` の 8 タスクと `flutter` の固定を確認)。`mise run check` は所有者の指示により全 issue の完了後に 1 回だけ実行するため、この issue では未実行 (最終確認に委ねる)。
- Flutter のタスクとツールを消していない: 上記のとおり。

方針からの乖離 (方式は変えていない):

- `dioxus:build` と `dioxus:build-e2e` は `dx` ではなく `mise exec -- dx` を使う。このマシンでは Homebrew の deno 2.9.7 が `/opt/homebrew/bin/dx` を提供し、mise を activate していないシェルではそちらが先に解決されるためである。`[tools]` に固定した版を使う方針は保たれている。
- `dioxus:test-web` は `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER` と `CHROMEDRIVER` を設定して mise の chromedriver を明示する。
- レビューの指摘を受けて次を変えた (方式は変えていない)。`tests/test_build.rs` の `index.html` の検査をコメントを除いた属性の組の検査に変え、`manifest.json` を `serde_json` で解析して値と `icons[].src` の実在を検査するようにした (`serde_json` を dev-dependency に追加。Cargo.lock に既に含まれる推移的な依存で、ビルドの構成は変わらない)。`tests/test_build.rs` に native の gate を足し、`dioxus:test-web` のテストのファイルの列挙を外して `--workspace` を付けた。`dioxus:build` に `set -e` と成果物の検査 (静的アセットを含む) を足した。`dioxus:test-web` に chromedriver の存在の確認を足した。

検証の補足:

- `mise run check` を既定の並列で実行したときに `frontend:test-integration` (chromedriver を `--port=4444` で起動する) と `dioxus:test-web` が競合しないことは、4444 を別の chromedriver で塞いだ状態でも `dioxus:test-web` が成功すること (wasm-bindgen-test-runner が空きポートを選ぶこと) で確認した。
- 未了の 3 行 (CI での実行、キャッシュのヒットログ、`mise run check` の通過) は所有者の指示による保留である。移行の全 issue の完了後に `mise run --jobs 1 check` を 1 回実行し、CI のログと併せて結果を追記する。
