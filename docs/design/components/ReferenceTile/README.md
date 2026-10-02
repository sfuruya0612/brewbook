# ReferenceTile

抽出から購入、商品、店へたどるタイルと、フォームで参照先を選ぶピッカー。

- たどるタイル: `roast-soft` の地、`radius-md`、左に種類 (`caption`、`ink-muted`)、その下に名前 (`heading`)、右端に `chevron_right`。押すとその記録の詳細へ (UC-6)。抽出の詳細では購入、商品、店の順に縦に並べ、店が無い購入では店のタイルを出さない。
- 購入のタイルの名前は「購入日 / 重量 / 価格」を「/」でつなぐ (商品名は上に出ているので繰り返さない)。
- ピッカー: フォームで購入、商品、店を選ぶ入口。`Field` と同じ枠 (`paper-sunken`、`line-strong`) に種類と選択中の名前、右端に `chevron_right`。未選択は「購入を選ぶ」「店を指定しない」を `ink-faint` で。押すとボトムシート (`radius-lg` の上の角) に一覧を出して 1 つ選ぶ。
- 店は任意なので、ピッカーの一覧の先頭に「店を指定しない」を置く (FR-9)。
- Dioxus では `frontend/src/ui/reference_tile.rs` で `.tile` と `.chain`、ピッカーは `.picker`、選択のシートは `.sheet-scrim` と `.sheet` のクラスを組む。
