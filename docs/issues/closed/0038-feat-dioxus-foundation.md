# Dioxus の Frontend 基盤を作る

Created: 2026-10-02
Model: deepseek-v4p1-flash
Completed: 2026-10-02
対応 ADR: ADR-0017、ADR-0005 (docs/adr/0005-same-origin-deployment.md)、ADR-0013 (docs/adr/0013-five-test-layers.md)
関連 PRD: FR-16、制約と前提 (テスト)、非機能要求 運用
依存: 0037

## 背景

本 issue は、2026-10-01 の所有者の決定 (ADR-0017) から起票した。

Flutter の基盤は次のファイルにある。
`frontend/lib/router/app_router.dart` は 18 経路の台帳 (`AppRoutes`) を持ち、経路の名前を経路のパターンと同じにして、画面数の成功指標を数えられるようにしている。
`frontend/lib/api/api_client.dart` は同一オリジンの `/api` を呼び、エラーの JSON を `ApiError` に変換し、401 で `onUnauthorized` を呼び、応答を取得できない失敗を `NetworkError` にする。
`frontend/lib/records/values.dart` と `frontend/lib/records/stats_period.dart` は値の変換と統計の期間の組み立てを持つ。
`frontend/lib/l10n/app_ja.arb` と `app_en.arb` は 227 キーの翻訳を持つ (複数形と選択は無い。2026-10-02 に確認)。
`frontend/test/l10n_check_test.dart` は UI のコードに文言を直書きしていないことと、日本語の 8 つの訳語を検査する (FR-16)。

## 目的

Dioxus のアプリの基盤 (クレートの構成、経路の台帳、API クライアント、値の変換、翻訳、起動時のセッション確認) が動き、画面の実装を載せられる状態にする。

## 設計判断

- クレートは 0037 の `frontend/` のワークスペースの `brew_book_frontend` を使う。
  Dioxus の依存は `dioxus` (web の feature) と `dioxus-router` に限定し、状態管理のライブラリは追加しない (ADR-0007 の「依存を最小にする」方針を ADR-0017 が引き継ぐ)。
  HTTP は `gloo-net` と `web-sys` の `fetch` を比較し、依存の少ない方を選ぶ (実装の最初に決めて記録する)。
- 経路の台帳は `frontend/src/router.rs` の 1 か所に置き、経路の名前は経路のパターンと同じにする。
  未知の経路はホームへ戻す (Dioxus のルーターの fallback として定める。Flutter は go_router の既定のエラー画面で、同じ挙動ではないため、意図的な変更として記録する)。
- 起動時のセッション確認は `GET /api/passkeys` の 401 で判定する (Flutter と同じ。専用のセッション確認 API は設けない)。
- 翻訳は型付きのキーの列挙と日本語と英語の定数の表で持つ。
  `t(Key::SaveButton)` の形にして、表示する文字列を直接書けないようにする。
  ARB と ICU の複数形は使わない (227 キーに複数形と選択は無い)。
  キーの列挙と 2 つの言語の表の対応 (全件) をテストで照合する。
- 文言の直書きの静的検査は native のテスト (`frontend/tests/test_i18n.rs`) として書き、`frontend/src/ui/` と `frontend/src/screens/` の `.rs` の `rsx!` で、次の位置の文字列リテラルを検出する。対象は (1) 要素の本文、(2) `placeholder`、`title`、`aria-label`、`alt` の属性の値、(3) デザインの部品 10 種の `label` と `title` の prop の値とする。
  対象外は、上記以外の属性の値 (Tailwind の `class`、`type`、`id`、`name`、`for`、`href`、`src`、`rel`、`role`、`value`、`kind`、`accept`、`autocomplete`、`inputmode`、`pattern`、`min`、`max`、`step`、`maxlength`、`method`、`action`、`target`)、`t(` の呼び出し、ログ、テストコード、翻訳の表、URL とする (Flutter の `l10n_check_test.dart` の検査を Rust に写す。FR-16)。
- 値の変換と統計の期間は Dioxus に依存しない純粋なモジュールとして写し、native の単体テストと `proptest` の PBT で守る (ADR-0013)。
  これが移行の品質の中心になる。
- 記録の依存 (API クライアント、時計、写真の選択と変換) を束ねた型を用意し、画面に配る (Flutter の `RecordServices` と同じ。0041 が使う)。
- API の入力と応答の対応 (要求の JSON と応答の JSON の往復) の PBT を native のテストとして持つ (ADR-0013)。
- 翻訳のキーの列挙は、日本語の 8 つの訳語 (生産者、生産国、地域、精製方法、品種、焙煎度、焙煎日、フレーバーノート) を値として持ち、テストで照合する (FR-16)。
- 採らない案: `dioxus-i18n` クレートを使う (227 キーの単純な表には過剰で、型で直書きを防げない)、`fluent` を使う (複数形が無いため不要)、`reqwest` を wasm で使う (依存が大きい)、状態管理のクレートを入れる (ADR-0007 の決定を維持する)。

## 完了条件

- `mise run dioxus:build` が基盤のアプリをビルドし、`mise run dioxus:lint` が警告なしで成功する。
- 経路の台帳に Flutter と同じ 18 経路がある (native のテストで経路の一覧を照合する)。
- API クライアントが `/api` の下を呼び、エラーの JSON を共通の型に変換し、401 でログイン画面へ遷移させ、ネットワークエラーで再試行を促す (native のテストで確認する)。
- 翻訳が日本語と英語であり、端末またはブラウザの言語が日本語なら日本語、それ以外なら英語になる (native のテストで確認する)。
- 日本語の翻訳ファイルに 8 つの訳語が値として存在する (native のテストで確認する)。
- `frontend/src/ui/` と `frontend/src/screens/` の UI コードの `rsx!` に、要素の本文と表示に使う属性の値に文字列リテラルの直書きが無い (native のテストで確認する)。
- 値の変換と統計の期間の単体テストと PBT がある。
- API の入力と応答の対応の PBT がある。
- 記録の依存を束ねた型があり、画面に配れる (native のテストで確認する)。
- `mise run check` が通過する。

## 関連

- 0037 がツールチェーンとタスクを作る。
- 0039 がデザインシステムを、0040 以降が画面を作る。

## 解決方法

Dioxus の Frontend の基盤 (クレートの構成、経路の台帳、API クライアント、起動時のセッション確認、翻訳、値の変換、記録の依存の束ね) を作った (ADR-0017)。

- `frontend/src/lib.rs` を追加し、`frontend/src/main.rs` を書き換えた。`main.rs` は native では空の `main` とし (Web だけを対象にする。native はテストと lint のためにコンパイルする)、wasm では `dioxus::launch(brew_book_frontend::app::App)` を呼ぶ。
- `frontend/src/router.rs` に 18 経路の台帳 (`APP_ROUTES`) を置いた。経路の名前は経路のパターンと同じにする。未知の経路は Dioxus のルーターの fallback (`Route::NotFound`) でホームへ戻す (Flutter の go_router の既定のエラー画面とは挙動が違う意図的な変更。`tests/test_router.rs` の `an_unknown_path_falls_back_to_the_home` で確認する)。
- `frontend/src/api/` に API クライアントを追加した。`ApiClient::DEFAULT_BASE_PATH = "/api"`、エラーの JSON を `ApiError` に変換し、401 で `set_on_unauthorized` の callback を呼び、接続の失敗と応答形式の違反を `NetworkError` にする。`ApiCallError::retry_keys` が再試行を促す案内のキー (`Key::ErrorNetwork` と `Key::RetryButton`) を返す (応答を取得できなかった失敗のときだけ。画面のバナーは 0040 以降が組む)。HTTP は `web-sys` の `fetch` を選んだ (`gloo-net` は `gloo-utils`、`http`、`thiserror 1` などを引き込むが、`fetch` は dioxus の web のビルドが既に使う `web-sys`、`js-sys`、`wasm-bindgen-futures`、`wasm-bindgen` だけで済む。実装は `frontend/src/api/fetch.rs`)。
- `frontend/src/auth.rs` に起動時のセッション確認 (`check_session` と `SessionStatus`) を追加した。`GET /api/passkeys` の 401 で `SignedOut`、成功で `SignedIn`、401 以外の失敗とネットワークエラーで `Unknown` にする。
- `frontend/src/app.rs` の `AppShell` が 401 で `unauthorized_route()` (`Route::Login`) へ遷移させる (遷移先はテストで固定するために切り出した)。
- `frontend/src/i18n/` に 227 キーの型付きの列挙 (`keys.rs`) と日本語 (`ja.rs`) と英語 (`en.rs`) の表を追加した。`t(Key::...)` と `t_args` (単純な差し込み) で引く。端末またはブラウザの言語が日本語 (`ja`、`ja-JP`、`ja_JP`、`JA`) なら日本語、それ以外は英語にする。
- `frontend/src/records/` に値の変換 (`values.rs`)、統計の期間 (`stats_period.rs`)、時計 (`clock.rs`)、写真の選択と変換 (`photo.rs`) を追加し、`records/mod.rs` の `RecordServices` で束ねた (Flutter の `RecordServices` と同じ。画面の prop に渡せることをテストで確認する)。
- `frontend/src/ui/mod.rs` と `frontend/src/screens/mod.rs` を置いた (中身の部品と画面は 0039 以降)。
- `frontend/pbt/` をワークスペースの成員として追加し、値の変換、統計の期間、API の入力と応答の対応の PBT を置いた (`proptest`。ADR-0013)。
- `frontend/tests/` に native のテスト (API クライアント、セッション確認、翻訳、値の変換、統計の期間、経路、記録の依存) と、UI コードの文言の直書きの検査 (`test_i18n.rs`。`src/ui/` と `src/screens/` の `rsx!` の本文と表示に使う属性の値と部品の `label` と `title` の prop を検査する。FR-16) を追加した。

完了条件の検証:

- `mise run dioxus:build` が基盤のアプリをビルドし、`mise run dioxus:lint` が警告なしで成功する: 作業ツリーで両方成功した (lint は clippy の `-D warnings` と `cargo fmt --check`)。
- 経路の台帳に Flutter と同じ 18 経路がある: `tests/test_router.rs` の `the_ledger_has_the_same_18_routes_as_flutter`、`a_route_name_is_the_same_as_its_pattern`、`the_patterns_are_unique`、`every_static_route_is_registered_in_the_router`、`a_dynamic_route_is_parsed_with_its_value`、`the_register_route_takes_the_token_from_the_query`、`an_unknown_path_falls_back_to_the_home` で確認した。
- API クライアントの動作: `tests/test_api_client.rs` の `a_get_calls_under_the_api_base_path`、`an_error_response_becomes_the_common_error`、`a_401_calls_the_unauthorized_callback`、`a_failed_connection_becomes_a_network_error`、`a_success_body_that_is_not_a_json_object_becomes_a_network_error`、`a_network_error_offers_the_retry_notice` で確認した。401 の遷移先は `tests/test_auth.rs` の `the_unauthorized_destination_is_the_login` が `unauthorized_route()` を固定し、`AppShell` はその関数で遷移する (この配線の実地の遷移は 0044 の E2E で確認する)。
- 翻訳: `tests/test_i18n.rs` の `the_language_is_japanese_only_for_japanese`、`the_key_enum_and_both_tables_cover_all_keys` (227 キー)、`the_tables_match_the_arb_values` (ARB の原本と値と順序を全件で照合する) で確認した。
- 日本語の翻訳に 8 つの訳語: `tests/test_i18n.rs` の `the_japanese_table_has_the_eight_words` で確認した。
- UI コードに文言の直書きが無い: `tests/test_i18n.rs` の `the_ui_code_does_not_hard_code_displayed_text` で確認した (検査の検出器自体も `the_check_finds_every_displayed_position` と `the_check_ignores_the_other_attributes_the_urls_and_the_interpolations` で固定した)。
- 値の変換と統計の期間の単体テストと PBT: `tests/test_values.rs` (17 件)、`tests/test_stats_period.rs` (8 件)、`pbt/tests/prop_values.rs` (7 件)、`pbt/tests/prop_stats_period.rs` (6 件。直近の月数の一般の性質と、0 年より前を指さない境界の性質)。
- API の入力と応答の対応の PBT: `pbt/tests/prop_api.rs` の `a_json_body_round_trips_through_the_request_and_the_response`。
- 記録の依存を束ねた型: `tests/test_records.rs` の 3 件 (`the_services_bundle_the_record_dependencies`、`the_services_can_be_cloned_and_compared_for_a_screen`、`a_screen_prop_can_take_the_services`)。
- ブラウザの API を使うコード: `frontend/tests/test_web.rs` の `the_browser_language_is_readable` (ブラウザの言語が読めることと、日本語か英語に解決することを検査する) と `the_device_clock_returns_a_plausible_local_time` (wasm-bindgen-test。headless Chrome で 3 件成功)。`fetch` の往復は 0044 の E2E (API の入出力の検査の移行) で確認し、写真の選択と変換は 0041 が wasm-bindgen-test を足す。
- `mise run check` が通過する: 所有者の指示により、移行の全 issue の完了後に 1 回だけ実行するため、この issue では未実行 (最終確認に委ねる)。

方針を保った実装詳細の乖離:

1. `serde_json` を `[dependencies]` にも追加した (dev-dependency と backend で既に使うクレート。API の要求と応答の JSON の符号化と復号に必要)。
2. `web-sys`、`js-sys`、`wasm-bindgen-futures`、`wasm-bindgen` を `[target.'cfg(target_arch = "wasm32")'.dependencies]` に置いた (native のテストと lint にブラウザの依存を持ち込まない)。
3. `proptest` を pbt の target 別 dev-dependency にし、pbt のテストに `#![cfg(not(target_arch = "wasm32"))]` を付けた (pbt をワークスペースの成員にしたまま `dioxus:test-web` の `--workspace` を通すため)。
4. `frontend/src/main.rs` は native では空の main にした (Web だけを対象にする ADR-0017)。
5. `RecordServices` に Flutter の記録と統計の API の薄いラッパーと revision の通知は入れていない。方針が名指しする依存 (API クライアント、時計、写真の選択と変換) は全て束ねており、画面の必要が分かる 0041 が足す。
6. 写真の選択と変換は Web の実装まで書いた (trait だけでは App を組み立てられないため)。ブラウザの API に依存するため native のテストは無く、0041 が wasm-bindgen-test を足す。JPEG の品質は Flutter の実装と同じ 0.92 にする (`to_blob_with_type_and_encoder_options` の第 3 引数に数値で渡す。`frontend/lib/photo/image_converter_web.dart` の `jpegQuality`)。
7. 翻訳の `{name}` の差し込みに `t_args` を用意した (複数形と選択は無いが、単純な差し込みは 15 キーあるため)。
8. `display_day` / `display_timestamp` の表示形式は日本語 `2026/10/2`、英語 `10/2/2026`、時刻は 24 時間の `HH:MM` とした (Flutter の `DateFormat.yMd` (`frontend/lib/records/values.dart`) に合わせる)。

レビューの指摘を受けて変えたもの (方式は変えていない):

- `ApiCallError::retry_keys` とそのテストを足した (再試行を促す案内の対応付け)。
- `unauthorized_route()` を切り出し、遷移先をテストで固定した (実地の遷移は 0044 の E2E で確認する)。
- `parse_decimal` が f64 で表せない桁数の入力を拒否するようにした (Backend の `validate_decimal` と同じ判定)。
- `recent_months` が 0 年より前になるときは 0 年 1 月で止めるようにした (負の年は値の範囲 (0 年から 9999 年) と API の日付の形式の外になるため)。
- 文言の直書きの検査の本文の判定を直し、子要素の後、別の本文の後、式ブロックの後の本文も検出するようにした (自己テストに 3 例を追加)。
- ARB の原本との全件の照合 (`the_tables_match_the_arb_values`) を足した。キーの順序は ARB と同じにする (`keys.rs`) ため、順序どおりに比べて値の入れ替えも検出する。
- ブラウザの API を使うコードの wasm テストを足した (`the_browser_language_is_readable` は言語が読めることまで検査し、`the_device_clock_returns_a_plausible_local_time` は値の範囲を検査する)。
- PBT で実現できる性質の単体テストを削った (`test_stats_period.rs` の `the_current_month_starts_at_the_first_day`、`test_values.rs` の `a_number_is_formatted_without_a_fraction_for_integers` と時刻の往復の一部)。前後の空白を除く受理の規則は性質ではないため単体テストに残した (`a_time_with_surrounding_spaces_is_accepted`)。
- 統計の期間の PBT を分けた。直近の月数の一般の性質は 1 年以上の入力で検査し、0 年より前を指さないことと開始が終了より後にならないことは 0 年を含む入力の別の性質で検査する (実装の丸めと矛盾しない)。期待値は実装の式を写さず、開始の月から当月までの月数が指定の月数と一致する性質で検査する。
- JPEG の品質を Flutter の実装と同じ 0.92 にした (`to_blob_with_type_and_encoder_options` の第 3 引数に数値で渡す。`frontend/lib/photo/image_converter_web.dart` の `jpegQuality` と同じ値)。0 年より前の丸めの単体テストの入力を、月が 1 以外の値 (0 年 2 月の直近 3 か月) にして、現在の月ではなく 0 年 1 月で止まることを固定した。
- `values.rs` の doc comment の行末の余分な `///` を直した。
