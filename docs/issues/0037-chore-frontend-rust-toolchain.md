# Frontend の Rust ツールチェーンと Dioxus のビルドのタスクを追加する

Created: 2026-10-02
Model: deepseek-v4p1-flash
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

---
