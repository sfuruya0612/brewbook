# 日付と時刻の入力をブラウザ標準の date input と time input に置き換える

Created: 2026-10-03
Model: DeepSeek V4.1 Flash

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
