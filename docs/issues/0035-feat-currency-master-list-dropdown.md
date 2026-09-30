# 購入の通貨コードをマスタのプルダウンで選べるようにする

Created: 2026-09-30
Model: deepseek-v4p1-flash

## 背景

購入の登録と編集の画面 (`frontend/lib/screens/purchase_form_screen.dart`) の通貨の欄は、自由入力の `AppTextField` (`_currency`、既定値 `defaultCurrency` = `'JPY'`) である。
保存のときに `_currencyPattern` (`^[A-Z]{3}$`) で検証し、誤りを `l10n.validationCurrency` で示し、`priceCurrency: price == null ? null : currency` を送る。
利用者が選べる通貨の一覧はリポジトリのどこにも無い。
`DropdownButton` と `DropdownMenu` は使われておらず (画面の遷移のメニューに `PopupMenuButton` はあるが、値の選択の欄ではない)、ISO 4217 のコードを知らないと目的の通貨を入力できない。

所有者の要望は「通貨コードはマスターデータとして持っておいてプルダウンで選択できるようにしたい」である。
2026-09-30 の確認で、所有者は次を選んだ。

- 通貨コードのマスタは Flutter 内の固定リストとして持つ (D1 のテーブルと配る API は追加しない)。
- プルダウンに載せるのは主要な通貨 30 種とする (対象と順序は起票の対話で所有者の承認を得て確定した。列挙は「設計判断」にある)。
- 選択肢はコードと通貨名を併記する。
- 対象は購入の登録と編集の画面の通貨の欄で、自由入力をプルダウンに変える。

バックエンドは `validate_currency` (`backend/brew_book_core/src/records.rs`) で形 (3 文字の英大文字) だけを検証し、通貨の一覧を持たない。
PRD FR-9 も「ISO 4217 の 3 文字の英大文字だけを受け付ける」と定めるだけで、値の一覧は定めていない。
保存済みの購入には、この issue で載せる主要な通貨の範囲の外のコード (例: `ETB`) があり得る。
通貨コードは購入の一覧、詳細、抽出の詳細、統計のグラフ、エクスポートで表示され、いずれもコードをそのまま出す (`docs/design/README.md` は「通貨は ISO 4217 のコードをそのまま表示し、記号に置き換えない」と定める)。

`frontend/test/purchase_screens_test.dart` の「価格と重量の検証エラーを表示する」は、購入のフォームの `TextField` を索引で掴んでいる (0 購入日、1 焙煎度、2 焙煎日、3 価格、4 通貨、5 重量)。
通貨の欄をプルダウンにすると `TextField` が 1 つ減る (`DropdownButtonFormField` は Flutter 3.47.5 の `material/dropdown.dart` で `DropdownButton` と `InputDecorator` から成り、`TextField` を含まない)。
フォーム内の `TextField` は 6 個から 5 個になり、重量の欄の索引が 5 から 4 にずれる。
同テストは `l10n.validationCurrency` の表示と、通貨の小文字の入力 (`usd`) が大文字 (`USD`) になることの確認を含むが、どちらも無くなる。

## 目的

利用者が購入の登録と編集で、通貨コードを自由入力せずに、主要な通貨のコードと名前の一覧から選べるようにする。

## 設計判断

- 通貨コードのマスタは、Flutter 内の固定リストとする (`frontend/lib/records/currencies.dart` を新設し、`const List<String> currencyCodes` を置く)。
  採らなかった案: D1 のマスタテーブルと配布 API。ISO 4217 はアプリが管理するデータではなく外部の規格である。
  API を増やすと読込中の表示、API の失敗時の扱い、マイグレーションが増え、選択肢を出すという目的に対して複雑さだけが増える。
- 一覧は主要な通貨 30 種とし、コードの昇順で並べる。
  所有者が 2026-09-30 の起票の対話で、種数だけでなく列挙した 30 種を提示して承認を得て確定した (主要な決済通貨を広く含める)。
  値は `AED`、`AUD`、`BRL`、`CAD`、`CHF`、`CNY`、`CZK`、`DKK`、`EUR`、`GBP`、`HKD`、`IDR`、`INR`、`JPY`、`KRW`、`MXN`、`MYR`、`NOK`、`NZD`、`PHP`、`PLN`、`SAR`、`SEK`、`SGD`、`THB`、`TRY`、`TWD`、`USD`、`VND`、`ZAR` とする。
  採らなかった案: ISO 4217 の全コード (約 180 種。プルダウンが長くなり選択に時間がかかる)、利用頻度の順 (順序の根拠を説明できず、テストで固定しにくい)。
  追加の要望があれば `currencyCodes` と名前を足す。
- 選択肢の表示は「コード 通貨名」(例: `JPY 日本円`) とし、選択中の表示はコードだけにする。
  価格との 2 対 1 の 2 列 (`docs/design/components/RecordForms/README.md`) では名前まで入らないため、`DropdownButtonFormField` の `selectedItemBuilder` で選択中だけコードにする。
  通貨名は ARB (`frontend/lib/l10n/app_ja.arb` と `app_en.arb`) に 30 種分を追加し (`currencyJpy` など)、`currencies.dart` の `currencyName` がコードから引く。
  日本語と英語のどちらも CLDR の displayName に合わせる (日本語の例: `米ドル`、`ユーロ`、`日本円`。英語の例: `US Dollar`、`Euro`、`Japanese Yen`)。
  選択肢の表示文字列は `currencies.dart` の `String currencyOptionLabel(AppLocalizations l10n, String code)` が組み立てる (マスタにあるコードは「コード 通貨名」、マスタに無いコードは名前を付けずコードだけ)。
  ウィジェット側で表示文字列を直書きしない (FR-16 の検査と ARB の規約のため)。
  採らなかった案: 選択中にも名前を出す (幅が足りず省略される)、名前を Dart の定数マップに直書きする (FR-16 の ARB での翻訳の規約に反する)、`intl` の表示名を使う (Flutter の `intl` は通貨の名前のデータを持たない)。
- プルダウンは `AppField` の枠 (項目名、必須、検証の誤り) の中に `DropdownButtonFormField<String>` を置く `AppSelectField` を `frontend/lib/widgets/app_field.dart` に追加する。
  枠の見た目は `InputDecorationTheme` (`paper-sunken`、`line-strong`、`radius-sm`、フォーカスは `crema-ink`、エラーは `signal`) に従う (`docs/design/components/Field/README.md`)。
  選択値の指定には `initialValue` を使う (`value` は Flutter 3.33 で非推奨)。`selectedItemBuilder` と `menuMaxHeight` は Flutter 3.47.5 の SDK (`material/dropdown.dart`) にあることを確認した。
  メニューの高さは `menuMaxHeight` で画面に収まる値に制限する (例: 320)。
  採らなかった案: メニューの高さを指定しない (既定ではメニューが画面の高さいっぱい近くまで伸び、フォームの他の欄が隠れる)、既存の `PickerTile` とダイアログ (参照の選択用であり、30 件のダイアログは操作が 2 段になる)、`DropdownMenu` (文字の入力を伴い、自由入力を再び許す構成になる)、画面の遷移のメニューに使う `PopupMenuButton` の流用 (フォームの欄としての枠と検証の表示を持たない)。
- 編集の互換: 開いた購入の通貨コードがマスタに無いときは、そのコードを選択肢に足して選ばれた状態で表示する。
  選択肢は `currencyOptions(current)` がマスタと現在値から組み立てる。現在値がマスタにあるときはマスタの 1 件だけとし、同じコードを 2 つ作らない (`DropdownButton` 系は選択値と同じ値の項目が 2 つ以上あると assert で落ちる)。保存では選ばれた値をそのまま送る。
  採らなかった案: マスタに無いコードを既定値 (JPY) に置き換える (記録を黙って書き換えることになり、金額の意味が変わる)、そのコードを選択肢に含めない (既存の購入を開いて保存するだけで通貨を選び直すことになる)。
- 価格が無い購入 (通貨コードが null) を編集で開いたときは、既定値の `JPY` を選ばれた状態で表示する (既存の `_load` の `purchase.priceCurrency ?? defaultCurrency` と同じ)。保存では `price_currency` に null を送る (既存の `frontend/lib/api/record_inputs.dart` と同じ)。
  採らなかった案: 未選択の表示にする (保存のときに何を送るかが曖昧になり、既存の `_load` と `_save` の組の挙動も変わる)、通貨の欄を無効にする (価格を入力した後に通貨を選べなくなる)。
- 保存中 (`_busy`) はプルダウンの選択を無効にする (既存の通貨の欄の `enabled: !_busy` と同じ)。
  採らなかった案: 保存中も有効にする (送信中に値を変えると、送った値と画面の表示が食い違う)。
- クライアントの自由入力の検証 (`_currencyPattern` と `validationCurrency` の表示) は削除し、使われなくなる l10n の `validationCurrency` (ARB と生成物) も削除する。
  値はマスタまたは現在値からしか選べず、形式の誤りが起こらないためである。
  バックエンドの `validate_currency` と PRD FR-9 は変えない。
  採らなかった案: バックエンドの検証をマスタとの照合にする (マスタがフロントにしか無く、API の契約を根拠なく狭めることになる)。
- `docs/design` に Select の設計は追加しない (プルダウンの見た目は Field の枠の規格に従う入力の一種であり、新しい見た目の規格を定めるものではない)。
  採らなかった案: `docs/design` に Select の設計 (プレビューを含む) を追加する (アプリの見た目の規格を増やす変更であり、フォームの入力の一種を追加する今回の範囲を超える)。
- 追加の API 呼び出し、権限、外部依存は無い。

## 完了条件

- 購入の登録の画面の通貨の欄がプルダウンになり、選択肢が `currencyCodes` の 30 種と一致し (件数と中身)、コードの昇順に並び、各選択肢が「コード 通貨名」で表示され、既定値として `JPY` が選ばれて表示は `JPY` である (FR-9)。
- プルダウンを開くと、メニューの高さが `menuMaxHeight` の指定 (320) 以下であり、30 種をスクロールして `ZAR` を選べることをウィジェットテストで確認する (メニューの高さは `tester.getSize` で確認する)。
- 30 種すべての通貨名が日本語と英語の両方の ARB にあり、空でなく、コードと同じ文字列でないことを `frontend/test/currencies_test.dart` の単体テストで確認する (日本語と英語の `AppLocalizations` を読み込んで確認する。ARB のファイルは読まないため `dart:io` は要らない)。
  マスタにある `JPY` では `currencyOptionLabel` が `JPY 日本円` を返し、マスタに無いコードでは `currencyName` がコードをそのまま返し、`currencyOptionLabel` の表示もコードだけになることを同じテストで確認する。
- 価格を入力して保存すると、選んだ通貨コードが `price_currency` として送られる。価格を入力しないときは `price_currency` が null になる。
- 編集の画面で、既定値と異なるマスタの通貨 (`USD`) の購入を開くと、`USD` が選ばれて選択中の表示が `USD` だけになり、そのまま保存すると `USD` が送られる。
- 編集の画面で、マスタに無い通貨コードの購入を開くと、そのコードが選択肢に含まれて選ばれた状態で表示され、そのまま保存すると同じコードが送られる (選択肢に同じコードが 2 つ現れない)。
- 編集の画面で、価格が無い購入 (通貨コードが null) を開くと、既定値の `JPY` が選ばれた状態で表示され、保存すると `price_currency` は null になる。
- 保存中は通貨のプルダウンの選択が無効である。
- 通貨の自由入力と `validationCurrency` の表示が無くなり、l10n の `validationCurrency` も削除される。
- `currencyCodes` が設計判断に列挙した 30 種と一致すること (テストに 30 種の並びをそのまま期待値として書く) と、ISO 4217 の 3 文字の英大文字であること、重複が無いこと、コードの昇順であること、`JPY` を含むことを `frontend/test/currencies_test.dart` の単体テストで確認する。
- プルダウンの選択、既定値、価格が空のときの null、編集の互換を `frontend/test/purchase_screens_test.dart` のウィジェットテストで確認する。
- 既存の `frontend/test/purchase_screens_test.dart` の「価格と重量の検証エラーを表示する」を、プルダウンの選択と重量の欄の新しい索引 (5 から 4) に合わせて書き換え、`validationCurrency` と小文字の入力の正規化の確認を削除する。
- `mise run check` が通過する。

扱わない範囲:

- バックエンドの検証 (`validate_currency`) と PRD FR-9 の変更。
- 通貨コードを表示する他の画面 (一覧、詳細、統計、エクスポート) の変更。
- `docs/design` への Select の設計の追加。
