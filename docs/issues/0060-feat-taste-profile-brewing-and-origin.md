# AI の推定に淹れ方、挽き目、精製方法の説明、地域の特色を加える

Created: 2026-11-01
Model: DeepSeek V4.1 Flash

## 背景

issue 0059 で、購入の豆の情報から AI が推定した味と特徴を購入の詳細に表示し、購入に保存する機能を実装した。
所有者は staging で動作確認した (2026-11-01)。
所有者は続けて、味と特徴だけでなく、次も返すことを求めた。

- おすすめの淹れ方
- 豆の引き具合 (挽き目)
- 精製方法の説明
- 地域の特色

現状の推定の応答は、まとめ、Flavor Notes、酸味、甘み、苦味、ボディ、香りの 7 項目を持つ。
入力には精製方法 (Process) と生産国、地域 (Origin、Region) が含まれるため、精製方法の説明と
地域の特色も推定に使える。

## 目的

利用者は、購入の詳細の AI の推定で、味と特徴に加えて、おすすめの淹れ方、挽き目、
精製方法の説明、地域の特色を見られる。推定は保存済みの `taste_profile` に含める。

## 設計判断

- `TasteProfile` に次の 4 項目を足す。値は推測できないとき null にする。
  - `brewing_method`: おすすめの淹れ方。上限 100 文字。
  - `grind_setting`: おすすめの挽き目。上限 80 文字。
  - `process_description`: 精製方法の説明。上限 200 文字。
  - `region_characteristics`: 地域の特色。上限 200 文字。
- プロンプトの JSON のキーとルールに 4 項目を足す。推定に使う項目は変えない。
  - 精製方法の説明は、入力の Process と、それが味に与え得る影響を説明する。
  - 地域の特色は、入力の Origin と Region のコーヒーの一般的な特徴を説明する。
- プロンプトは、指示 (system) と豆の情報 (user) を別のメッセージに分ける。
  豆の情報は `<bean_data>` で囲んだ JSON として渡し、その中の指示に従わないことを指示側に明記する。
- プロンプトに埋め込む豆の情報は、制御文字と `<`、`>`、バッククォートを空白に置き換え、
  連続する空白を 1 つにまとめ、項目ごとの上限で切り詰める。
  - 商品名、Producer、Origin、Region、Process、Variety、Roast は各 200 文字。
  - Flavor Notes は 10 件まで、1 件 40 文字まで。
  - これにより入力の tokens の上限が決まり、1 回の推定が無料枠の大きな割合を消費しない (PRD の「コスト」節)。
- `MAX_TOKENS` を 1536 に増やす。出力の項目が増えるため。
- 購入の応答の `taste_profile` と保存済みの JSON に 4 項目を含める。
  - `purchases.taste_profile` の JSON に足すだけで、データベースのマイグレーションは不要。
  - 0059 で保存した古い推定は 4 項目を持たないため、null として読む。画面では行を出さない。
- 画面の「AI による味と特徴」の表に 4 行を足す。ラベルは i18n のキーを足す。
  - `tasteBrewingMethod` (おすすめの淹れ方)
  - `tasteGrindSetting` (おすすめの挽き目)
  - `tasteProcessDescription` (精製方法の説明)
  - `tasteRegionCharacteristics` (地域の特色)
  - `KEY_COUNT` を 254 にする。
- 応答の JSON、解析の上限、保存済みの購入の応答、画面の表示をテストする。
  AI の正常系は staging で確認する (FR-24 と同じ例外)。
- ADR-0023、PRD、デザインの `Detail`、`CHANGES.md` を更新する。

## 完了条件

- `POST /api/purchases/:id/taste-profile?lang=ja` と `?lang=en` の応答に
  `brewing_method`、`grind_setting`、`process_description`、`region_characteristics` が含まれる。
  推測できない項目は null になる。
- 各項目の上限を超える値は null になり、上限以内の値は保存される。
- 成功した推定は購入の `taste_profile` に 4 項目を含めて保存され、次の購入の取得で返る。
  古い保存済み推定に 4 項目が無いときは null として読める。
- 購入の詳細の AI の推定に 4 行が表示され、null の項目は行を出さない。
  保存済みの推定でも同じ行を表示する (Frontend のブラウザテストで判定する)。
- プロンプトの指示と豆の情報が別のメッセージになり、豆の情報が `<bean_data>` で囲まれた JSON として渡る
  (単体テストで検証する)。
- 任意の入力から組み立てた user メッセージは、`<bean_data>` のタグが 1 組だけで、豆の情報が JSON として読め、
  各項目が上限以内で、制御文字と `<`、`>`、バッククォートと連続する空白を含まない (PBT で検証する)。
- i18n のキー、`KEY_COUNT`、`ALL`、日本語と英語の表、`test_i18n.rs` が一致する。
- `docs/design/components/Detail/README.md` と `preview.html` に 4 行がある。
- PRD の「味と特徴の推定」の受入基準と「コスト」「セキュリティ」の節、ADR-0023 が更新されている。
  `CHANGES.md` は、未リリースの同じ機能の変更であるため追記しない。
- `mise run check` が通る。
- staging へのデプロイで、実在の購入を指定した推定が 4 項目を含むことを確認し、
  結果をこの issue に記録する。

## 関連

- issue `docs/issues/closed/0059-feat-purchase-taste-profile.md` (元の機能)
- FR-24 (味と特徴の推定)、ADR-0023 (Workers AI の採用と保存)
- ADR-0016 (Workers AI の無料枠)

## 解決方法

- `brew_book_core::taste` の `TasteProfile` に 4 項目と上限の定数を足し、プロンプトの JSON と
  ルールに 4 項目を足した。解析は既存の `text_field` で項目ごとに検証する。
- `brew_book::records::purchase_taste` の `MAX_TOKENS` を 1536 に増やした。
  保存と購入の応答は既存の `taste_profile` の JSON を使うため、経路とスキーマは変えない。
- Frontend の `TasteProfile` に 4 項目を足し、購入の詳細の表に 4 行を足した。
- i18n のキーを 4 個足し、`KEY_COUNT` を 254 にした。
- 単体テスト、PBT、Frontend の API とブラウザのテスト、結合テストの保存済み推定の検査を更新した。
- `brew_book_core::taste` のプロンプトを `system_prompt` と `user_prompt` に分け、`user_prompt` で
  豆の情報を無害化して切り詰めた。上限は `MAX_PROMPT_FIELD_CHARS`、`MAX_PROMPT_NOTES`、
  `MAX_PROMPT_NOTE_CHARS` とする。
- `brew_book::records::purchase_taste` は、system と user の 2 つのメッセージを送る。
- 単体テストと PBT で、無害化、切り詰め、データの境界、指示とデータの分離を検証した。
- PRD、ADR-0023、デザインの `Detail`、`CHANGES.md` を更新した。

## 検証

- `brew_book_core` の単体テスト: `test_taste.rs` の 14 件が通過した。
- バックエンドの PBT: `prop_taste.rs` の 4 件が通過した。
- フロントエンドのネイティブテスト: `test_records_api.rs` の 24 件、`test_i18n.rs` の 9 件、
  `prop_taste.rs` の 2 件が通過した。
- フロントエンドの Web テスト: `test_records_lists_web.rs` の 21 件が通過した。
  保存済みの推定と 4 項目の表示、AI の推定後の 4 項目の表示を含む。
- 結合テスト: `wrangler_taste_profile_api.rs` の 5 件が通過した。
  保存済みの購入の応答から 4 項目を読むことを含む。
- `MISE_JOBS=2 mise run check` が通過した。既定の並列度 8 では、既知の `wrangler dev` の
  ポート競合が `d1_binding` に当たるため、手元だけ並列度を下げた。
- プロンプトの分離と切り詰めを入れた後に実行した検査: `cargo fmt --all --check`、
  `cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace -- --skip wrangler_`、
  `cargo check -p brew_book --target wasm32-unknown-unknown` が通過した。
  結合テストと Frontend のテストは、プロンプトの組み立てを観測しないため再実行していない。
- staging での確認: 未実施。デプロイの権限を持つ環境で実在の購入を指定し、AI の推定の応答と
  保存済みの購入の取得に 4 項目が含まれることを確認する。
