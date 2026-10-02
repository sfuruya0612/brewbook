# ADR-0014: デザインのフォントを Google Fonts から読む

Created: 2026-09-27
Model: DeepSeek V4.1 Flash
Status: Accepted
改訂: 2026-10-02 (issue 0039。Frontend を Dioxus の Web だけにする決定 (ADR-0017) に合わせて、Flutter と iOS の記述を Web の `@font-face` と Web だけの対象に改めた)

## 背景

`docs/design` に brewbook の視覚言語 (デザインシステム) が置かれた。
本文は IBM Plex Sans JP、数値と日付は IBM Plex Mono、アプリ名だけ IBM Plex Serif で組む。
デザインの README は「3 つの族は Google Fonts から読み込む (フォントファイルは持たない)」と定め、
必要なサブセットをアセットに同梱する案も挙げている。
依存の追加は理由を ADR または issue に記す規約がある (ADR-0001)。

2026-10-01 の所有者の決定と ADR-0017 で、Frontend は Flutter をやめて Rust と Dioxus の
Web だけにする。フォントの読み込みも Flutter の `google_fonts` ではなく、Web の
`@font-face` (Google Fonts の CSS) を対象にする (2026-10-02、issue 0039)。

## 決定

- Web のフォントは `frontend/index.html` の `<link>` で Google Fonts の CSS を読み込む。
  フォントファイルはリポジトリに含めない。
- 3 つの族は次の対応で使う。
  - IBM Plex Sans JP: 本文、見出し、項目名 (400、500、600)
  - IBM Plex Mono: 数値と日付 (`value`、`value-large`。`font-variant-numeric: tabular-nums` を付ける)
  - IBM Plex Serif: brewbook の名前 (`wordmark`。見出しと本文には使わない)
- 画面内のアイコンは Material Icons Outlined の Web フォントも同じ `<link>` で読み、
  24 px、`ink` か `ink-muted` で使う (docs/design/README.md の「アイコン」)。
- フォントが読めない環境のために、`docs/design/tokens.css` のフォールバック (Hiragino Sans、
  Noto Sans JP、Noto Sans CJK JP など) をそのまま使う。実装 (`frontend/src/ui/design.css`) は
  トークンの変数を参照し、値を写さない。
- 色と余白と角丸と影はデザインのトークン (`docs/design/tokens.css`) を Tailwind CSS v4 の
  `@theme` から参照し、Paper (ライト) と Night (ダーク) の 2 テーマを `data-theme` で切り替える。
  画面は色を直に書かず、トークンの変数と Tailwind のユーティリティから取る。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| 必要なサブセットをフォントファイルとして同梱する | IBM Plex Sans JP は 1 ウェイトが約 2 MB あり、3 ウェイトで初回の転送が重くなる。デザインも「フォントファイルは持たない」としている |
| Flutter の `google_fonts` (2026-09-27 時点の解決版は 8.2.1) を使う | Frontend を Dioxus の Web だけにする決定 (ADR-0017) で、Flutter の実装ごと対象から外れた |
| 自前の `FontLoader` と HTTP 取得を書く | Web では `@font-face` と Google Fonts の CSS を読むだけで足り、取得、キャッシュ、フォールバックを自前で持つ必要がない |
| 端末の標準フォントだけを使う | 数値と日付を等幅で揃えるデザインの要件 (等幅で桁を揃える) を満たせない |

## 結果

- 初回の表示時に Google Fonts への通信が発生し、以後はブラウザのキャッシュを使う。
  通信できない環境では、`docs/design/tokens.css` のフォールバックの族で組む (情報は運べる)。
- ブラウザのテスト (`frontend:test-web`) は計算済みスタイルで書体と `tabular-nums` を検査する
  (issue 0039)。フォントファイルの取得そのものは検査しない。
- iOS は対象から外れた (ADR-0017 で Web だけにする決定)。配布の対象を広げるときは、
  オフラインでもデザインどおりに組めるようにフォントの同梱を再検討し、本 ADR を改訂する。
