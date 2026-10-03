# 幅 375 px のフォームで横スクロールが出るのを直し、入力欄を縦スクロールで扱えるようにする

Created: 2026-10-03
Model: DeepSeek V4.1 Flash

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
