# 記録の画面を Dioxus で実装する

Created: 2026-10-02
Model: deepseek-v4p1-flash
対応 ADR: ADR-0017、ADR-0003 (docs/adr/0003-photo-storage-r2-presigned-upload.md)、ADR-0016 (docs/adr/0016-photo-inference-with-workers-ai.md)
関連 PRD: FR-6 から FR-13、FR-19
依存: 0038, 0039, 0040

## 背景

本 issue は、2026-10-01 の所有者の決定 (ADR-0017) から起票した。

Flutter の記録の画面は `frontend/lib/screens/` の 10 ファイル (ホーム、抽出の詳細と入力、購入の一覧と詳細と入力、商品の一覧と入力、店の一覧と入力) である。
写真は `frontend/lib/photo/photo_picker_web.dart` (`<input type="file">`)、`image_converter_web.dart` (Canvas で JPEG に変換して長辺 2048 px 以下に縮小)、`photo_uploader.dart` (署名付き URL への PUT と完了通知) にある。
推測 (FR-19) は `POST /api/purchase-suggestions` を呼び、空の入力欄にだけ適用する。
自由記述のサジェスト (FR-13) は `frontend/lib/widgets/suggestion_field.dart` が API を呼ぶ。
記録の依存は `frontend/lib/records/record_services.dart` の `RecordServices` が束ねる。

## 目的

ホーム、抽出、購入、商品、店の記録の画面で、登録、閲覧、編集、アーカイブ、アーカイブ解除、写真の添付と差し替えと削除、推測の適用、サジェストができる。

## 設計判断

- 写真の変換はブラウザの Canvas API (`HTMLCanvasElement` の `drawImage` と `toBlob`) を使う (Flutter の `frontend/lib/photo/image_converter_web.dart` と同じ経路)。
  `image` クレートを wasm で使う案は、バンドルが大きくなるため採らない。
- 写真の選択は `<input type="file">` の `accept` に `image/jpeg,image/png,image/webp` を指定する (Flutter と同じ)。
- 署名付き URL への PUT は `web-sys` の `fetch` を使い、`Content-Type` を付ける (`Content-Length` はブラウザが自動で付ける。ADR-0003 の署名の条件 (Content-Type と Content-Length) と一致することは、開発用のバケットへの実リクエストで確認する。ローカルの wrangler は R2 の S3 互換のエンドポイントを提供しない)。
- 変換の結果の検証 (長辺 4,000 px の PNG と JPEG から JPEG かつ長辺 2048 px 以下の出力) は `wasm-bindgen-test` で行う (FR-10)。
- 画面の状態は Dioxus の signal で持ち、フォームの入力と API の呼び出しは画面ごとのモジュールに分ける。
  記録の依存 (API、時計、写真の選択と変換) は 0038 の束ねた型で配る (Flutter の `RecordServices` と同じ)。
- 一覧はカーソル方式のページングで、末尾に近づくと次のページを読み、アーカイブ済みを含める切り替えを持つ (Flutter の `frontend/lib/widgets/record_list_view.dart` と同じ。PRD の非機能要求)。
- 画面数の成功指標 (ホームから抽出の保存まで 3 画面以内) を守るため、抽出の入力は 1 画面に収め、購入の選択はボトムシートで行う (デザインの README の指示)。
- 採らない案: 画像の変換をサーバーで行う (ADR-0003 でクライアントと決めた)、`image` クレートを wasm で使う (バンドルが増える)、フォームのライブラリを入れる (依存を増やさない)。

## 完了条件

- ホーム、抽出、購入、商品、店の一覧と詳細と入力の画面があり、登録、閲覧、編集、アーカイブ、アーカイブ解除ができる (native のテストで確認する)。
- 一覧がカーソル方式のページングで次のページを読み、アーカイブ済みを含める切り替えができる (native のテストで確認する)。
- 抽出の入力の画面が購入の項目 (商品と店) をたどって表示する (FR-11、UC-6)。
- 写真の添付、差し替え、削除ができ、変換の結果が JPEG かつ長辺 2048 px 以下になる (`wasm-bindgen-test` で確認する)。
- 写真のアップロードが署名付き URL への直接の PUT と完了通知で完了する (0044 の E2E で確認する。R2 の資格情報 (`backend/brew_book/.dev.vars`) がある環境で行い、ない環境では対象外として記録する)。
- 推測 (FR-19) が空の入力欄にだけ適用され、商品が未選択のときだけ一致する商品を選び、一致が無いときは登録の導線を表示する (native のテストで確認する)。
- 自由記述の項目のサジェスト (FR-13) が表示され、選択できる (native のテストで確認する)。
- 画面数の成功指標を満たす (ホームから抽出の保存までが 3 画面以内。0044 の E2E で数える)。
- ADR-0003 の署名の条件 (Content-Type と Content-Length) の確認を、開発用のバケットへの実リクエストで行い、結果を issue に記録する (R2 の資格情報 (`backend/brew_book/.dev.vars`) がない環境では対象外として記録する。docs/issues/pending/0009-feat-photo-r2-presigned-upload.md と同じ確認)。
- `mise run dioxus:lint`、`mise run dioxus:test`、`mise run dioxus:test-web` が成功する。
- `mise run check` が通過する。

## 関連

- 0038 が基盤を、0039 が部品を、0040 が認証を作る。
- 0042 が統計の画面を、0043 が設定の画面を作る。
- 0044 が E2E と画面数の計測を用意する。

---
