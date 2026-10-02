# デザインシステムを Tailwind CSS で実装する

Created: 2026-10-02
Model: deepseek-v4p1-flash
Completed: 2026-10-02
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
- 既定のテーマと `prefers-color-scheme` への対応は、2026-10-02 に所有者が決めた。既定は Paper とし、`prefers-color-scheme: dark` のときは Night を表示する (Flutter の現在の挙動 (theme と darkTheme を渡し、themeMode を指定しない) と同じ)。
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

## 解決方法

デザインシステム (部品 10 種、2 段組のレイアウト、2 テーマ、3 書体、アイコン) を Tailwind CSS で実装した (ADR-0017、ADR-0014)。

- 部品の CSS の方式は、`docs/design/components/bundle.css` のクラス名をそのまま `frontend/src/ui/design.css` の `@layer components` に写し、`rsx!` からも同じクラス名 (`appbar`、`btn primary`、`row` など) を使う方式を採用した (原本との突き合わせが機械的にできるため)。`tests/test_ui.rs` の `the_component_css_covers_the_classes_of_the_bundle` が原本のクラスの網羅を検査する。
- `frontend/tailwind.css` が `docs/design/tokens.css` を `@import` (`layer(base)`) で読み、`@theme inline` で色と余白のユーティリティを原本の変数 (`--paper` など) に結び付けた。角丸と書体は Tailwind の名前空間と名前が同じで自己参照になるため `@theme` に値を写し、`tests/test_ui.rs` の `the_theme_values_in_tailwind_css_match_the_tokens` が原本との一致を検査する。影は Paper と Night で変わるため通常の `@theme` に写し、原本の `[data-theme]` の上書きを効かせる。
- `frontend/src/ui/` に部品 10 種 (AppBar、Button (Fab、IconButton)、Field (TextField)、Chip (TagChip)、Rating (RatingInput)、ListRow (ListThumb、RowValue、ArchivedBadge)、Ledger (LedgerRow)、ReferenceTile (ReferenceChain、PickerTile)、Feedback (Banner、Snackbar、ConfirmDialog)、Charts (ChartSection、ChartFrame、StatTile、StatTiles)) と `WideLayout` (幅 840 px 以上でナビゲーションレール 88 px と一覧 400 px、詳細は最大 720 px) を実装した。
- 2 テーマは `data-theme` (paper / night) で切り替える。`frontend/index.html` の起動スクリプトが `matchMedia("(prefers-color-scheme: dark)")` で既定を決め (既定 Paper。2026-10-02 の所有者の決定と同じ)、OS の設定の変更にも追随する。Dioxus の起動前に付けるため、最初の描画からテーマが合う。
- `frontend/index.html` の `<link>` で Google Fonts の 3 書体 (IBM Plex Sans JP、IBM Plex Mono、IBM Plex Serif) と Material Icons Outlined を読み込む。数値と日付には `font-variant-numeric: tabular-nums` を付けた。アイコンは 24 px で `ink` か `ink-muted` にした。
- `frontend/tests/test_ui.rs` が静的な配線 (Google Fonts の読み込み、起動スクリプト、Tailwind の結び付け、原本のクラスの網羅、アイコンの名前) を検査する (5 件)。
- `frontend/tests/test_ui_web.rs` が wasm-bindgen-test で実部品をテストページに描画し、`getComputedStyle` の計算済みスタイルを `docs/design/tokens.css` の期待値と比較する (17 件。部品に加えて、Fab、ListThumb、ReferenceChain と、起動スクリプトのテーマの決定)。2 段組の検査のため `frontend/webdriver.json` でブラウザの画面幅を 1280 px にした。
- `frontend/pbt/tests/prop_ui.rs` が評価の丸の点灯の性質 (2 件) を検査する。
- `frontend/Cargo.toml` の web-sys に `CssStyleDeclaration` と `NodeList` を足した (計算済みスタイルの取得に必要)。
- `mise.toml` の `dioxus:test-web` に `frontend/webdriver.json` の説明を足した。
- `docs/adr/0014-google-fonts-for-the-design.md` を改訂した (Flutter の `google_fonts`、`FontLoader`、アセット同梱の記述を、Web の `<link>` と `@font-face`、Web だけの対象に合わせた。「改訂: 2026-10-02」を追記)。

完了条件の検証:

- 部品 10 種と 2 段組のレイアウト: 上記のとおり。`tests/test_ui_web.rs` の 17 件と `tests/test_ui.rs` の 5 件で確認した。
- 部品の色、余白、角丸、書体が `docs/design/tokens.css` の値と一致する: `tests/test_ui_web.rs` の計算済みスタイルの比較 (`the_button_matches_the_paper_tokens`、`the_field_matches_the_tokens`、`the_ledger_matches_the_tokens`、`the_list_row_matches_the_tokens`、`the_app_bar_matches_the_tokens`、`the_chip_matches_the_tokens`、`the_rating_matches_the_tokens`、`the_reference_tile_matches_the_tokens`、`the_feedback_matches_the_tokens`、`the_charts_match_the_tokens`、`the_fab_matches_the_tokens`、`the_list_thumb_matches_the_tokens`、`the_reference_chain_matches_the_tokens`、`the_wide_layout_matches_the_documented_widths`、`the_icon_is_material_icons_outlined_at_24_px`、`the_data_theme_switches_between_paper_and_night`) で確認した (ブラウザで 17 passed)。
- 2 テーマと既定の決定: `data-theme` の切り替えは `the_data_theme_switches_between_paper_and_night`、既定と `prefers-color-scheme` の対応は `the_boot_script_sets_the_theme_from_the_system` (matchMedia を差し替えて `index.html` の起動スクリプトを実行し、ダークのとき night、そうでないとき paper になることを確認する) で確認した。所有者の決定は 2026-10-02 に issue へ記録済み。
- 3 書体と tabular-nums: `tests/test_ui.rs` の `index_html_loads_the_fonts_and_icons_from_google_fonts` と、`tests/test_ui_web.rs` の計算値の検査で確認した。
- アイコン: `the_icon_is_material_icons_outlined_at_24_px` (font-family、24 px、色) と `tests/test_ui.rs` の `the_icon_names_are_not_the_flutter_api_names` (リガチャの基底名であること) で確認した。
- スクリーンショットは 0044: この issue では取得していない (`docs/design/screenshots/` は未変更)。
- ADR-0014 の改訂: 上記のとおり。
- `mise run dioxus:lint`、`mise run dioxus:test`、`mise run dioxus:test-web`: 作業ツリーで成功した (lint は clippy と fmt とも警告なし、native は 94 件、ブラウザは 17 件)。
- `mise run check` が通過する: 所有者の指示により、移行の全 issue の完了後に 1 回だけ実行するため、この issue では未実行 (最終確認に委ねる)。

方針を保った実装詳細の乖離:

- 部品の CSS を `frontend/tailwind.css` 内ではなく `frontend/src/ui/design.css` (追跡ファイル) に分けた。ブラウザテストが dx の生成物 (`frontend/public/tailwind.css` は gitignore 対象) に依存せず、原本の CSS を注入して計算済みスタイルを検査できるようにするため。クラスは `@layer components` にあり、方針の方式は保っている。
- アイコンを Web フォントで実装するため、bundle.css の `svg` を対象にした寸法・色の規則を `.icon` にも足した。ほかに `.chip.mini`、`.chip-remove`、`.row .tags`、`.rating .num.none`、`.mark` の色、`.wide-layout` 系、`.photo img` / `.thumb img`、`.rail .item` の button 用リセット、入力の placeholder の色 (`.field .box .in::placeholder`。原本の `.ph` と同じ `--ink-faint`) を足した (design.css の冒頭コメントに列挙)。
- アイコンの名前は Material Icons のリガチャの基底名 (例: `settings`、`chevron_right`) を `Icon` の prop で受ける (Flutter の `Icons.*_outlined` の API 名とは違う)。
- レビューの指摘を受けて変えたもの (方式は変えていない): アイコンの名前を基底名に直し、`_outlined` が混ざらないことを静的なテストで固定した。`LedgerRow` は `.k` と `.v` を wrapper で包まず、`.ledger` の直接の子にした (原本と同じ 2 列の grid にするため)。`role="button"` の `ListRow`、`ReferenceTile`、`PickerTile` は Enter と Space でも押せるようにした (ハンドラは `EventHandler<()>` に変えた)。`RatingInput` は `role="radiogroup"` を付け、丸をフォーカス可能にして Enter、Space、左右の矢印で選べるようにし、`aria-checked` は選んだ値の丸だけ true にした。`ConfirmDialog` は `aria-labelledby` を付けた。入力の placeholder の色を原本に合わせ、計算済みスタイルで検査するようにした。起動スクリプトのテーマの決定は、matchMedia を差し替えて実行するブラウザテストで検査するようにした。Fab、ListThumb、ReferenceChain の計算済みスタイルの検査を足した。

レビューの指摘のうち、対応しないと決めたもの:

- 840 px の境界 (839 px と 840 px) の表示: 完了条件は「840 px 以上」であり、境界は CSS の `min-width: 840px` / `max-width: 839px` と 1280 px のブラウザテスト、0044 のスクリーンショットで確認する。
- 生成された Tailwind の CSS (`frontend/public/tailwind.css`) をブラウザテストで読むこと: 部品は design.css のクラスで組んでおり、ユーティリティの効果は 0040 以降の画面で使う時点で確認する。テストはユーティリティと原本の変数の結び付けを静的に検査する。
