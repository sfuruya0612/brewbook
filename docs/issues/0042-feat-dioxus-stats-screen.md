# 統計の画面を Dioxus で実装する

Created: 2026-10-02
Model: deepseek-v4p1-flash
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

---
