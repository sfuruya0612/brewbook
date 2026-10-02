# brewbook design (2026-09-27)

Design System アーティファクト「brewbook」の内容一式。

- `README.md` — ブランドブック (色、文字、余白、部品、画面、アイコン、Dioxus と Tailwind への対応表)
- `tokens.json` — トークンの原本。`tokens.css` はそこから生成した CSS 変数 (Paper と Night の 2 テーマ)
- `assets/AppIcon/` — アプリアイコン (SVG 原本と PNG 書き出し)。`Icon-*.png` は `frontend/public/icons/` の同名ファイルの置き換え用
- `assets/Marks/` — 印とワードマークの SVG
- `components/<名前>/preview.html` — 部品と画面のプレビュー (ブラウザで開ける単体の HTML。`tokens.css` を同じ階層構造で読む前提なので、そのまま開くときは `<head>` に `../../tokens.css` を足す)
- `components/<名前>/README.md` — 各部品と画面のガイドライン
- `components/bundle.css` — 部品の CSS の参照
- `screenshots/` — 各プレビューを両テーマで描画した PNG
