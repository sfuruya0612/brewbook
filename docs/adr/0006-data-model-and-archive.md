# ADR-0006: データモデルを店、商品、購入、抽出の 4 つで構成し、論理削除を採用する

Created: 2026-09-21
Model: Claude Fable 5.1
Status: Partially superseded by ADR-0018 (2026-10-03)

ADR-0018 (アーカイブ (論理削除) を廃止する) が本 ADR のアーカイブ (論理削除) の決定だけを置き換えた (2026-10-03)。
データモデルの他の決定 (4 つの記録の関係、物理削除はアカウント削除だけ、`user_id` を全クエリの条件に含める規則など) は残る。
本文の `archived_at` の記述は決定の履歴として残す。

## 背景

所有者の要求は次の 3 種類の記録だった。

- 購入した豆の記録 (パッケージの写真、商品名、Producer、Origin、Region、Process、Variety、Roast、Roast Date、Flavor Notes、価格と重量)
- 豆を購入した場所 (店名、住所)
- コーヒーを淹れた記録 (豆の量、湯量、湯の温度、時間)

2026-09-21 の会話で、筆者が選択肢と説明を提示し、所有者は次を決めた。

- 同じ商品を複数回買うため、商品マスタと購入履歴を分ける。
- 淹れた記録は購入履歴 (特定の袋) に紐づけ、商品と店は購入からたどる。
- 店、Roast、Roast Date、価格、重量、写真は購入に持つ (筆者が提示した案 C)。
  商品名、Producer、Origin、Region、Process、Variety、Flavor Notes は商品に持つ。
- Flavor Notes はタグ (複数) にする。
- 削除は論理削除 (アーカイブ) にする。
- 淹れた記録には抽出方法、挽き目 (グラインダーの設定値)、評価 (5 段階)、感想を追加する。
- 価格は通貨の最小単位の整数と ISO 4217 の通貨コード (既定値 JPY) で持ち、為替変換はしない。
- 購入に購入日、抽出に抽出日時を必須で持つ。
- 店の住所は任意 (NULL 許容) とする。
- 購入の店は任意 (NULL 許容) とする (2026-09-21 に所有者が決定。もらい物など店が無い購入を認める)。

## 決定

テーブルは次の通りとする (列の詳細は PRD の FR-6 から FR-11)。

| テーブル | 主な列 | 参照 |
| --- | --- | --- |
| users | id、表示名 (管理者が管理者画面で利用者を識別するために使う。PRD の FR-17)、作成日時 | |
| registration_tokens | id、user_id、トークンのハッシュ、有効期限、使用日時 | users |
| passkey_credentials | id、user_id、credential_id、公開鍵 (COSE)、署名カウンタ、名前、作成日時、最終使用日時 | users |
| webauthn_challenges | id、user_id (ログイン時は NULL)、チャレンジ、種別 (登録 / ログイン)、有効期限 | users |
| sessions | id、user_id、トークンのハッシュ、有効期限、作成日時 | users |
| shops | id、user_id、店名、住所、created_at、updated_at、archived_at | users |
| products | id、user_id、商品名、producer、origin、region、process、variety、created_at、updated_at、archived_at | users |
| flavor_tags | id、user_id、名前 | users |
| product_flavor_tags | user_id、product_id、tag_id | products、flavor_tags |
| purchases | id、user_id、product_id、shop_id (NULL 許容)、purchased_on、roast、roast_date、price_amount、price_currency、weight_grams、photo_key、created_at、updated_at、archived_at | users、products、shops |
| brews | id、user_id、purchase_id、brewed_at、dose_grams、water_grams、water_temp_c、brew_time_seconds、method、grind_setting、rating、notes、created_at、updated_at、archived_at | users、purchases |

「利用者データのテーブル」は shops、products、flavor_tags、product_flavor_tags、purchases、brews の 6 つを指す (PRD の FR-14 のエクスポート対象)。

- 抽出は購入だけを参照し、商品と店の ID を重複して持たない。
  商品と店は購入から結合して返す。
  店は LEFT JOIN で結合し、`shop_id` が NULL の購入では `shop` を null で返す。
- 論理削除は `archived_at` 列 (NULL なら有効) で表す。
  既定の一覧は `archived_at IS NULL` で絞る。
- 親をアーカイブしても子はアーカイブしない。
  アーカイブ済みの親を新規の子の参照先に指定すると 409 を返す。
- 存在しない ID と他の利用者の ID は区別せず 404 を返す。
- タグは利用者ごとに名前で一意にする。
  商品のタグは配列で置き換え、どの商品からも参照されなくなったタグは削除せず残す。
- アカウント削除 (PRD の FR-15) だけは物理削除とし、利用者に属する全行と R2 の全オブジェクトを削除する。
- 外部キー制約を有効にし、物理削除はアカウント削除の batch の中で子から順に行う。
- shops、products、purchases、brews は `created_at` と `updated_at` を持つ。
  登録時に両方を現在時刻にし、更新とアーカイブとアーカイブ解除で `updated_at` を現在時刻にする。
  サジェスト (PRD の FR-13) の並び順は `updated_at` を使う。
- users を除く全テーブルに `user_id` を持ち、全クエリで `user_id` を条件に含める。
  中間テーブル (product_flavor_tags) も `user_id` を持つ。
  例外は webauthn_challenges のログイン用の行で、`user_id` を持たずチャレンジ値で引く。
  この行はアカウント削除の対象外とし、有効期限で失効する。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| 購入ごとに 1 レコード (商品マスタ無し) | 同じ商品を 2 回買うと商品の項目が重複する。所有者が商品マスタ方式を選んだ |
| 店を商品に固定する (案 B) | 同じ商品を別の店で買えない。所有者が選ばなかった |
| Roast を商品に持つ (案 A) | 筆者が「焙煎度は購入ごとに変わり得る」と説明して案 C を併記し、所有者は案 C を選んだ。所有者が理由を述べた記録は無い |
| 抽出に店の ID も持つ | 購入の店と矛盾しうる。購入からたどれる。所有者が選ばなかった |
| 参照されている親の削除を拒否する | 使い切った豆や閉店した店を一覧から消せない。所有者が論理削除を選んだ |
| カスケード削除 | 子の記録が黙って消える。所有者が選ばなかった |
| Flavor Notes を 1 つの文字列にする | タグでの検索と集計ができない。所有者がタグを選んだ |
| グラインダー名と挽き目を別の項目にする | 所有者が決めたのは「挽き目 (グラインダー設定)」の 1 項目。分割は所有者に諮っていない |
| 参照されなくなったタグを自動で削除する | 商品の更新でタグを外した直後に再利用できなくなる。残す方が単純 |

## 結果

- 抽出の一覧と単件の取得には、購入、商品、店の結合が必要になる。
  SQL は結合を 1 回で行う。
- 論理削除のため、全ての一覧クエリに `archived_at` の条件が必要になる。
  条件の付け忘れを防ぐため、クエリの組み立てを 1 か所に集約し、アーカイブ済みの行が既定の一覧に出ないことを自動テストで検証する。
- サジェスト (PRD の FR-13) は、products、purchases、brews の該当列から `user_id` で絞って DISTINCT を取る。
- 統計 (PRD の FR-18) は brews と purchases に対する集計クエリで実装する。
  日別と月別の区切りは、brews は `brewed_at` に UTC オフセット (分) を加えた値を SQLite の日時関数で切り、purchases は `purchased_on` をそのまま切る方針とする。
  D1 (workerd) の SQLite 関数の許可リストに `date`、`datetime`、`strftime` が含まれることと、ISO 8601 の UTC 文字列 (末尾 `Z`、小数秒付きを含む) に `'+540 minutes'` のような修飾子を与えて日と月に切れることは、2026-09-21 に許可リストのソースと SQLite 3.51.0 で確認した。
  修飾子の文字列はオフセットの整数から Rust 側で組み立て、利用者の入力をそのまま SQL に連結しない。
  集計は `archived_at IS NULL` の行だけを対象にし、参照先の購入のアーカイブは見ない。
- 管理者 Worker (ADR-0008) は users と registration_tokens を読み書きし、passkey_credentials を読むだけとする。
- 主キーは UUID v4 の文字列とする (ADR-0002)。
