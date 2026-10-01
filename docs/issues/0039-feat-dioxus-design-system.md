# デザインシステムを Tailwind CSS で実装する

Created: 2026-10-02
Model: deepseek-v4p1-flash
対応 ADR: ADR-0017、ADR-0014 (docs/adr/0014-google-fonts-for-the-design.md)
関連 PRD: なし (デザインは `docs/design/` が原本)
依存: 0037, 0038

## 背景

本 issue は、2026-10-01 の所有者の決定 (デザインはそのままにコードだけを Rust に置き換える。ADR-0017) から起票した。

`docs/design/` に視覚言語がある。
`docs/design/tokens.css` は Paper と Night の 2 テーマの CSS 変数 (色、余白、角丸、書体、影、フォーカスリング) を持つ。
`docs/design/components/bundle.css` は部品 10 種 (AppBar、Button、Field、Chip、Rating、ListRow、Ledger、ReferenceTile、Feedback、Charts) と画面のクラスを持つ。
`docs/design/components/<名前>/preview.html` はブラウザで開けるプレビューで、`docs/design/screenshots/` に両テーマの描画結果がある。
Flutter の実装は `frontend/lib/theme/tokens.dart` と `frontend/lib/theme/app_theme.dart` に写し、`frontend/lib/widgets/` の 17 ファイルで部品を組んでいる。

## 目的

Dioxus の部品 10 種と 2 段組のレイアウトが、`docs/design/` のプレビューと同じ見た目で使えるようにする。

## 設計判断

- Tailwind CSS v4 の `@theme` に `docs/design/tokens.css` の CSS 変数を写し、`bg-paper`、`text-ink`、`p-4`、`rounded-md` などのユーティリティをデザイントークンに結び付ける。
  原本の値を変えるときは `docs/design/` を先に変え、実装は追随する。
- 部品の CSS は `docs/design/components/bundle.css` のクラスを Tailwind の `@layer components` に写す案と、`rsx!` にユーティリティを直接書く案を比較し、原本との対応が追いやすい方を実装の最初に決めて記録する。
  プレビューと同じクラス名を残すと、原本との突き合わせが機械的にできるためである。
- アイコンは Material Icons Outlined の Web フォントを Google Fonts から読み、24 px、`ink` か `ink-muted` で使う (デザインの README の指示。Flutter の `Icons.*_outlined` に対応する)。
- フォントは IBM Plex Sans JP、IBM Plex Mono、IBM Plex Serif を Google Fonts の CSS から読み、数値と日付に `font-variant-numeric: tabular-nums` を付ける (ADR-0014 の決定を Web の `@font-face` で実現する。Flutter の `google_fonts` は不要になる)。
- 2 テーマは `data-theme` 属性 (paper と night) で切り替える。
  既定のテーマと `prefers-color-scheme` への対応は実装の最初に所有者へ確認して決め、結果を issue に記録する。
- 部品の値の検証は、`wasm-bindgen-test` (ブラウザで動くテスト) で計算済みスタイルを取り、`docs/design/tokens.css` の期待値と比較する (色、余白、角丸、書体)。
  部品のスクリーンショットの取得と `docs/design/screenshots/` との比較は、0044 の WebDriver のハーネスで行う。
- `Charts` はグラフの色と軸の枠だけを作る (中身は 0042 が実装する)。
- 採らない案: `tailwind-rs` などのユーティリティのクレートを使う (Tailwind の CLI と `dx` の連携で足りる)、CSS を手書きして Tailwind を使わない (ADR-0017 で Tailwind を使うと決めた)、スタイルを全部ユーティリティで書き原本のクラスを捨てる (原本との突き合わせができなくなる)。

## 完了条件

- `frontend/src/ui/` に部品 10 種と 2 段組のレイアウト (幅 840 px 以上でナビゲーションレール 88 px と一覧 400 px、詳細は最大 720 px) がある。
- 部品の色、余白、角丸、書体が `docs/design/tokens.css` の値と一致する (`wasm-bindgen-test` の計算済みスタイルの比較で確認する)。
- Paper と Night の 2 テーマが `data-theme` で切り替わり、既定のテーマと `prefers-color-scheme` への対応が実装の最初に決まって issue に記録されている。
- 3 つの書体が Google Fonts から読み込まれ、数値と日付に `tabular-nums` が付く。
- アイコンが Material Icons Outlined の 24 px で表示される。
- 部品のスクリーンショットは 0044 で取得する (この issue では計算済みスタイルの比較まで)。
- ADR-0014 の Flutter と iOS の記述 (`google_fonts`、`FontLoader`、アセットへの同梱の再検討) を、Web の `@font-face` と Web だけの対象に合わせて改訂する。
- `mise run dioxus:lint`、`mise run dioxus:test`、`mise run dioxus:test-web` が成功する。
- `mise run check` が通過する。

## 関連

- 0037 がツールチェーンとタスクを、0038 が基盤を作る。
- 0040 から 0043 の画面がこの部品を使う。

---
