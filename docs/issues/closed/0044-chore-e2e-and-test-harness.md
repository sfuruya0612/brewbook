# E2E とテストの基盤を Dioxus に差し替える

Created: 2026-10-02
Model: deepseek-v4p1-flash
Completed: 2026-10-02
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

## 解決方法

E2E とテストの基盤を Dioxus に差し替えた (ADR-0017、ADR-0013)。

- `backend/brew_book/tests/support/e2e.rs` に WebDriver のハーネスを追加した。`thirtyfour` 0.37 を選び (`fantoccini` と比較し、CDP を送れること (`Emulation.setEmulatedMedia` でテーマを切り替える) と、要素と画面のスクリーンショットの API を持つことから)、WebAuthn の仮想認証器の拡張コマンドは型が無いため同じセッションへ `reqwest` で直接送る。仮想認証器は `WebAuthn.addVirtualAuthenticator` で `protocol: ctap2`、`transport: internal`、`hasResidentKey: true`、`hasUserVerification: true`、`isUserVerified: true`、`automaticPresenceSimulation: true` を設定して付ける (Backend が `userVerification: required` を要求するため既定値に依存しない)。画面の比較は Chrome の非同期のスクリプトで 2 つの画像を縮小して差の平均と最大と差のある画素の割合を出す。
- `backend/brew_book/Cargo.toml` に `thirtyfour` (既定の feature の `manager` は使わない) と `tokio` を開発の依存に足し、`reqwest` に `json` の feature を足した。`backend/Cargo.lock` が追随した。
- `backend/brew_book/tests/wrangler_same_origin_e2e.rs` を Dioxus の E2E に差し替えた。`wrangler dev` と chromedriver を起動し、テスト用の feature `e2e` を有効にしたビルドを Static Assets として配信し、登録 (FR-1)、ログイン (FR-2)、ホームのメニューからのログアウトと 401 の遷移 (FR-2、FR-4)、設定の画面からのログアウト (FR-4、0043)、抽出の保存 (FR-11)、エクスポートのダウンロード (FR-14)、画面数の成功指標、主要な画面と部品のスクリーンショットの比較 (0039) を検証する。Flutter の成果物 (`build/e2e-web`、`main.dart.js`) の検証は Dioxus の成果物に合わせた (wasm の全てに `e2e` のマーカーがあることの静的検査と、`assets/*.js` のローダーの存在の静的検査、起動時の `data-e2e` の印の実行中の検査)。
- 画面数の成功指標は、アプリが現在の経路を `#main` の `data-route` 属性に出し、E2E が経路の種類を数えて 3 以下であることを検証する。
- スクリーンショットの比較は `frontend/target/e2e-screenshots/comparison.json` に記録する。画面 12 種 x 2 テーマ (24 件)、部品 10 種 x 2 テーマとスナックバー (21 件)、同じ画面の Paper と Night の差 (11 件)、2 段組 (2 件) の 58 件である。部品とログインの主ボタンの要素が見つからないときは失敗させ、必須の件数を下回るときも失敗させる (無言で比較が減らないようにする)。失敗の経路でもブラウザを閉じる。
- `frontend/src/app.rs` に `data-route` の設定を、`frontend/src/lib.rs` と `frontend/src/main.rs` に `e2e` の feature の印を追加した。`frontend/src/router.rs` に経路の名前を引く `route_name` を足した。`frontend/tests/test_build.rs` と `frontend/tests/test_router.rs` を追随させた。
- `frontend/index.html` の favicon と `manifest.json` と `tailwind.css` の参照を絶対パスに直した。相対パスのままだと、`/brews/new` などの経路を直接開いたときに `/brews/tailwind.css` を読んでスタイルが外れる (E2E のスクリーンショットの比較で見つけた)。
- `frontend/src/ui/design.css` の `.icon` の色を周囲の文字の色の継承に直した (原本の svg の `stroke: currentColor` と同じ)。塗りボタンと FAB のアイコンが `ink` のまま暗くなる差を、スクリーンショットの比較で見つけた。
- `mise.toml` の `frontend:test-same-origin` を Dioxus の E2E に差し替えた (依存は `dioxus:build-e2e` と `backend:build`)。`dioxus:build-e2e` は古い成果物が混ざらないように出力を消してから作る。Flutter のテストとタスクは消していない (0045 が消す)。
- `backend/brew_book_core/src/routes.rs` と `backend/brew_book_core/tests/test_routes.rs`、`backend/brew_book/tests/api_suite.rs`、`backend/brew_book/tests/wrangler_purchase_suggestions_api.rs`、`backend/brew_book/tests/support/mod.rs` の `OkTest::Manual` を `OkTest::Staging` に改名し、「手元で確認する」の記述を「staging へのデプロイで実写真を送って確認する」に合わせた (FR-19 の正常系を staging で確認する扱い。PRD の成功指標の測定方法)。
- `backend/brew_book/tests/support/mod.rs` に `pub mod e2e;` を足した (E2E のテストが Dioxus の成果物のパスを明示して渡す。既定の `DEFAULT_ASSETS_DIR` は 0045 まで Flutter のビルドのままで、`wrangler_same_origin_api.rs` は Flutter の成果物を検証し続ける)。
- `docs/adr/0013-five-test-layers.md` を改訂した (Flutter の PBT (`kiri_check`) とウィジェットテストと統合テストと E2E の方式 (Chrome DevTools Protocol から ChromeDriver の WebAuthn の拡張コマンドへ) の記述を、Rust のテストの 3 層に合わせた。「改訂: 2026-10-02」を追記)。

完了条件の検証:

- `mise run frontend:test-same-origin` が Dioxus のビルドを配信し、登録、ログイン、ログアウト、抽出の保存、エクスポートのダウンロードの E2E を WebDriver で実行して成功する: 作業ツリーで実行し、`test wrangler_web_routes_registration_login_and_brew_save_ok ... ok` (1 passed。全体で 349 秒) で成功した。写真のアップロードは `backend/brew_book/.dev.vars` がこの環境に無いため対象外とした (issue の記載どおり。ハーネスにアップロードの手順は未実装で、資格情報がある環境でもこの issue では行われない)。
- `backend/brew_book/tests/wrangler_same_origin_e2e.rs` の Flutter の成果物の検証を Dioxus の成果物に合わせる: wasm の全ての `e2e` のマーカーと `assets/*.js` の存在を静的検査し、起動時の `data-e2e` の印を実行中に確認する。
- E2E が画面数の成功指標 (ホームから抽出の保存までが 3 画面以内) を数えて検証する: `data-route` で記録した経路の種類が 3 以下であることを検証し、`/` と `/brews/new` を含むことを確認した (成功)。
- E2E が主要な画面と部品 (0039) のスクリーンショットを Paper と Night で取得し、`docs/design/screenshots/` との比較の結果が issue に記録されている: 上記の 58 件を記録した。差分の判断は下記の「スクリーンショットの比較の判断」に書く。
- Flutter のテストのうち、値の変換、統計の期間、API の入出力、パスキーの変換の検査が native または `wasm-bindgen-test` に移り、`mise run dioxus:test` と `mise run dioxus:test-web` が成功する: 作業ツリーで `mise run dioxus:test` (native 231 件) と `mise run dioxus:test-web` (40 件) が成功した。値の変換は `tests/test_values.rs` と `pbt/tests/prop_values.rs`、統計の期間は `tests/test_stats_period.rs` と `pbt/tests/prop_stats_period.rs`、API の入出力は `pbt/tests/prop_api.rs` と `tests/test_records_api.rs` ほか、パスキーの変換は `tests/test_passkey_web.rs` (wasm) にある。
- 0040、0041、0043 の E2E の完了条件を検証し、結果をそれぞれの issue に記録する: E2E で確認できた範囲をそれぞれの closed の issue に追記した (登録とログインとログアウトと 401 の遷移、抽出の保存と画面数とフォームの検証のバナー、設定の画面のログアウト、エクスポートのダウンロード。写真のアップロードは資格情報が無いため対象外)。
- ADR-0013 の改訂: 上記のとおり。
- `backend/brew_book_core/src/routes.rs` ほかの `OkTest::Manual` の記述: 上記のとおり。
- `mise run check` が通過する: 所有者の指示により、移行の全 issue の完了後に 1 回だけ実行するため、この issue では未実行 (最終確認に委ねる)。

スクリーンショットの比較の判断 (58 件):

- 比較は、スクリーンショットと原本を共通の大きさに縮小して画素の差を見る。画面 (24 件) は差のある画素の割合が 0.059 から 0.376 で、レイアウトと色は原本と一致する。最大は設定の画面の paper の 0.376 で、これは E2E が登録したパスキーの名前と日時 (実データ) と、原本のサンプルのデータが違うためである (画面の構成は一致する)。ほかの差は、E2E が携帯の幅 (390x844) と英語 (`--lang=en-US`。文言で操作するため) で動かすことと、原本が日本語のデスクトップのプレビューであることによる (意図的な差として許容する)。
- 部品 (21 件) は 0.065 から 0.863。割合が高いもの (Button 0.86、Rating 0.55) は、部品の要素だけの切り出しと、余白のあるプレビュー全体を縮小して比べるため、周囲の地の差が支配する。これは比較の方法の限界であり、部品そのものの差ではない (Ledger 0.065、ReferenceTile 0.084 のように小さいものもある)。
- Paper と Night の差 (11 件) は 0.955 から 0.997 で、2 つのテーマが実際に切り替わっていることを確認した。2 段組 (2 件) は 0.127 と 0.13 である。
- 直すと判断した差 (この issue で直した): (1) 塗りボタン (primary と danger) と FAB のアイコンが `ink` のままで暗く、原本の `stroke: currentColor` (ボタンの文字の色) と違っていた。`.icon` の色を継承に直し、E2E を回し直して原本と同じ明るさになることを確認した。 (2) `frontend/index.html` の静的アセットの参照が相対パスで、`/brews/new` などを直接開くとスタイルが外れていた (スクリーンショットの比較の前提に関わる)。絶対パスに直した。 (3) ホームのメニューのログアウトは検証していたが、設定の画面のログアウトの入口が未検証だった。E2E に設定の画面からのログアウトを足した。
- 許容すると判断した差: 言語 (英語と日本語)、画面の幅 (携帯とデスクトップ)、実データとサンプルのデータ、部品の切り出しとプレビュー全体の比較による差、フォントの描画の差。いずれも実装の欠陥ではない。

方針を保った実装詳細の乖離:

- `thirtyfour` の既定の feature (`manager`。chromedriver の自動取得と起動) は使わず、chromedriver はハーネスが起動する (mise の固定した版を使うため)。
- WebAuthn の拡張コマンドは `thirtyfour` に型が無いため、`reqwest` で同じセッションへ直接送る。
- スクリーンショットの比較は Chrome の非同期のスクリプトで行う (画像のクレートを足さない)。
- 画面数の経路は `data-route` 属性で出す (PRD の測定方法を維持する)。E2E の実行の印は `html` の `data-e2e` 属性で別に出す。
- `dioxus:build-e2e` が出力を消してから作るのは、`dx` が古い成果物を残し、配布用のビルドと混ざるためである (E2E が `e2e` の印を全ての wasm に要求する)。
- スクリーンショットの比較の閾値は Chrome のスクリプトに直書きし、Rust 側に未使用の定数を置かない。

対象外とした完了条件:

- 写真のアップロードの E2E (完了条件 1 の後段): `backend/brew_book/.dev.vars` がこの環境に無いため未実施 (issue の「ない環境では対象外として記録する」に従う。資格情報がある環境向けの手順も未実装である)。
- `mise run check` の通過 (完了条件の最終行): 所有者の指示により、移行の全 issue の完了後に 1 回だけ実行する。

作業ツリーの扱い:

- `docs/issues/0045-chore-switch-delivery-and-remove-flutter.md` の追記 (0036 を 0045 で close する所有者の決定) は作業ツリーにあるが、0045 の変更として扱い、この issue のコミットには含めない。
