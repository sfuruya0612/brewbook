# ListRow

一覧の行。1 行目は名前 (`heading`)、2 行目は日付 (`mono`) と参照先 (`caption`、`ink-muted`)、右端は数値か評価。

- 抽出の行: 商品名、抽出日時と店、右端に `Rating` (ARB の `brewRowSubtitle`)。
- 購入の行: 左に 44 px の写真 (無ければ `paper-sunken` にカメラの印)、商品名、購入日と店、右端に重量と価格 (`mono`)。
- 商品の行: 商品名、生産地と精製方法と品種を「/」でつなぐ、その下にタグ (小さな `Chip`)。
- 店の行: 店名と住所。住所が無ければ「住所は未設定」を `ink-muted` で。
- 高さは 64 px 以上、内側は `space-3` と `space-4`、下端に `line` の罫線。行の地は `paper`。押すと詳細へ。
- アーカイブ済みは名前と 2 行目を `ink-muted` に落とし、右端に「アーカイブ済み」のバッジ (`paper-sunken`)。既定では一覧に出さず、上部の「アーカイブ済みを含める」の切り替えで出す (FR-12)。
- 広い画面で選択中の行は `roast-soft` の地。
- 末尾までスクロールしたら次の 50 件を読む。読み込み中は最後の行の下に「読み込み中」を `caption` で。
- Flutter では `ListTile` (`minTileHeight: 64`、`contentPadding` 16) と `Divider`。写真は `ClipRRect` (`radius-sm`)。
