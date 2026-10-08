# ADR-0019: 店の地図と住所の補完に Google Maps Platform を使う

Created: 2026-10-07
Model: DeepSeek V4.1 Flash
Status: Accepted

## 背景

2026-10-07 に所有者が、店の住所から地図を表示し、店名から住所の入力を補完したいと求めた。
PRD に FR-22 を追加する。

店は名前と住所だけを持ち、座標を持たない (ADR-0006)。
住所の地図を表示するには、住所から位置を引く (ジオコーディングする) 必要がある。
店名から住所を補完するには、店名で場所を検索する必要がある。

利用者向けの Worker は Rust (workers-rs) で Cloudflare Workers 上にあり、依存を可能な限り
少なくする方針である (ADR-0001)。
Frontend は同一オリジンの `/api` だけを呼び、外部サービスは Worker 経由で使う (ADR-0005)。
外部サービスへの送信は、写真からの推測 (FR-19) では写真を Cloudflare の外に出さないと決めたが
(ADR-0016)、店名と住所は Google に送ることを所有者が 2026-10-07 に承認した。

Google Maps Platform は無料枠でも Google Cloud プロジェクトの課金の有効化が要る。
2026-09-14 更新の公式の料金ページで、次の無料枠を確認した。

| SKU | 無料枠 |
| --- | --- |
| Maps Embed API (Embed) | 無制限 (無料) |
| Maps JavaScript API (Dynamic Maps) | 月 10,000 回 |
| Static Maps API (Static Maps) | 月 10,000 回 |
| Places API Text Search (IDs Only) | 無制限 (無料) |
| Places API Text Search (Pro) | 月 5,000 回 |
| Places API Place Details (Essentials) | 月 10,000 回 |

## 決定

- 地図の表示は Maps Embed API の iframe (`https://www.google.com/maps/embed/v1/place?key=...&q=<住所>`)
  で行う。無料で無制限のため、課金の心配が無い。
- 店名から住所を補完する検索は、Places API (New) の Text Search を Worker 経由で呼ぶ
  (`GET /api/place-search?q=<店名>&lang=<ja|en>`)。
  `places.displayName` と `places.formattedAddress` のフィールドマスクで 1 回呼び、
  名前と住所の候補を最大 5 件返す (Text Search Pro の SKU)。
- 検索の API キー (`GOOGLE_PLACES_API_KEY`) は Worker の Secret に置き、ブラウザに出さない。
- 地図の API キー (`GOOGLE_MAPS_EMBED_API_KEY`) は iframe の URL に載るためブラウザに出る。
  HTTP リファラの制限 (ローカル、staging、production のオリジンだけ) で保護する。
  キーは `GET /api/maps/config` (認証が必要) で配り、未設定のときは null を返して画面は地図を出さない。
- キーの実値はリポジトリに含めず、ローカルは `.dev.vars` (git 管理外)、staging と production は
  `wrangler secret put` で設定する (R2 の資格情報と同じ扱い。ADR-0003、ADR-0015)。
- 課金は無料枠の範囲で運用する。
  Google Cloud の予算アラートと API の割り当て (クォータ) を設定し、想定外の課金を防ぐ (README の手順)。
- 座標と place_id は保存しない。`shops` のスキーマは変えない。
  地図は住所から Embed API が引くため、座標を持つ必要が無い。
- 地図は店のフォーム (登録と編集) だけに出す。
- 店名と住所 (検索の入力と応答) はログに出さない (PRD のセキュリティと同じ扱い)。
- 検索は利用者の操作 (「住所を検索」ボタン) で行い、入力のたびには呼ばない。
  呼び出しの回数を抑え、部分的な店名での誤った候補を出さないためである。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| Maps JavaScript API で地図を描く | 月 10,000 回の無料枠を消費し、Google の JS ライブラリの読み込みと Dioxus との連携の実装が増える。地図を見るだけの用途には Embed で足りる |
| Static Maps API で地図の画像を出す | 操作できず、無料枠も月 10,000 回で Embed より劣る |
| ブラウザから Places API を直接呼ぶ | API キーがブラウザに公開され、リファラ制限だけに依存する。Worker 経由ならキーを Secret に置け、同一オリジンの方針 (ADR-0005) にも合う |
| Geocoding API で住所から座標を引いて保存する | `shops` のスキーマと API とエクスポートの変更が増える。Embed API は住所から地図を出せるため、座標は要らない |
| 座標と place_id を `shops` に足す | 上の理由と同じ。必要になったら別の ADR で足す |
| OpenStreetMap (Nominatim と Leaflet) を使う | API キーと課金が不要だが、所有者の要望 (Google の Map API) に反する |
| Places API の Autocomplete で住所の欄を補完する | 所有者の要望は店名からの補完である。住所の入力中の補完は別の機能になる |
| Text Search (IDs Only) と Place Details (Essentials) の 2 回呼び | 無料枠の合計は大きい (無制限と月 10,000 回) が、呼び出しが 2 回になり失敗の経路も 2 つになる。想定規模では Text Search Pro の月 5,000 回で足りる |

## 結果

- 無料枠と費用: Embed は無料無制限である。
  Text Search Pro は月 5,000 回まで無料で、超えると $32/1,000 回である。
  想定規模 (利用者 20 人、店の登録は月に数回) では無料枠に収まる。
  Google Cloud の予算アラート (例: $1) と API の割り当て (例: 1 日 100 回) を設定して、
  想定外の課金を防ぐ (README の手順)。
- プライバシー: 店名と住所が Google に送られる。
  写真 (ADR-0016) は Cloudflare の外に出さない扱いのままである。
  検索の入力と応答はログに出さない。
- CI: 住所の補完の正常系は Google の呼び出しを要するため CI では実行できない。
  応答の解析と検証は `brew_book_core` の単体テストで確かめ、経路の正常系は staging への
  デプロイで実在の店名で確認する (FR-19 と同じ扱い。PRD の成功指標の測定方法)。
- キーが無い環境 (CI など) では、住所の補完は 500 を返し、地図は出ない (config が null)。
  ローカルの開発は `.dev.vars` にキーを置くと動く。
- 地図の iframe は `loading="lazy"` と `referrerpolicy="strict-origin-when-cross-origin"` を
  付ける (Google のサンプルに合わせる)。
- 住所の検索の候補は最大 5 件とする。
  店名の一致は Google の順位付けに従い、候補に無い住所も手入力できる。
- Google の応答はサーバー側で検証し、名前か住所が欠けた場所は候補から除く。
  応答が JSON として読めないときは 500 を返す。
