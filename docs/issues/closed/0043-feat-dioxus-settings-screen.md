# 設定の画面を Dioxus で実装する

Created: 2026-10-02
Model: deepseek-v4p1-flash
Completed: 2026-10-02
対応 ADR: ADR-0017、ADR-0004
関連 PRD: FR-3、FR-4、FR-14、FR-15、FR-16
依存: 0038, 0039, 0040

## 背景

本 issue は、2026-10-01 の所有者の決定 (ADR-0017) から起票した。

Flutter の設定は `frontend/lib/screens/settings_screen.dart` と `frontend/lib/settings/settings_services.dart` にあり、パスキーの一覧と名前の変更と追加と削除 (最後の 1 つは削除不可)、全記録の JSON のエクスポート、アカウントと全データの削除 (確認ダイアログ)、ログアウトを持つ。
エクスポートのダウンロードは `frontend/lib/download/file_download_web.dart` が Blob とオブジェクト URL で行う。

## 目的

設定の画面で、パスキーを管理し、全記録をエクスポートし、アカウントと全データを削除できる。

## 設計判断

- エクスポートは `web-sys` の `Blob` と `Url::create_object_url` と `HTMLAnchorElement` で行う (Flutter と同じ経路)。
  オブジェクト URL は押した後に解放する (Flutter と同じ)。
- 削除の確認はデザインの `Feedback` のダイアログで行い、確認のボタンだけを `signal` の塗りにする。
  確認しないと API を呼ばないことをテストで確認する (FR-15)。
- パスキーの一覧は登録日時と最終使用日時を表示し、最後の 1 つの削除のボタンを無効にする (FR-3)。
- 採らない案: ブラウザの `confirm()` を使う (デザインに合わない)、削除の確認を別の画面で行う (画面数の成功指標に影響しないが、デザインの指示に反する)。

## 完了条件

- パスキーの一覧、名前の変更、追加、削除ができ、最後の 1 つは削除できない (native のテストで確認する)。
- エクスポートが JSON のファイルをダウンロードする (0044 の E2E で確認する)。
- アカウントの削除が確認ダイアログを経て行われ、確認しないと API を呼ばない (native のテストで確認する)。
- ログアウトができる (0044 の E2E で確認する)。
- `mise run dioxus:lint` と `mise run dioxus:test` が成功する。
- `mise run check` が通過する。

## 関連

- 0040 が認証の画面を作る (ログアウトは 0040 と共有する)。
- 0044 が E2E を用意する。

## 解決方法

設定の画面 (パスキーの管理、全記録のエクスポート、アカウントの削除、ログアウト) を実装した (ADR-0017、ADR-0004)。

- `frontend/src/auth.rs` にパスキーの型 (`Passkey` と `passkeys_from_json`) と、一覧、追加、名前の変更、削除、アカウントの削除 (`passkeys`、`add_passkey`、`rename_passkey`、`delete_passkey`、`delete_account`) を追加した。名前の検証は既存の `passkey_name_for_request` (1 文字以上 50 文字以下、前後の空白を除く) を使う。最後の 1 つの削除の 409 は `delete_passkey_error_key` が「最後の 1 つ」の文言にする。応答の形の違反は `AuthError::Format` にした。
- `frontend/src/settings.rs` にエクスポートの依存を追加した。`DownloadedFile`、`FileDownload` trait、`SettingsServices`、`export_all` を持ち、Web の実装は web-sys の `Blob` (`BlobPropertyBag` で `application/json`)、`Url::create_object_url_with_blob`、`HtmlAnchorElement` でリンクを作って押し、押した後にリンクを外してオブジェクト URL を解放する (Flutter の `BrowserFileDownload` と同じ経路)。ファイル名は Backend と同じ `brewbook-export.json`。
- `frontend/src/screens/settings.rs` に設定の画面を追加した。パスキーの行 (鍵の印、名前、登録日時と最終使用日時、名前の変更、削除)、確認ダイアログ (`ConfirmDialog` の `danger: true`) によるアカウントの削除、エクスポート、ログアウトを持つ。最後の 1 つは削除のボタンを無効にし、ハンドラでも API を呼ばない (`can_delete_passkey`)。追加と名前の変更の名前入力は `PasskeyNameDialog` で行う (Flutter の `_PasskeyNameDialog` に対応)。削除の確認は `AccountDelete` の状態機械で持ち、ブラウザの `confirm()` は使わない。
- `frontend/src/router.rs` の `/settings` を設定の画面に配線し、使われなくなった `Placeholder` と `title_key` を削除した。`frontend/src/screens/mod.rs` に設定の画面の再公開を足し、`frontend/src/app.rs` が `SettingsServices::web` を context に配る。
- `frontend/Cargo.toml` の web-sys に `BlobPropertyBag` と `HtmlAnchorElement` を追加した (新しいクレートは増やしていない)。
- `frontend/src/ui/design.css` に無効なアイコンボタンの規則 (`.iconbtn[disabled]` を `ink-faint`) を追加した (Settings の設計の指示に対応)。
- `frontend/src/lib.rs` に `pub mod settings;` を足し、クレートの説明を更新した。`frontend/src/screens/mod.rs` に設定の画面の再公開を足し、使われなくなった `Placeholder` と `title_key` を削除した (経路の配線は `router.rs`)。
- `frontend/tests/support/mod.rs` に偽のダウンロード (`FakeFileDownload`) を追加し、`frontend/tests/test_records_screens.rs` の設定経路の仮画面の検査を削除した (設定の配線の検査は `test_settings.rs` が引き継ぐ)。
- `frontend/tests/` に `test_settings.rs` (7 件)、`test_settings_web.rs` (wasm 2 件)、`test_ui_web.rs` の無効なアイコンボタンの検査 (1 件) を追加し、`test_auth.rs` にパスキーの管理の検査 (7 件) を足した。`frontend/pbt/tests/prop_settings.rs` にアカウントの削除の PBT (1 性質) を追加した。

完了条件の検証:

- パスキーの一覧、名前の変更、追加、削除ができ、最後の 1 つは削除できない: 画面の配線は `tests/test_settings_web.rs` の `the_settings_screen_gates_the_delete_actions` (1 件のパスキーでは削除のボタンが無効で API を呼ばないこと) で確認した。API と純粋関数は `tests/test_auth.rs` の `the_passkey_list_is_read_with_the_timestamps`、`a_malformed_passkey_list_is_a_format_error`、`a_successful_passkey_add_sends_the_name_and_the_credential`、`a_rename_patches_the_name_of_the_passkey`、`a_delete_sends_the_request_for_the_passkey`、`the_delete_error_uses_the_last_passkey_message_for_409` と、`tests/test_settings.rs` の `the_last_passkey_cannot_be_deleted` (`can_delete_passkey`)、`the_passkey_subtitle_shows_the_created_and_last_used_timestamps`、`the_settings_route_is_wired_to_the_settings_screen` で確認した。
- エクスポートが JSON のファイルをダウンロードする: 実装は `settings.rs` の `export_all` と `BrowserFileDownload`。native の `tests/test_settings.rs` の `the_export_is_downloaded_as_the_json_file` (GET /api/export と `brewbook-export.json` という名前で応答のバイト列をそのまま保存する) と `a_failed_export_is_not_saved` (失敗時は保存しない)、ブラウザの `tests/test_settings_web.rs` の `the_download_link_points_to_the_object_url_with_the_file_name` (リンクの href と download 属性) で確認した。実際の保存は 0044 の E2E で確認する。
- アカウントの削除が確認ダイアログを経て行われ、確認しないと API を呼ばない: 画面の配線は `tests/test_settings_web.rs` の `the_settings_screen_gates_the_delete_actions` (確認のダイアログを出すだけでは API を呼ばず、確認のボタンで `DELETE /api/account` を呼ぶこと) で確認した。`pbt/tests/prop_settings.rs` の `the_account_delete_calls_the_api_only_after_the_confirmation` (任意の操作列で API を呼ぶ回数が入口のボタンの回数を超えない)、`tests/test_auth.rs` の `an_account_delete_sends_the_request` (DELETE /api/account) で確認した。
- ログアウトができる: 0040 の `logout` を再利用し、設定の画面の末尾の文字ボタンから呼ぶ。遷移の確認は 0044 の E2E で行う。
- `mise run dioxus:lint` と `mise run dioxus:test`: 作業ツリーで成功した (lint は clippy と fmt とも警告なし、native は 230 件)。`mise run dioxus:test-web` も成功した (40 件)。`mise run dioxus:build` も成功した。
- `mise run check` が通過する: 所有者の指示により、移行の全 issue の完了後に 1 回だけ実行するため、この issue では未実行 (最終確認に委ねる)。

方針を保った実装詳細の乖離:

- 追加と名前の変更の名前入力は、方針が削除の確認だけを指定していたため、`PasskeyNameDialog` を新設した (Feedback のダイアログと同じ `.scrim` と `.dialog` の見た目で、`Field` と `Button` を使う。Flutter の `_PasskeyNameDialog` に対応)。
- パスキーの行は `ListRow` ではなく、設計のプレビュー (`docs/design/components/Settings/preview.html`) と同じ `.pk` のクラスで組んだ。
- パスキーの一覧の応答の形が規約と違う場合は、`AuthError::Format` を追加して `errorUnexpected` の文言にした (Flutter の `FormatException` に対応)。
- ダウンロードのリンクの組み立ては、ブラウザのテストから参照できるよう `download_link` として公開した (Flutter の `BrowserFileDownload.downloadLink` と同じ扱い)。
- 無効なアイコンボタンの見た目は、既存の `.iconbtn` に無効時の規則が無かったため、`design.css` に `.iconbtn[disabled]` を追加した (Settings の設計が `ink-faint` を求めているため)。
- 設計のプレビューには設定画面の AppBar に戻る操作があるが、既存の Dioxus の最上位の画面 (店、購入、統計など) は戻る操作を置いていないため、同じ形 (題だけ) にした。

レビューの指摘を受けて変えたもの (方式は変えていない):

- 無効なアイコンボタンのグリフの色を落とす規則 (`.iconbtn[disabled] .icon`) を足し、検査もグリフ (`.icon`) の計算済みスタイルを見るように直した (`.icon` が `ink` を直接指定するため、ボタンの色だけでは見た目に効いていなかった)。
- 設定の画面の配線の wasm テスト (`the_settings_screen_gates_the_delete_actions`) を足した (最後の 1 つの削除が無効で API を呼ばないこと、アカウントの削除が確認の後だけ呼ばれること)。
- パスキーの一覧の形式違反の検査が `AuthError::Format` を直接見るようにした。
- `BrowserFileDownload::save` の失敗 (文書が無い、Blob とオブジェクト URL の作成の失敗) をコンソールの警告に残すようにした (握り潰さない。画面の通知は仕様どおり「エクスポートしました」のままにする)。
- PBT と重複する単体テスト (`the_account_is_deleted_only_after_the_confirmation`) を削った (ADR-0013 の役割分担。画面の配線は上記の wasm テストが担う)。

レビューの指摘のうち、記録として残すもの:

- エクスポートの `save` の内部 (Blob の種別、応答のバイト列、オブジェクト URL の解放) はブラウザテストで確認していない (`download_link` の href と download 属性まで)。実際の保存は 0044 の E2E が確認する。
- `docs/issues/0045-chore-switch-delivery-and-remove-flutter.md` の追記 (0036 を 0045 で close する所有者の決定) は作業ツリーにあるが、0045 の変更として扱い、この issue のコミットには含めない。

## 0044 の E2E の検証 (2026-10-02)

0044 の E2E (`mise run frontend:test-same-origin`) が、この issue の E2E の完了条件を検証した。

- エクスポートのダウンロード: 設定の画面からエクスポートを実行し、JSON のファイル (`brewbook-export.json`) が一時ディレクトリへ保存され、中身が確認できることを確かめた。
- ログアウト: 設定の画面のログアウトでログイン画面へ遷移することを確かめた。
