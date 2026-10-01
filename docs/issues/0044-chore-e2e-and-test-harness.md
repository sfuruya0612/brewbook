# E2E とテストの基盤を Dioxus に差し替える

Created: 2026-10-02
Model: deepseek-v4p1-flash
対応 ADR: ADR-0017、ADR-0013
関連 PRD: 成功指標 (画面数)、制約と前提 (テスト)、FR-1、FR-2、FR-10
依存: 0040, 0041, 0042, 0043

## 背景

本 issue は、2026-10-01 の所有者の決定 (ADR-0017) から起票した。

現在の E2E は `backend/brew_book/tests/wrangler_same_origin_e2e.rs` が `wrangler dev` と chromedriver と `flutter drive` を組み合わせ、テストコード入りの Web ビルド (`frontend/build/e2e-web`、`mise run frontend:build-e2e`) を Static Assets として配信する。
仮想認証器は `frontend/test_driver/same_origin_test.dart` が Chrome DevTools Protocol で付ける。
画面数の成功指標は `frontend/integration_test/app_test.dart` の `_ScreenCountObserver` がルーターの経路を数える。
Flutter のテストは `frontend/test/` (約 7,000 行) にあり、`mise run frontend:test`、`frontend:test-web`、`frontend:test-integration`、`frontend:test-same-origin` が実行する。

## 目的

Dioxus のアプリに対して、実 Worker と仮想認証器の E2E と、画面数の成功指標と、デザインの一致の確認が自動で動くようにする。

## 設計判断

- E2E のブラウザの操作は WebDriver (chromedriver) を Rust から呼ぶ。
  `thirtyfour` と `fantoccini` を比較して選ぶ (実装の最初に決めて記録する)。
  `flutter drive` は使わなくなる。
  Playwright は Node の依存とブラウザの入手が増えるため採らない (ADR-0009 のツールチェーンの方針)。
- 仮想認証器は ChromeDriver の WebAuthn の拡張コマンド (`WebAuthn.addVirtualAuthenticator`) で、`protocol: ctap2`、`transport: internal`、`hasResidentKey: true`、`hasUserVerification: true`、`isUserVerified: true`、`automaticPresenceSimulation: true` を設定して付ける (現在の Chrome DevTools Protocol の設定を写す。Backend は `userVerification: required` を要求するため、既定値に依存しない)。
  設定で登録とログインが通ることを実装の最初に E2E で確認する。
- テスト用のビルドは 0037 の `dioxus:build-e2e` (Cargo の feature `e2e` でテスト用のフックを有効にする) とする。
  配布用のビルドと混ぜない (Flutter と同じ方針)。
  `frontend:test-same-origin` は `dioxus:build-e2e` に依存させ、Flutter の `frontend:build-e2e` は使わなくする。
- 画面数の成功指標は、ルーターの現在の経路をアプリのルート要素の `data-route` 属性に出し、E2E が経路の種類を数える。
  ダイアログ、ボトムシート、保存完了の通知は数えない (PRD の成功指標)。
- デザインの一致は、E2E が主要な画面と部品 (0039) のスクリーンショットを Paper と Night の両方で取得し、`docs/design/screenshots/` と比較する。
  差分は issue に記録し、判断 (許容する差分と直す差分) を書く。直すと判断した差分は、この issue か元の issue で直す。
- Flutter のテストのうち、値の変換と API の入出力の PBT は 0038 から 0043 で native のテストとして移す。
  この issue は、移したテストが `mise run dioxus:test` と `mise run dioxus:test-web` の対象に入っていることを確認する。
- 採らない案: Playwright を入れる (Node の依存が増える)、`flutter drive` を残す (Flutter を消すため)、画面数の計測を API の呼び出し回数で代用する (PRD の測定方法と違う)。

## 完了条件

- `mise run frontend:test-same-origin` が Dioxus のビルドを配信し、登録、ログイン、ログアウト、抽出の保存、エクスポートのダウンロードの E2E を WebDriver で実行して成功する (仮想認証器を使う。写真のアップロードは R2 の資格情報 (`backend/brew_book/.dev.vars`) がある環境で行い、ない環境では対象外として記録する)。
- `backend/brew_book/tests/wrangler_same_origin_e2e.rs` の Flutter の成果物 (`build/e2e-web`、`main.dart.js`) の検証を Dioxus の成果物に合わせる。
- E2E が画面数の成功指標 (ホームから抽出の保存までが 3 画面以内) を数えて検証する。
- E2E が主要な画面と部品 (0039) のスクリーンショットを Paper と Night で取得し、`docs/design/screenshots/` との比較の結果が issue に記録されている。
- Flutter のテストのうち、値の変換、統計の期間、API の入出力、パスキーの変換の検査が native または `wasm-bindgen-test` に移り、`mise run dioxus:test` と `mise run dioxus:test-web` が成功する (API の入出力の PBT は 0038 が持つ)。
- 0040、0041、0043 の E2E の完了条件を検証し、結果をそれぞれの issue に記録する。
- ADR-0013 の Flutter の PBT (`kiri_check`) とウィジェットテストと統合テストと E2E の方式 (Chrome DevTools Protocol から ChromeDriver の WebAuthn の拡張コマンドへ) の記述を、Rust のテストの 3 層に合わせて改訂する。
- `backend/brew_book_core/src/routes.rs`、`backend/brew_book/tests/support/mod.rs`、`backend/brew_book/tests/api_suite.rs`、`backend/brew_book/tests/wrangler_purchase_suggestions_api.rs`、`backend/brew_book_core/tests/test_routes.rs` の `OkTest::Manual` と「手元で確認する」の記述を、FR-19 の正常系を staging で確認する扱い (PRD の成功指標の測定方法) に合わせる。
- `mise run check` が通過する。
- この issue では Flutter のテストとタスクを消さない (0045 で消す)。

## 関連

- 0040 から 0043 が画面を作る。
- 0045 が配信を差し替え、Flutter を消す。
- 0039 と 0042 は部品と画面のスクリーンショットを、0040、0041、0043 は E2E の完了条件を、この issue で検証する。

---
