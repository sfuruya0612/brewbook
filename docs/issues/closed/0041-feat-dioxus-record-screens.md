# 記録の画面を Dioxus で実装する

Created: 2026-10-02
Model: deepseek-v4p1-flash
Completed: 2026-10-02
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

## 解決方法

記録の画面 (ホーム、抽出、購入、商品、店) と写真の添付を実装した (ADR-0017、ADR-0003、ADR-0016)。

- `frontend/src/records/` に記録のモデルと処理を追加した。`models.rs` (API の型)、`api.rs` (一覧と詳細と登録と更新とアーカイブの呼び出し)、`list.rs` (カーソル方式のページングとアーカイブ済みの切り替え)、`forms.rs` (フォームの検証と送信の本文の組み立てと保存の対象の判断 `save_target`)、`inputs.rs` (入力の正規化)、`display.rs` (一覧と参照の表示の組み立て)、`suggestions.rs` (自由記述のサジェスト)、`upload.rs` (署名付き URL への PUT と完了通知)、`currencies.rs` (通貨のマスタと選択肢)。
- `frontend/src/screens/records/` に 10 画面 (ホーム、抽出の詳細と入力、購入の一覧と詳細と入力、商品の一覧と入力、店の一覧と入力) と、共通の一覧 ([`RecordListView`])、選択のシート ([`RecordPickerSheet`])、サジェストの入力 ([`SuggestionField`]) を追加した。
- `frontend/src/router.rs` の記録の 14 経路を実画面に配線し、`frontend/src/screens/mod.rs` に記録の画面の再公開を追加した (統計と設定は 0042 と 0043 まで仮の画面のまま)。
- `frontend/src/app.rs` に記録の変更の通知 (保存とアーカイブの後に一覧を読み直す) を追加した。
- `frontend/src/records/mod.rs` に `RecordError`、`record_error_key`、`record_error_retry` を追加し、`RecordServices` に写真のアップロード (`uploader`) を加えた。
- `frontend/src/ui/design.css` に記録の画面の土台、ボトムシート (`.sheet` など)、通知のクラスを足し、既存の `.switch` に button 用のリセット (`background: transparent`、`border: 0`、`padding: 0`、`font: inherit`、`cursor: pointer`) を足した (デザインの原本にボトムシートのクラスが無いため、README のトークン `radius-lg` と `shadow-float` に従った)。
- `frontend/src/ui/button.rs` の `IconButton` に `disabled` を足し、アーカイブの実行中は二重に要求を送らないようにした。
- `frontend/Cargo.toml` の web-sys に `MediaQueryList` を追加した (2 段組の切り替えの検出。新しいクレートは増やしていない)。
- `frontend/tests/support/mod.rs` に偽の時計、写真の選択と変換、アップロードの送信 (`FakeUploadTransport`) を足し、`frontend/tests/test_records.rs` を `RecordServices::new` の引数に追随させた。
- `frontend/tests/` に `test_records_api.rs` (18 件)、`test_records_list.rs` (8 件)、`test_records_forms.rs` (9 件)、`test_records_display.rs` (7 件)、`test_records_screens.rs` (3 件)、`test_currencies.rs` (2 件)、`test_photo_web.rs` (wasm 2 件) を、`frontend/pbt/tests/prop_records.rs` に記録の PBT (5 性質) を追加した。

完了条件の検証:

- ホーム、抽出、購入、商品、店の一覧と詳細と入力の画面があり、登録、閲覧、編集、アーカイブ、アーカイブ解除ができる: `tests/test_records_screens.rs` の `the_ten_record_screens_have_the_expected_props` (10 画面の prop の型) と `the_record_routes_are_wired_to_the_record_screens` (14 経路の配線)、`tests/test_records_api.rs` の `a_shop_is_read_created_updated_and_archived`、`a_product_is_read_created_updated_and_archived`、`a_purchase_is_read_created_updated_and_archived`、`a_brew_is_read_created_updated_and_archived` (経路と本文、アーカイブとアーカイブ解除の両方) で確認した。新規と更新の判断は `save_target` (`tests/test_records_forms.rs` の `a_save_target_depends_on_the_id`) で固定し、商品と店の入力の画面は新規でも保存のボタンを出す。
- 一覧がカーソル方式のページングで次のページを読み、アーカイブ済みを含める切り替えができる: `tests/test_records_list.rs` の 8 件 (`the_first_load_requests_the_first_page_without_archived`、`the_next_page_uses_the_cursor_of_the_previous_page`、`the_archived_toggle_reloads_from_the_first_page`、`the_next_page_is_loaded_only_near_the_end`、`a_reset_while_loading_is_run_after_the_page_finishes`、`a_failed_load_keeps_the_cursor_and_retries_from_the_first_page`、`a_successful_load_clears_the_previous_error`、`a_failed_reset_can_be_retried`) と、PBT の `a_page_is_never_requested_twice_at_once`、`a_failed_load_keeps_the_list_usable` で確認した。
- 抽出の入力の画面が購入の項目 (商品と店) をたどって表示する: 購入の選択のシートは商品名と購入日と店の補足を出し (`display::purchase_row_subtitle`)、抽出の詳細は購入と商品と店の連鎖を出す (`display::brew_reference_tiles`)。`tests/test_records_display.rs` の `the_brew_reference_tiles_follow_the_purchase_product_and_shop`、`the_purchase_row_subtitle_shows_the_day_and_the_shop`、`the_brew_row_subtitle_shows_the_local_time_and_the_shop` で確認した。
- 写真の添付、差し替え、削除ができ、変換の結果が JPEG かつ長辺 2048 px 以下になる: `tests/test_photo_web.rs` の `a_large_png_becomes_a_jpeg_with_a_long_side_of_2048` と `a_large_jpeg_becomes_a_jpeg_with_a_long_side_of_2048` (4000x3000 の PNG と JPEG から 2048x1536 の JPEG) で確認した。添付と差し替えと削除の API の手順は `tests/test_records_api.rs` の `the_photo_urls_are_requested_completed_and_deleted`、`the_uploader_requests_the_url_puts_the_jpeg_and_completes`、`a_failed_upload_is_a_retryable_upload_error`、`a_failed_completion_is_reported` で確認した。
- 写真のアップロードが署名付き URL への直接の PUT と完了通知で完了する: 実装は `records/upload.rs` の web-sys の `fetch` の PUT (`Content-Type: image/jpeg`) と完了通知まで完成し、native のテストで 3 呼び出しを確認した。実バケットへの PUT は 0044 の E2E で行う (issue の記載どおり)。
- 推測 (FR-19) が空の入力欄にだけ適用され、商品が未選択のときだけ一致する商品を選び、一致が無いときは登録の導線を表示する: `tests/test_records_forms.rs` の `the_product_match_selects_only_when_no_product_is_selected` と、PBT の `the_suggestion_never_overwrites_an_input`、`a_selected_product_is_never_replaced` で確認した。画面は一致が無いとき「商品を登録」の導線を出し、推測値 (Flavor Notes を含む) を引き継ぐ登録のシートを開く。
- 自由記述の項目のサジェストが表示され、選択できる: `tests/test_records_forms.rs` の `the_suggestion_state_shows_selects_and_ignores_stale_responses`、`the_highlight_splits_the_matched_prefix` と、PBT の `a_stale_suggestion_response_is_ignored` で確認した。
- 画面数の成功指標 (ホームから抽出の保存までが 3 画面以内): 数えるのは 0044 の E2E。実装はホーム → 抽出の入力の 2 画面で、購入と商品と店の選択はボトムシートにし、経路を増やしていない。
- ADR-0003 の署名の条件 (Content-Type と Content-Length) の確認: `backend/brew_book/.dev.vars` がこの環境に無いため対象外とした (実リクエストは未実施。issue の記載どおり、資格情報がある環境での確認に委ねる)。
- `mise run dioxus:lint`、`mise run dioxus:test`、`mise run dioxus:test-web`: 作業ツリーで成功した (lint は clippy と fmt とも警告なし、native は 180 件、ブラウザは 28 件)。`mise run dioxus:build` も成功した。
- `mise run check` が通過する: 所有者の指示により、移行の全 issue の完了後に 1 回だけ実行するため、この issue では未実行 (最終確認に委ねる)。

native のテストが覆う範囲 (0041 のレビューの指摘への回答):

- 覆う: API クライアントの経路と本文と応答の変換、フォームの検証、保存の対象の判断 (`save_target`)、一覧の状態機械 (ページング、切り替え、失敗の状態)、表示の組み立て (副題と参照の連鎖)、サジェストの状態、推測の適用、通貨の選択肢、プレビューの base64、失敗の文言と再試行の判定。
- 覆わない: 画面の内部の配線 (どのハンドラがどの API をどの状態で呼ぶか) はコードで確認し、抽出の保存の流れ (ホームから保存まで) は 0044 の E2E で確認する。購入、商品、店のフォームの配線は native のテストの対象外である (0044 の E2E の対象でもない)。この範囲の確認は今後の issue の課題として残す。

方針を保った実装詳細の乖離:

- `RecordServices` に写真のアップロード (`uploader`) を加えた (Flutter の `RecordServices` と同じ束ね方。方針の「API、時計、写真の選択と変換」にアップロードを加えた)。
- 2 段組の切り替えの検出のため、web-sys に `MediaQueryList` を追加した。
- デザインの原本にボトムシートのクラスが無いため、`docs/design/README.md` のトークン (`radius-lg`、`shadow-float`) に従って `.sheet` などを `src/ui/design.css` に足した。
- FR-19 の商品の登録の導線は、購入のフォームの状態を失わないよう、画面遷移ではなくフォーム内のボトムシートで実装した。
- 購入の詳細の「評価の推移」のグラフ (FR-18) は 0042 の完了条件のため、この issue では入れていない。

レビューの指摘を受けて変えたもの (方式は変えていない):

- 商品と店の入力の画面で、新規のときも保存のボタンを出すようにした (編集のときだけアーカイブを足す)。Flutter と同じ。
- `record_error_retry` が 401 以外の API エラー (500 など) でも再試行を出すようにした (デザインの Feedback の指示。これまでは読み込みの失敗で画面が行き止まりになっていた)。
- 一覧の失敗の状態を直した (読み込みの開始と成功で直近の失敗を消し、読み直しの失敗では前の条件の行を残さない)。
- `record_error_key` と `record_error_retry` の全分岐のテスト、`save_target` のテスト、通貨の選択肢とプレビューの base64 のテスト、形式違反と完了通知の失敗のテストを足し、PBT で実現できる単体テスト (`the_suggestion_fills_only_empty_fields`) を削った。
- 推測からの商品の登録で Flavor Notes を引き継ぐようにした (Flutter と同じ)。
- 使われていない `go_back`、`route_title`、`ProductSuggestion::is_empty` を削り、`archive_button` の `busy` を実際に使うようにした (実行中は押せない)。

対象外とした完了条件:

- 写真のアップロードの実バケットへの確認 (完了条件 5) と、ADR-0003 の署名の条件の実リクエストの確認 (完了条件 9): `backend/brew_book/.dev.vars` がこの環境に無いため未実施 (issue の「ない環境では対象外として記録する」に従う)。
- 画面数の計測 (完了条件 8): 0044 の E2E で数える。
- `mise run check` の通過 (完了条件 11): 所有者の指示により、移行の全 issue の完了後に 1 回だけ実行する。

## 0044 の E2E の検証 (2026-10-02)

0044 の E2E (`mise run frontend:test-same-origin`) が、この issue の E2E の完了条件のうち次を検証した。

- ホームから抽出の保存まで: ホームの「抽出を記録」から入力の画面を開き、購入をボトムシートで選び、保存の通知が出て一覧に載ることを確認した。
- 画面数の成功指標: 記録した経路の種類が 3 以下 (`/` と `/brews/new`) であることを確認した。
- 入力の検証のバナー: 購入を選ばずに保存を押すと、検証のバナーが出ることを確認した。

写真のアップロードは `backend/brew_book/.dev.vars` がこの環境に無いため対象外とした (issue の記載どおり)。
