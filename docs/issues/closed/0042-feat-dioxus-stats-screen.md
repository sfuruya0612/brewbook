# 統計の画面を Dioxus で実装する

Created: 2026-10-02
Model: deepseek-v4p1-flash
Completed: 2026-10-02
対応 ADR: ADR-0017
関連 PRD: FR-18
依存: 0038, 0039, 0041

## 背景

本 issue は、2026-10-01 の所有者の決定 (ADR-0017) から起票した。

Flutter の統計は `frontend/lib/screens/stats_screen.dart` と `frontend/lib/widgets/stats_charts.dart` にあり、グラフは fl_chart で描く。
`frontend/lib/records/stats_period.dart` が期間の切り替え (当月、3 か月、6 か月、12 か月、全期間、任意) を API の開始日、終了日、粒度に変換する。
購入の詳細の評価の推移も fl_chart の折れ線である。
集計は Backend の API が行い、Frontend は結果を描くだけにする (ADR-0007 の決定を ADR-0017 が引き継ぐ)。

## 目的

統計の画面で、期間を選んで抽出回数と豆の消費量、購入金額と重量、抽出条件と評価の関係をグラフで見られ、購入の詳細で評価の推移を見られる。

## 設計判断

- グラフは SVG を自前で描く。
  対象は棒グラフ 2 つ (抽出回数と豆の消費量、通貨ごとの購入金額と重量)、散布図 4 つ (豆の量、湯量、湯の温度、時間)、折れ線 1 つ (購入の評価の推移) で、色は `chart-count` と `chart-grams` の 2 つだけである (デザインの README)。
  SVG なら `dioxus-ssr` でマークアップを検証でき、バンドルも増えない。
- 採らない案: `plotters` と `plotters-canvas` (canvas への描画になるため、描画の結果をマークアップで検証できず、デザインの細部の制御が弱い)、JavaScript のグラフライブラリ (JS の依存とブリッジが増え、ADR-0017 の依存を最小にする方針に反する)。
- 軸、目盛り、単位、空の状態の表示はデザインの README と `docs/design/components/Charts/` に合わせる。
- 散布図は条件が null の点を描かない (FR-18)。
- 期間の切り替えは、0038 が写した統計の期間のモジュールを使う。
- グラフの座標の計算 (値から SVG の座標への変換、目盛りの刻み) は純粋な関数として切り出し、native の単体テストと PBT で守る。

## 完了条件

- 統計の画面が、初期状態で当月の日別の棒グラフを表示し、3 か月、6 か月、12 か月、全期間、任意の期間に切り替えられる (期間の切り替えは API に渡すパラメータの変換を native のテストで、グラフの表示を `wasm-bindgen-test` で確認する)。
- 棒グラフ 2 つ (抽出回数と豆の消費量、通貨ごとの購入金額と重量)、散布図 4 つ (豆の量、湯量、湯の温度、時間)、折れ線 1 つ (購入の評価の推移) がある (FR-18)。
- 折れ線は 0041 の購入の詳細に足す (native のテストで確認する)。
- グラフの色が `chart-count` と `chart-grams` の 2 つだけである (`wasm-bindgen-test` の計算済みスタイルの比較で確認する)。
- 座標の計算の単体テストと PBT がある。
- `mise run dioxus:lint`、`mise run dioxus:test`、`mise run dioxus:test-web` が成功する。
- `mise run check` が通過する。

## 関連

- 0041 が記録の画面を作る (購入の詳細の折れ線は 0041 の画面に足す)。
- 0039 の `Charts` の枠を埋める。

## 解決方法

統計の画面 (期間の切り替えと棒グラフと散布図) と、購入の詳細の評価の推移の折れ線を実装した (ADR-0017)。

- `frontend/src/records/chart.rs` に座標の計算を純粋な関数として置いた (値の軸の上限と目盛り、棒の配置と高さ、散布図の範囲、評価の軸、折れ線の点、軸ラベルの位置)。枠は 0039 の `ChartFrame` と同じ 340 x 120 の viewBox で、描画領域は x 28 から 340、y 8 から 92。値の軸の上限は値の最大の 1.1 倍 (Flutter の fl_chart と同じ)、目盛りは最大、半分、0。棒は区間の間に 3 の隙間、幅の上限 19。日別の軸ラベルは先頭、中央、末尾。壊れた応答のキーでも panic しない (文字の境界で切らない)。
- `frontend/src/records/stats.rs` に `StatsApi` (抽出の集計、購入の集計、抽出の評価、購入の評価の推移) と応答の型、`stats_path` を追加した。応答の JSON の読み取り補助は `records::models` と共有する。
- `frontend/src/screens/stats/mod.rs` に統計の画面を追加した。初期状態は当月の日別で、3 か月、6 か月、12 か月、全期間、任意の期間に切り替えられる (任意は 62 日までが日別、63 日以上は月別)。
- `frontend/src/screens/stats/charts.rs` にグラフの組み立て (抽出回数と豆の消費量の棒、通貨ごとの購入金額と重量の棒、豆の量と湯量と湯の温度と時間の散布図、評価の推移の折れ線) を追加した。
- `frontend/src/ui/charts.rs` に描画の部品 (`ChartBars`、`ChartScatterFrame`、`ChartScatter`、`ChartLine`、`ChartSeries`) を足した。
- `frontend/src/ui/wide_layout.rs` に `WidePage` (レール + 残り幅いっぱいの内容) を足し、`frontend/src/ui/design.css` に `.wide-page` と統計の画面の並びのクラス (`.stats`、`.stats-columns`、`.stats-section`、`.stats-scatter`、`.stats-custom`) を足した (デザインの原本に一覧と詳細に分かれない広い画面のクラスが無いため。Flutter の統計はレール + `Expanded` で組んでいる)。
- `frontend/src/router.rs` の `/stats` を統計の画面に配線し、`frontend/src/screens/records/purchase_detail.rs` に評価の推移の折れ線 (`RatingHistoryChart`) を足した。`frontend/src/records/mod.rs`、`frontend/src/screens/mod.rs`、`frontend/src/ui/mod.rs` に新しいモジュールと部品の再公開を足した。
- `frontend/tests/` に `test_chart.rs` (14 件)、`test_stats_api.rs` (10 件)、`test_stats_web.rs` (wasm 8 件) を、`frontend/pbt/tests/prop_chart.rs` に座標の PBT (11 性質) を追加した。`tests/test_records_screens.rs` に購入の詳細の折れ線の配線の検査 (1 件) を足し、ブラウザテストの補助は `frontend/tests/support/web.rs` に追加した (既存の `test_ui_web.rs` の補助はそのまま)。

完了条件の検証:

- 統計の画面が初期状態で当月の日別の棒グラフを表示し、3 か月、6 か月、12 か月、全期間、任意の期間に切り替えられる: 期間の切り替えの API のパラメータへの変換は `tests/test_stats_api.rs` の `the_initial_period_is_the_current_month_daily`、`the_period_switching_becomes_the_api_parameters`、`the_custom_period_switches_the_granularity_at_62_days` で確認した。画面の初期表示と切り替えは `tests/test_stats_web.rs` の `the_stats_screen_shows_the_current_month_daily_charts` (要求が 3 経路だけであること、棒が 1 本であること、当月のチップが選ばれていること) と `the_period_chip_reloads_with_the_selected_period` (3 か月のチップを押すと、3 か月の期間と月別の粒度で 3 経路を読み直し、選んだチップが 1 つだけになること) で確認した。
- 棒グラフ 2 つ (抽出回数と豆の消費量、通貨ごとの購入金額と重量)、散布図 4 つ (豆の量、湯量、湯の温度、時間)、折れ線 1 つ (購入の評価の推移) がある: `tests/test_stats_web.rs` の `the_brew_charts_draw_the_count_and_dose_series`、`the_purchase_charts_are_split_by_currency`、`the_scatter_charts_skip_the_null_conditions` (条件が null の点を描かない)、`the_rating_history_line_chart_draws_the_line`、`the_empty_charts_show_the_no_records_message` で確認した。
- 折れ線は 0041 の購入の詳細に足す: `tests/test_records_screens.rs` の `the_purchase_detail_has_the_rating_history_line_chart` が、`RatingHistoryChart { entries: ratings() }` の描画と `rating_history(&id)` の呼び出しを確認した。
- グラフの色が `chart-count` と `chart-grams` の 2 つだけである: `tests/test_stats_web.rs` の `the_charts_use_only_the_two_chart_colors` が全 `rect` と `circle` と `polyline` の計算済みスタイルを比べ、塗りが `chart-count` (rgb(74, 47, 28)) と `chart-grams` (rgb(184, 116, 42)) と折れ線の点の `paper-raised` の許容集合の外に出ないこと、線が `chart-grams` の許容集合の外に出ないことを確認した。
- 座標の計算の単体テストと PBT がある: `tests/test_chart.rs` (14 件。境界の固定と、壊れたキーで panic しないこと) と `pbt/tests/prop_chart.rs` (11 性質。枠内、単調、等間隔、目盛りの順序)。
- `mise run dioxus:lint`、`mise run dioxus:test`、`mise run dioxus:test-web`: 作業ツリーで成功した (lint は clippy と fmt とも警告なし、native は 216 件、ブラウザは 37 件)。`mise run dioxus:build` も成功した。
- `mise run check` が通過する: 所有者の指示により、移行の全 issue の完了後に 1 回だけ実行するため、この issue では未実行 (最終確認に委ねる)。

方針を保った実装詳細の乖離:

- `WidePage` と `.wide-page` を足した (デザインの原本に一覧と詳細に分かれない広い画面のクラスが無いため。Flutter の統計はレール + `Expanded`)。
- `design.css` に統計の画面の並びのクラス (`.stats` など) を足した (design.css の冒頭の一覧に追記した)。
- 散布図は 0039 の `ChartFrame` ではなく `ChartScatterFrame` (縦軸 x=22、上の横罫、下の軸) にした (デザインのプレビューの散布図の軸の位置に合わせるため)。棒と折れ線は `ChartFrame` を使う。
- 散布図の横軸の両端に最小と最大を単位付きで出した (デザインのプレビューにあり、Flutter の fl_chart は出していないが、デザインを優先した)。
- 折れ線の横軸の日付は ISO 8601 の日付の部分 (`MM-DD`) を使う (`chart::period_label`)。Flutter は端末のタイムゾーンへ直してから月日を出すため、タイムゾーンによる日付のずれは表示だけの差異として残る。
- 応答の JSON の読み取り補助を `pub(crate)` にして `records/stats.rs` と共有し、`count_field` を足した。
- `dioxus-ssr` は追加していない (方針では SVG の利点として触れられているが、完了条件の検証は `wasm-bindgen-test` と定められており、クレートの追加は依存の判断が必要になるため)。マークアップの検査は `wasm-bindgen-test` の SVG 要素と計算済みスタイルで行っている。
- 統計の画面は `navigator()` を要求するため、画面全体の wasm テストは `HistoryProvider` と `MemoryHistory` とテスト用の経路でルーターの文脈を与えて描いた (`dioxus::history` は `dioxus` の再公開で、依存は増えていない)。

レビューの指摘を受けて変えたもの (方式は変えていない):

- `chart::period_label` と `is_daily_key` と `is_timestamp` を全域にした (壊れた応答のキーで文字の境界を切って panic しない。`tests/test_chart.rs` の `a_broken_key_is_not_sliced_in_the_middle_of_a_character` で固定した)。
- 折れ線の等間隔の PBT を、実装の式を写さずに隣り合う点の間隔が等しい性質で検査するように直した (ADR-0013 の方針)。
- 統計の画面の期間の組み立てに失敗したときに、読み込み中のままにせずエラーを出す防御を足した。
- 期間のチップの押下の wasm テストを足した (画面側の切り替えの配線)。
- `WidePage` の計算済みスタイルの wasm テストを足した (0039 の `WideLayout` と同じ水準)。
- 負の件数の応答の検査を足した (`count_field` の共通の分岐)。
- 購入の詳細の折れ線の検査を、`RatingHistoryChart { entries: ratings() }` の配線まで強くした。
- テスト名 `the_ten_record_screens_have_the_expected_props` を `the_record_screens_have_the_expected_props` に直した (画面が 11 になったため)。
- 作業中に、`pbt/tests/prop_values.rs` の `a_count_with_a_non_digit_is_rejected` が前後の空白を除く実装と食い違い、proptest が反例 (`"0\t"`) を出して失敗するのを見つけた。期待値の側を「空白を除いた後に数字以外が残るときだけ拒否する」に直した (実装は変えていない。既存のテストの誤り)。
