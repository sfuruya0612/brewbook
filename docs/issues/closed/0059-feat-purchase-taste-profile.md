# 購入の豆の情報から AI が推定した味と特徴を表示し、保存する

Created: 2026-11-01
Model: DeepSeek V4.1 Flash
Completed: 2026-11-01

## 背景

所有者の要望は「豆の情報からどんな味、特徴があるか AI で検索して表示するようにしてみたい」である。
購入は商品名、Producer、Origin、Region、Process、Variety、Flavor Notes、Roast を持ち
(ADR-0022)、購入の詳細はそれらを帳簿の表に表示している。
写真からの購入の項目の推測 (FR-19) は Cloudflare Workers AI の vision モデルを使っている
(ADR-0016)。味と特徴の推定には、購入に保存済みの豆の情報を使える。

2026-11-01 に所有者は次を求めた。

- 購入の豆の情報から、どんな味や特徴があるかを AI に調べさせて、購入の詳細に表示する。

所有者は同日、次の点を決めた (ADR-0023)。

- 推定は Cloudflare Workers AI の言語モデルで行い、外部 (Cloudflare 以外) の AI サービスや
  Web の検索は使わない。「AI に検索させる」は、豆の情報とコーヒーの一般的な知識から
  味と特徴を推定することを指す。
- 推定は保存し、次に購入の詳細を開いたときに表示する。
- AI の推定であることと、実際の味と異なる場合があることを画面に明記する。
- 推定の言語は UI の言語 (日本語か英語) に合わせる。
- 課金は発生させず、Workers AI の無料枠の範囲で動かす。

## 目的

利用者は、購入の詳細で、その豆の情報から推定された味と特徴
(まとめ、Flavor Notes、酸味、甘み、苦味、ボディ、香り) を見られる。
推定は購入に保存し、次に購入の詳細を開いたときも表示する。

## 設計判断

所有者の要望と決定は上の「背景」の通りである (ADR-0023)。

筆者が置いた実装上の判断は次のとおりである。

- 経路は `POST /api/purchases/:id/taste-profile?lang=<ja|en>` とする。
  - 認証を必須にし、未認証は 401、言語が無いか `ja` と `en` 以外のときは 400、
    存在しない購入と他の利用者の購入は 404 を返す (FR-5)。
  - 本文は送らない。言語はクエリパラメータで渡す (FR-22 の住所の補完と同じ形)。
  - 正常系は Workers AI の推論を要するため CI では実行せず、staging へのデプロイで
    実在の購入を指定して確認する (FR-19 と同じ扱い。経路の台帳は `OkTest::Staging`)。
- 応答は `summary`、`flavor_notes`、`acidity`、`sweetness`、`bitterness`、`body`、`aroma` とする。
  - `summary` は 400 文字以内 (プロンプトでは 2 から 4 文を指示する)、`flavor_notes` は 10 件以内で 1 件 40 文字以内、
    特徴は 80 文字以内とする。
  - 推測できない特徴は null、Flavor Notes は空の配列にする。
  - 表示できる内容が 1 つも無いときは 500 を返す (AI の失敗と同じ扱い)。
- 成功した推定は購入の `taste_profile` に JSON の文字列で保存する。
  - 購入の応答は `taste_profile` に保存済みの推定のオブジェクトを含め、未生成のときは null にする。
  - 推測をやり直すと保存済みの推定を上書きする。失敗したときは前の推定を残す。
  - マイグレーション `0006_add_purchase_taste_profile.sql` で purchases に列を足す。
    purchases の列なので、購入の削除とアカウント削除で一緒に消え、エクスポートに含まれる。
- 入力は購入の豆の情報だけとする。店、価格、重量、写真は使わない。
  - 豆の情報は JSON としてプロンプトに埋め込み、その中の指示に従わないこと、
    特定の農園や受賞を捏造しないこと、不確かなことを断定しないことをプロンプトに明記する。
  - 入力と出力はログに出さない (FR-19 と同じ扱い)。
- モデルは vars の `AI_TASTE_MODEL` とし、既定値は写真からの推測と同じ
  `@cf/meta/llama-4-scout-17b-16e-instruct` にする。AI バインディング (`AI`) は既存のものを使う。
  `max_tokens` は 800、`temperature` は 0.2 とする。
- 画面は購入の詳細の帳簿と評価の推移の間に「AI による味と特徴」の節を置く。
  - 購入の応答に保存済みの推定があれば、画面を開いたときに表示する。AI は呼ばない。
  - 保存済みの推定が無いときは、説明と「AI に聞く」のボタンを出す。
  - 押すと推定中の表示を出し、結果をまとめ、Flavor Notes のタグ、特徴の表、
    AI の推定である旨の注記で表示する。推定後はボタンを「もう一度聞く」にする。
  - 失敗したときは「味と特徴を推定できませんでした」のバナーと再試行の操作を出す。
    保存済みの推定があるときはそれを残す。古い応答は捨てる。
  - 保存済みの推定は購入の応答から signal に入れ、お気に入りの切り替えで購入を読み直しても消さない。
- i18n のキーを 12 個足す (節の題、説明、ボタン 2 種、推定中、失敗、注記、特徴の 5 項目)。
  `KEY_COUNT` を 250 にし、`Key::ALL`、日本語と英語の表を更新する。
- デザインの原本 (`docs/design/components/Detail/README.md` と `preview.html`) に節を足す。
  CSS は既存の `.section`、`.chips`、`.ledger` を使い、新しいクラスを足さない。
- PRD に FR-24、目的、ユースケース (UC-21)、スコープ、成功指標、性能、コスト、セキュリティ、
  関連資料を足す。ADR-0023 を追加する。`CHANGES.md` に `[ADD]` を足す。
- 扱わないこと: Web の検索、外部の AI サービス、購入のフォームへの推定の表示、
  推定の履歴、利用者による推定の評価 (フィードバック)。

## 完了条件

- `POST /api/purchases/:id/taste-profile?lang=ja` と `?lang=en` が認証付きで動き、
  未認証 401、言語の入力不正 400、存在しない購入と他の利用者の購入の 404 を返す。
  AI の失敗は 500 を返し、入力 (豆の情報) と出力をログに出さない。
- `brew_book_core::taste` にプロンプトの組み立て、言語の検証、応答の解析があり、
  単体テストと PBT が通過する。
- 結合テスト `wrangler_taste_profile_api.rs` が通過し、`mise.toml` の
  `backend:test-integration` に含まれる。経路の台帳とスイートが一致する。
- `AI_TASTE_MODEL` が `wrangler.toml` のトップレベル、staging、production にある。
- 購入の詳細に「AI による味と特徴」の節があり、保存済みの推定があれば開いたときに表示する。
  保存済みの推定が無いときは AI を呼ばず、
  「AI に聞く」で `POST /api/purchases/<ID>/taste-profile?lang=<UI の言語>` を本文なしで呼び、
  まとめ、タグ、特徴、注記を表示する。失敗のバナーと再試行がある。
  Frontend のブラウザテストで判定する。
- `0006_add_purchase_taste_profile.sql` が purchases に `taste_profile` を足し、
  購入の応答、エクスポート、スキーマのテストが新しい列に追随している。
  保存済みの推定が購入の応答に含まれることを結合テストで確認する。
- 購入を切り替えたときに前の推定を残さず、2 段組の右の面でも作り直す。
  Frontend のブラウザテストで判定する。
- 抽出の詳細の 2 段組でも、別の抽出を選んだら詳細を作り直す (レビューで見つけた同型の不具合)。
  Frontend のブラウザテストで判定する。
- i18n のキー、`KEY_COUNT`、`ALL`、日本語と英語の表、`test_i18n.rs` が一致する。
- `docs/design/components/Detail/README.md` と `preview.html` に節がある。
- PRD、ADR、issue、`CHANGES.md` が更新されている。
- `mise run check` が通る。
- staging へのデプロイで、実在の購入を指定した正常系が FR-24 の形の応答を返し、
  次の購入の取得で保存済みの推定が返ることを確認し、結果をこの issue に記録する
  (Workers AI の推論を要するため。FR-19 と同じ例外)。

## 関連

- ADR-0023 (Accepted)
- FR-24 (追加する要求)、FR-19 (CI で正常系を実行できない扱いを同じにする)
- ADR-0016 (Workers AI の採用と無料枠と検証)
- ADR-0022 (購入が豆の情報を持つ)
- 0041 (購入の詳細の画面)、0055 (AI を呼ぶ経路の入力の検証と CI の例外の前例)
- 0034 (写真からの推測の実装と CI の例外の前例)

## 解決方法

- `brew_book_core::taste` を追加した。豆の情報 (`BeanInfo`) から JSON を埋め込んだプロンプトを
  組み立てる `prompt`、`ja` と `en` を検証する `validate_language`、モデルの応答から
  最初の JSON を取り出して項目ごとに検証する `parse_response`、`TasteProfile` を持つ。
- Worker に `records::purchase_taste` を追加した。購入を利用者 ID で引き (他の利用者の ID は 404)、
  Flavor Notes を付けてプロンプトを組み立て、AI バインディング (`AI`) の
  `@cf/meta/llama-4-scout-17b-16e-instruct` (vars の `AI_TASTE_MODEL` で差し替え可能) を呼ぶ。
  応答が空のときは 500 を返す。入力と出力はログに出さない。成功した推定は
  `query::purchase_set_taste_profile` で購入の `taste_profile` に JSON で保存し、
  保存に失敗したときは前の推定を残して 500 を返す。
- マイグレーション `0006_add_purchase_taste_profile.sql` で purchases に `taste_profile` を足した。
  購入の応答 (`PurchaseResponse`) は保存済みの JSON を `TasteProfile` に戻して `taste_profile`
  として返し、保存済みの推定が無いときは null にする。エクスポートの購入の列にも加えた。
- 経路の台帳に `purchases_taste_profile` を追加し、`SUITE` に未認証 401 と入力不正 400 を
  追加した。正常系は staging で確認する区分にした。
- Frontend に `TasteProfile` の型、`RecordsApi::taste_profile`、`ApiClient::post_empty_json` を
  追加した。購入の `taste_profile` を `Purchase` の型で読み、購入の詳細の帳簿と評価の推移の
  間に節を置く。保存済みの推定は購入の読み込み時に表示し、押したときだけ推定し直す。
  詳細は購入の ID を鍵にした 1 件の一覧として描き、別の購入を選んだら作り直す。
  抽出の詳細も同じ形の 1 件の一覧にして、別の抽出を選んだら作り直す。
- i18n の 12 キー、`wrangler.toml` の `AI_TASTE_MODEL`、結合テスト
  `wrangler_taste_profile_api.rs`、`mise.toml` のテストの一覧を追加した。
- PRD の FR-24 と関連する節、ADR-0023、デザインの `Detail`、`CHANGES.md` を更新した。

## 検証

- `brew_book_core` の単体テスト: `test_taste.rs` の 8 件と `test_routes.rs` の 6 件が通過した。
- バックエンドの PBT: `prop_taste.rs` の 3 件が通過した。
- バックエンドのネイティブテスト: `cargo test --workspace -- --skip wrangler_` が通過した
  (台帳とスイートの照合を含む)。
- フロントエンドのネイティブテスト: `test_records_api.rs` の 24 件と `test_i18n.rs` の 9 件が
  通過した。Frontend の PBT `prop_taste.rs` の 2 件も通過した。
- フロントエンドの Web テスト: `frontend:test-web` が通過した
  (`test_records_lists_web.rs` の 21 件。保存済みの推定の表示、味と特徴の推定の表示、
  開いただけでは AI を呼ばないこと、失敗の再試行、購入を切り替えたときに前の推定が残らないこと、
  2 段組の右の面で別の購入や抽出を選んだときの作り直しを含む)。
- 結合テスト: `wrangler_taste_profile_api.rs` の 5 件が通過した
  (未認証 401、言語の入力不正 400、存在しない購入と他の利用者の購入の 404、AI の失敗の 500、
  保存済みの推定が購入の応答に含まれること)。
  `wrangler_export_api.rs` の 5 件と `backend:test-integration` の全 suite も通過した。
- E2E: `frontend:test-same-origin` が通過した。
- `mise run check`: 既定の並列度 8 では `backend:test-integration` の `d1_binding` が
  `wrangler dev` のポート競合 (`Address already in use`) で失敗した。これは新機能と無関係な
  環境の問題である。README の案内に従って手元だけ並列度を下げ、`MISE_JOBS=2 mise run check` で
  fmt、lint、formal、build、frontend:test、frontend:test-web、frontend:test-same-origin、
  backend:test、backend:test-integration が全て通過した。
- staging での確認: 所有者が staging で動作確認した (2026-11-01)。
  実在の購入で推定が返り、その後の購入の取得で保存済みの推定が `taste_profile` に含まれることを
  確認した。
