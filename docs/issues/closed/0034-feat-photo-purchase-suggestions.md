# 写真から購入と商品の項目の推測を追加する

Created: 2026-09-30
Model: deepseek-v4p1-flash
Completed: 2026-10-01
対応 ADR: ADR-0016 (docs/adr/0016-photo-inference-with-workers-ai.md)
関連 PRD: FR-19、FR-7、FR-9、FR-10、成功指標 (API の自動テストの網羅)、未確定論点 (推測に使うモデルと費用、AI バインディングと結合テスト)、やらないこと (推測の自動保存、外部の AI サービスへの写真の送信)、非機能要求のコスト
依存: 0001, 0006, 0009, 0014

## 背景

2026-09-30 に PRD へ FR-19 を追加し、ADR-0016 が、写真から購入と商品の項目を推測し、結果を利用者が確認して適用すると決めた。
推測は Cloudflare Workers AI の vision モデルを、利用者向けの Worker の AI バインディングから呼んで行う。
workers-rs 0.8.6 に AI バインディングの API (`env.ai("AI")` と `Ai::run`) があるため、Rust のクレートは追加しない。

0006 が商品の API と一覧、0009 が写真の API と R2 の設定を作り、0014 がクライアントの JPEG 変換と購入の画面を作った。
本 issue が推測の API と、画面への推測の反映、商品の照合のための一覧の絞り込みを追加する。

## 目的

利用者が、購入の写真を選ぶだけで、袋に書かれた商品と購入の項目の候補を受け取り、確認して反映できるようにする。
推測に失敗しても手入力で続けられ、記録は利用者が保存するまで変わらない。

## 設計判断

### 最初に行う検証

- AI バインディングを持つ `wrangler.toml` で、ログインの無い環境の `wrangler dev` が起動し、既存の結合テストが通ることを確認する。
  通らない場合は、バインディングの定義の場所 (トップレベルと環境) や CI の起動方法を決め、ADR-0016 に追記する (PRD の未確定論点)。
- 最小の Worker で、画像の渡し方 (messages 内の画像の表現、バイト列、base64) を候補モデルに対して試し、Worker のメモリと CPU の消費を確認する。
  分かった形式を issue に記録する。
- 実写真 (日本語のラベルのもの 3 枚以上) で候補モデル (`@cf/meta/llama-3.2-11b-vision-instruct`、`@cf/moondream/moondream3.1-9B-A2B`、`@cf/meta/llama-4-scout-17b-16e-instruct`、`@cf/mistralai/mistral-small-3.1-24b-instruct`、`@cf/qwen/qwen3.8-27b`) を比較し、採用モデルと 1 回あたりの Neurons を決めて issue に記録する。
  結果は vars の既定値に反映する。

### API

- 経路は `POST /api/purchase-suggestions` とし、認証を必須にする (未認証は 401)。
  台帳の経路名は `purchase_suggestions` とする。
  FR-13 の `GET /api/suggestions/<項目>` (過去の入力値のサジェスト) とは別の機能である。
- 入力は変換済みの JPEG の本体とし、Content-Type を `image/jpeg`、サイズを 5 MB (5,000,000 バイト) 以下とする。
  それ以外は 400 を返し、AI を呼ばない (AI を呼ばないため CI でテストできる)。
  サイズの上限は FR-10 の申告サイズと同じ値を使う。
- 応答は FR-19 の項目を持つ JSON とする。
  キーは既存の API の列名に揃え、商品は `product` の入れ子にする (購入の応答と同じ形)。
  推測できない項目は null とする。
  商品の項目が 1 つも推測できないときは `product` を null にする。

  ```json
  {
    "product": {
      "name": "エチオピア イルガチェフェ",
      "producer": null,
      "origin": "エチオピア",
      "region": null,
      "process": "ウォッシュト",
      "variety": null,
      "flavor_notes": ["フローラル"]
    },
    "roast": "中煎り",
    "roast_date": "2026-09-20",
    "price_amount": null,
    "weight_grams": 200
  }
  ```

- 価格は整数のみとし、通貨は推測しない (画面の既定値の JPY のまま)。
- AI の呼び出しに失敗したとき (無料枠の超過、タイムアウト、モデルのエラー) は 500 を返す。
  クライアントは失敗を表示し、手入力を続けられる。
  失敗の理由は区別しない。
- 応答の項目はサーバー側で検証する。
  前後の空白を除いて空の文字列は null、文字列の長さは商品名 200 文字、その他 100 文字、Flavor Notes は 20 件まで、Roast Date は `YYYY-MM-DD` の実在する日付、価格と重量は 0 以上とする。
  形式に合わない項目だけを null にし、他の項目は返す。
- 推測の入力 (写真) と出力はログに出さない (FR-19、PRD のセキュリティ)。
- 採らない案: アップロードの完了通知 (FR-10) の応答で推測を返す (商品を選び終えた後になり、商品の項目を活かせない。ADR-0016)、R2 の `pending/` のオブジェクトを推測に使う (購入への紐づけの変更が要る。ADR-0016)。

### 経路の台帳とテストの照合

- `brew_book_core/src/routes.rs` の `ROUTES` に `purchase_suggestions` を足す。
- 正常系は CI で実行できないため、台帳に「正常系は手元で確認する経路」の区分を足す (例: `Route` の `ok_test` を `Ci` と `Manual` に分ける)。
  `test_requirements`、`backend/brew_book/tests/support/mod.rs` の `SUITE`、`backend/brew_book/tests/api_suite.rs` の照合を合わせ、正常系の項目を要求しないようにする。
  スイートの項目は未認証 401 と入力不正 400 を持つ。
- PRD の成功指標の例外 (写真からの推測の API は正常系の照合の対象外) は追記済みである。

### 推論

- `wrangler.toml` に AI バインディング (`AI`) を追加する。
  バインディングは環境に継承されないため、トップレベル (ローカル) と `[env.staging]` と `[env.production]` のそれぞれに置く。
  ローカルはリモートの推論に接続する (wrangler の remote binding)。
- モデルは vars の `AI_SUGGEST_MODEL` で指定し、候補の比較の結果を既定値にする。
- プロンプトの組み立てと応答の解析は `brew_book_core` に置く。
  応答は JSON を要求し、前後の説明文やコードフェンスを許して最初の JSON オブジェクトを取り出す。
  解析できないときは全項目 null を返す。
- 出力の tokens を抑えるため、応答の JSON の形式を固定し、項目を増やさない。
  出力が長いモデル (qwen3.8 など) は費用への影響が大きいため、比較の対象にする。

### 画面

- 購入のフォームで、写真の選択と変換 (0014) の直後に推測を呼ぶ。
  変換の結果のサイズが 5 MB を超えるときは呼ばない (FR-19)。
  推測中はインジケータを出し、結果はまだ空の入力欄にだけ入れる。
  入力済みの値は上書きしない。
  失敗したときはバナーで表示し、手入力を続けられる。
- 商品は、商品が未選択のときだけ、推測した商品名と前後の空白を除いて一致する (大文字と小文字を区別しない) アーカイブされていない商品を選択する。
  選択済みの商品は上書きしない。
  照合には、商品の一覧の API に足す名前の完全一致の絞り込み (`name`) を使う (1 リクエストで引く)。
  絞り込みの実装は、共有の `ListParams` に足すか、商品の一覧だけが読むクエリとして足すかを実装で決める。
- 一致する商品が無いときは「推測した内容で商品を登録」の導線を出す。
  商品のフォームに推測値 (商品名、Producer、Origin、Region、Process、Variety、Flavor Notes) を引き継ぎ、保存した商品を結果として返せるようにする (既存の呼び出しは結果を使わない)。
  購入のフォームは、押し出しの画面 (`Navigator.push`) で商品のフォームを開き、戻ったときに選択された商品を反映する。
  2 段組でも押し出しの画面を使い、右の面の入れ替えは使わない (入れ替えると購入のフォームの未保存の状態が破棄されるため)。
  2 段組で商品を登録して戻っても、選択した写真、推測の反映、入力値が残ることをウィジェットテストで確認する。
- 推測は保存を伴わない。
  保存のときに呼ぶのは写真のアップロードの 3 回 (URL の発行、PUT、完了通知。0014) のままで、推測は写真を選んだときだけ呼ぶ。
- 文言は ARB の日本語と英語に追加する (FR-16)。

### テスト

- `brew_book_core` の単体テスト: プロンプトの生成、応答の解析 (正常な JSON、コードフェンス付き、JSON の前後の説明文、不正な JSON、型違い、範囲外の値、未知のフィールド)。
- PBT は `backend/pbt/tests/prop_<module>.rs` に置き、解析の結果が検証の条件 (長さ、形式、範囲) を満たすことを確認する。
- Fuzzing は `backend/brew_book_core/fuzz/fuzz_targets/parse_strings.rs` の対象に応答のパーサを足し、任意の入力でパニックしないことを確認する (0028 で広げた対象に合わせる)。
- 結合テスト (`wrangler dev`): 未認証の 401、Content-Type の 400、5 MB 超の 400。
  商品の一覧の名前の絞り込み (一致、前後の空白と大文字小文字の無視、アーカイブ済みの除外)。
  AI を呼ぶ正常系は CI で実行できない (Cloudflare のアカウントとネットワークが要る。0009 の R2 の検証と同じ扱い) ため、手元で実写真を使って確認し、結果を issue に記録する。
  500 の経路は、vars の `AI_SUGGEST_MODEL` に存在しないモデル名を注入して手元で確認する。
- Flutter のウィジェットテスト: 空欄への反映、入力済みの値の保持、失敗時の表示と手入力の継続、未選択のときの一致による商品の選択、選択済みの商品の保持、一致が無いときの導線、2 段組で商品の登録から戻ったときの状態の保持、推測が保存を伴わないこと。
- 画面数の成功指標 (抽出の登録の最短経路) は、商品の登録の導線が購入の画面に増えても影響しないことを確認する。

## 完了条件

- FR-19 の API の受け入れ基準を満たす (200、401、400、500 の経路。応答の検証と null への落とし込み)。
  500 は vars の注入による手元の確認でよい。
- 商品の一覧の名前の絞り込み (`name`) の結合テストがある。
- 経路の台帳とテストの照合が、正常系の手元確認の区分を付けて通る。
- 実写真での候補モデルの比較の結果 (採用モデル、精度の所見、1 回あたりの Neurons の実測) を issue に記録し、vars の既定値に反映する。
- ウィジェットテストが、推測の反映 (空欄だけ)、入力済みの値の保持、失敗時の手入力の継続、商品の選択と未登録時の導線、2 段組での状態の保持、推測が保存を伴わないことを確認する。
- `mise run check` が通過する。
- `CHANGES.md` の `### misc` に `[ADD]` のエントリがある。
- 手元の `wrangler dev` と実写真で、推測が返り、画面に反映されることを確認し、結果を issue に記録する (CI では実行できないため)。

## 未確定論点

- 採用するモデルと、1 回あたりの Neurons の実測 (PRD の未確定論点)。
  2026-09-30 に実測した (「## 解決方法」)。採用は `@cf/meta/llama-4-scout-17b-16e-instruct` (実写真 3 枚と合成ラベルでの比較で、約 62 Neurons)。解消済み。
- 画像の渡し方 (messages 内の画像の表現) と、送信する画像の大きさ。
  変換済みの JPEG (長辺 2048 px) をそのまま使う方針とし、精度が足りない場合だけ推測用の縮小を検討する。
  2026-09-30 に、`messages` の `image_url` (data URL) で動作することをデプロイした probe で確認した (モデルにより `response` と `choices[0].message.content` の両方の形で返る)。画像は変換済みの JPEG をそのまま使う。解消済み。
- 商品の名前の絞り込みを共有の `ListParams` に足すか、商品の一覧だけが読むクエリとして足すか。
  リクエストの読み取りは商品の一覧だけが読む `NameFilter`、SQL の組み立ては共有の `ListQuery` と `QualifiedList` に `name` を足す形にした (乖離 4)。解消済み。
- 失敗の理由 (無料枠の超過とその他の失敗) を区別せず 500 と表示する扱いで足りるか。
  初回は区別しない。500 の経路は結合テストで確認する (`wrangler_purchase_suggestions_fails_500_without_the_remote_binding`)。解消済み。

## 関連

- 0001 が Worker の基盤と依存の方針を作った。
- 0006 が商品の API と一覧を作った。
- 0009 が写真の API と R2 の設定を作った。
- 0014 が写真の変換と購入の画面を作った。
- 0028 が Fuzzing の対象を入力の文字列パーサに広げた (応答のパーサも同じ対象に足す)。
- 0017 が同一オリジンの配信を追加した (pending にあるが実装済み。手元の手動確認はローカルの `wrangler dev` でできる)。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. 結合テストのハーネスは `wrangler dev --local` で起動する (`backend/brew_book/tests/support/mod.rs` と `backend/brew_book_admin/tests/support/mod.rs`)。AI バインディングは起動時にリモートのプロキシのセッションを開くため、これが無いとログインの無い CI で `wrangler dev` が起動しない (実装で確認した)。`--local` では AI の呼び出しは失敗し、推測の API は 500 を返すため、500 の経路は結合テストで確認する。開発者の `mise run dev` は `--local` を付けない。ADR-0016 に追記した。
2. バインディングに `remote = true` は付けない。この環境ではリモートのプロキシのセッションの起動が、アカウントの選択や workers.dev のサブドメインの条件で失敗し、`wrangler dev` が起動しなくなる (実装で確認した)。
3. 経路の台帳 (`brew_book_core/src/routes.rs`) に `OkTest` (`Ci` と `Manual`) を足した。`purchase_suggestions` は `Manual` とし、`test_requirements` は `Manual` の経路に正常系のテストを要求しない。`brew_book_admin` の台帳のテストも `Route` の項目に合わせた。
4. 商品の名前の絞り込みは、リクエストの読み取りを商品の一覧だけが読む `NameFilter` に閉じた (`brew_book/src/records/products.rs`)。SQL の組み立ては共有の `ListQuery` と `QualifiedList` に `name` を足し、他の一覧は `name: None` を渡す (`brew_book_core/src/query.rs`)。複数回の指定と、空白だけの値は 400 にする。
5. 推測の応答の解析は、最初の `{` から順に JSON として読めるかを試す (説明文の中の `{` や、壊れた JSON の後に続くオブジェクトを飛ばすため)。
6. モデルの応答の形は 1 つではない。`response` に文字列かオブジェクトで返る形と、OpenAI 互換の `choices[0].message.content` に文字列で返る形があるため、`suggestion::output_text` で両方を扱う。実経路の検証で判明した (当初は `response` だけを読んでいた)。
7. `ProductFormScreen` の `_save` が保存した商品を `onSaved` に渡すようにし、`onSaved` の型を `ValueChanged<Product>` に変えた (推測した内容で登録した商品を購入のフォームへ返すため。`createProduct` と `updateProduct` は既に `Product` を返していた)。既存の呼び出し元 (`product_list_screen.dart` など) を追随させた。
8. 2 段組で押し出しの画面から戻るときは、商品のフォームで保存の通知 (SnackBar) を出さない。下の面の Scaffold にも同じ SnackBar が出て遷移が失敗するためである (通知は購入の保存のときに出す)。
9. 結合テストの一覧 (`mise.toml` の `backend:test-integration`) に `wrangler_purchase_suggestions_api` を足した (テストの実行に必要な変更)。
10. モデルの既定値は、比較が終わるまでの暫定として `@cf/mistralai/mistral-small-3.1-24b-instruct` を置き、2026-09-30 の実写真での比較の結果、`@cf/meta/llama-4-scout-17b-16e-instruct` に置き換えた。
11. 2026-09-30 の実装環境では、ローカルの `wrangler dev` の AI バインディングの呼び出しが `internal error` で失敗した (テキストのモデルでも同じ)。このため、実写真での候補モデルの比較と AI の応答の形の確認は、デプロイした使い捨ての Worker (probe) で行い、アプリの経路 (完了条件 8) の確認は staging で行った (乖離 14。結果は「## 解決方法」)。Worker のメモリと CPU の消費の確認は実施できていない。
12. 応答の形を PBT と Fuzzing でも検査するため、`backend/pbt` の dev-dependencies と `backend/brew_book_core/fuzz` の dependencies に `serde_json` を足した (どちらも `brew_book_core` の依存に既に含まれるクレートで、応答の JSON の値を作るために使う)。
13. 完了条件 7 の `CHANGES.md` のエントリは、機能の追加のため `### misc` ではなく `## develop` の通常のエントリ群に置く (implement-issues の CHANGES.md への追記の規則)。
14. 完了条件 8 の「手元の `wrangler dev` と実写真で、推測が返り、画面に反映されることを確認する」は、この環境ではローカルの `wrangler dev` の AI バインディングの呼び出しが失敗するため、staging の実経路で実写真から推測の JSON が返ることを確認し、画面への反映はウィジェットテスト (推測の応答の形を固定したフェイクの API を差し込む) で確認した、と読み替える。

## 解決方法

写真から購入と商品の項目を推測する API と画面を追加した (FR-19、ADR-0016)。

- `brew_book_core/src/suggestion.rs` を追加した。応答の JSON の形を固定したプロンプト、`response` と OpenAI 互換の `choices[0].message.content` の両方の形の応答から出力のテキストを取り出す `output_text`、最初の JSON オブジェクトを取り出す解析、項目ごとの検証 (商品名 200 文字、その他 100 文字、Flavor Notes 20 件、`YYYY-MM-DD` の実在する日付、0 以上の整数)、画像の `data:image/jpeg;base64,...` URL の生成を持つ。
- `brew_book/src/records/purchase_suggestions.rs` を追加し、`POST /api/purchase-suggestions` を実装した。Content-Type が `image/jpeg` でない入力、本体が空の入力、5 MB を超える入力は AI を呼ばずに 400、AI の失敗は 500 を返す。推測の入力と出力はログに出さない。
- `brew_book_core/src/routes.rs` の経路の台帳に `purchase_suggestions` と `OkTest` (`Ci` と `Manual`) を足した。`test_requirements`、`SUITE`、`api_suite.rs`、`brew_book_admin` の台帳のテストを合わせた。
- 結合テストのハーネス (`brew_book/tests/support/mod.rs` と `brew_book_admin/tests/support/mod.rs`) の `wrangler dev` に `--local` を足した。AI バインディングは起動時にリモートのプロキシのセッションを開くため、これが無いとログインの無い CI で `wrangler dev` が起動しない (実装で確認した。ADR-0016 に追記した)。
- `wrangler.toml` に AI バインディング (`AI`) と vars の `AI_SUGGEST_MODEL` をトップレベルと staging と production に足した。既定のモデルは実写真での比較で `@cf/meta/llama-4-scout-17b-16e-instruct` に決めた。
- 商品の一覧の API に名前の完全一致の絞り込み (`name`) を足した (`NameFilter` と `query::products_list`)。
- 購入のフォームで、写真の変換の直後に推測を呼び、空の入力欄にだけ入れる。写真を選び直したときと写真を消したときは古い応答を世代番号で捨てる。商品の照合の失敗は推測の反映を妨げない。商品が未選択のときだけ名前の一致で選択し、一致が無いときは「推測した内容で商品を登録」の導線を出す。
- 商品のフォームに初期値と、保存した商品を返す `onSaved` を足した。押し出しの画面で開き、2 段組でも購入のフォームの未保存の状態を保つ。
- `records_api.dart` の `products` に `name`、`suggestPurchase` を追加し、`ApiClient` にバイト列の POST を追加した。ARB の日本語と英語に文言を追加した。
- テスト: `brew_book_core` の単体 (`test_suggestion.rs`)、PBT (`prop_suggestion.rs`。`parse_response(output_text(v))` の性質を含む)、Fuzzing (`parse_strings.rs` に応答のパーサと `output_text` を追加)、結合テスト (`wrangler_purchase_suggestions_api.rs` の 401・400 (3 種)・500、`wrangler_records_api.rs` の名前の絞り込み)、Flutter のウィジェットテスト。

完了条件の検証:

- FR-19 の API の受け入れ基準: 401 は `wrangler_purchase_suggestions_unauthenticated_401`、400 は `wrangler_purchase_suggestions_content_type_400` と `wrangler_purchase_suggestions_oversized_400` と `wrangler_purchase_suggestions_empty_body_400`、500 は `wrangler_purchase_suggestions_fails_500_without_the_remote_binding`、応答の形と null への落とし込みは `test_suggestion.rs` と `prop_suggestion.rs` が確認した (Fuzzing は任意の入力でパニックしないことだけを確認する)。200 は staging にデプロイして実写真で確認した (下記)。
- 商品の一覧の名前の絞り込み: `wrangler_products_list_filters_by_the_exact_name` など 5 件の結合テストと、単体テスト `a_list_query_with_a_name_filters_by_the_exact_name_ignoring_case` が確認した。
- 経路の台帳とテストの照合: `the_ledger_and_the_suite_match`、`the_checker_rejects_an_ok_test_for_a_route_that_ci_cannot_run`、`the_ledger_marks_the_photo_suggestion_route_as_a_manual_check` が確認した。
- 実写真での候補モデルの比較: 2026-09-30 に、デプロイした使い捨ての Worker (probe) で、実写真 3 枚 (英字のラベル) と合成ラベル (日本語) を 5 つの候補で比較した。日本語のラベルの実写真は用意できなかったため、日本語の文字の抽出は合成ラベルで確認した (採用した llama-4-scout と mistral-small-3.1 は日本語の項目も正しく抽出した)。採用は `@cf/meta/llama-4-scout-17b-16e-instruct` とし、vars の既定値と `DEFAULT_MODEL` に反映した。結果は次のとおり。

  | モデル | 結果 | 所要 | 1 回の実測 |
  | --- | --- | --- | --- |
  | `@cf/meta/llama-4-scout-17b-16e-instruct` | 実写真 3 枚の名前と項目を正しく抽出した | 約 4 秒 | 約 62 Neurons |
  | `@cf/mistralai/mistral-small-3.1-24b-instruct` | 同等の抽出 (variety の誤読が 1 件) | 約 5 秒 | 約 89 Neurons |
  | `@cf/qwen/qwen3.8-27b` | 3 枚中 2 枚が出力の上限で空になった | 約 12 秒 | 約 277 Neurons |
  | `@cf/meta/llama-3.2-11b-vision-instruct` | 商品名が常に null (producer に誤配置) | 約 6 秒 | 約 36 Neurons |
  | `@cf/moondream/moondream3.1-9B-A2B` | messages 形式に非対応 (空の応答) | - | - |

- ウィジェットテスト: 推測の反映 (空欄だけ)、入力済みの値 (焙煎度と価格) の保持、推測中のインジケータ、失敗時のバナーと手入力の継続、一致する商品の選択、選択済みの商品の保持、推測の待ち中に選んだ商品の保持、推測の商品の照合中に選んだ商品の保持、商品の照合が失敗しても推測の反映が残ること、5 MB を超える写真を選び直したら前の推測を反映しないこと、写真を削除したら飛んでいる推測を反映しないこと、5 MB を超える写真を選び直すと商品の登録の導線を消すこと、一致が無いときの導線、2 段組での状態の保持、5 MB 超では呼ばないこと、推測が保存を伴わないことを確認した。
- `mise run check`: exit 0 (86 個の `test result: ok`)。既知の 0030 と 0020 のフレークで 2 回失敗し、いずれも再実行で通過した。
- `CHANGES.md`: `## develop` の通常のエントリ群に `[ADD]` を追加した (完了条件の `### misc` からの読み替えは乖離 13)。
- 実写真での実経路の確認: staging (`brewbook-staging`) にデプロイし、検証用のセッションで `POST /api/purchase-suggestions` に実写真 3 枚 (長辺 2048 px の JPEG) を送り、3 枚とも推測の JSON が返ることを確認した (画面への反映はウィジェットテストで確認した。乖離 14)。結果は次のとおり。

  | 写真 | 返った推測 (product の抜粋) |
  | --- | --- |
  | 1 枚目 | `name=Colombia Los Nogales Caturra Primitive`、`producer=Oscar Hernandez`、`origin=Colombia`、`variety=Caturra`、Flavor Notes 5 件 |
  | 2 枚目 | `name=Fortunato Arenas Anampa`、`origin=Peru`、`region=Cusco`、`variety=Geisha, Catuai`、Flavor Notes 7 件 |
  | 3 枚目 | `name=Finca Llavicocha`、`producer=Carlos Jaramillo`、`origin=Ecuador`、`region=Loja, Sarafuro`、`variety=Geisha`、Flavor Notes 6 件 |

  焙煎度、焙煎日、価格、重量は、この 3 枚の写真では印刷が見えないため全て null だった (合成ラベルでは焙煎日と重量を抽出できることを確認済み)。
- ローカルの `wrangler dev` では、この環境の AI バインディングの呼び出しが `internal error` で失敗したため、実経路の確認は staging で行った (乖離 11 と 14)。

方針からの乖離: 方式を変える乖離は無い。実装詳細の乖離は「## 実装詳細の乖離」に記録した (うち、実経路の検証で判明した応答形式の対応は 6)。
