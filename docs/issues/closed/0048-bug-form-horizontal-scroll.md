# 幅 375 px のフォームで横スクロールが出るのを直し、入力欄を縦スクロールで扱えるようにする

Created: 2026-10-03
Model: DeepSeek V4.1 Flash
Completed: 2026-10-04

## 症状

幅 375 px で商品の登録 (`/products/new`) などのフォームを開くと、2 列の入力欄が画面の右にはみ出し、`.body` の横スクロールが必要になる。
購入のフォームの「推測した内容で商品を登録する」のシート (`frontend/src/screens/records/purchase_form.rs` の `register_open`) は、内容が `.sheet` の高さ (70 vh) を超えると下部の入力欄に到達できない。

最小再現の計測 (2026-10-03、Chrome headless、`frontend/src/ui/design.css` の `.screen`、`.body`、`.form`、`.grid2`、`.field`、`.box`、`.in` と同じ規則を持つ HTML を幅 375 px の枠で描画):

- `grid-template-columns: 1fr 1fr` のとき、`.body` の `scrollWidth` が 386 px、`clientWidth` が 375 px になる。
- `grid-template-columns: repeat(2, minmax(0, 1fr))` のとき、`.body` の `scrollWidth` と `clientWidth` がどちらも 375 px になる。

## 再現手順

1. リポジトリのルートで `mise run dev` を実行する。
2. 表示された `http://localhost:8787/` を開き、ログインする。
3. ブラウザの開発者ツールで表示幅を 375 px にする。
4. 商品の登録 (`/products/new`) を開く。
5. 「生産国」と「地域」、「精製方法」と「品種」の 2 列の入力欄が画面の右にはみ出し、`.body` に横スクロールが出る。期待する挙動は、入力欄が画面幅に収まり、縦スクロールだけで入力できることである。
6. 購入の登録 (`/purchases/new`) を開き、「推測した内容で商品を登録する」のシートを開く。内容が 70 vh を超えると、シートの下部の入力欄が見えなくなり、スクロールできない。

## 原因

`frontend/src/ui/design.css` の `.grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3) var(--space-3); }` の `1fr` は `minmax(auto, 1fr)` と同じで、トラックの最小値が中身の min-content になる。
`frontend/src/ui/field.rs` の `TextField` が描く `input` は `size` 属性の既定値による固有幅を持つため、2 列のグリッドのトラックの最小幅が `input` 2 つ分になり、グリッドの中身が親の幅を超える。
`.screen .body { overflow-y: auto; }` は、もう一方の軸の `overflow-x` の `visible` が `auto` に変わる CSS の規則により、はみ出した分を横スクロールとして扱う。
購入の登録のシートは `.sheet { overflow: hidden; }` の中に `.form` を置き、`.form` にスクロールの規則が無いため、内容が高さを超えると切れる。

## 修正方針

- `frontend/src/ui/design.css` の `.grid2` を `grid-template-columns: repeat(2, minmax(0, 1fr));` にする。
- あわせて `.form`、`.field`、`.field .box` に `min-width: 0;` を足し、グリッドと flex の子が内容より小さく縮めるようにする。
- `.sheet` の中のフォーム (購入の登録のシート) に縦スクロールを足す。`.sheet` の本体の領域に `overflow-y: auto;` を足す (対象は `frontend/src/screens/records/purchase_form.rs` の登録のシートと、`frontend/src/screens/records/picker.rs` の `RecordPickerSheet` の中身の一覧。選択のシートの一覧は既存の `list-scroll` を持つため、必要なら確認のうえ最小の変更にする)。
- `.screen` と `.screen .body` に `overflow-x: hidden` を足すだけの対処はしない。はみ出しを隠すだけで原因 (トラックの最小幅) が残り、入力欄の右端が切れるためである。
- 原本 (`docs/design/components/bundle.css` と、`.grid2` を持つ `docs/design/components/*/preview.html`) を先に更新し、`design.css` を追随させる。
- 対象はフォームを持つ全ての画面 (抽出の登録と編集、購入の登録と編集、商品の登録と編集、店の登録と編集、統計の任意の期間、購入の推測から商品を登録するシート) とする。

## 完了条件

- E2E の `set_window_size` (`backend/brew_book/tests/support/e2e.rs`) で幅 375 px にし、商品の登録と編集、購入の登録と編集、抽出の登録と編集、店の登録と編集、統計の任意の期間の各画面の `.body` の `scrollWidth` が `clientWidth` 以下になる (E2E の `frontend:test-same-origin` のブラウザで検証する。390 px では直っていなくても `scrollWidth` が切り上げられて条件を満たし得るため、症状を計測した 375 px で検証する)。
- 幅 840 px 以上で `.grid2` の計算済みの `grid-template-columns` が 2 トラックになる (ブラウザの自動テスト。既存の `the_wide_layout_matches_the_documented_widths` は `.grid2` を見ていないため、新しい検査を足す)。
- 幅 375 px で各フォームの最後の入力欄まで縦スクロールで到達でき、到達した入力欄の位置が `.body` の中にある (E2E のブラウザで、`.body` の `scrollTop` を末尾にして検証する)。
- 購入の登録のシートで、内容が `.sheet` の高さを超えるとき、シートの中を縦にスクロールして下部の入力欄と保存の操作に到達できる (E2E のブラウザで検証する)。
- `docs/design/components/bundle.css` と、`.grid2` を持つ `docs/design/components/*/preview.html` の値が `design.css` と一致する。
- E2E のスクリーンショット比較の差分を確認し、意図した差分は `docs/design/screenshots/` を更新し、意図しない差分は直す (比較の結果を issue に記録する)。
- 既存の Frontend のテストと E2E が通過する。
- `mise run check` が通過する。

## 解決方法

幅 375 px のフォームの横スクロールを、グリッドのトラックの最小幅を 0 にする修正で直し、シートの中のフォームに縦スクロールを足した。

- `frontend/src/ui/design.css`: `.grid2` を `grid-template-columns: repeat(2, minmax(0, 1fr))` にし、`.form`、`.field`、`.field .box` に `min-width: 0` を足した。`.sheet .body { flex: 1; min-height: 0; overflow-y: auto; }` を足した。
- `frontend/src/screens/records/purchase_form.rs`: 推測から商品を登録するシートの `.form` を `div { class: "body" }` で包み、シート本体を縦スクロールできるようにした。選択のシート (`frontend/src/screens/records/picker.rs`) は `.list-view` (`flex: 1; min-height: 0`) と `.list-scroll` で既に縦スクロールできるため変更していない。
- 原本 (`docs/design/components/bundle.css` と `.grid2` を持つ 19 の `preview.html`) の同じ 4 規則 (`.grid2`、`.field`、`.field .box`、`.form`) を先に更新し、`design.css` を追随させた。
- テスト: `frontend/tests/test_ui_web.rs` に `the_form_grid_keeps_two_tracks_on_the_wide_layout` を足した (幅 840 px 以上で `.grid2` の計算済み `grid-template-columns` が 2 トラックで同幅、入力欄がグリッドの中にある)。E2E (`backend/brew_book/tests/wrangler_same_origin_e2e.rs`) に `form_layouts` (9 画面で `.body` の `scrollWidth <= clientWidth` と、`.body` を末尾までスクロールして最後の入力欄に到達できること) と `register_sheet_scrolls` (購入の登録のシートの縦スクロール) を足した。`backend/brew_book/tests/support/e2e.rs` に `set_viewport_size` と `clear_viewport_size` を足した。

完了条件の検証:

- 幅 375 px で各画面の `.body` の `scrollWidth` が `clientWidth` 以下: E2E の `form_layouts` が 9 画面 (商品の登録と編集、購入の登録と編集、抽出の登録と編集、店の登録と編集、統計の任意の期間) で `scrollWidth > clientWidth` を失敗にし、通過した (`frontend:test-same-origin` 1 passed)。
- 幅 840 px 以上で `.grid2` の計算済みの `grid-template-columns` が 2 トラック: `the_form_grid_keeps_two_tracks_on_the_wide_layout` が 2 トラックで同幅であることを検証した (`frontend:test-web` の `test_ui_web` 20 passed に含めて通過)。
- 幅 375 px で各フォームの最後の入力欄まで縦スクロールで到達: `form_layouts` が `.body` を末尾までスクロールできること (`endScrollTop` が最大値) と、最後の入力欄を `scrollIntoView({ block: 'end' })` で表示の位置に入れたときの矩形が `.body` と表示の領域の中にあることを検証した (統計の画面は入力欄の下にグラフが続くため、到達は `scrollIntoView` で確かめる。方針の方式を保った乖離に記録した)。
- 購入の登録のシートの縦スクロール: `register_sheet_scrolls` がシートを開き、表示の高さを 520 px にして内容が 70 vh を超える状態にし、`.sheet .body` の `scrollTop` を末尾にして下部の入力欄と保存の操作に到達できることを検証した。
- `docs/design/components/bundle.css` と `.grid2` を持つ `preview.html` の値が `design.css` と一致: 4 規則を 20 ファイル (bundle.css と 19 previews。`Cover/preview.html` は `.grid2` を持たないため対象外) で機械的に比較し、全一致を確認した。
- スクリーンショット比較: `frontend/target/e2e-screenshots/comparison.json` の 58 件を確認し、参照の更新は不要と判断した。`.grid2` を持つフォームの `ProductEdit` は paper/night とも mean 0.0、`Stats` は mean 0.0 (max 13/12)。`BrewForm` の mean 1.0/3.8 は日付の文字の差でレイアウトは同一、`ShopEdit` (4.2/2.5) と `PurchaseDetail` (10.3/9.6) は参照が別画面のための既存の差である。参照が `Lists` の `Shops` (1.8/1.6) と `Products` (1.4/1.2) も同じく参照が別画面のための既存の差である。参照と大きさが違う 21 件も既存の環境差である。
- 既存の Frontend のテストと E2E、`mise run check` の通過: レビューの指摘の反映後、`mise run check` のフル実行は `frontend:test-web` の ChromeDriver の起動失敗 (既知の失敗、`docs/issues/0046-bug-frontend-test-web-chromedriver-under-parallel-load.md`。署名は `driver status: signal: 9 (SIGKILL)` / `Failed to detect test as having been run` と `webdriver POST /session timed out after 60.0s`) で 3 回中断した。メモリの圧迫 (36 GB 中 34 GB 使用) が疑われる。ユーザーの確認を得て既知の失敗として扱い、検査コマンドの通過を「新たな失敗が無いこと」と読み替え、段ごとに個別に実行して確認した。fmt、lint、formal は直列の `mise run check` で通過し、frontend:build (7.11 秒)、frontend:lint (2.53 秒)、frontend:test (41 suites ok)、frontend:test-web (9 suites ok。新しい `the_form_grid_keeps_two_tracks_on_the_wide_layout` と `the_form_grid_does_not_overflow_at_the_narrow_width` を含む)、frontend:test-same-origin (1 passed。`form_layouts` と `register_sheet_scrolls` を含む)、backend:test (68 suites ok)、backend:test-integration (17 suites ok) がすべて通過した。新たな失敗は無い。

方針の方式を保ったままの実装詳細の乖離:

- 幅 375 px の作り方: 完了条件は `set_window_size` (`backend/brew_book/tests/support/e2e.rs`) を挙げているが、macOS の headless Chrome は窓の幅を 500 px に丸めるため (`set_window_rect` で 375 を要求しても `innerWidth` は 500)、`set_window_size(375, 844)` に加えて CDP の `Emulation.setDeviceMetricsOverride` (`mobile: false`) で表示の領域を 375 px にした。`E2eBrowser::set_viewport_size` と `clear_viewport_size` を追加し、検査は実際の `innerWidth` が 375 であることも確かめる。方式 (E2E のブラウザで 375 px を検証する) は変えていない。
- 統計の画面の縦スクロール: 統計の画面は任意の期間の入力欄の下にグラフが続くため、`.body` の `scrollTop` を末尾にすると入力欄は画面の上に出る。`.body` を末尾までスクロールできること (`endScrollTop` が最大値) を確かめたうえで、入力欄を `scrollIntoView({ block: 'end' })` で表示の位置に入れ、その矩形が `.body` の内側にあることを検査した。
- シートを開く仕掛け: 実環境の推測は Workers AI を呼ぶため E2E では使えず、写真の選択は OS のダイアログを開く。`SUGGESTION_STUB_SCRIPT` で `window.fetch` の `/api/purchase-suggestions` だけを差し替え、`HTMLInputElement.prototype.click` を file のときだけ止めて、WebDriver で `input[type=file]` にファイル (`frontend/public/favicon.png`) を設定した。アプリの変更は不要で、画面の操作は実際のアプリの経路のままである。
- `.sheet .body` は `design.css` にだけ足した。`.sheet` は原本 (`bundle.css`) に無い (0041 で画面の土台として足した) 規則のためである。

bug の再現確認:

- 修正前: `docs/design/tokens.css` と修正前の `design.css`、`.screen`/`.body`/`.form`/`.grid2`/`.field`/`.box`/`.in` と同じ構造の HTML を幅 375 px の枠で Chrome headless に描画し、`.body` の `scrollWidth` 386 / `clientWidth` 375 を計測した (issue の計測値と一致)。入力欄の固有幅によるトラックの最小幅の合計が `.form` の内側の幅を超えていた。
- 修正後: 同じ HTML で `scrollWidth` 375 / `clientWidth` 375、`.grid2` の計算済み `grid-template-columns` は `165.5px 165.5px`。実アプリでも E2E が 375 px で全 9 画面の非超過とシートのスクロールを確認した。

レビューの指摘を受けて変えたもの (方式は変えていない):

- 幅 375 px の狭い画面でのはみ出しを検出するブラウザテスト `the_form_grid_does_not_overflow_at_the_narrow_width` を足し、広い窓の検査 `the_form_grid_keeps_two_tracks_on_the_wide_layout` の doc コメントのメディアクエリの記述を直した (レビューの指摘、中・低)。
- `register_sheet_scrolls` の後片付けを、検査が失敗しても表示の上書きを外し、最初の失敗を保つ形にした (レビューの指摘、低)。
- `check_form_metrics` に、最後の入力欄が表示の領域の中にあることの検査を足した (レビューの指摘、低)。
- 記録の修正: スクリーンショット比較に `Shops` (1.8/1.6) と `Products` (1.4/1.2、参照は `Lists`) を足し、完了条件 3 の検証に `scrollIntoView` を明記し、再現確認の数値を計測値 (386/375) に合わせた (レビューの指摘、低)。

却下した指摘:

- 再現用の HTML と実行ログを残すこと: 一時ファイルは実装の規則で報告前に削除し、回帰は E2E (`form_layouts`、`register_sheet_scrolls`) が担うため、却下する (レビューの指摘、低)。
- `design.css` と原本の値の一致を守る自動テストの追加: 完了条件は今回の一致の確認までで、CSS の値の突き合わせの仕組みの整備はスコープ外の改善として報告する (レビューの指摘、低)。
