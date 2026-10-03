# ヘッダーにアプリの印とハンバーガーメニューを置き、どの画面からでも遷移できるようにする

Created: 2026-10-03
Model: DeepSeek V4.1 Flash
Completed: 2026-10-04

## 背景

幅 840 px 未満の画面では、ホーム以外の記録の画面から他の主要な行き先へ行く手段が無い。統計と設定へはホームのメニューから行けるが、購入、商品、店の一覧のヘッダーからはどの行き先へも行けない。

- ヘッダーは `frontend/src/ui/app_bar.rs` の `AppBar` が担う。`title`、`leading_icon`、`leading_label`、`on_leading`、`actions`、`wordmark` を置く。
- 画面遷移のレールは `frontend/src/ui/wide_layout.rs` の `NavigationRail` にあり、`frontend/src/ui/design.css` の `@media (max-width: 839px) { .wide-layout .rail { display: none; } }` で幅 840 px 未満では表示されない。レールを持つのは `WideLayout` または `WidePage` を使う 6 画面 (ホーム、購入、商品、店、統計、設定) だけである。
- 幅 840 px 未満で一覧のヘッダーから他の行き先へ行けるのは、ホーム (`frontend/src/screens/records/home.rs`) の `AppBar` の末尾にある `more_vert` のメニュー (aria-label が `Key::MenuTooltip` のボタンと `.menu` の面) だけである。このメニューには統計 (`Key::StatsTitle`) と設定 (`Key::SettingsTitle`) も含まれる。購入、商品、店の一覧のヘッダーには他の画面への操作が無い。
- 詳細と編集の画面への遷移は、一覧の行の押下 (`frontend/src/screens/records/home.rs`、`purchase_list.rs`、`product_list.rs`、`shop_list.rs`) と、参照のタイル (`brew_detail.rs`、`purchase_detail.rs`) にある。ただしこれらの画面から統計と設定へは行けない。
- ホームのヘッダーは `wordmark: true` で "brewbook" を表示する。アプリの印 (アイコン) は無い。
- 所有者の要望は、ヘッダーにアプリ名を残してその下に画面名を表示し、どの画面からでもハンバーガーメニューで他画面へ遷移できることである。アプリ名を押すと抽出の画面 (ホーム) に戻れるようにする。

## 目的

記録、統計、設定のどの画面からでも、ヘッダーのハンバーガーメニューから抽出、購入、商品、店、統計、設定へ遷移できる。
ヘッダーの左にアプリの印 (`BrewbookMark`) と "brewbook" を置き、その下に現在の画面名を出す。アプリ名を押すとホーム (`/`) へ戻る。
認証前の画面 (ログイン、登録) は対象外とする (ログイン後の画面へ遷移するメニューを出さない)。

## 設計判断

- ヘッダーの構成は、先頭からハンバーガーボタン、既存の先頭の操作 (戻るまたは閉じる)、アプリの印 (`BrewbookMark`、32 px) とアプリ名 (`Key::AppTitle`)、その下の画面名 (既存の `title` prop)、末尾の操作とする。ホームの `title` は `Key::BrewsLabel` (「抽出」) に変える。
- ハンバーガーボタンとメニューの面は `frontend/src/screens/` の新しい部品 (`AppNav`) が担う。メニューの面は `.menu` と同じ見た目のドロップダウンとし、ハンバーガーの下に開く左寄せの位置に置く (`docs/design/components/bundle.css` と `frontend/src/ui/design.css` の `.menu` の左寄せの規則を足す)。抽出、購入、商品、店、統計、設定の 6 項目とログアウトを置く。項目を押すと対応する経路へ遷移してメニューを閉じる。現在の経路と同じ項目を押したときは遷移せず、メニューだけ閉じる。ログアウトは既存の `crate::auth::logout` (`frontend/src/auth.rs`) を呼び、`SessionStatus::SignedOut` にする (ホームのメニューと同じ)。
- ハンバーガーボタンは記録、統計、設定の全ての画面と全ての幅で表示する。認証前の画面 (ログイン、登録) は対象外とする。登録の画面 (`frontend/src/screens/register.rs`) は `ui::AppBar` のままとし、印とアプリ名と画面名は出るがハンバーガーは出ない。幅 840 px 以上で隠す案は、詳細とフォームの画面 (商品、店、購入、抽出のフォームと詳細) がレールを持たないため、幅 840 px 以上で遷移手段が消える。要望は「どこからでも他画面に遷移できるように」であるため却下する。
- `ui::AppBar` は表示だけを担い、`dioxus_router` に依存しない。`menu: Option<Element>` と `on_home: Option<EventHandler<MouseEvent>>` の prop を足し、`on_home` を省略したときはアプリ名を押しても何もしない。`frontend/src/screens/` の `ScreenAppBar` (新しい部品) が `navigator` と `auth` を使って `AppNav` と `on_home` を組む。各画面は `AppBar` の代わりに `ScreenAppBar` を使う。登録の画面は `ui::AppBar` をそのまま使い、`on_home` を渡さない。`ui::AppBar` が `navigator` を直接呼ぶ案は、デザインシステムの部品が `crate::router` に依存し、既存のブラウザテスト (`frontend/tests/test_ui_web.rs` の `the_app_bar_matches_the_tokens`) が Router を組む必要が生じるため却下する。
- アプリ名の押下は、現在の経路がホームのときは何もしない。それ以外は `navigator.push(Route::Home {})` にする (戻るで元の画面に戻れる)。
- 経路の並びは `frontend/src/screens/records/mod.rs` の `rail_items` が持つ 6 項目を共通の定数 (`nav_entries`) に切り出す。`rail_items` のホームのラベルは現在 `Key::HomeTitle` ("brewbook") であり、`Key::BrewsLabel` (「抽出」) に変える。`NavigationRail` と `AppNav` の両方が `nav_entries` を使う。
- 未保存の入力がある状態でメニューから他画面へ移ると、入力は破棄する (確認のダイアログは出さない)。既存の「閉じる」と同じ挙動に揃える。
- i18n は新しいキーを足さない。ボタンの名前は `Key::MenuTooltip`、アプリ名は `Key::AppTitle`、画面名は既存の `*Title` を使う。
- 原本 (`docs/design/`) は先に更新する。`docs/design/components/AppBar/README.md` と `preview.html`、`docs/design/components/Home/README.md` と `preview.html` (ホームのメニューの記述を削除)、`docs/design/components/Lists/README.md` と `docs/design/components/Settings/README.md` (ホームのメニューへの参照を直す)、`docs/design/components/{Detail,Settings,Stats,Auth,RecordForms,BrewForm,Lists,WideLayout}/preview.html` (AppBar の見本を新しい構成に直す)、`docs/design/components/bundle.css` (ハンバーガーとメニューの見た目) を更新する。`docs/design/components/WideLayout/README.md` は既にレールの 1 項目目を「抽出」としており、直すのは実装 (`rail_items`) の側である。

採らなかった案は次のとおりである。

- ハンバーガーボタンを `use_wide_layout` (`frontend/src/screens/records/mod.rs`) で幅 840 px 未満のときだけ Rust 側で描画する: CSS のメディアクエリと Rust の判定が二重になり、wasm のテストで幅を切り替えないとメニューを検査できない。DOM を常に同じにして表示を切り替えない方がテストしやすく、判定も 1 か所になるため却下する。
- メニューをボトムシート (`frontend/src/screens/records/picker.rs` の `RecordPickerSheet` と同じ `.sheet`) にする: 項目が 7 つあり画面の下半分を占める。遷移のたびに開閉が大きく、一覧の操作を隠すため却下する。
- 幅 840 px 以上で `NavigationRail` を出す画面ではハンバーガーを隠す: 詳細とフォームの画面にレールが無いため、上のとおり却下する。
- ホームの `more_vert` のメニューを残す: 同じ項目のメニューが 2 つになり、押す場所によって挙動が変わるため却下する。ホームのメニューは削除する。

追加の API 呼び出しと権限は無い。

## 完了条件

- `ui::AppBar` がアプリの印 (`BrewbookMark`) とアプリ名 (`Key::AppTitle`)、その下に画面名 (`title`) を表示し、`wordmark` prop を持たず、`dioxus_router` を参照しない (ソースの検査)。
- アプリ名を押すと `Route::Home {}` へ遷移する。現在の経路がホームのときは何もしない (ブラウザの部品テストまたは E2E)。
- 記録、統計、設定の全ての画面が `ScreenAppBar` を使う (ソースの検査)。
- 登録の画面 (`frontend/src/screens/register.rs`) は `ui::AppBar` のままで、ハンバーガーが出ない (ソースの検査)。
- ハンバーガーボタンを押すとメニューが開き、ハンバーガーの下に左寄せで表示され、抽出、購入、商品、店、統計、設定の 6 項目とログアウトが表示される (wasm の部品テスト。メニューの左端の位置も検証する)。
- メニューの各項目を押すと対応する経路へ遷移し、メニューが閉じる。現在の経路と同じ項目を押したときは遷移しない (E2E の `frontend:test-same-origin` で確認する)。
- ホームの `more_vert` のメニューが削除されている (ソースの検査)。
- `rail_items` のホームのラベルが `Key::BrewsLabel` になっている (ソースの検査)。
- `docs/design/components/AppBar/README.md` と `preview.html`、`docs/design/components/Home/README.md` と `preview.html`、`docs/design/components/Lists/README.md`、`docs/design/components/Settings/README.md`、`docs/design/components/{Detail,Settings,Stats,Auth,RecordForms,BrewForm,Lists,WideLayout}/preview.html`、`docs/design/components/bundle.css` が新しい構成になっている。
- E2E のスクリーンショット比較の差分を確認し、意図した差分は `docs/design/screenshots/` を更新し、意図しない差分は直す (比較の結果を issue に記録する)。
- `frontend/tests/test_ui_web.rs` の `the_app_bar_matches_the_tokens` を含む既存のテストを新しい構成に合わせ、全て通過する。
- `mise run check` が通過する。

扱わない範囲: 未保存の入力の保護 (遷移前の確認のダイアログ) は扱わない。

## 関連

- 0039 のデザインシステム (`frontend/src/ui/`) の `AppBar` を拡張する。
- 0041 の記録の画面の `AppBar` の使い方 (`ScreenAppBar` への置き換え) を変える。
- 0044 の E2E (`frontend:test-same-origin`) のメニューの操作を新しいハンバーガーメニューに合わせる。

## 解決方法

ヘッダーを、ハンバーガーメニュー、既存の先頭の操作、アプリの印 (`BrewbookMark`、32 px) とアプリ名、その下の画面名、末尾の操作の構成にし、記録、統計、設定のどの画面からでも 6 つの行き先とログアウトへ移れるようにした。

- `frontend/src/ui/app_bar.rs`: `AppBar` から `wordmark` prop を削除し、`menu: Option<Element>` と `on_home: Option<EventHandler<MouseEvent>>` の prop を足した。先頭に `menu`、続いて既存の先頭の操作、`BrewbookMark { size: 32 }` と `Key::AppTitle` を持つ `.app-name` ボタン、その下の `.ttl` に画面名、末尾に `actions` を置く。`dioxus_router` は参照しない。
- `frontend/src/screens/app_nav.rs` (新規): ハンバーガーボタンと `.menu.left` の面を組む `AppNav`、`navigator` と `auth` を組む `ScreenAppBar`、メニューの項目を組む `nav_items` を追加した。メニューは 6 項目とログアウトを持ち、項目を押すと遷移して閉じ、現在の経路と同じ項目では遷移せずに閉じる。ログアウトは `crate::auth::logout` を呼び、`SessionStatus::SignedOut` にする。
- `frontend/src/screens/records/mod.rs`: 6 つの行き先を `NAV_ENTRIES` に切り出し、`rail_items` と `AppNav` の両方が使うようにした。ホームのラベルを `Key::HomeTitle` から `Key::BrewsLabel` (「抽出」) に変えた。
- 記録、統計、設定の画面 (`home.rs`、`purchase_list.rs`、`product_list.rs`、`shop_list.rs`、`purchase_detail.rs`、`purchase_form.rs`、`brew_form.rs`、`product_form.rs`、`shop_form.rs`、`brew_detail.rs`、`stats/mod.rs`、`settings.rs`) を `ScreenAppBar` に置き換えた。ホームの `more_vert` のメニューは削除した。登録の画面 (`register.rs`) は `ui::AppBar` のままとした。
- `frontend/src/ui/design.css` と `docs/design/components/bundle.css`: `.menu.left { left: 0; right: auto; }` を足し、ハンバーガーの下に左寄せで開くようにした。
- 原本 (`docs/design/components/AppBar/README.md` と `preview.html`、`Home`、`Lists`、`Settings` の README と `preview.html`、`Detail`、`Stats`、`Auth`、`RecordForms`、`BrewForm`、`WideLayout` の `preview.html`、`bundle.css`) を新しい構成に更新した。
- テスト: `frontend/tests/test_app_nav_web.rs` (新規) に `the_app_nav_opens_the_menu_under_the_hamburger` を置き、メニューの開閉、区切りの div が 1 つであること、6 項目とログアウト、ハンバーガーとの左端の一致と下への開きを検証する。`the_app_bar_matches_the_tokens` (`frontend/tests/test_ui_web.rs`) と `frontend/tests/test_ui.rs`、`test_records_screens.rs`、`test_stats_web.rs` を新しい構成に合わせた。E2E は `backend/brew_book/tests/wrangler_same_origin_e2e.rs` に `nav_menu` を足し、6 つの行き先への遷移、現在の経路と同じ項目で履歴が増えないこと、アプリ名でホームへ戻りホームでは履歴が増えないこと、レールを持たないフォームの画面 (`/products/new`) からも移れることを検証する。`backend/brew_book/tests/support/e2e.rs` に `click_menu_item` と `wait_menu_closed` を足した。

完了条件の検証:

- `ui::AppBar` が印とアプリ名と画面名を表示し、`wordmark` prop を持たず `dioxus_router` を参照しない: `frontend/src/ui/app_bar.rs` を確認し、`frontend/tests/test_ui_web.rs` の `the_app_bar_matches_the_tokens` で構成を検証した。
- アプリ名を押すと `Route::Home {}` へ遷移し、ホームでは何もしない: E2E の `nav_menu` が `.appbar .app-name` の押下でホームへ移ることと、ホームでの再押下で `window.history.length` が変わらないことを検証した。
- 記録、統計、設定の全画面が `ScreenAppBar` を使う: `frontend/src/screens/` を検索し、`ui::AppBar` を使うのは登録の画面だけであることを確認した。`frontend/tests/test_records_screens.rs` の `the_signed_in_screens_use_the_screen_app_bar` が 12 ファイルを検査する。
- 登録の画面は `ui::AppBar` のままでハンバーガーが出ない: `frontend/src/screens/register.rs` を確認した。
- ハンバーガーでメニューが開き、ハンバーガーの下に左寄せで 6 項目とログアウトが出る: `the_app_nav_opens_the_menu_under_the_hamburger` が `.menu` の項目数と、ハンバーガーの左端とメニューの左端の差 (1 px 未満)、メニューの上端がハンバーガーの下端以降であることを検証した。
- メニューの各項目で対応する経路へ移り、メニューが閉じ、現在の経路と同じ項目では遷移しない: E2E の `nav_menu` が 6 経路への遷移と `.menu` の消滅、`/purchases` での履歴長の不変を検証した。
- ホームの `more_vert` のメニューが削除されている: `frontend/src/screens/records/home.rs` を確認し、E2E と部品テストも `Menu` の aria-label を使う新しいメニューだけを操作する。
- `rail_items` のホームのラベルが `Key::BrewsLabel` になっている: `the_navigation_entries_start_with_the_brews_label` が `NAV_ENTRIES[0].0` を実行時に確かめる (`rail_items` と `AppNav` が共有する値)。
- `docs/design/` の原本が新しい構成になっている: 上記のとおり更新した。
- スクリーンショット比較: `frontend/target/e2e-screenshots/comparison.json` で、更新した原本は Home、AppBar、BrewForm、Detail、Lists、RecordForms、Stats、Settings、WideLayout の 9 種 (18 ファイル) で、対応する比較 (BrewDetail、Purchases、ProductEdit を含む) は mean 0.00 である。残差は PurchaseDetail 10.3/9.6、Products 1.4/1.2、Shops 1.8/1.6、ShopEdit 4.2/2.5 (paper/night) で、複数の画面が 1 つの原本を共有することによる内容の差である。その他の部品 (Button、Chip、Rating、Feedback、ReferenceTile、Field、Ledger、Charts、ListRow、Auth) の比較は 0044 のハーネスの既存の差で、今回の変更の対象外である。
- 既存の Frontend のテストと E2E、`mise run check` が通過する: `mise run check` が通過した (2026-10-04、2643 秒。fmt、lint、frontend:build、frontend:lint、formal、backend:test、backend:test-integration、frontend:test、frontend:test-web、frontend:test-same-origin の全てが成功)。

レビューの指摘を受けて変えたもの (方式は変えていない):

- 統計の画面 (`frontend/src/screens/stats/mod.rs`) に、ヘッダーのメニューのログアウトの失敗を伝える通知 (`notice` と `Snackbar`) を足した。統計の画面だけ通知の表示が無く、失敗が利用者に伝わらなかった (レビューの指摘、中)。
- メニューのブラウザテストを `frontend/tests/test_ui_web.rs` から `frontend/tests/test_app_nav_web.rs` に移した (`src/screens/app_nav.rs` に対応する置き場。レビューの指摘、低)。
- `the_navigation_entries_start_with_the_brews_label` を、実装の行の写しではなく `NAV_ENTRIES[0].0` を実行時に確かめる形にした (レビューの指摘、低)。
- `docs/design/components/AppBar/preview.html` のメニューの区切りを、実装と同じ組み方 (空の `div.sep` とログアウトの `div`) に直した (レビューの指摘、低)。
- E2E の `nav_menu` に、レールを持たないフォームの画面 (`/products/new`) からメニューで店へ移る検査を足した (レビューの指摘、低)。

却下した指摘:

- メニュー項目のキーボード操作 (role、tabindex、キーの処理) の追加: 完了条件に無く、以前のホームのメニューと同じ組み方の引き継ぎである。今回のスコープ外の改善として報告する (レビューの指摘、低)。
