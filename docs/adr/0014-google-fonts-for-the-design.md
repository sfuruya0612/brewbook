# ADR-0014: デザインのフォントを Google Fonts から読む

Created: 2026-09-27
Model: DeepSeek V4.1 Flash
Status: Accepted

## 背景

`docs/design` に brewbook の視覚言語 (デザインシステム) が置かれた。
本文は IBM Plex Sans JP、数値と日付は IBM Plex Mono、アプリ名だけ IBM Plex Serif で組む。
デザインの README は「3 つの族は Google Fonts から読み込む (フォントファイルは持たない)」と定め、
Flutter では `google_fonts` を使うか、必要なサブセットをアセットに同梱する、としている。
依存の追加は理由を ADR または issue に記す規約がある (ADR-0001)。

## 決定

- Flutter のフォントは `google_fonts` (2026-09-27 時点の解決版は 8.2.1) で Google Fonts から読み込む。
  フォントファイルはリポジトリに含めない。
- 3 つの族は次の対応で使う。
  - IBM Plex Sans JP: 本文、見出し、項目名 (400、500、600)
  - IBM Plex Mono: 数値と日付 (`value`、`value-large`。`FontFeature.tabularFigures()` を付ける)
  - IBM Plex Serif: brewbook の名前 (`wordmark`。見出しと本文には使わない)
- フォントが読めない環境のために `fontFamilyFallback` (Hiragino Sans、Noto Sans JP、Noto Sans CJK JP など) を
  各スタイルに付け、デザインの README のフォールバックに合わせる。
- 色と余白と角丸と影はデザインのトークン (`docs/design/tokens.json`) を `lib/theme/` に写し、
  Paper (ライト) と Night (ダーク) の 2 テーマを `ThemeData` として組む。
  画面は色を直に書かず、`ThemeData` と `BrewbookTheme` (ThemeExtension) から取る。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| 必要なサブセットをフォントファイルとして同梱する | IBM Plex Sans JP は 1 ウェイトが約 2 MB あり、3 ウェイトで初回の転送が重くなる。デザインも「フォントファイルは持たない」としている |
| Web だけ `index.html` の CSS で読み込む | Flutter Web の描画 (CanvasKit) はページの CSS の `@font-face` を参照せず、フォントは Flutter が `FontLoader` で登録する必要がある |
| 自前の `FontLoader` と HTTP 取得を書く | 取得、キャッシュ、フォールバックを自前で持つことになる。`google_fonts` は同じことを行い、実績がある |
| 端末の標準フォントだけを使う | 数値と日付を等幅で揃えるデザインの要件 (等幅で桁を揃える) を満たせない |

## 結果

- 初回の表示時に Google Fonts への通信が発生し、以後はブラウザまたは端末のキャッシュを使う。
  通信できない環境では、埋め込んだフォールバックの族で組む (情報は運べる)。
- ウィジェットテストではフォントの取得を待たないため、テストは端末の既定のフォントで描く。
  文言と配置の検証には影響しない。
- iOS を配布するときは、オフラインでもデザインどおりに組めるようにアセットへの同梱を再検討する。
  そのときは本 ADR を改訂する。
