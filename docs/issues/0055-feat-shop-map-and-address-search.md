# 店のフォームに住所の地図を表示し、店名から住所を補完する

Created: 2026-10-07
Model: DeepSeek V4.1 Flash

## 背景

所有者の要望は次の 2 点である。

- Google の Map API を使って、お店の住所からマップ表示したい。
- 店名から住所入力を補完して欲しい。

現状は次のとおりである。

- 店は名前と住所だけを持ち (`shops` テーブル。ADR-0006)、フォームは `frontend/src/screens/records/shop_form.rs` の店名と住所の 2 欄だけである。
- 外部サービスは写真からの推測の Workers AI (ADR-0016) だけである。Google Maps Platform は使っていない。
- Frontend は同一オリジンの `/api` だけを呼び、秘密の値は Worker の Secret に置く (ADR-0005、ADR-0003)。

2026-10-07 に所有者が次の 4 点を決めた (ADR-0019)。

- Google Cloud プロジェクトと API キーは所有者が用意する。作成手順は README に書く。
- 地図は Maps Embed API (無料で無制限) の iframe にする。
- 住所の補完は Worker 経由の Places API (New) の Text Search にする (キーは Worker の Secret)。
- 地図は店のフォームだけに出す。

## 目的

- 店のフォームの住所の下に、住所の位置の地図が出る。住所が空のときは出ない。
- 店名から住所の候補を検索し、選ぶと店名と住所の欄が候補の値になる。候補に無い住所も手入力できる。

## 設計判断

- 経路を 2 つ足す (`backend/brew_book_core/src/routes.rs` の台帳)。
  - `GET /api/place-search?q=<店名>&lang=<ja|en>` (認証あり、入力あり、正常系は staging で確認)
  - `GET /api/maps/config` (認証あり、入力なし、正常系は CI)
- 検索は Places API (New) の Text Search を 1 回呼ぶ。フィールドマスクは `places.displayName,places.formattedAddress` とし、`pageSize` は 5、`languageCode` は `lang` の値にする。
  応答の解析と検証は `brew_book_core` の純粋な関数に置き、単体テストで確かめる (wasm に依存しない。ADR-0001)。
- Google の呼び出しに失敗したときと、キー (`GOOGLE_PLACES_API_KEY`) が未設定のときは 500 を返す。理由は区別せず、店名と応答をログに出さない (FR-19 と同じ扱い)。
- 地図のキー (`GOOGLE_MAPS_EMBED_API_KEY`) は `GET /api/maps/config` で配る。未設定のときは `embed_api_key` を null にし、画面は地図を出さない。キーは iframe の URL に載ってブラウザに出るため、Google Cloud 側の HTTP リファラの制限で保護する (ADR-0019)。
- キーの実値はリポジトリに含めない。ローカルは `.dev.vars`、staging と production は `wrangler secret put` で設定する。README に Google Cloud の作成手順 (API の有効化、キーの制限、予算アラート、割り当て) を書く。
- 画面は店のフォームに次を足す。
  - 店名の下に「住所を検索」のボタン (店名が空のときは無効)。押すと検索し、候補 (名前と住所) を一覧で出す。候補を選ぶと店名と住所の欄が候補の値になる。検索中の表示、候補が無いときの案内、失敗の再試行の案内を出す。
  - 住所の下に地図 (iframe)。住所があるときだけ出し、住所の確定 (入力の確定か候補の選択) で更新する。`loading="lazy"` と `referrerpolicy="strict-origin-when-cross-origin"` を付ける。
- 地図のキーはフォームを開いたときに 1 回だけ引く。キーが無いときは地図を出さない (ローカルや CI でキーが無くてもフォームは動く)。
- 座標と place_id は保存しない。`shops` のスキーマは変えない (ADR-0019)。
- i18n のキーを足す (`searchAddressButton`、`searchingLabel`、`addressSearchEmpty`、`mapTitle`)。`KEY_COUNT` を更新する。
- デザインの原本 (`docs/design/components/bundle.css` と `RecordForms/README.md`) に、候補の一覧と地図の見た目を足す。
- CI では Google を呼べないため、正常系は staging で確認する (FR-19 と同じ例外)。
  結合テストは未認証 401、入力不正 400、キーの未設定の 500 を確認する。`maps_config` はテストが `--var` で偽のキーを注入し、返ることを確認する。

## 完了条件

- 店のフォームで店名から住所を検索でき、候補を選ぶと店名と住所の欄が候補の値になる。
- 住所があるとき、フォームに地図が出る。住所が空のときは出ない。
- 検索は「住所を検索」の操作でだけ行われ、入力のたびには呼ばれない。
- 検索の失敗に再試行の案内が出て、手入力を続けられる。
- 台帳とスイート (`backend/brew_book/tests/support/mod.rs`) が一致し、`mise run check` が通過する。
- README に Google Cloud の設定手順 (キーの作成と制限、Secret の設定、予算アラート、割り当て) が書かれている。
- staging へのデプロイで、実在の店名から住所の候補が返ることを確認し、結果をこの issue に記録する (Google の呼び出しを要するため。ADR-0019)。

## 解決方法

- 台帳 (`backend/brew_book_core/src/routes.rs`) に 2 経路を足した。
  - `place_search`: GET `/api/place-search`、認証あり、入力あり、正常系は staging で確認 (`OkTest::Staging`)。
  - `maps_config`: GET `/api/maps/config`、認証あり、入力なし、正常系は CI。
  スイート (`backend/brew_book/tests/support/mod.rs`) にも同じ種別を足し、`test_routes.rs` の
  「Staging は写真からの推測だけ」の前提を 2 経路に直した。
- `brew_book_core::maps` を足した。検索の入力の検証 (`validate_search_input`)、
  Google に渡す本体の組み立て (`search_request_body`)、応答の解析 (`parse_search_response`) を持ち、
  単体テスト (`tests/test_maps.rs`、10 件) と PBT (`pbt/tests/prop_maps.rs`) で確かめる。
- Worker に `records::place_search` と `records::maps` を足した。
  - `place_search` は Secret の `GOOGLE_PLACES_API_KEY` を読み、Places API (New) の Text Search を
    `X-Goog-FieldMask: places.displayName,places.formattedAddress` で 1 回呼ぶ。キーの未設定と
    失敗は理由を区別せず 500 にし、店名と応答をログに出さない。
  - `maps` は Secret の `GOOGLE_MAPS_EMBED_API_KEY` を返し、未設定のときは null にする。
- 結合テスト `backend/brew_book/tests/wrangler_maps_api.rs` を足した (未認証 401、入力不正 400、
  キーの未設定の 500、地図の設定の正常系と null)。`mise.toml` の `backend:test-integration` に足した。
- Frontend に `records::maps` の `map_embed_url` (キーと住所のパーセントエンコード) と、
  `RecordsApi` の `place_search` と `maps_config` を足した。
- 店のフォーム (`frontend/src/screens/records/shop_form.rs`) に「住所を検索」のボタン、候補の一覧、
  地図の iframe を足した。検索は操作でだけ行い、候補を選ぶと店名と住所の欄が候補の値になる。
  地図は住所の確定 (入力欄から離れたとき) と候補の選択で更新する。
  `TextField` に値の確定の `onchange` を足した (地図の更新に使う)。
- i18n に 4 キー (`searchAddressButton`、`searchingLabel`、`addressSearchEmpty`、`mapTitle`) を
  足した (`KEY_COUNT` は 237)。
- デザインの原本に候補の一覧と地図のクラスを足し (`docs/design/components/bundle.css`、
  `RecordForms/README.md`、`RecordForms/preview.html` の店の登録の画面)、
  `frontend/src/ui/design.css` に写した。
- README に Google Cloud の設定手順 (API の有効化、キー 2 つの制限、予算アラートと割り当て、
  Secret の設定、デプロイ後の確認) を足した。`wrangler.toml` の秘密情報の説明にも触れた。
- CHANGES.md に 1 件を足した。
- テスト: `test_records_api.rs` に経路と応答の検査、`test_records_maps.rs` に URL の検査、
  `test_shop_map_web.rs` にブラウザの検査 (地図の表示、検索、候補の選択、空と失敗の状態、
  キーの無いとき)、`frontend/pbt/tests/prop_maps.rs` に URL の PBT を足した。
  店のフォームを描く既存の Web テスト (`test_records_lists_web.rs`) に地図の設定の応答を足した。

## 検証

- `brew_book_core` の単体テスト: `test_maps.rs` の 10 件が通過した。
- バックエンドの PBT: `prop_maps.rs` が通過した。
- バックエンドのネイティブテスト: `cargo test --workspace -- --skip wrangler_` が通過した
  (台帳とスイートの照合の `api_suite` と `test_routes` を含む)。
- フロントエンドのネイティブテスト: `frontend:test` が通過した
  (`test_records_api.rs`、`test_records_maps.rs`、`prop_maps.rs` を含む)。
- フロントエンドの Web テスト: `frontend:test-web` が通過した。
  `test_shop_map_web.rs` の 4 件 (保存済みの住所の地図、店名からの検索と複数候補の表示と選択、
  候補が無いときと失敗の再試行、キーの無いとき) が通過した。
- 結合テスト: 全 suite が通過した。`wrangler_maps_api` の 6 件 (住所の補完の未認証 401、
  入力不正 400、キーの未設定の 500、地図の設定の未認証 401、正常系、キーの無いときの null) を含む。
  1 回目は `wrangler_passkey_flow` の 1 件で Chrome のセッションが落ちて失敗したが
  (環境の問題)、単独で再実行して 9 件の通過を確認した。
- E2E: `frontend:test-same-origin` が通過した (店のフォームの幅 375 px の計測を含む)。
- `mise run check` を実行し、fmt、lint、formal、backend:build、backend:test、frontend:build、
  frontend:lint、frontend:test、frontend:test-web、frontend:test-same-origin が通過した。
  backend:test-integration は並列実行の負荷の下でコンパイル中に外部から SIGTERM で停止されたため
  (0054 と同じ環境の問題)、suite を分けて単独で再実行して全 suite の通過を確認した。
  テストの失敗は 0 件である (2026-10-07 から 2026-10-08)。
- staging での確認: 未実施。所有者が Google Cloud のプロジェクトとキーを用意し、
  Secret を設定してデプロイした後に、実在の店名で住所の候補が返ることを確認する (FR-22)。

## 追記 (2026-10-08)

所有者が、キーは地図と住所の補完で同じ 1 つを使うと決めた (ADR-0019 の追記)。
次のように変更した。

- Secret を `GOOGLE_MAPS_API_KEY` の 1 つにし、地図と住所の補完の両方で使う。
- キーのアプリケーションの制限はリファラ (ローカル、staging、production のオリジン) のままとし、
  API の制限を Maps Embed API と Places API (New) にする。
- 住所の補完の Worker の呼び出しは、アプリのオリジンの `Referer` を付けて同じリファラの制限を
  通す (`brew_book_core::maps::referer_header`)。workerd が `Referer` を送信できることは、
  手元の Worker とローカルの HTTP サーバーで確認した (付けたヘッダが届くことを実測)。
- 結合テストは、住所の補完をキーの無いサーバーで確かめるように変えた (キーを注入したサーバーで
  有効な入力を送ると、本物の Google を呼んでしまうため)。地図の設定の正常系はキーを注入した
  サーバーで確かめる。
- staging での確認に、Worker の呼び出しがリファラの制限を通ることを加える。
  拒否された場合はアプリケーションの制限を外す (README と ADR-0019 に記す)。

## 追記 (2026-10-08 の候補の適用)

所有者が、住所の検索の候補を選んだときに、住所だけでなく店名も候補の名前で上書きすると決めた
(ADR-0019 の追記)。店名と住所の 2 欄があるフォームでは、選んだ場所の名前と住所を両方入れる形が
一般的であるためである。次を変更した。

- 店のフォームの候補の選択で、店名と住所の両方の signal を候補の値にする。
- ブラウザテストの候補の選択の検査を、店名と住所の両方を確かめるように変えた
  (2 番目の候補を選ぶと店名が「丸山珈琲 中目黒店」になる)。
- PRD の FR-22 の受け入れ基準と、デザインの `RecordForms` の説明を直した。

## 関連

- ADR-0019 (決定)
- FR-6 (店の登録と編集)、FR-22 (追加する要求)
- ADR-0016 と 0034 (FR-19。CI で正常系を実行できない扱いを同じにする)
- 0041 (店のフォームと画面の土台)
- 0009 (R2 の資格情報の扱い。キーの管理を同じにする)
