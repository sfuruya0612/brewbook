# 店、商品、購入、抽出の画面と写真のアップロードを作る

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-25
対応 ADR: ADR-0007 (docs/adr/0007-frontend-flutter-web-first.md)、ADR-0003 (docs/adr/0003-photo-storage-r2-presigned-upload.md)
関連 PRD: FR-6 から FR-13、FR-10 のクライアント側、成功指標 (抽出の登録に必要な画面の数)、やらないこと (複数枚の写真、サーバー側の変換)
依存: 0006, 0007, 0008, 0009, 0013

## 背景

ADR-0007 は、利用者向けの画面を Flutter で作ると決めた。
ADR-0003 は、写真の JPEG 変換と長辺 2048 px 以下への縮小をクライアントで行い、変換後のサイズを申告してアップロード用 URL を要求し、PUT の後に完了を通知する 3 回の呼び出しを決めた。
PRD の成功指標は、抽出一覧 (ホーム) から保存完了までの最短経路で表示する画面 (ルーターに登録した画面) が、ホームを含めて 3 つ以内であることを統合テストで数えることを求める。
0006 から 0009 で記録と写真とサジェストの API ができ、0013 で基盤と認証の画面ができた。本 issue が記録の画面を作る。

## 目的

利用者が、店、商品、購入、抽出を画面から登録、閲覧、編集、アーカイブでき、購入に写真を添付でき、自由記述の項目で過去の入力値の候補を受けられるようにする。

## 設計判断

- 画面は、ホーム (抽出一覧)、抽出の詳細と入力、購入の一覧と詳細と入力、商品の一覧と入力、店の一覧と入力とする。
  抽出の詳細から、使った購入、商品、店 (登録している場合) をたどれる (UC-6)。
  購入の詳細に評価の推移のグラフを置く (描画は 0015 が作る)。
- 一覧はアーカイブ済みを含める切り替えを持ち、各行からアーカイブとアーカイブ解除ができる。
- 一覧はカーソル方式の API に対応し、末尾までスクロールすると次のページ (既定 50 件) を読み込む。
  50 件を超える記録にも到達できることをウィジェットテストで確認する (PRD の性能と想定規模)。
  採らない案: 追加読み込みをせず先頭の 50 件だけを表示する (想定規模では大半の記録に到達できない)。
- 商品の入力は Flavor Notes のタグを追加と削除のできる形で扱い、更新では入力した配列で置き換える (FR-8)。
- 自由記述の項目 (商品の Producer、Origin、Region、Process、Variety、購入の Roast、抽出の抽出方法と挽き目) は、入力中にサジェスト API (0008) を呼び、候補を最大 20 件表示する (FR-13)。
  候補の選択は任意とし、候補に無い値も入力できる。
- 日付と日時の既定値は、購入日が端末のタイムゾーンでの当日、抽出日時が現在時刻とする。
  抽出日時は端末のローカル時刻で入力させ、送信時に UTC の ISO 8601 に変換する (API は UTC の文字列を受け取る)。
- 写真は、ファイルの選択、`ImageConverter` インターフェースの後ろの Canvas API による JPEG への変換と長辺 2048 px 以下への縮小、変換後のサイズの取得、`upload-url` の要求、R2 への PUT、完了通知の順で扱う。
  PUT は R2 の S3 互換エンドポイント (別オリジン) に行い、`Content-Type: image/jpeg` を付ける (CORS は 0009 の設定)。
  差し替えと削除の操作を置き、失敗時は再試行を促す表示にする。
  写真は 1 購入につき 1 枚とし、変換はクライアントで行う (PRD のやらないこと)。
  採らない案: `package:image` で変換する (純 Dart で動くが、ADR-0007 は Web で Canvas API を使うと決めた)、サーバー側で変換する (PRD のやらないこと。ADR-0003)。
- 画像の変換のテストは、Canvas を使うため `mise run frontend:test-web` (`flutter test --platform chrome`) に含める。
  長辺 4,000 px の PNG と JPEG を Canvas で生成し、変換の結果が JPEG で長辺 2048 px 以下であることを検証する (FR-10)。
- 画面数の成功指標は、テスト用の API クライアントを差し込んだ統合テストで測る。
  ホームから抽出の保存完了までの最短経路を実行し、ルーターに登録した画面の種類を数える。
  戻る操作は経路に含めず、ダイアログ、ボトムシート、保存完了の通知は画面に数えない。
  採らない案: 手動で数える (回帰を検出できない)、画面を 1 つにまとめる (入力の項目が多く 1 画面に収まらない)。
- 実バックエンドと仮想認証器を使う統合テスト (ログインから抽出の保存まで) は、Flutter の成果物を `wrangler dev` から配信できる 0017 が追加する。
  本 issue の時点では、Flutter の開発サーバーは同一オリジンにならず、アプリのページを `wrangler dev` から配信できないためである。

## 完了条件

- 店、商品 (タグを含む)、購入 (写真と参照を含む)、抽出の一覧、詳細、登録、編集、アーカイブ、アーカイブ解除がウィジェットテストで確認できる。
  各画面の操作をテストで列挙する。
- 抽出の詳細から購入、商品、店をたどれる (UC-6)。
- 自由記述の 8 項目でサジェストの候補が表示され、候補に無い値を入力できる (FR-13)。
- 一覧の追加読み込みがウィジェットテストで確認できる (50 件を超える記録に到達できる)。
- 抽出日時が端末のローカル時刻で入力され、UTC の ISO 8601 で送信される。
- FR-10 のクライアント側の受け入れ基準を満たす。
  長辺 4,000 px の PNG と JPEG から、JPEG で長辺 2048 px 以下の出力が得られること、変換後のサイズを申告すること、URL の発行と PUT と完了通知の 3 回の呼び出しを行うことを `frontend:test-web` のテストで確認する。
- 成功指標を満たす。
  ホームから抽出の保存完了までの最短経路で表示した画面の種類が 3 つ以内であることを統合テストで確認する (ホームを含み、戻る操作とダイアログと通知は数えない)。
- 入力の検証エラー (400) と認証の失効 (401) の表示のウィジェットテストがある。
- `mise run check` が通過する。

## 関連

- 0006 と 0007 が記録の API を作る。
- 0008 がサジェストの API を作る。
- 0009 が写真の API と R2 の設定を作る。
- 0013 がアプリの基盤と認証の画面を作る。
- 0015 が統計のグラフを作り、購入の詳細の評価の推移を描く。
- 0017 が実バックエンドを使う統合テストを追加する。

## 解決方法

記録の画面一式 (店、商品、購入、抽出の一覧、詳細、フォーム)、タグの入力、写真の変換とアップロード、サジェストの入力欄、追加読み込み、画面数の統合テストを追加した。

- `frontend/lib/api/` に `models.dart` (API の応答の型)、`record_inputs.dart` (入力の型と JSON への変換)、`records_api.dart` (記録の API の呼び出しと URL の組み立て) を追加した。
- `frontend/lib/records/` に `record_services.dart` (画面が使う依存の束 `RecordServices` と記録の変更の通知) と `values.dart` (日付、時刻、数の読み書き) を追加した。
- `frontend/lib/photo/` に `image_converter.dart` (インターフェース) と `image_converter_web.dart` (Canvas API による JPEG への変換と長辺 2048 px 以下への縮小)、`photo_picker.dart` と `photo_picker_web.dart` (ファイルの選択)、`photo_uploader.dart` (upload-url の要求、R2 への PUT、完了通知の 3 回の呼び出し) を追加した。
- `frontend/lib/screens/` に `shop_list_screen.dart`、`shop_form_screen.dart`、`product_list_screen.dart`、`product_form_screen.dart`、`purchase_list_screen.dart`、`purchase_form_screen.dart`、`purchase_detail_screen.dart`、`brew_detail_screen.dart`、`brew_form_screen.dart` を追加し、`home_screen.dart` を抽出一覧にした。一覧は `widgets/record_list_view.dart` が末尾のスクロールによる追加読み込み (既定 50 件) とアーカイブの切り替えと通知による読み直しを持ち、フォームは `widgets/record_picker.dart`、`widgets/suggestion_field.dart`、`widgets/day_time_fields.dart`、`widgets/picker_tile.dart`、`widgets/detail_row.dart` で参照の選択と候補の表示と日付と日時の入力を共通化した。
- `frontend/lib/app.dart` と `frontend/lib/router/app_router.dart` に画面と経路を配線し、`frontend/lib/l10n/` の ARB に文言を追加した。
- テストは `frontend:test` が 109 件 (画面のウィジェットテストと単体テスト。`frontend/test/api/records_api_test.dart` の 3 件を含む)、`frontend:test-web` が 116 件 (Canvas の変換、写真のアップロード、JS の変換のテスト。`frontend/test/web/` の新規は 5 件)、`frontend/integration_test/app_test.dart` が 2 件 (起動のスモークテストと、画面数を数える「ホームから抽出の保存完了までの最短経路の画面は 3 つ以内である」) である。

完了条件の検証:

- 店、商品 (タグを含む)、購入 (写真と参照を含む)、抽出の一覧、詳細、登録、編集、アーカイブ、アーカイブ解除: `frontend/test/shop_screens_test.dart`、`product_screens_test.dart`、`purchase_screens_test.dart`、`brew_screens_test.dart`、`home_screen_test.dart` が各画面の操作を列挙して確認した (frontend:test 109 件)。
- 抽出の詳細から購入、商品、店をたどれる (UC-6): `brew_screens_test.dart` の「詳細から購入と商品と店をたどれる」が確認した。
- 自由記述の 8 項目のサジェスト: 商品の 5 項目は `product_screens_test.dart`、購入の焙煎度は `purchase_screens_test.dart`、抽出の抽出方法と挽き目は `brew_screens_test.dart` のテストが、候補の表示と候補に無い値の入力を確認した。
- 一覧の追加読み込み: `record_list_view_test.dart` の「末尾までスクロールすると次のページ (既定 50 件) を読み込む」が 51 件目に到達することを、「続きが無いときは追加の要求をしない」が確認した。
- 抽出日時のローカル時刻の入力と UTC の送信: `brew_screens_test.dart` の「抽出を登録し、抽出日時を UTC の ISO 8601 で送る」と「編集では抽出日時を端末のローカル時刻で表示する」、`values_test.dart` の日時変換が確認した。
- FR-10 のクライアント側: `frontend/test/web/image_converter_test.dart` の「長辺 4,000 px の PNG と JPEG から、JPEG で長辺 2,048 px 以下の出力が得られる」、`photo_upload_test.dart` の「4,000 px の写真を変換し、URL の発行と PUT と完了通知を行う」(サイズの申告と `Content-Type: image/jpeg` の PUT) と「PUT が失敗すると再試行を促す表示の失敗になる」が確認した (frontend:test-web 116 件)。
- 画面数の成功指標: `frontend/integration_test/app_test.dart` の「ホームから抽出の保存完了までの最短経路の画面は 3 つ以内である」が、`/` と `/brews/new` の 2 画面で確認した (ダイアログと通知は数えない)。
- 400 と 401 の表示: 各フォームのテストが 400 を、4 つの画面のテストが 401 でログイン画面へ遷移することを確認した。
- `mise run check`: exit 0 (ベースラインからの新たな失敗は無し)。

方針からの乖離: 方式を変える乖離は無い。レビューの指摘への対応で、次の実装の詳細を直した。

- 編集フォーム 4 画面で、読み込みに失敗した後の再試行で失敗の表示を消してから読み直すようにした (再試行で回復できるようにするため)。
- 日付ピッカーの初期値を範囲 (2000 年から 2100 年) に収めた (入力の検証が許す範囲外の年で `showDatePicker` の前提を満たさないため)。
- 購入の写真の削除に成功したら画面の状態を更新し、変更を通知するようにした (保存が失敗しても再試行で 2 回目の削除にせず、写真が消えたことを一覧と詳細に伝えるため)。
- 抽出と購入の詳細で、アーカイブの切り替えを応答の行で画面に反映するようにした (通知の文言も状態に合わせる)。
- 一覧の `next_cursor` が文字列でも null でもない場合は応答の形式の違反にした (形式の違反で一覧を静かに打ち切らないため)。写真の URL は `ApiClient` の `basePath` を基準にした。
- 一覧の読み込み中に届いた読み直しの要求を保留して、完了後に実行するようにした (古い表示を残さないため)。
- Canvas の `toBlob` が null を返した場合は失敗として返すようにした (完了しないまま待たないため)。
- 使われていない ARB の文言 4 件を削除した。
