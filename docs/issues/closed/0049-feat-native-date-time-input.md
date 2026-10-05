# 日付と時刻の入力をブラウザ標準の date input と time input に置き換える

Created: 2026-10-03
Model: DeepSeek V4.1 Flash
Completed: 2026-10-05

## 背景

日付と時刻の入力は、現在 `frontend/src/ui/field.rs` の `TextField` が描く `input { r#type: "text" }` で `YYYY-MM-DD` と `HH:MM` を手入力する。右端のアイコン (`icon` prop) は表示だけで、押してもピッカーは開かない。

- 購入の購入日 (`purchased_on`) と焙煎日 (`roast_date`): `frontend/src/screens/records/purchase_form.rs` の `Field { label: Key::PurchasedOnLabel }` と `Field { label: Key::RoastDate }`。
- 抽出の抽出日と時刻: `frontend/src/screens/records/brew_form.rs` の `Field { label: Key::BrewedAtLabel }` と、その隣の空ラベルの `Field`。
- 統計の任意の期間の開始日と終了日: `frontend/src/screens/stats/mod.rs` の `custom_fields` の `TextField { icon: Some("calendar_today") }` 2 つ。
- `docs/design/components/Field/README.md` は「日付は `YYYY-MM-DD`、時刻は `HH:MM` の文字入力とし、右端のアイコンでカレンダーと時計を開く」としているが、アイコンにその動きは無い。

所有者の要望は、日付を yyyy/mm/dd の形式で表示し、手入力ではなく date picker で選べるようにすることである。日付と時刻の方式を所有者に確認し、ブラウザ標準の date input と time input を使う方式が選ばれた (自作のカレンダーピッカーと、スマホと PC で方式を分けるハイブリッドは却下された)。

## 目的

日付はブラウザ標準の date input、時刻は time input で選べるようにする。内部の値は `YYYY-MM-DD` と `HH:MM` のまま (API と検証の形式は変えない)。

## 設計判断

- `frontend/src/ui/field.rs` の `TextField` に `kind: TextFieldKind` の prop を足す。`TextFieldKind` は `Text` (既定)、`Date`、`Time` の 3 つとし、`Date` は `input { r#type: "date" }`、`Time` は `input { r#type: "time" }` を描く。
- どの項目を `Date` と `Time` にするかは次のとおりである。購入日、焙煎日、抽出日、統計の開始日、統計の終了日の 5 つを `Date` にし、抽出の時刻だけを `Time` にする。
- `mono`、`unit`、`area`、`disabled` の扱いは変えない。date と time ではブラウザが持つピッカーの表示に任せるため、`icon` は渡さず、`placeholder` も使わない。値の形式の案内は `Field` の `help` に `Key::DayFormatHint` (「YYYY-MM-DD」) と `Key::TimeFormatHint` (「HH:MM」) を出す。
- 日付の表示形式はブラウザの言語に従う。日本語環境では yyyy/mm/dd、英語環境では mm/dd/yyyy になる。内部の値は常に `YYYY-MM-DD` である。所有者はこの方式を承認した (質問への回答で、日本語環境では yyyy/mm/dd と表示されることを含めて選んだ)。
- 推測 (FR-19) が返す `roast_date` は `YYYY-MM-DD` のため、そのまま date input の値になる。`frontend/src/records/forms.rs` の `apply_purchase_suggestion` は変えない。
- `frontend/src/records/values.rs` の `parse_day`、`parse_time`、`format_day`、`format_time` は変えない。検証の誤り (必須の欠落、実在しない日付) の文言も変えない。
- 原本 (`docs/design/components/Field/README.md` と `docs/design/components/Field/preview.html`) を先に更新し、実装を追随させる。

採らなかった案は次のとおりである。

- 自作のカレンダーピッカー: 全環境で yyyy/mm/dd の表示を固定できるが、カレンダーの描画、月の移動、キーボード操作、テストを自作する分の実装量と不具合の危険が大きいため却下する (所有者の回答で却下)。
- スマホは標準のピッカー、PC は自作のカレンダーのハイブリッド: 実装が 2 系統になり、テストも倍になるため却下する (所有者の回答で却下)。
- 文字入力のまま、アイコンを押したときに自作のピッカーを開く: 所有者の要望は手入力を無くすことであり、文字入力の経路を残す理由が無いため却下する。
- 表示形式を yyyy/mm/dd に固定する: ブラウザ標準の input の表示形式はブラウザの言語に従い、CSS や HTML の属性では固定できない。固定するには自作のピッカーが必要になり、上の理由で却下する。

追加の API 呼び出しと権限は無い。

## 完了条件

- 購入の登録と編集の購入日と焙煎日、抽出の登録と編集の抽出日、統計の任意の期間の開始日と終了日が `input { type: "date" }` で描画される (Frontend の自動テストで `input` の `type` 属性を検証する)。
- 抽出の登録と編集の時刻が `input { type: "time" }` で描画される (Frontend の自動テストで `input` の `type` 属性を検証する)。
- date input の値が `YYYY-MM-DD`、time input の値が `HH:MM` で、保存の検証 (`validate_purchase_form`、`validate_brew_form`、`stats_period_for`) の結果が今までと同じになる (既存の Frontend のテストと PBT が通過する)。
- 日付と時刻を空にしたときは、今までどおり必須の誤りになる (既存の検証のテストが通過する)。
- 推測 (FR-19) の `roast_date` の適用が今までどおり動く (既存の Frontend のテストが通過する)。
- `docs/design/components/Field/README.md` が、日付と時刻は date input と time input を使い、`icon` と `placeholder` を使わず、`help` に `Key::DayFormatHint` と `Key::TimeFormatHint` を出す、という記述になっている。`docs/design/components/Field/preview.html` も date input と time input の見た目になっている。
- E2E のスクリーンショット比較の差分を確認し、意図した差分は `docs/design/screenshots/` を更新し、意図しない差分は直す (比較の結果を issue に記録する)。
- 既存の Frontend のテストと E2E が通過する。
- `mise run check` が通過する。

扱わない範囲: 表示形式 (ブラウザの言語に従う yyyy/mm/dd などの見え方) の自動テストは扱わない。自動テストは内部値の形式を確認する。

## 解決方法

日付と時刻の入力を、ブラウザ標準の date input と time input に置き換えた。

- `frontend/src/ui/field.rs`: `TextFieldKind` (`Text` / `Date` / `Time`) を追加し、`TextField` の `kind` prop で `input { r#type: "date" | "time" }` を描くようにした。date と time では `icon` と `placeholder` を渡さない。
- `frontend/src/ui/mod.rs`: `TextFieldKind` を re-export した。
- `frontend/src/screens/records/purchase_form.rs`: 購入日と焙煎日を `TextFieldKind::Date` にし、`placeholder` をやめ `help: Key::DayFormatHint` にした。
- `frontend/src/screens/records/brew_form.rs`: 抽出日を `Date`、時刻を `Time` にし、`placeholder` をやめ `help: Key::DayFormatHint` / `Key::TimeFormatHint` にした。
- `frontend/src/screens/stats/mod.rs`: 任意の期間の開始日と終了日を `Date` にし、`icon: calendar_today` をやめ `help: Key::DayFormatHint` にした。
- 原本: `docs/design/components/Field/README.md` と `preview.html` を date input / time input の見た目と説明に更新した。
- スクリーンショット: E2E の比較で BrewForm と Field の差分を確認し、`docs/design/screenshots/BrewForm-{paper,night}.png` と `Field-{paper,night}.png` を更新した。
- テスト: `frontend/tests/test_records_forms.rs` に `an_empty_day_or_time_is_a_required_error` を、`frontend/tests/test_stats_web.rs` に `the_custom_period_dates_use_the_native_date_inputs` を、`frontend/tests/test_ui_web.rs` に `the_date_and_time_fields_draw_the_native_inputs` を足した。`frontend/tests/test_records_screens_web.rs` (新規) が購入と抽出の登録・編集の 4 フォームの `input` の `type` と値を検証する。

完了条件の検証:

- 購入の登録と編集の購入日と焙煎日、抽出の登録と編集の抽出日、統計の任意の期間の開始日と終了日が `input { type: "date" }` で描画される: `frontend/tests/test_records_screens_web.rs` の 4 テストと `frontend/tests/test_stats_web.rs` の `the_custom_period_dates_use_the_native_date_inputs`、`frontend/tests/test_ui_web.rs` の `the_date_and_time_fields_draw_the_native_inputs` で `input` の `type` 属性を検証した。
- 抽出の登録と編集の時刻が `input { type: "time" }` で描画される: 同じテストで検証した。
- date input の値が `YYYY-MM-DD`、time input の値が `HH:MM` で、保存の検証の結果が今までと同じ: テストで入力の `value` を検証し、`mise run frontend:test` (native と PBT) が全て通過した。
- 日付と時刻を空にしたときは必須の誤りになる: `an_empty_day_or_time_is_a_required_error` で検証した。
- 推測 (FR-19) の `roast_date` の適用が今までどおり動く: `frontend/src/records/forms.rs` の `apply_purchase_suggestion` は未変更で、既存の Frontend のテストが通過した。
- `docs/design/components/Field/README.md` と `preview.html`: 上記のとおり更新した。
- スクリーンショット比較: `frontend/target/e2e-screenshots/comparison.json` で BrewForm と Field の差分を確認し、原本を更新した。更新直後の比較は mean 0.0 だった。その後の実行で BrewForm (1.2/6.0) に出る差は日付と時刻の値 (実行した日時) が変わることによる差である。Field (6.3/6.3) は、取得 (228x94) と参照 (221x94) の大きさが違うことによる環境差 (スクロールバーなど) が主因である (0048 の「参照と大きさが違う 21 件」と同じ扱い)。
- 既存の Frontend のテストと E2E、`mise run check`: ユーザーの指示により check は 0049 から 0051 の実装が完了した後にまとめて実行した。fmt、lint、frontend:build、frontend:lint、formal、backend:test、backend:test-integration、frontend:test、frontend:test-web、frontend:test-same-origin を段ごとに実行し、全て通過した (2026-10-05)。

方針の方式を保ったままの実装詳細の乖離:

- `TextField` の `icon` と `placeholder` の prop 自体は残し、date / time の呼び出し元が渡さない形にした (issue は「`icon` は渡さず、`placeholder` も使わない」としており、prop の削除までは求めていない)。date / time では `placeholder=""` の属性が出力されるが値は空である。`icon` は 0049 以降どの呼び出し元も渡さなくなり未使用である。
- `docs/design/screenshots/Field-{paper,night}.png` は、プレビュー全体の描画 (760x326、複数の状態) から E2E の `.field` 要素の取得 (221x94、日付欄 1 つ) に置き換わっている (0047 の AppBar と同じ扱い)。このため Field の他の状態 (文字、数値、エラー、サジェスト) の見た目は E2E の比較の対象外になった。

スコープ外とする指摘:

- 年 0000 の日付は API と `parse_day` が受理するが、ブラウザ標準の date input には表示できない (input の値域の制約)。実データでは作れない値のため、今回のスコープ外とする (レビューの指摘、低)。
- date / time で `placeholder=""` の属性が出力されること: ブラウザは date / time の placeholder を無視し、値も空である。呼び出し元が渡さない形で要件を満たしており、属性の出力を条件分岐にする変更は却下する (レビューの指摘、低)。
