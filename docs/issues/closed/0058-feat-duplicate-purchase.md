# 過去の購入をもとに新しい購入を登録する導線を足す

Created: 2026-10-14
Model: Claude Fable 5.1
Completed: 2026-10-16

## 背景

所有者の要望は TODO (`docs/issues/TODO.md`) の「購入と商品の項目を統合したい」の 2 つ目の小項目である。

- 同じ商品を 2 回目以降に買うときは、購入の一覧と詳細の「この購入をもとに登録」から、商品の項目と店と Roast を引き継いだ新規フォームを開く。
  購入日と写真は引き継がない。

商品と購入の統合 (TODO の他の 3 つの小項目) は issue 0057 で扱う。
本 issue は 0057 の後に実装する。
0057 で商品マスタが無くなるため、同じ商品を再び買うときに商品名、Producer、Origin、Region、Process、Variety、Flavor Notes を毎回入力することになる。
本 issue はその手間を、過去の購入からの引き継ぎで補う。

現状 (0057 の完了時点の想定を含む) は次のとおりである。

- 購入の新規フォームの経路は `frontend/src/router.rs` の `/purchases/new` (`Route::PurchaseNew {}`、引数無し) である。
  クエリを持つ経路は `/register?:token` (`Route::Register { token: Option<String> }`) が 1 つあり、Dioxus のルートのクエリの書き方の前例になる。
- 購入の一覧 (`frontend/src/screens/records/purchase_list.rs`) の新規の登録は右下の `Fab` から行い、幅 840 px 以上の 2 段組では右の面に埋め込みのフォーム (`creating`) を出し、それ以外は `Route::PurchaseNew {}` へ遷移する。
  ブラウザテストの画面幅は 1280 px で 2 段組が有効である (`mise.toml` の `frontend:test-web`)。
- 一覧の行の共有部品 `frontend/src/ui/list_row.rs` の `ListRow` が持つ操作は、行の押下 (詳細を開く) と星 (お気に入り。`stop_propagation` で行の押下と分ける) だけで、他の操作を置く口が無い。
  `docs/design/components/ListRow/README.md` にも行の操作は星だけが書かれている。
- 購入の詳細 (`frontend/src/screens/records/purchase_detail.rs`) のヘッダーの操作はお気に入り、編集、削除の 3 つである (0056)。
- 購入のフォーム (`frontend/src/screens/records/purchase_form.rs`) は、0057 の後は商品名、Producer、Origin、Region、Process、Variety、Flavor Notes、店、購入日 (既定は当日)、Roast、Roast Date、価格と通貨 (通貨の既定は JPY)、重量、写真を持つ。
  編集のときは `RecordsApi::purchase` で単件を取得して欄を埋める。
- API の失敗の扱いは `frontend/src/records/mod.rs` の `record_error_retry` が決め、401 以外の API エラーでは再試行のバナーを出す。
  0056 は、削除と影響の取得の 404 を成功と同じ扱い (一覧に戻る) にした。

## 目的

利用者は、購入の一覧の行と購入の詳細から「この購入をもとに登録」を選び、商品の項目と店と Roast が入った新規の購入のフォームを開ける。

## 設計判断

所有者が 2026-10-13 に決めたことは次のとおりである。

- 導線は購入の一覧と詳細に置く。
- 引き継ぐ項目は商品の項目 (商品名、Producer、Origin、Region、Process、Variety、Flavor Notes) と店と Roast とする。
  購入日と写真は引き継がない。
- 再購入の入力の手間をサジェスト (FR-13) に商品名を足すことで補う案は採らなかった。

筆者が置いた実装上の判断は次のとおりである。

- 経路は `/purchases/new?:from` とし、`Route::PurchaseNew { from: Option<String> }` にする (`/register?:token` と同じ書き方)。
  `from` が無いときは従来どおり空の新規フォームである。
  パスのセグメント (`/purchases/:id/duplicate`) にしなかったのは、新規フォームの画面と経路を増やさず、`from` の有無だけで同じ画面を使うためである。
- 引き継がない項目の値は、購入日は当日 (新規フォームの既定値)、Roast Date と写真は空、価格と重量は空、通貨は既定の JPY とする。
  Roast Date は袋ごとに変わるため引き継がない。
  価格と重量は所有者の列挙に無く、値上げや容量の変更で変わり得るため引き継がない。
- フォームは `from` の購入を `RecordsApi::purchase` で単件取得して欄を埋める。
  却下した案: 引き継ぐ値をクエリに載せて渡す案。URL に値を載せると改竄でき、URL が長くなり、正は DB にあるため採らない。
  取得が終わるまでは読み込み中の表示にし、終わってから入力を受け付ける。
  引き継いだ値だけの状態は `dirty` にしない (利用者がまだ何も入力していないため。閉じるときに破棄の確認を出さない)。
- 取得の失敗の扱い。
  - 404 (元の購入が他の端末で削除された、古い URL など) は空の新規フォームにし、バナーを出さない。
    再試行しても直らないためである。
  - 401 は既存の扱いでログインへ移る。
  - それ以外の失敗 (通信エラー、500) は空の新規フォームと再試行のバナーを出す。
    再試行は元の購入の取得をやり直し、成功したら欄を埋める。
    利用者が既に入力した値があるときは、再試行の成功で引き継いだ値を入れず、入力を保つ。
- 一覧の行の導線は、`ListRow` に任意の末尾の操作 (`action`: アイコンのボタン 1 つ。星と同じく `stop_propagation` で行の押下と分ける) を足し、購入の一覧だけがそれを渡す。
  `action` は星の左に置き、星は行の右端のまま保つ (`docs/design/components/ListRow/README.md` の「右端に星のボタン」を変えない)。
  却下した案: 購入の行だけの部品を組む案。`ListRow` を抽出、購入、店の一覧が共有しており、購入専用の行を複製すると見た目と操作の規則が 2 か所に分かれるため採らない。
  アイコンは Material Icons Outlined の `content_copy` とし、アクセシブルな名前を「この購入をもとに登録」にする。
  幅 840 px 以上の 2 段組では、`Fab` の新規の登録と同じく右の面に埋め込みのフォームを出し (`from` を渡す)、それ以外は `/purchases/new?from=<購入 ID>` へ遷移する。
- 詳細の導線は、ヘッダーの操作の並びをお気に入り、この購入をもとに登録、編集、削除の順にする (削除を右端に保つ。0056)。
  2 段組の右の面の詳細でも同じで、押すと右の面を新規の埋め込みのフォームに切り替える。
  これで購入の詳細の AppBar の末尾の操作は 4 つになる。`docs/design/components/AppBar/README.md` が定める「右端の操作は最大 3 つ」を「最大 4 つ (購入の詳細のお気に入り、この購入をもとに登録、編集、削除)」に改め、`frontend/src/ui/app_bar.rs` と `frontend/src/screens/app_nav.rs` の「最大 3 つ」の doc コメントも直す (0056 が「最大 2 つ」を 3 つにしたときと同じ扱い)。
  却下した案: 本文 (Ledger の下) にボタンを置く案は、記録への操作をヘッダーに集める既存の形 (お気に入り、編集、削除) と割れるため採らない。
  編集と複製をメニューにまとめる案は、AppBar のメニューの部品が無く、1 操作のために足すのは過剰なため採らない。
- i18n のキー (操作の名前 1 つ) を足す。
- ドキュメント: PRD のユースケースと FR-9 (購入の登録) に導線の記述を足し、`docs/design/components/ListRow/README.md`、`Lists/README.md`、`Detail/README.md`、`AppBar/README.md` と対応する `preview.html` に操作を足す。
  `docs/design/README.md` の AppBar の形 (詳細は「戻る、画面名、お気に入り、編集、削除」) は上限を書いていないため、購入の詳細だけ「この購入をもとに登録」が入る注記を足す (一般形は変えない)。
  E2E のスクリーンショット比較の差分を確認し、意図した差分は `docs/design/screenshots/` の原本を更新し、意図しない差分は直す (比較の結果を issue に記録する)。
- 扱わないこと: 抽出や店の複製、引き継ぐ項目の選択 UI、サジェストへの商品名の追加、Backend の変更 (API は変えない)。

## 完了条件

- `Route::PurchaseNew { from: Option<String> }` があり、`/purchases/new` と `/purchases/new?from=<購入 ID>` の両方を解釈する。
  `frontend/tests/test_router.rs` の経路の台帳と往復の期待値が更新されている。
- `/purchases/new?from=<購入 ID>` で開いた新規フォームに商品名、Producer、Origin、Region、Process、Variety、Flavor Notes、店、Roast が元の購入の値で入り、購入日が当日、Roast Date と写真と価格と重量が空、通貨が JPY である。
  保存すると元の購入とは別の ID の購入が登録される。
- 引き継いだ値だけの状態で閉じると、破棄の確認が出ない。
  1 つでも入力を変えてから閉じると、破棄の確認が出る (既存の挙動)。
- 元の購入の取得が 404 のときは空の新規フォームになり、バナーが出ない。
  通信エラーと 500 のときは空の新規フォームと再試行のバナーが出て、再試行で取得をやり直し、成功したら欄が埋まる。
  再試行の前に入力した値があるときは、再試行の成功で入力が上書きされない。
  401 はログインへ移る。
- 購入の一覧の各行に「この購入をもとに登録」の操作 (アイコン `content_copy`、アクセシブルな名前「この購入をもとに登録」。星の左にあり、星が行の右端) があり、押しても行の押下 (詳細を開く) が起きない。
  幅 840 px 以上では右の面に引き継いだ値の入った新規フォームが出て、幅 840 px 未満では `/purchases/new?from=<購入 ID>` へ遷移する。
- 購入の詳細のヘッダーの操作がお気に入り、この購入をもとに登録、編集、削除の順にあり、押すと引き継いだ値の入った新規フォームが開く (2 段組では右の面)。
- 店、抽出の一覧の行と、店の編集フォーム、抽出の詳細には、この操作が無い。
- `ListRow` の `action` を渡さない一覧の行の見た目と操作が変わらない。
  `frontend/tests/test_records_lists_web.rs` では、購入の詳細の AppBar のアイコンの数が 3 から 4 になり、削除の位置が `icons[2]` から `icons[3]` に変わる。抽出の詳細と他の一覧の期待値は変えない。
- Frontend の単体テスト (`frontend/tests/test_records_forms.rs` に引き継ぐ項目と引き継がない項目の組み立て、`test_router.rs` に経路、`test_records_screens.rs` のルーターの経路の文字列の検査を `/purchases/new?:from` に合わせる) とブラウザテスト (`test_records_lists_web.rs` と `test_records_screens_web.rs` に一覧の行と詳細の操作、2 段組の動き、引き継いだ値、404 とその他の失敗と再試行、破棄の確認の有無) がある。
  ブラウザテストの窓は 1280 px に固定されて狭くできず、1 段組の見た目を再現できないため、1 段組の動きはブラウザテストでは確かめず、E2E で確かめる (次の項目)。
  `frontend/pbt/tests/prop_records.rs` に、引き継ぎの組み立てが元の購入の商品の項目と店と Roast を必ず写し、購入日と Roast Date と写真と価格と重量を写さない性質がある。
- i18n のキーの増減に合わせて `frontend/src/i18n/keys.rs` の `KEY_COUNT` と `ALL`、`ja.rs` と `en.rs` の表、`frontend/tests/test_i18n.rs` の期待値を更新する。
- E2E (`backend/brew_book/tests/wrangler_same_origin_e2e.rs`) に、表示の領域を 390 px にした状態 (`set_window_size` は OS の下限で約 500 px に丸められるため `set_viewport_size` を併用する。`form_layouts` と同じ形) で購入の一覧の行の操作を押すと `/purchases/new?from=<購入 ID>` へ遷移し、商品名と店と Roast が元の購入の値で入る検査がある。
  E2E の下ごしらえの購入は `seed.purchase` で Roast が NULL のため、`seed.purchase_with_roast` で Roast を持つ購入にして、その値が入ることを確かめる。
- E2E のスクリーンショット比較の差分を確認し、意図した差分で `docs/design/screenshots/` の原本を更新し、比較の結果を close 時の「## 解決方法」に記録する。
- PRD のユースケースと FR-9、`docs/design` の `ListRow`、`Lists`、`Detail`、`AppBar` の README と `preview.html` に操作の記述がある。
  `docs/design/components/AppBar/README.md`、`frontend/src/ui/app_bar.rs`、`frontend/src/screens/app_nav.rs` に「最大 3 つ」の記述が無く、末尾の操作の上限が 4 つと書かれている。`docs/design/README.md` の AppBar の形に購入の詳細の注記がある。
- `CHANGES.md` の `## develop` に `[ADD]` のエントリがある。
- 扱わない範囲: 抽出や店の複製、引き継ぐ項目の選択 UI、サジェストへの商品名の追加、Backend の変更は扱わない。
- `mise run check` が通る。

## 関連

- issue 0057 (商品を購入に統合する。本 issue はその後に実装する。TODO の 1、3、4 つ目の小項目に対応する)。
- issue 0056 (詳細のヘッダーの操作の並び)、0047 (ヘッダーのナビゲーション)。

## 解決方法

- 経路: `frontend/src/router.rs` の `Route::PurchaseNew` に `from: Option<String>` を足し、パターンを `/purchases/new?:from` にした (`/register?:token` と同じ書き方)。
  `PurchaseFormScreen(from)` が `PurchaseForm` に `from` を渡す。
- 引き継ぎの組み立て: `frontend/src/records/forms.rs` に `PurchasePrefill` (商品名、Producer、Origin、Region、Process、Variety、Flavor Notes、店、Roast) と `purchase_prefill(&Purchase)` を足した。
  Flavor Notes は `dedup_tags` で整える。
  購入日、Roast Date、写真、価格、重量、通貨は持たない。
  `frontend/src/records/mod.rs` で再公開した。
- フォーム: `frontend/src/screens/records/purchase_form.rs` の `PurchaseForm` に prop `from: Option<String>` を足した。
  `id` が無く `from` があるときは `RecordsApi::purchase` で元の購入を取得し、終わるまで読み込み中を出す。
  取得できたら `purchase_prefill` の値を欄に入れ、`dirty` は false のままにする。
  404 は空の新規フォームにし、バナーを出さない。
  それ以外の失敗は空の新規フォームの上に再試行のバナー (`retryable_banner`) を出し、再試行は取得をやり直す。
  再試行の開始時点で `dirty` なら、成功しても欄を上書きしない。
  401 は既存の `RecordsApi` の扱い (ログインへ移る) のままである。
  `/purchases/new?from=` のように空の `from` は引き継ぐ元が無いものとして扱い、何も読まずに空の新規フォームにする。
  画面名は `from` の有無によらず「購入を記録」(`PurchaseNewTitle`) である。
- 一覧の行: `frontend/src/ui/list_row.rs` に `ListRowAction { icon, label, on_press }` と `ListRow` の prop `action: Option<ListRowAction>` を足した。
  `action` は `div.act` の `IconButton` (`stop_propagation: true`) として星 (`div.fav`) の左に置く。
  `frontend/src/ui/design.css` に `.row .act` の規則 (36 px、`ink-muted`) を足し、`frontend/src/ui/mod.rs` で `ListRowAction` を再公開した。
  `frontend/src/screens/records/purchase_list.rs` は購入の行に `content_copy` の `action` を渡す。
  押すと、幅 840 px 以上では右の面に `PurchaseForm { from: Some(id), embedded: true }` を出し (`creating_from` シグナル)、幅 840 px 未満では `Route::PurchaseNew { from: Some(id) }` へ遷移する。
  `Fab` は `creating_from` を None にして空の新規フォームを出す。
  右の面の `PurchaseForm` には `from` を `key` に渡し、フォームを開いたまま別の行の操作や `Fab` を押したときにフォームを作り直す (作り直さないと `use_effect` の読み込みが再実行されず、前の購入の値が残る)。
- 詳細: `frontend/src/screens/records/purchase_detail.rs` の `PurchaseDetail` に prop `on_duplicate: Option<EventHandler<()>>` を足し、ヘッダーの操作をお気に入り、この購入をもとに登録 (`content_copy`)、編集、削除の順にした。
  `on_duplicate` が無いときは `Route::PurchaseNew { from: Some(id) }` へ遷移する。
  2 段組の右の面では購入の一覧が `on_duplicate` を渡し、右の面を引き継ぎの新規フォームに切り替える。
- doc コメント: `frontend/src/ui/app_bar.rs` と `frontend/src/screens/app_nav.rs` の「最大 3 つ」を「最大 4 つ (購入の詳細のお気に入り、この購入をもとに登録、編集、削除)」に改めた。
- i18n: `frontend/src/i18n/keys.rs` に `Key::DuplicatePurchaseLabel` を足し (`KEY_COUNT` 238)、`ja.rs` に「この購入をもとに登録」、`en.rs` に "Register from this purchase" を足した。
- E2E の下ごしらえ: `backend/brew_book/tests/support/seed.rs` に `Seeder::purchase_with_roast` (`purchase_with_texts` の Roast だけを渡す薄い包み) を足した。
  `backend/brew_book/tests/wrangler_same_origin_e2e.rs` は店 `E2E の店` と Roast `E2E Medium` を持つ購入を下ごしらえし、`duplicate_from_the_list_and_the_detail` を足した。
- ドキュメント: PRD (`docs/prd/brewbook.md`) に UC-20 と FR-9 の受け入れ条件を足し、`docs/design/README.md`、`docs/design/components/{ListRow,Lists,Detail,AppBar}/README.md` と `preview.html` に操作の記述を足した。
- スクリーンショット: `docs/design/components/{ListRow,Lists,Detail,AppBar}/preview.html` を headless Chrome で描画し、`docs/design/screenshots/{ListRow,Lists,Detail,AppBar}-{paper,night}.png` の 8 枚を置き換えた。
  同じ手順で、変更していない Ledger の paper を描画して commit 済みの原画像と比べたところ、画素は一致しないが見た目は同じだった。
  置き換えた 8 枚にも同じ描画環境の差が混ざる。
  再生成した 8 枚は、購入の行と購入の詳細のヘッダーにコピーのアイコンが出ることを目視で確認した。
  E2E のスクリーンショット比較 (`frontend/target/e2e-screenshots/comparison.json`、55 件) の平均画素差は次のとおり。
  paper: Purchases 14.2、PurchaseDetail 11.3、PurchaseEdit 12.1、ListRow 12.9、AppBar 14.4、WideLayout 11.4、Home 21.3。
  night: Purchases 11.5、PurchaseDetail 14.7、PurchaseEdit 14.9、ListRow 12.0、AppBar 13.3、WideLayout 9.5、Home 12.2。
  (レビューの反映後の最後の実行の値。反映前の実行では paper の Purchases 14.1、ListRow 13.8、AppBar 12.9、night の ListRow 13.1、AppBar 11.7 で、他は同じだった。)
  比較は E2E の撮影 (500x701) とサイズの異なる原画像を突き合わせるため、値は変更前から 0 にならない。
  意図した差分は 2 つで、購入の行の星の左と購入の詳細のヘッダー (星、コピー、編集、削除) のコピーのアイコンと、E2E の購入が Roast を持つようになったことによる購入の詳細の焙煎度の行 (未設定から値) である。
  E2E の撮影 `Purchases-paper.png` と `PurchaseDetail-paper.png` で、コピーのアイコンが出ることを目視で確認した。
- `CHANGES.md` の `## develop` に `[ADD]` を 1 件足した。
  併せて、0057 の `[CHANGE]` のエントリが `### misc` の末尾に置かれていたため、`### misc` の前の通常のエントリ群の末尾へ移した (機能の変更であり misc ではない)。
  0058 の `[ADD]` は種別の順序に従い、この `[CHANGE]` の前に置いた。
- テスト:
  - `frontend/tests/test_router.rs`: `the_purchase_new_route_takes_the_source_purchase_from_the_query` (`/purchases/new`、`/purchases/new?from=p1`、`/purchases/new?from=` の解釈と往復)。
    経路の台帳の期待値を `Route::PurchaseNew { from }` にした。
  - `frontend/tests/test_records_forms.rs`: `the_prefill_copies_the_product_fields_the_shop_and_the_roast_only`、`the_prefill_of_a_bare_purchase_has_only_the_name`。
  - `frontend/pbt/tests/prop_records.rs`: `the_prefill_copies_the_product_fields_and_nothing_else` (任意の購入で商品の項目と店と Roast を写し、購入日、Roast Date、写真、価格、重量が違う購入でも同じ引き継ぎになる)。
  - `frontend/tests/test_records_screens.rs`: `PurchaseFormScreenProps { from }` と経路の文字列 `/purchases/new?:from`。
  - `frontend/tests/test_i18n.rs`: `KEY_COUNT` 238。
  - `frontend/tests/test_records_screens_web.rs`: `the_purchase_form_from_a_purchase_prefills_the_product_fields_the_shop_and_the_roast` (引き継いだ値、購入日が当日、Roast Date と価格と重量と写真が空、通貨が JPY、引き継いだだけの状態で閉じると確認が出ない)、`the_prefilled_purchase_form_confirms_before_discarding_the_changes`、`the_purchase_form_from_a_missing_purchase_is_empty_without_a_banner`、`the_purchase_form_from_a_failed_load_retries_and_then_prefills`、`the_purchase_form_retry_does_not_overwrite_the_typed_values`、`the_prefilled_purchase_form_saves_a_new_purchase` (保存が `/api/purchases` への POST になる)、`the_purchase_form_from_an_empty_source_is_empty_without_a_request`。
  - `frontend/tests/test_records_lists_web.rs`: `only_the_purchase_rows_have_the_duplicate_action` (購入の行にだけあり、星の左にある。抽出と店の行には無い)、`the_row_action_opens_the_prefilled_form_in_the_wide_pane` (行の押下が起きず、右の面に引き継いだ値のフォームが出る)、`the_detail_action_replaces_the_wide_pane_with_the_prefilled_form` (詳細の 4 つの操作の順と、右の面の切り替え)、`the_row_action_rebuilds_the_wide_form_for_another_purchase` (フォームを開いたまま別の行の操作を押すとその購入を読んで作り直し、`Fab` で空になる)。
    `the_detail_screens_have_the_star` は購入の詳細の 4 つのアクセシブルな名前の順を確かめ、購入の削除のテストは `icons[3]` を押す。
    抽出の詳細と店のフォームの期待値は変えていない。
  - `backend/brew_book/tests/wrangler_same_origin_e2e.rs`: `duplicate_from_the_list_and_the_detail` (窓と表示の領域を 390 px にして、購入の一覧の行の操作と購入の詳細のヘッダーの操作のそれぞれから `/purchases/new` と `?from=<購入 ID>` へ遷移し、商品名、Roast、店が元の購入の値で入る)。
- 保存で別の ID の購入が登録されることは、`id` が無い `PurchaseForm` の保存が従来どおり `RecordsApi::create_purchase` (POST) を呼ぶことで満たす (`from` は保存の経路に関与しない)。
- 扱わなかったこと: 抽出や店の複製、引き継ぐ項目の選択 UI、サジェストへの商品名の追加は、この issue では行っていない。
  Backend の本番コードと API は変えていない (Backend 側の変更はテストの支援 `support/seed.rs` と E2E のテストだけである)。
- レビューの反映: 多観点レビューの指摘のうち、2 段組でフォームを開いたまま別の行の操作を押すと前の購入の値が残る問題 (`key` による作り直し)、空の `from` の扱い、引き継いだフォームの保存の検査、詳細の 1 段組の E2E、E2E の `set_window_size` の併用、`ListRowAction` の doc コメントのアイコンの表記、AppBar の `preview.html` の副題を反映した。
  却下したのは、0057 の `[CHANGE]` エントリの移動が完了条件の外であるという指摘 (0057 のレビューで記録した置き場所の誤りの是正であり、本文は変えていない) と、`.row .act` の規則が原本の `bundle.css` に無いという指摘 (`.row .fav` と同じ扱いで、原本は 0054 以降 `design.css` に追随していない) である。

## 検証

`mise run check` の段を順に実行した (2026-10-15 から 2026-10-16)。

- fmt: `cargo fmt --all --check` (backend) が通過した。
  Frontend の fmt は差分が出たため `cargo fmt --all` を当て、再検査で通過した。
- lint: 通過した (clippy 32 分、fuzz の check、絵文字の検査)。
- frontend:build と frontend:lint: 通過した。
- formal: TLC が完了し、エラーは無い。
- frontend:test: 46 binary が通過した。
- frontend:test-web: 1 回目は `the_purchase_form_from_a_purchase_prefills_the_product_fields_the_shop_and_the_roast` が Flavor Notes のチップの文字の比較 (`["berryclose", "floralclose"]`) で失敗した。
  原因は、テストのヘルパー `chip_labels` がチップの `text_content` を読み、削除アイコンの文字 `close` が混ざることにある。
  先頭のテキストノードだけを読むように直した。
  同じ理由で `field_input` も、必須の印を数えないようにラベルの先頭のテキストノードで比べるようにし、`Weight (g)` の期待を `Weight` に直した。
  直した後の単独の再実行は、1 回目がページロードの 300 秒のタイムアウト (`webdriver POST /session/.../url timed out`) で失敗し、2 回目で 10 件が通過した。
  1 回目の失敗は backend の clippy と並行して実行していた負荷によるもので、環境要因である。
  全体の再実行はブラウザの 10 suite、74 件が通過した。
- backend:test: `cargo test --workspace -- --skip wrangler_` の 71 の test result が全て通過した (Backend 側の変更はテストの支援 `support/seed.rs` と E2E のテストだけで、本番コードは変えていない)。
- backend:test-integration: brew_book の 17 suite と brew_book_admin の 1 suite の 207 件が 1 回で全て通過した (`wrangler_passkey_flow` を含む)。
- frontend:test-same-origin: 1 passed (18.89 秒)。
  スクリーンショット比較の結果は「## 解決方法」のスクリーンショットの項に記録した。
- レビューの反映後の再検査 (2026-10-16): fmt (backend) が通過し、Frontend の fmt は差分が出たため `cargo fmt --all` を当てて再検査で通過した。
  frontend:build、frontend:lint、frontend:test (46 binary)、frontend:test-web (12 suite、77 件)、lint (clippy 8 分 32 秒、fuzz の check、絵文字の検査)、backend:test (71 の test result)、frontend:test-same-origin (1 passed、20.17 秒) が通過した。
  formal と backend:test-integration は再実行していない (Backend の本番コードを変えておらず、backend:test-integration の対象の一覧に E2E のテストは含まれない)。
  `key` による作り直しの検査 `the_row_action_rebuilds_the_wide_form_for_another_purchase` は、`key` を外すと要求の数が 3 ではなく 2 で失敗することを確かめてから `key` を戻した。
  lint の絵文字の検査 `rg` は、バックグラウンドの実行で標準入力が unix socket だったため標準入力を読んで止まり、標準入力を `/dev/null` にして再実行した (環境要因で、コードの問題ではない)。
