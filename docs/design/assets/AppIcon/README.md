# AppIcon

クラフト紙の地 (`#e6d5b8`) に `roast` (`#4a2f1c`) の印。ドリッパーの一滴が記録の行に落ちる (brew と book)。

- `icon.svg` — 原本。角丸 (512 px で半径 116) で四隅は透過。
- `icon-maskable.svg` — 全面を地で塗り、印を中央 80% の安全領域に収めた版 (Web の maskable アイコン用)。
- `icon-dark.svg` — 地と印を入れ替えた版。ダークの壁紙で並べたいときの代替。既定はクラフト紙の版。
- `Icon-192.png`、`Icon-512.png` — `frontend/public/icons/` の同名ファイルを置き換える。
- `Icon-maskable-192.png`、`Icon-maskable-512.png` — 同じく `frontend/public/icons/` の maskable の版。
- `apple-touch-icon-180.png` — iOS のホーム画面用 (全面塗り。iOS が角を丸める)。iOS は対象外のため実装には移していない (ADR-0017)。
- `favicon-32.png`、`favicon-48.png` — `frontend/public/favicon.png` を置き換える。
- `Icon-dark-512.png` — `icon-dark.svg` の書き出し。

`manifest.json` の `background_color` と `theme_color` は `paper` (`#f4ede2`) にする。印の色は固定で、テーマに追従しない。
