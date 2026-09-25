# 設定の画面 (パスキー管理、エクスポート、アカウント削除) を作る

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-25
対応 ADR: ADR-0007 (docs/adr/0007-frontend-flutter-web-first.md)
関連 PRD: FR-3、FR-4、FR-14、FR-15
依存: 0005, 0011, 0012, 0013

## 背景

PRD の FR-3 は、ログイン済みの利用者がパスキーを追加し、名前を変更し、2 つ以上あるときは削除できることを求める。
FR-14 は全記録のエクスポートを、FR-15 は確認ダイアログを経たアカウントと全データの削除を求める。
ADR-0007 は、これらの画面を Flutter で作ると決めた。
0005 でパスキー管理の API が、0011 でエクスポートが、0012 でアカウント削除ができた。本 issue が設定の画面を追加する。

## 目的

利用者が、パスキーを管理でき、全記録をエクスポートでき、アカウントと全データを自分で削除できるようにする。

## 設計判断

- 設定の画面は 1 つとし、パスキーの管理、エクスポート、アカウントの削除、ログアウトを置く。
  採らない案: 機能ごとに画面を分ける (3 機能はどれも操作が少なく、経路を増やすと画面数の管理が煩雑になる)。
- パスキーの管理は、一覧 (名前、登録日時、最終使用日時)、追加、名前の変更、削除を置く。
  利用者のパスキーが 1 つしかないときは削除の操作を無効にし、サーバーの 409 も表示する。
- エクスポートは `GET /api/export` を呼び、応答の JSON をファイルとしてダウンロードする。
  認証は Cookie で行い、応答を Blob にしてダウンロードのリンクを作る。
  採らない案: 写真の実体を含める (PRD の FR-14 と ADR-0003 に反する)、署名付き GET URL を発行する (ADR-0003 が却下した)。
- アカウント削除は、確認ダイアログで明示的な確認ボタンを押させ、押した場合だけ `DELETE /api/account` を呼ぶ。
  確認ボタンを押さずに削除 API が呼ばれないことをウィジェットテストで検証する (FR-15)。
  削除の後は保持している状態を消してログイン画面へ遷移する。
- ログアウトは `POST /api/auth/logout` を呼び、状態を消してログイン画面へ遷移する。

## 完了条件

- FR-3 の画面の受け入れ基準を満たす。
  パスキーの一覧 (名前、登録日時、最終使用日時)、追加、名前の変更、削除がウィジェットテストで確認でき、最後の 1 つは削除できず 409 の表示になる。
- FR-14 の画面の受け入れ基準を満たす。
  エクスポートの JSON がファイルとしてダウンロードされることをテストで確認する。
- FR-15 の画面の受け入れ基準を満たす。
  確認ボタンを押さずに削除 API が呼ばれないことをウィジェットテストで確認する。
  削除後に状態が消えてログイン画面へ遷移することを確認する。
- ログアウト後にログイン画面へ遷移することを確認する。
- `mise run check` が通過する。

## 関連

- 0005 がパスキー管理の API を作る。
- 0011 がエクスポートを作る。
- 0012 がアカウント削除を作る。
- 0013 がアプリの基盤を作る。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. `createAppRouter` に 3 つ目の位置引数 `SettingsServices` を追加した (呼び出し元は `frontend/lib/app.dart` のみで、既存の呼び出しに影響しないため)。
2. `frontend/lib/api/api_client.dart` の `_send` を `http.Response` を返す形に変え、`getJson` などの JSON の経路は `_decodeJson` を通す形にした (`getBytes` と同じ応答の扱いを共有し、エラーの解釈を 1 か所に保つため。外部から見た挙動は不変)。
3. エクスポートのファイル名はフロント側の定数 `SettingsServices.exportFileName` とした (Backend の `Content-Disposition` と同じ値。単一の内部クライアントのため応答ヘッダの解釈は行わない)。
4. パスキーの管理とアカウント削除は `AuthController` に置いた (セッション状態の変更を 1 か所に集約するため)。
5. パスキー名の検証は `frontend/lib/auth/passkey_name.dart` に抽出し、register 画面と設定の画面で共用した (完了条件の外の変更。register 画面の挙動は不変で、既存のテストも通過。検証の規則を 1 か所に保つため)。
6. ホームのログアウトボタンは残した (ホームからのログアウトを消すことは方針に根拠の無い利用者に見える変更のため)。設定の画面には方針どおりログアウトを置いた。
7. 設定の画面へはホームの AppBar のメニューから開く (他の一覧画面にメニューが無い既存の作りに合わせた)。

## 解決方法

設定の画面を追加した。パスキーの管理 (FR-3)、全記録のエクスポート (FR-14)、アカウントと全データの削除 (FR-15)、ログアウトを 1 つの画面に置く。

- `frontend/lib/screens/settings_screen.dart` を新設した (パスキーの一覧と追加と名前の変更と削除、エクスポートの保存、アカウント削除の確認ダイアログ、ログアウト)。`frontend/lib/router/app_router.dart` に `/settings` を、`frontend/lib/screens/home_screen.dart` のメニューに「設定」の項目を追加した。
- `frontend/lib/auth/passkey.dart` を新設した (パスキーの型と `GET /api/passkeys` の応答の解釈)。`frontend/lib/auth/passkey_name.dart` を新設し、名前の検証を register 画面と設定の画面で共用した (register 画面の挙動は不変)。`frontend/lib/auth/auth_controller.dart` に `passkeys`、`addPasskey`、`renamePasskey`、`deletePasskey`、`deleteAccount` を追加した。
- `frontend/lib/download/file_download.dart`、`file_download_stub.dart`、`file_download_web.dart` を新設した (FR-14 の保存。Web は Blob とダウンロードのリンク、それ以外は未対応の例外)。`frontend/lib/settings/settings_services.dart` を新設し、`GET /api/export` の取得と保存を束ねた。`frontend/lib/app.dart` と `frontend/lib/router/app_router.dart` に `SettingsServices` を配線した。
- `frontend/lib/api/api_client.dart` に `getBytes` (応答のバイト列) を追加し、`_send` が `http.Response` を返して `getJson` などの JSON の経路が `_decodeJson` を通る形に変えた (エラーの解釈を 1 か所に保つため。外部から見た挙動は不変)。`frontend/lib/widgets/error_message.dart` に最後の 1 つのパスキーを削除できないときの応答の解釈 (`deletePasskeyErrorMessage`) を追加し、`frontend/lib/l10n` の ARB にその文言 (`passkeyLastDeleteError`) を追加した。
- `frontend/lib/l10n/app_ja.arb`、`app_en.arb` と生成された `app_localizations*.dart` に設定画面の文言を追加した。
- テストは `frontend/test/settings_screen_test.dart` (14 件) と `frontend/test/support/fake_file_download.dart`、`frontend/test/web/export_download_test.dart` (3 件) を追加し、`frontend/test/api_client_test.dart` に `getBytes` のテストを追加した。`frontend/test/support/pump_app.dart` に `fileDownload` を追加した。

完了条件の検証:

- FR-3 の画面の分: `settings_screen_test.dart` の「パスキーの一覧に名前と登録日時と最終使用日時を表示する」が一覧と未使用の表示を、「パスキーを追加する」が `begin` → `PasskeyClient.createCredential` → `complete` と名前の前後の空白の除去と一覧の読み直しを、「パスキーの名前を変更する」が `PATCH` を、「パスキーを削除する」が `DELETE` を、「パスキーが 1 つしかないときは削除を無効にする」が削除の操作の無効化を、「サーバーが 409 を返したら最後の 1 つを消せないことを表示する」が 409 の表示を確認した。
- FR-14 の画面の分: `settings_screen_test.dart` の「エクスポートの JSON をファイルとしてダウンロードする」が `GET /api/export` の応答のバイト列とファイル名 `coffee-log-export.json` を保存の実装に渡すことを、`export_download_test.dart` の 3 件が Blob の種別と中身、オブジェクト URL の解放、リンクの `download` 属性を確認した。失敗時は「エクスポートの失敗は再試行を促す表示にする」が確認した。
- FR-15 の画面の分: 「確認のボタンを押さなければアカウント削除の API を呼ばない」がキャンセルとダイアログの外を押した場合の両方で `DELETE /api/account` を呼ばないことを、「確認を押すとアカウントを削除し、ログイン画面へ遷移する」が削除の呼び出しとログイン画面への遷移を確認した。
- ログアウト: 「ログアウトするとログイン画面へ遷移する」が `POST /api/auth/logout` と遷移を確認した。ホームの既存のログアウトのテストも通過した。
- `mise run check`: exit 0 (ベースラインからの新たな失敗は無し)。

方針からの乖離 (方式は変えていない): 1 から 7 は issue の「## 実装詳細の乖離」に記載した。

なお、依存の 0012 (アカウントと全データの削除) は本番の Workers のサブリクエスト上限の確認を残して `docs/issues/pending/` にあり、本 issue は画面の受け入れ基準 (FR-15) の範囲を満たす。
