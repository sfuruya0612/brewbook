# 店、商品、購入、抽出の記録を削除できるようにする

Created: 2026-10-08
Completed: 2026-10-09
Model: DeepSeek V4.1 Flash

## 背景

所有者の要望は「登録した内容を削除できるようにしたい。各項目の詳細に削除ボタンを設置して削除をさせたい」である。

現状は次のとおりである。

- 4 つの記録 (店、商品、購入、抽出) の削除は API にも画面にも無い。
  `backend/brew_book_core/src/routes.rs` の台帳の DELETE の経路は 7 つあり、内容はパスキーの削除 (`passkeys_delete`)、お気に入りを外す 4 経路、購入の写真の削除 (`purchases_photo_delete`)、アカウントと全データの削除 (`account_delete`) である。記録そのものを消す経路は無い。
- 抽出と購入の詳細の画面 (`frontend/src/screens/records/brew_detail.rs`、`purchase_detail.rs`) のヘッダーの操作はお気に入りと編集の 2 つだけである。
- 店と商品は詳細画面が無く、編集フォーム (`frontend/src/screens/records/shop_form.rs`、`product_form.rs`) が詳細を兼ねる (`frontend/src/router.rs` の経路は `/shops/:id/edit` と `/products/:id/edit` だけである)。フォームは新規と編集を兼ね、削除の対象になる記録の ID は編集のときだけある。
- 記録の参照は、購入が商品 (必須) と店 (任意) を、抽出が購入 (必須) を参照する。商品は Flavor Notes のタグとの対応 (`product_flavor_tags`) を持ち、外部キー制約のため子の行を先に消す必要がある (`backend/brew_book/migrations/0001_initial_schema.sql`)。購入の写真は `photo_key` で参照する R2 のオブジェクトである (ADR-0003、`backend/brew_book/src/records/photos.rs`)。
- ADR-0006 は「アカウント削除 (FR-15) だけは物理削除とし、利用者に属する全行と R2 の全オブジェクトを削除する」と定めており、記録の削除を足すにはこの決定の更新が要る。
- ADR-0018 はアーカイブ (論理削除) を廃止した。廃止の検討では「新しい削除の API と画面は要求に無い」として物理削除の機能は作らなかった。今回は所有者が削除の機能を要求した。
- 削除の確認の UI は、設定画面のアカウント削除 (`frontend/src/screens/settings.rs`) が `ConfirmDialog` (`frontend/src/ui/feedback.rs`) を使っており、これを記録の削除にも使える。

## 目的

利用者が、店、商品、購入、抽出の記録を、それぞれの詳細 (店と商品は編集フォーム) のヘッダーの削除の操作から、確認の後に削除できる。削除は元に戻せない。

## 設計判断

所有者が 2026-10-08 の会話で決めたことは次の 4 点である (決定の記録は ADR-0020 に置く)。

- 対象は 4 つの記録 (店、商品、購入、抽出) とする。
- 参照があるときは連鎖削除にする。
  - 店: 購入の `shop_id` を NULL にして購入は残す (店は購入にとって任意の項目のため)。
  - 商品: その商品の購入、購入に紐づく抽出、購入の写真 (R2)、商品の Flavor Notes の対応も削除する。
  - 購入: その購入の抽出、購入の写真 (R2) も削除する。
  - 抽出: 他から参照されないため、行の削除だけ。
- 削除の操作は末尾の操作の並びに置く。抽出と購入の詳細はお気に入り、編集、削除の順 (削除が右端)、店と商品の編集フォームはお気に入り、削除、保存の順 (保存が右端の既存の設計は保つ)。
- 削除は確認ダイアログを経て行い、連鎖で消える記録の件数をダイアログに出す。

筆者が置いた実装上の判断は次のとおりである。

- 削除は物理削除とし、204 を返す (アカウント削除と同じ)。
- 削除の操作は、店と商品のフォームでは編集 (ID があるとき) にだけ出し、読み込み中と読み込みの失敗のときは出さない (お気に入りの星と同じ条件)。新規の登録の画面には削除の対象が無いためである。
- 商品の削除で `flavor_tags` の行は消さない (参照されなくなったタグを残す。ADR-0006)。消すのは `product_flavor_tags` の対応だけである。
- 削除は子のテーブルから順に行い、1 つの batch で実行する (ADR-0002)。R2 の写真の削除は D1 の前に行う。R2 の削除に失敗した場合は記録が残ったままになり、利用者が同じセッションで再試行できる。R2 の削除の後に D1 の batch が失敗した場合は、記録は残るが写真だけが消えた状態になる。これも再試行で記録を消せるため許容する (アカウント削除と同じ順序と判断)。
- 写真の削除は R2 の複数キーの削除 (`Bucket::delete_multiple`。1 回に最大 1,000 キー) を使う。worker クレート 0.8.6 に `delete_multiple` があることは、2026-10-08 に cargo レジストリのソース (`worker-0.8.6/src/r2/mod.rs`) で確認した。Workers のサブリクエストの上限 (2026-02-11 の変更で有料は既定 10,000、無料は Cloudflare のサービスへの 1,000。R2 の操作は対象に含まれる。2026-10-08 に Cloudflare のドキュメントで確認) に対して、商品の削除で写真が想定規模の 3,000 件でも削除の呼び出しは 3 回に収まる。
- 店の削除で店の指定を外す購入の `updated_at` は現在時刻にする (行の内容が変わるため。ADR-0006)。
- 影響の件数はサーバーが数える。一覧の API に親 ID の絞り込みが無く、クライアントでは全件を引かないと数えられないためである。
- フロントエンドは、削除の操作を受けてから影響の件数を引き、ダイアログを出す。件数の取得に失敗したときはダイアログを出さず、再試行のバナーを出す。再試行は件数の取得をやり直す。
- 削除の API の呼び出しの応答が失敗のとき (通信エラー、500) は、画面に再試行のバナーを出す。再試行の操作で削除をもう一度実行する (利用者が確認済みの操作のため、確認ダイアログは再度出さない)。通信エラーの場合は、記録が残っている場合と、削除が完了した後に応答が失われて既に消えている場合の両方があり得る。再試行が 404 を返したら、記録は既に無いため (他の端末での削除を含む) 成功と同じ扱いにして一覧に戻る。影響の取得の 404 も同じ扱いにする (再試行を繰り返す行き止まりにしないため)。401 は既存の扱いでログインへ移る。
- 連鎖が 0 件のときのダイアログは、件数に触れない文言にする (「購入 0 件」のような表示はしない)。文言は対象ごとに分ける (店は「購入の店の指定が外れる」、商品は「購入と抽出が消える」、購入は「抽出が消える」で意味が異なるため、共通の文にできない)。
- 削除の後は元の一覧に戻り (幅 840 px 以上の 2 段組では右の面を閉じ)、「削除しました。」のスナックバーを出し、一覧を読み直す。店と商品のフォームでは破棄の確認 (`DiscardConfirm`) を経ずに閉じる (削除済みの記録に「保存せずに閉じますか？」を出さないため)。
- ADR-0020 (記録の削除) を足し、ADR-0006 の「物理削除はアカウント削除だけ」の決定と、検討した選択肢の「カスケード削除 (子の記録が黙って消える)」の却下を部分置き換えにする。連鎖で消える件数を確認ダイアログに出すため、「黙って消える」問題は解消する。
- i18n のキーを足す (確認ダイアログの題 4 種、本文は対象ごとに、抽出 1 種、購入 2 種 (件数なし、抽出の件数あり)、商品 3 種 (件数なし、購入の件数あり、購入と抽出の件数あり)、店 2 種 (件数なし、店の指定が外れる購入の件数あり) の 8 種、削除の完了のスナックバー 1 種。合計 13 種)。`frontend/src/i18n/keys.rs` の `KEY_COUNT`、`frontend/src/i18n/ja.rs` と `en.rs` の表、`frontend/tests/test_i18n.rs` を更新する。
- ヘッダーの右端の操作の上限 (現在は 2 つ。`docs/design/components/AppBar/README.md`) を 3 つに変え、詳細とフォームの記述に削除を足す。同 README の詳細の「アーカイブと編集」は ADR-0018 以降の実装 (星、編集) とずれており、`Detail/README.md` の「AppBar の右端は編集」は星 (0051) の記載が無く、今回の削除の追加で右端が編集でなくなるため、同じ変更で直す。`frontend/src/ui/app_bar.rs` と `frontend/src/screens/app_nav.rs` の「最大 2 つ」の doc コメントも直す。`preview.html` の詳細の枠 (AppBar、Detail、WideLayout) は、アーカイブのアイコンを星に置き換え、削除のアイコンを足す (AppBar の副題も直す)。フォームの枠 (AppBar、RecordForms) は新規の登録の画面のため削除を出さないままにして、README に「削除は店と商品の編集フォームにだけ出す (購入と抽出の削除は詳細に置く)」と記す。

採らなかった案は次のとおりである。

| 案 | 却下理由 |
| --- | --- |
| 参照があるときは 409 で拒否する | 使い切った豆や閉店した店を消すのに、子の記録を先に全部消す手間が要る。ADR-0006 の検討で「参照されている親の削除を拒否する」を所有者が選ばなかったのと同じ理由である |
| 削除を論理削除 (アーカイブ) にする | アーカイブは ADR-0018 で画面、API、データベースから廃止済みである |
| 店の削除で購入も削除する | 店は購入の任意の項目である。店を消しただけで購入の履歴を失うのは過剰である |
| 確認ダイアログを出さない | 削除は元に戻せないため、FR-15 と同じく明示的な確認を経る |
| 削除の影響の件数をダイアログに出さない | 商品の削除では多数の購入と抽出が消え得る。消える件数を事前に見せないと取り返しがつかない |
| 対象を抽出だけにする (詳細画面がある記録だけにする) | 所有者の要望は「登録した内容」「各項目」であり、店と商品も含む。店と商品は編集フォームに削除を置く |
| 削除の操作を本文の下のボタンにする | 所有者がヘッダー右端を選んだ。記録ごとの操作 (お気に入り、編集、保存) と同じ場所にまとまる |

追加する API は 7 経路である。全て認証が必要で、入力は無い。

| 経路 | 経路の名前 | 応答 |
| --- | --- | --- |
| `DELETE /api/shops/:id` | `shops_delete` | 204 |
| `DELETE /api/products/:id` | `products_delete` | 204 |
| `DELETE /api/purchases/:id` | `purchases_delete` | 204 |
| `DELETE /api/brews/:id` | `brews_delete` | 204 |
| `GET /api/shops/:id/delete-impact` | `shops_delete_impact` | `{"purchases": N}` |
| `GET /api/products/:id/delete-impact` | `products_delete_impact` | `{"purchases": N, "brews": M}` |
| `GET /api/purchases/:id/delete-impact` | `purchases_delete_impact` | `{"brews": M}` |

- 存在しない ID と他の利用者の ID は、削除も影響の取得も 404 にする (ADR-0006)。
- 影響の応答の `purchases` は削除される (店では店の指定が外れる) 購入の件数、`brews` は削除される抽出の件数とする。店の削除では抽出は消えないため `brews` を含めない。

## 完了条件

- 抽出と購入の詳細、店と商品の編集フォームのヘッダーに削除の操作がある。店と商品の新規の登録の画面には出ない。読み込み中と読み込みの失敗のときも出ない。
- 確認の後に記録が消える。削除の後は元の一覧に戻り (2 段組では右の面が閉じ)、「削除しました。」のスナックバーが出て、一覧が読み直される。
- 商品の削除で、その商品の購入、購入に紐づく抽出、購入の写真 (R2)、商品の Flavor Notes の対応が消え、`flavor_tags` の行は残る。購入の削除で、その購入の抽出と写真が消える。店の削除で、購入の店の指定だけが外れて購入は残る。抽出の削除で、その抽出だけが消える。
- 削除の確認ダイアログに、連鎖で消える記録の件数 (購入 N 件、抽出 M 件。店は店の指定が外れる購入の件数) が出る。連鎖が 0 件のときは件数に触れない文言になる。
- 店と商品のフォームでは、削除の後に破棄の確認が出ない。
- 影響の件数の取得に失敗したときはダイアログが出ず、再試行のバナーが出る。再試行で件数の取得をやり直せる。
- 削除の API の失敗 (通信エラーと 401 以外の API エラー。401 は既存の扱いでログインへ移る) では再試行のバナーが出て、再試行で削除をやり直せる。削除の API と影響の取得の 404 は成功と同じ扱いで一覧に戻る。
- 他の利用者に属する記録の削除と削除の影響の取得は 404 を返し、記録は消えない。
- SQL の組み立ての単体テストが `backend/brew_book_core/tests/test_query.rs` にあり、削除 4 種と影響 3 種の SQL と束縛する値を確かめる。新しい SQL が `backend/brew_book/tests/test_queries.rs` の検査 (表示名を取得しないこと、`SELECT *` を使わないこと) の対象に入る。
- 結合テストが `backend/brew_book/tests/wrangler_records_api.rs` (店と商品) と `wrangler_purchases_brews_api.rs` (購入と抽出) にあり、204、連鎖削除の実体 (商品 1 件に購入 N 件と抽出 M 件)、影響の件数と実際に消える件数の一致、他の利用者の 404、未認証 401 を確かめる。どちらのファイルも `mise.toml` の `backend:test-integration` に登録済みであり、新しいファイルは作らない。R2 の写真が消えることは、ハーネスの `put_r2_object` と `get_r2_object` で確かめる。
- Frontend の単体テストが `frontend/tests/test_records_api.rs` にあり、削除 4 種と影響 3 種の経路、メソッド、応答の読み取りを確かめる。ブラウザテストは `frontend/tests/test_records_lists_web.rs` に足し、削除の操作 (ヘッダーのアイコン、確認ダイアログ、件数の表示、確認の前に削除の API を呼ばないこと、スナックバー、一覧の読み直し、2 段組の右の面を閉じること、削除の失敗と再試行、影響の取得の失敗と再試行、404 を成功と同じ扱いにすること) を確かめる。同ファイルのヘッダーのアイコンの数の期待値 (編集フォームは 1、詳細は 2) を、削除の追加後に合わせて更新する。
- i18n のキー (確認ダイアログの題 4 種と本文 8 種、スナックバー 1 種) を足し、`frontend/src/i18n/keys.rs` の `KEY_COUNT` と `ALL`、`frontend/src/i18n/ja.rs` と `en.rs` の表、`frontend/tests/test_i18n.rs` の期待値 (237 + 13 = 250) を更新する。
- 台帳 (`backend/brew_book_core/src/routes.rs`) とスイート (`backend/brew_book/tests/support/mod.rs`) が一致し、スイートの doc コメントの経路を追加した issue の列挙に 0056 (記録の削除の 7 経路) を足す。
- PRD に FR-23 (記録の削除) と受け入れ基準が書かれ、FR-5 が更新される。スコープの「やること」に記録の削除を足し、ユースケースに記録の削除を足す。目的の「登録、閲覧、編集」の列挙に削除を足し、背景と課題に 2026-10-08 の決定の段落を足す。関連資料に ADR-0020 の行を足し、ADR-0006 の行に部分置き換えの注記を足す。ADR-0020 が追加され、ADR-0006 の Status が ADR-0018 と ADR-0020 の両方による部分置き換えを指し (ADR-0018 の記載を消さない)、本文冒頭の注記も両方を指し、「残る」の一覧から「物理削除はアカウント削除だけ」を外す (ADR-0020 が部分置き換えにする対象として書く)。
- デザインの原本 (`docs/design/components/AppBar/README.md`、`Detail/README.md`、`RecordForms/README.md`、`WideLayout/README.md`、`docs/design/README.md`、`frontend/src/ui/app_bar.rs` と `frontend/src/screens/app_nav.rs` の doc コメント、`AppBar`、`Detail`、`WideLayout` の `preview.html` の詳細の枠) が、設計判断の並び (詳細は お気に入り、編集、削除。店と商品の編集フォームは お気に入り、削除、保存。購入と抽出のフォームは保存だけ) を記す。`preview.html` の詳細の枠はアーカイブのアイコンを星に置き換え、削除のアイコンを足す。E2E のスクリーンショット比較の差分を確認し、意図した差分 (詳細のアーカイブから星への置き換えと削除のアイコン、店と商品の編集フォームの削除のアイコン) は `docs/design/screenshots/` の原本 (AppBar、Detail、RecordForms、WideLayout) を更新し、比較の結果を issue に記録する。Home の AppBar は末尾の操作が無く、E2E で差分が出ないことを確認する。
- CHANGES.md に追記する。
- `mise run check` が通過する。
- staging での確認は不要である (外部サービスの呼び出しが無く、正常系を CI で実行できるため)。

## 解決方法

- 台帳 (`backend/brew_book_core/src/routes.rs`) に 7 経路を足した。
  - 削除の 4 経路: `shops_delete`、`products_delete`、`purchases_delete`、`brews_delete` (認証あり、入力なし、正常系は CI)。
  - 削除の影響の 3 経路: `shops_delete_impact`、`products_delete_impact`、`purchases_delete_impact`。
  スイート (`backend/brew_book/tests/support/mod.rs`) に同じ種別を足し、doc コメントの列挙に 0056 を足した。
- `brew_book_core::query` に削除と影響の SQL を足した (`shop_delete`、`shop_clear_purchases`、`shop_delete_impact`、`product_delete`、`product_purchases_delete`、`product_brews_delete`、`product_purchase_photo_keys`、`product_delete_impact`、`purchase_delete`、`purchase_brews_delete`、`purchase_delete_impact`、`brew_delete`)。
- Worker にハンドラを足した (`records::shops::delete` と `delete_impact`、`records::products::delete` と `delete_impact`、`records::purchases::delete` と `delete_impact`、`records::brews::delete`)。子の行は 1 つの batch で子から消し、R2 の写真は `records::photos::delete_objects` (複数キーの削除、1 回 1,000 キー) で D1 の前に消す。
- Frontend に `RecordsApi` の削除 4 種と影響 3 種、`DeleteImpact`、`DeleteConfirm`、`record_error_not_found` を足した。抽出と購入の詳細、店と商品の編集フォームのヘッダーに削除を足し (店と商品は編集のときだけ)、確認の後に削除し、失敗は再試行のバナー、404 は成功と同じ扱い、2 段組では右の面を閉じる。
- i18n の 13 キーを足した (`KEY_COUNT` 250)。
- PRD に FR-23 を足し、FR-5、目的、ユースケース、スコープ、背景、関連資料を更新した。ADR-0020 を足し、ADR-0006 の Status と本文冒頭の注記を ADR-0018 と ADR-0020 の両方を指すようにした。
- デザインの原本 (AppBar、Detail、RecordForms、WideLayout の README と preview.html、`docs/design/README.md`、`app_bar.rs` と `app_nav.rs` の doc コメント) を更新し、スクリーンショット (AppBar、Detail、RecordForms、WideLayout の paper と night) を再生成した。
- CHANGES.md に 1 件を足した。
- テスト: `test_query.rs` に SQL の検査、`test_queries.rs` に台帳の検査、`wrangler_records_api.rs` と `wrangler_purchases_brews_api.rs` に結合テスト (削除の正常系、連鎖削除、影響の件数、R2 のオブジェクトの削除、404、401)、`test_records_api.rs` に経路の検査、`test_records_lists_web.rs` にブラウザの検査 (確認の前に API を呼ばないこと、件数の表示、2 段組の面を閉じること) を足した。

## 検証

- `brew_book_core` の単体テスト: `test_query.rs` の新しい検査 (削除 4 種と影響 3 種の SQL、`conditions` の 12 文) を含めて通過した。
- バックエンドのネイティブテスト: `cargo test --workspace -- --skip wrangler_` が通過した (台帳とスイートの照合の `api_suite` と、SQL の台帳の `test_queries` を含む)。
- フロントエンドのネイティブテスト: `frontend:test` が通過した (`test_records_api` の削除 4 種と影響 3 種の検査、`test_i18n` の `KEY_COUNT` 250 を含む)。
- フロントエンドの Web テスト: `frontend:test-web` が通過した。新しい 4 件 (`the_delete_confirmation_gates_the_request`、`the_purchase_delete_shows_the_brew_count_and_deletes_on_the_confirmation`、`the_shop_form_delete_shows_the_affected_purchases`、`the_wide_layout_closes_the_detail_pane_after_the_delete`) を含む。
- 結合テスト: `wrangler_records_api` (53 件) と `wrangler_purchases_brews_api` (44 件) が通過した。新しい検査 (店 4、商品 4、購入 5、抽出 3) を含む。他のスイート (d1_binding、dev_server、account、auth、display_name、export、maps、passkey_flow、photos、purchase_suggestions、records_scale、same_origin_api、stats_api、stats_scale、suggestions、admin) も通過した。`wrangler_stats_scale` は一括実行の途中で外部から SIGTERM で停止したため (0054、0055 と同じ環境の問題)、suite を分けて単独で再実行して通過を確認した。
- E2E: `frontend:test-same-origin` が通過した。1 回目は `document.fonts.ready` の script timeout (0052 と同じ環境の問題) で失敗したため再実行して通過した。スクリーンショット比較 (58 件) の結果を確認し、意図した差分 (詳細のアーカイブから星への置き換えと削除のアイコン、店と商品の編集フォームの削除のアイコン) で `docs/design/screenshots/` の原本 (AppBar、Detail、RecordForms、WideLayout) を更新した。比較の mean は BrewDetail 10.3、PurchaseDetail 12.7、ProductEdit 12.2、ShopEdit 11.9、WideLayout 12.0、AppBar 12.9 (paper。night も同程度) である。Home の AppBar は末尾の操作が無く、今回の変更で比較の対象は変わらない。
- `mise run check`: 一括実行を 4 回試した。1 回目は clippy の指摘 1 件 (修正済み) で失敗し、2 回目以降は結合テストのコンパイル中に環境の約 60 分の制限で SIGTERM された (0054、0055 と同じ環境の問題)。所有者の指示により一括実行を打ち切り、上のとおり全てのスイートを個別に実行して通過を確認した。fmt、lint (clippy と絵文字の検査)、formal (TLC) は check の中で通過した。

## 関連

- ADR-0006 (データモデル。本 issue の ADR-0020 が「物理削除はアカウント削除だけ」の決定と、カスケード削除の却下を部分置き換えにする)
- ADR-0018 (アーカイブの廃止)
- ADR-0020 (記録の削除。新規)
- FR-5 (利用者間のデータ分離)、FR-6 (店)、FR-7 (商品)、FR-9 (購入)、FR-11 (抽出)、FR-23 (記録の削除。新規)
- 0007 (購入と抽出の API)、0009 (写真の R2 の扱い)、0012 (アカウント削除。pending の R2 の削除の規模の論点は、複数キーの削除とサブリクエストの上限の確認で解消できる見込みである。0012 の記録の上限の値 (有料 1,000、無料 50。2026-09-23) は 2026-02-11 の変更より前の値であり、現行のドキュメント (有料は既定 10,000、無料は Cloudflare のサービスへの 1,000。2026-10-08 に確認) と一致しない。0012 の実装の変更はこの issue に含めない)
