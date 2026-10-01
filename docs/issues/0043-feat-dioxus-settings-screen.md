# 設定の画面を Dioxus で実装する

Created: 2026-10-02
Model: deepseek-v4p1-flash
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

---
