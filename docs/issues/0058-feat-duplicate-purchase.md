# 過去の購入をもとに新しい購入を登録する導線を足す

Created: 2026-10-14
Model: Claude Fable 5.1

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
