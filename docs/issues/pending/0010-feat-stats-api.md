# 統計と評価の推移の API を実装する

Created: 2026-09-21
Model: deepseek-v4p1-flash
対応 ADR: ADR-0006 (docs/adr/0006-data-model-and-archive.md)
関連 PRD: FR-5、FR-12、FR-18、性能 (統計の p95 は 500 ms、想定規模)
依存: 0001, 0002, 0003, 0005, 0006, 0007

## 背景

ADR-0006 は、統計 (PRD の FR-18) を brews と purchases に対する集計クエリで実装すると決めた。
日別と月別の区切りは、抽出が `brewed_at` に UTC オフセット (分) を加えた値を SQLite の日時関数で切り、購入が `purchased_on` をそのまま切る。
集計は `archived_at IS NULL` の行だけを対象にし、参照先の購入のアーカイブは見ない。
オフセットの修飾子は Rust 側で整数から組み立て、利用者の入力を SQL に連結しない。
0007 で購入と抽出の API ができた。本 issue が統計を追加する。
統計画面とグラフは 0015 が作る。

## 目的

利用者が、期間を選んで自分の抽出と購入の統計と、購入ごとの評価の推移を取得できるようにする。

## 設計判断

### 経路とパラメータ

| 操作 | 経路 | クエリパラメータ |
| --- | --- | --- |
| 抽出回数と豆の消費量 | `GET /api/stats/brews` | `start`、`end`、`granularity`、`utc_offset_minutes` |
| 購入金額と重量 | `GET /api/stats/purchases` | `start`、`end`、`granularity` |
| 抽出条件と評価の関係 | `GET /api/stats/brew-ratings` | `start`、`end`、`utc_offset_minutes` |
| 購入ごとの評価の推移 | `GET /api/purchases/<購入 ID>/rating-history` | 無し |

- `start` と `end` は `YYYY-MM-DD` の形式で実在する日付だけを受け付け、それ以外は 400 を返す。
  どちらも省略でき、省略した側は端が無いものとして扱う (全期間)。
  両方を省略した場合は最初の記録から最後の記録までを返す。
  `start` が `end` より後なら 400 を返す。
- `granularity` は `day` または `month` で必須とし、それ以外は 400 を返す。
  期間が 62 日以下なら `day`、それ以外は `month` をクライアントが選ぶ (FR-18)。
  採らない案: サーバーが期間から粒度を決める (クライアントが表示の切替を伝えられず、62 日の境界の意図がサーバーに伝わらない)。
- `utc_offset_minutes` は -840 から 840 の整数 (分) で必須とし、それ以外は 400 を返す。
  抽出の API だけが受け取り、購入の API は受け取らない (購入日はタイムゾーンを持たない)。
- 抽出の日と月の区切りは、`brewed_at` にオフセットを加算した値で行う。
  オフセットは SQLite の日時関数の修飾子 (`+540 minutes` など) とし、整数から Rust 側で文字列を組み立てる。
  利用者の入力の文字列を SQL に連結しない。
- 開始日と終了日は端末のローカル時刻での日付として扱う (FR-18 の受け入れ基準)。
  抽出の API では、開始日の 00:00 と終了日の翌日の 00:00 を UTC オフセットで UTC の瞬間に直し、その範囲で `brewed_at` を絞る。
  購入の API は購入日をそのまま使う。
- 購入日は `purchased_on` をそのまま区切る。

### 応答

- `GET /api/stats/brews` は、区間ごとに区間のキー (日別は `YYYY-MM-DD`、月別は `YYYY-MM`)、抽出の件数 (整数)、豆の量の合計 (小数第 1 位まで) を返す。
  豆の量が NULL の抽出は件数に数え、消費量には加えない。
  区間はキーの昇順で並べ、記録の無い区間は返さない。
- `GET /api/stats/purchases` は、区間と通貨コードの組ごとに区間のキー、通貨コード、価格の合計 (整数)、重量の合計 (整数)、購入の件数 (整数) を返す。
  価格が NULL の購入は金額に加えず、重量が NULL の購入は重量に加えない。件数には数える。
  価格が無く通貨コードも null の購入は、通貨コードが null の組として返し、金額を 0 とする。
  同じ区間では通貨コードの昇順で並べ、null は先頭にする。
- `GET /api/stats/brew-ratings` は、期間内の評価を持つ抽出ごとに、抽出の ID、豆の量、湯量、湯の温度、時間、評価を返す。
  条件が NULL の項目は null で返し、集計しない。
- `GET /api/purchases/<購入 ID>/rating-history` は、その購入に紐づく評価を持つ抽出の、抽出の ID、抽出日時、評価を抽出日時の昇順で返す。
  評価が NULL の抽出は含めず、期間で絞らない。
  存在しないか他の利用者の購入は 404 を返し、アーカイブ済みの購入は指定できる (単件取得と同じ扱い。FR-12)。
- 集計と散布図の対象は、行自身がアーカイブ済みの抽出と購入を含めない。
  参照先の購入がアーカイブ済みでも、抽出自身がアーカイブ済みでなければ含める。
  為替換算は行わない (通貨コードごとに分けて返す。PRD のやらないこと)。
- 集計の SQL は `coffee_log_core` の 1 つのモジュールで組み立て、`user_id` と `archived_at` の条件を必ず含める (ADR-0006)。
- 採らない案: 4 つの API を 1 つにまとめる (パラメータと粒度が異なり、クライアントの呼び分けが複雑になる)、集計を Rust 側で行う (全行を Worker のメモリに載せると想定規模の抽出 30,000 件で Worker の CPU とメモリを圧迫する)、日別と月別の区切りを Rust で行う (SQLite の日時関数で足りる)。

## 調査タスク

2026-09-23 に実装と自動テストと想定規模の計測を完了した (自動テストは `mise run check` で通過)。残るのは次の確認である。

- 利用する Workers のプラン (Free は CPU 10 ms、Paid は既定 30 秒) を決め、issue に記録する。PRD にも ADR にも定められていない。
- 決めたプランの CPU 時間の上限に対して、統計の API が余裕を持つことを確認する。ローカルの計測は HTTP の往復の壁時計 (最も重い散布図で中央値 131 ms) であり、CPU 時間そのものではないため、本番の Workers Logs の集計 (リリース後) で確認する。
- 確認の結果、上限に収まらない場合は、対象期間の制限や集計の分割などの対応を別 issue として起案する。

## 関連

- 0007 が抽出と購入の API を作る。
- 0015 が統計の画面とグラフを作る。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. 応答の形は方針に指定が無いため、トップレベルのキーを `brews`、`purchases`、`brew_ratings`、`ratings` とし、項目名は既存の応答 (スキーマの列名) に合わせて `period`、`brew_count`、`dose_grams`、`price_currency`、`price_amount`、`weight_grams`、`purchase_count`、`id`、`brewed_at`、`water_grams`、`water_temp_c`、`brew_time_seconds`、`rating` とした (0015 の画面が読む契約になる)。
2. `brews_stats` と `brew_ratings` は、期間の端を UTC の瞬間に変換する処理が失敗しうるため `Result<Statement, StatsError>` を返す。`purchases_stats` と `rating_history` は変換を行わないため `Statement` を返す。
3. 散布図 (`brew_ratings`) の並び順は方針に指定が無いため `brewed_at ASC, id ASC` にして決定的にした。
4. `has_input` の説明は 0008 が「JSON の本体かクエリパラメータを読む経路」に更新済みである。0010 の実装は「必須のクエリパラメータを持つ経路」への更新を提案したが、統合時に 0008 の定義を採った。統計の 3 経路は `has_input: true` (必須のクエリパラメータを持ち 400 を返す)、`purchases_rating_history` は入力が無いため `has_input: false`、一覧の 4 経路は true のままである。
5. `mise.toml` の `backend:test-integration` に新しい 2 つの結合テストを追加した (0007 の乖離 9 と同じ扱い)。
6. テストハーネスに `Seed::brew_with_numbers` と `Seed::purchase_with_numbers` を追加した (統計は数値と評価と価格を読むため)。
7. 扱える日時の範囲の端をまたぐ入力 (例: `start=0000-01-01` と `utc_offset_minutes=840`、`end=9999-12-31` と `-840`) は 400 とし、`StatsError::Date` を返す (単体テスト `queries::an_end_outside_the_supported_range_is_rejected_as_a_date`)。
8. 想定規模のテストは、統計 SQL が読まない店と商品も想定規模どおり投入した (購入の外部キーを満たしつつ DB 全体を想定規模にするため)。

## 想定規模での処理時間

2026-09-23 に `backend/` で `cargo test -p coffee_log --test wrangler_stats_scale -- --nocapture` を実行した。
利用者 20 人分 (店 100、商品 1,000、購入 3,000、抽出 30,000) の合計 682,000 行をローカルの D1 に入れ、`wrangler dev` とローカル D1 で HTTP の往復を計測した。各 10 回の最小、中央値、最大である。

```
brews stats (day, all):       1250 rows, min 24 ms,  median 25.5 ms, max 27 ms
brews stats (month, all):       42 rows, min 19 ms,  median 20 ms,   max 29 ms
purchases stats (day, all):    730 rows, min 8 ms,   median 9 ms,    max 11 ms
purchases stats (month, all):   24 rows, min 4 ms,   median 5 ms,    max 6 ms
brew ratings (all):          20000 rows, min 127 ms, median 131 ms,  max 138 ms
rating history (10 brews):       6 rows, min 7 ms,   median 7.5 ms,  max 9 ms
```

中央値は測定回数が偶数のときは中央の 2 つの平均とする。この計測は `wrangler dev` への HTTP の往復の壁時計であり、Worker の CPU 時間そのものではない。想定規模の全期間の集計は最も重い散布図 (20,000 件の応答) でも中央値 131 ms で、ローカルの応答としては十分に速い。
Workers の CPU 時間の上限に対する余裕の確認は、利用するプラン (Free は 10 ms、Paid は既定 30 秒) が PRD にも ADR にも定められていないため、プランの決定後に行う (本 issue は pending に置く)。p95 500 ms の合否は issue の記載どおりリリース後に本番の Workers Logs の `duration_ms` の集計で確認する。

## pending にした理由

2026-09-23 に実装と自動テストと想定規模の計測を完了し、コミット fbdff32 に記録した。完了条件の「Workers の CPU 時間の上限に対して余裕があることの確認」は、利用するプラン (Workers Free は CPU 10 ms、Workers Paid は既定 30 秒) が PRD にも ADR にも定められておらず、所有者の決定 (プラン) と、本番の Workers Logs での CPU 時間の確認 (リリース後) が要る。所有者の指示により close せず pending に置く。

再開には、次が必要である。

- 利用するプラン (Free か Paid) の決定と issue への記録
- 決定したプランの CPU 時間の上限と、本番の Workers Logs の `duration_ms` (または CPU 時間の指標) との突き合わせ
- 計測は壁時計であるため、必要なら本番での CPU 時間の確認 (ローカルの計測では代替できない)

実装と自動テスト (FR-18 の API の分、他利用者とアーカイブ済みの除外、境界とオフセット、台帳とスイート) は完了しており、`CHANGES.md` の `[ADD]` エントリも実装のコミットに含める。
