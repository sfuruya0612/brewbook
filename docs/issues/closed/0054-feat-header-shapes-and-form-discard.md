# ヘッダーを画面の種類で 3 つの形に分け、フォームの破棄の確認と抽出方法の例を足す

Created: 2026-10-06
Model: DeepSeek V4.1 Flash
Completed: 2026-10-06

## 背景

所有者から、実機の画面 (抽出の記録と商品の登録) について次の 3 点の指摘があった。

- 抽出日時の日付と時刻の入力欄がずれている。時刻の欄が上に寄る。
- 抽出方法に何を書けばよいか分からない。挽き目と同じように例を出してほしい。
- ヘッダーを、最上位の画面は「ハンバーガーメニュー、印、画面名」、記録と編集のフォームは「閉じる、画面名、保存」の並びにしてほしい。

現状の実装は次のとおりである。

- 日付と時刻は `frontend/src/screens/records/brew_form.rs` が `Field` を 2 つ横に並べる。時刻の `Field` は項目名が空で、`frontend/src/ui/design.css` の `.field .lbl` が空のとき高さ 0 になるため、時刻の欄が項目名の行 (18 px) と余白 (6 px) の分だけ上にずれる。
- ヘッダーは `frontend/src/ui/app_bar.rs` の `AppBar` が「ハンバーガー、先頭の操作、印とアプリ名、その下の画面名、末尾の操作」の 1 形で組んでいた (0047)。印とアプリ名は `.app-name`、画面名は `.ttl` の 13 px である。
- 抽出方法の欄は `frontend/src/screens/records/suggestion_field.rs` の `SuggestionField` で、プレースホルダーを渡していなかった。挽き目は `grindSettingHint` (「例: Comandante 24」) を渡している。
- `docs/design/` に新しい AppBar のデザイン (最上位はハンバーガー、印、画面名。詳細は戻る、画面名、操作。フォームは閉じる、画面名、保存) が未コミットで入っていた。同じ変更にドロワー化、アーカイブの復活、並び替えとお気に入りの削除も含まれていた。

所有者の回答で、今回の範囲は次のとおりになった。

- ヘッダーの 3 つの形と、フォームの破棄の確認を実装する。
- フォームのキャンセルの文字ボタンは実装しない。
- 並び替えとお気に入り (0051) は残し、アーカイブは復活させない。幅 840 px 以上は今のナビゲーションレールを維持する (ドロワー化はしない)。
- 日付と時刻のずれと、抽出方法の例も直す。

## 目的

- 抽出日時の日付と時刻の入力欄と案内の位置が揃う。
- 抽出方法の入力に例が出る。
- 最上位の画面は「ハンバーガーメニュー、印、画面名」、詳細は「戻る、画面名、操作」、フォームは「閉じる、画面名、保存」のヘッダーになる。詳細とフォームにはメニューと印を出さない。
- フォームに変更があるときは、閉じる前に「保存せずに閉じますか？」の確認を出す。

## 設計判断

- `AppBar` に `brand` (印を出すか) を足し、`ScreenAppBar` に `menu` (ハンバーガーを出すか) と `brand` を足して 3 つの形を組む。最上位の画面は `menu: true` (既定) と `brand: true`、詳細とフォームは `menu: false` (印は既定の false) にする。先頭の操作 (戻るか閉じる) と末尾の操作は今までどおり呼び出し側が渡す。
- 印は 22 x 26 px の `mark.svg` にする (docs/design/components/AppBar の高さ 26 px)。印は押すとホームへ戻るボタンにし、0047 の動きを保つ (ホームでは何もしない)。`on_home` が無いとき (登録の画面) は押せない印にする。
- 画面名は印の右の 1 行に 20 px の 600 で出す。`flex: 1` にして末尾の操作を右端へ寄せる。
- フォームの保存は高さ 36 px の `roast` の塗り (`ButtonVariant::Primary` と `ButtonSize::Sm`) にする。キャンセルの文字ボタンは所有者の指示で置かない (閉じるが同じ役割を持つ)。
- 日付と時刻のずれは `.field .lbl` に `min-height: 18px` を足して直す。項目名の無い欄も項目名の行の高さを確保し、隣の欄と枠と理由の位置を揃える。原本 (`docs/design/components/bundle.css` と `components/Field/README.md`) にも同じ規則を足す。
- 破棄の確認は、フォームごとに `dirty` の signal を持ち、入力、ピッカーの選択、タグの追加と削除、写真の選択と削除、評価で true にし、編集の読み込みの後に false に戻す。閉じる操作は `dirty` のときだけ確認を開き、確認の「破棄する」で閉じる。確認は `frontend/src/screens/records/mod.rs` の `DiscardConfirm` に共通化し、`ConfirmDialog` で出す。
- `SuggestionField` に `on_change` を足し、入力と候補の選択で知らせる (自由記述の項目も変更に数えるため)。
- i18n: `methodHint` (「例: ハンドドリップ」)、`discardConfirmTitle` (「保存せずに閉じますか？」)、`discardConfirmMessage` (「入力した変更は破棄されます。」)、`discardConfirmButton` (「破棄する」) の 4 キーを足す (`KEY_COUNT` は 229 から 233)。
- 見送ったもの: ドロワー化 (幅 840 px 以上は今のレールを維持)、アーカイブの復活、並び替えとお気に入りの削除、フォームのキャンセルの文字ボタン。`docs/design/` の未コミットのデザインは所有者の作業としてそのまま残し、実装はヘッダーの 3 つの形と破棄の確認、日付のずれ、抽出方法の例だけを取り込む。

## 完了条件

- 抽出日時の日付と時刻の枠と案内の縦の位置が揃う。
- 抽出方法の入力に例がプレースホルダーで出る。
- 最上位の画面のヘッダーが「ハンバーガーメニュー、印、画面名」、詳細が「戻る、画面名、操作」、フォームが「閉じる、画面名、保存」になる。詳細とフォームにはメニューと印が出ない。
- フォームに変更があるときだけ、閉じる前に破棄の確認が出る。確認のキャンセルでは閉じず、破棄では閉じる。変更が無いときは確認を出さずに閉じる。
- i18n の 4 キーが足され、`KEY_COUNT` と日本語と英語の表の要素数が一致する。
- 既存の自動テストと `mise run check` が通過する。

## 関連

- 0047 (ヘッダーにアプリの印とハンバーガーメニューを置く)。今回の 3 つの形はこの 1 形を置き換える。
- 0050 (アーカイブの削除) と 0051 (並び替えとお気に入り)。一覧の機能は変えない。
- 0049 (日付と時刻の入力の date input と time input)。今回のずれはこの置き換えの後に見つかった。
- `docs/design/` の未コミットの変更 (AppBar の 3 つの形、ドロワー、アーカイブ)。ドロワーとアーカイブは今回の範囲外である。

## 解決方法

- 日付と時刻のずれ: `frontend/src/ui/design.css` と `docs/design/components/bundle.css` の `.field .lbl` に `min-height: 18px` を足し、`docs/design/components/Field/README.md` に規則を書いた。
- 抽出方法の例: `frontend/src/i18n/` に `methodHint` を足し、`brew_form.rs` の抽出方法の `SuggestionField` に `hint` として渡した。
- ヘッダー: `frontend/src/ui/app_bar.rs` の `AppBar` を、先頭の操作、印 (`brand` のとき)、画面名、末尾の操作の並びに組み直した。印は `BrewbookMark` の 22 x 26 px で、`on_home` があるときはボタンにした。`frontend/src/screens/app_nav.rs` の `ScreenAppBar` に `menu` と `brand` を足し、最上位の画面 (ホーム、購入、商品、店、統計、設定) は `brand: true`、詳細 (抽出と購入) とフォーム (抽出、購入、商品、店) は `menu: false` にした。フォームの保存は `ButtonVariant::Primary` と `ButtonSize::Sm` にした。`frontend/src/ui/design.css` に `.brandmark`、`.ttl` (20 px、600)、`.acts` と、`docs/design/components/bundle.css` に追随する `.drawer`、`.badge`、`.toolbar`、`.switch`、`.row.archived` のクラスを足した (原本との突き合わせのテストのため)。
- 破棄の確認: `frontend/src/screens/records/mod.rs` に `DiscardConfirm` を足し、抽出、購入、商品、店のフォームに `dirty` と `discard_open` の signal を足した。入力、ピッカーの選択、タグ、写真、評価、自由記述 (`SuggestionField` の `on_change`) で `dirty` を true にし、編集の読み込みの後に false に戻す。閉じる操作は `dirty` のときだけ確認を開く。
- i18n: `methodHint`、`discardConfirmTitle`、`discardConfirmMessage`、`discardConfirmButton` の 4 キーを足した (`KEY_COUNT` は 233)。
- テスト: `frontend/tests/test_records_screens_web.rs` に日付と時刻の枠と案内の位置の検査、抽出方法の例の検査、破棄の確認の検査 (`the_brew_form_confirms_before_discarding_the_changes`) を足した。`frontend/tests/test_ui_web.rs` の AppBar の検査を 3 つの形に合わせて書き直し、フォームの形の検査 (`the_form_app_bar_matches_the_tokens`) を足した。`frontend/tests/test_records_lists_web.rs` の操作の選択子を `.appbar .acts > ...` に更新した。`frontend/tests/test_ui.rs` の AppBar の静的検査の名前と説明を直した。`backend/brew_book/tests/wrangler_same_origin_e2e.rs` のナビゲーションの検査を更新した (印でホームへ戻る。フォームにはメニューと印を出さず、閉じると一覧へ戻る)。
- CHANGES.md に 4 件 (日付のずれの修正、ヘッダーの 3 つの形、破棄の確認、抽出方法の例) を足した。

## 検証

- 日付と時刻の枠と案内の位置が揃う: `the_brew_new_form_draws_the_date_and_time_with_the_native_pickers` が 2 つの枠と 2 つの案内の `get_bounding_client_rect` の上端の差が 1 px 未満であることを検証した。
- 抽出方法の例: 同じテストが `input[placeholder='e.g. Pour over']` を検証した。
- ヘッダーの 3 つの形: `the_app_bar_matches_the_tokens` が印 22 x 26 px、画面名 20 px の 600、末尾の操作 40 px を検証し、`the_form_app_bar_matches_the_tokens` がメニューと印が出ないこと、閉じるが 40 px、保存が高さ 36 px の `roast` の塗りであることを検証した。
- 破棄の確認: `the_brew_form_confirms_before_discarding_the_changes` が、変更が無いときは確認を出さずに閉じ、変更があるときは確認を出し、キャンセルでは閉じず、破棄では閉じることを検証した。
- 一覧と詳細と編集の操作: `test_records_lists_web.rs` の星の検査 (選択子の更新) が通過した。
- E2E: `wrangler_web_routes_registration_login_and_brew_save_ok` が通過した。ナビゲーションの検査は、6 つの行き先、現在の経路と同じ項目で遷移しないこと、印でホームへ戻ること、フォームにメニューと印が無く閉じると一覧へ戻ることを検証した。
- `mise run frontend:test` (43 suite)、`mise run frontend:test-web` (全 suite)、`mise run frontend:test-same-origin`、`mise run frontend:lint`、`mise run fmt`、backend の `cargo clippy --workspace --all-targets -- -D warnings` が通過した (2026-10-06)。
- `mise run check` を実行し、fmt、lint、formal、backend:build、backend:test (68 suite)、frontend:build、frontend:lint、frontend:test、frontend:test-web、frontend:test-same-origin が通過した。backend:test-integration は並列実行の負荷 (backend:test のコンパイルに 35 分) の下でコンパイル中に外部から SIGTERM で停止されたため、単独で再実行して 17 suite の通過を確認した (2026-10-06)。テストの失敗は 0 件である。

## 原本との意図的な違い

- `docs/design/` のフォームの形は「閉じる、画面名、キャンセル、保存」であるが、所有者の指示でキャンセルの文字ボタンは実装していない (閉じるが同じ役割を持つ)。
- `docs/design/` の詳細の形は「戻る、画面名、アーカイブ、編集」であるが、アーカイブは 0050 で削除済みのため、お気に入りと編集にしている。
- `docs/design/` の幅 840 px 以上の配置は常設のドロワーであるが、所有者の回答で今のナビゲーションレールを維持している。
