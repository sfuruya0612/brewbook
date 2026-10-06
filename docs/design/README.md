brewbook は、自宅で淹れたコーヒーを記録する Web アプリ (Flutter Web、後に iOS) のための視覚言語である。
方向は「記録帳らしい落ち着き」。クラフト紙の地に濃いコーヒー色の文字と罫線を引き、数値がよく読める静かな帳簿にする。装飾は足さず、記録そのものを主役にする。

## コンセプト

- 紙に書く帳簿を手本にする。面を重ねて影で区切るのではなく、`line` の罫線で区切る。影は浮くもの (`shadow-float`: FAB、メニュー、ダイアログ、スナックバー) だけに使う。
- 数値と日付は等幅 (`value`、`value-large`) で組み、右揃えで桁を揃える。単位は `caption` を `ink-muted` で添える。
- 色は地 (`paper` の 3 段)、文字 (`ink` の 3 段)、ブランドの濃いコーヒー色 (`roast`)、琥珀のアクセント (`crema`)、エラー (`signal`) の 5 系統だけ。それ以外の色を画面に足さない。
- 主要な操作は 1 画面に 1 つ。`roast` の塗り (主要ボタンか拡張 FAB) はその 1 つにだけ使う。
- 抽出の記録はホームから 2 画面 (ホームと「抽出を記録」) で完了させる (PRD の成功指標)。フォームは 1 画面に収め、途中で画面を増やさない。

## 文章

- 文体は PRD と ARB に合わせる。「です・ます」で短く、命令形にしない。「保存しました。」「記録がありません」。
- 項目名は日本語 UI では訳語を使う: Producer は生産者、Origin は生産国、Region は地域、Process は精製方法、Variety は品種、Roast は焙煎度、Roast Date は焙煎日、Flavor Notes はフレーバーノート (FR-16)。
- 数字と単位の間、日本語と英数字の間には半角スペースを置く: 「15.0 g」「92.0 ℃」「165 秒」「1,980 JPY」「2026-09-26 08:12」。
- 日付は `YYYY-MM-DD`、時刻は 24 時間の `HH:MM`。通貨は ISO 4217 のコードをそのまま表示し、記号に置き換えない。
- 絵文字を使わない (所有者の共通規約)。感嘆符も使わない。
- 「必須」は項目名の横に `label` の太さ 400 で添える。任意の項目には何も付けない。
- アプリ名は常に小文字の brewbook。文中でも見出しでも大文字にしない。

## 色

- 地は `paper` (画面)、`paper-raised` (行、カード、シート、ダイアログ)、`paper-sunken` (入力欄、写真の枠、バッジ) の 3 段。段の差だけで面を分け、枠線や影で持ち上げない。
- 文字は `ink` (本文と数値)、`ink-muted` (補足、項目名、単位)、`ink-faint` (プレースホルダーと無効化した操作だけ) の 3 段。`ink-faint` で情報を運ばない。
- `roast` は主要な操作の塗り、選択中のチップ、抽出回数と購入金額の棒に使う。塗りの上の文字は必ず `on-roast`。白を直に書かない。
- `roast-soft` は参照先をたどるタイル (購入、商品、店) と選択中の行の地。上の文字は `ink` と `ink-muted`。
- `crema` は評価の印、豆の消費量と購入重量の棒、散布図の点、折れ線に使う塗り。ライトの `paper` の上では 3.2:1 なので本文の文字にはしない。文字として使うときは `crema-ink` (リンク、サジェストの一致部分、評価の数字)。
- `crema-soft` はフレーバーノートのタグの地。上の文字は `ink`。
- `signal` はエラーと取り消せない操作だけ。検証エラーの文字と枠、通信エラーのバナーの文字 (`signal-soft` の地)、アカウント削除の確認ボタンの塗り (文字は `on-signal`)。成功に色を使わない。「保存しました。」はスナックバーの文字だけで伝える。
- 罫線は `line`。意味を持つ線 (入力欄の枠、チップの枠、グラフの軸) は `line-strong`。
- キーボードフォーカスは `focus-ring` (2 px の `paper` の隙間の外に 2 px の `crema-ink` の実線)。
- 上に挙げた文字と地の組は、Paper と Night の両テーマで 4.5:1 以上 (`line-strong` は意味を持つ線なので 3:1 以上)。`crema` の塗りの上には文字を載せない。

## タイポグラフィ

- 本文は IBM Plex Sans JP (`sans`)。太さは 400、500、600 の 3 段だけで、700 以上は使わない。
- 数値と日付は IBM Plex Mono (`mono`) の `value` と `value-large`。`font-variant-numeric: tabular-nums` を必ず指定する。
- brewbook の名前だけを IBM Plex Serif (`serif`) の `wordmark` で組む。見出しや本文に serif を使わない。
- 段階は `title` (画面の題) > `heading` (見出し、行の 1 行目) > `body` > `label` (項目名、チップ、ボタン) > `caption` (行の 2 行目、単位、注記)。
- 3 つの族は Google Fonts から読み込む (フォントファイルは持たない)。Flutter では `google_fonts` を使うか、必要なサブセットをアセットに同梱する。依存の追加は ADR に理由を記す (ADR-0001 の規約)。IBM Plex が使えない環境では `sans` のフォールバック (Hiragino Sans、Noto Sans JP) で組む。

## 余白、角丸、レイアウト

- 4 px を基準にした 8 段。画面の左右は `space-4`、入力欄の間は `space-5`、セクションの間は `space-6`、統計のグラフの間は `space-8`。
- 角は `radius-sm` (入力欄、バッジ、写真)、`radius-md` (ボタン、カード、タイル、ダイアログ)、`radius-lg` (ボトムシート、拡張 FAB)。チップと評価の印だけ `radius-full`。
- 幅 840 px 未満は 1 列。AppBar は 56 px、一覧の行は 64 px 以上、入力欄とボタンは 48 px。
- 画面の移動はドロワー (抽出、購入、商品、店、統計、設定、ログアウト) で行う。幅 840 px 未満では AppBar のハンバーガーで開き、840 px 以上では常設 (240 px) にして、一覧 (400 px) と詳細の 2 段組を右に置く (`WideLayout`)。詳細の内容は最大 720 px に収める。
- AppBar は 3 つの形: 最上位の画面は「ハンバーガー、印、画面名」、詳細は「戻る、画面名、操作」、フォームは「閉じる、画面名、キャンセル、保存」。フォームにはメニューと印を出さない (`AppBar`)。

## 部品と画面

- 部品は `AppBar`、`Button`、`Field`、`Chip`、`Rating`、`ListRow`、`Ledger`、`ReferenceTile`、`Feedback`、`Charts` の 10 種。それぞれのガイドラインが、Flutter のどのウィジェットで作るかを示す。
- 画面は `Auth`、`Home`、`BrewForm`、`Detail`、`Lists`、`RecordForms`、`Stats`、`Settings`、`WideLayout`。ルーターに登録する画面 (ログイン、登録、ホーム、抽出の詳細と入力、購入の一覧と詳細と入力、商品の一覧と入力、店の一覧と入力、統計、設定) を全て覆う。
- 一覧はどの記録も同じ `ListRow` で並べる。1 行目は名前、2 行目は日付 (等幅) と参照先、右端は数値か評価。アーカイブ済みは文字を `ink-muted` に落として「アーカイブ済み」のバッジを付ける。
- 詳細は `Ledger` (項目名と値の表) と `ReferenceTile` (購入から商品、店へたどる連鎖) の 2 つで組む。
- グラフは fl_chart で描き、色は `chart-count` (件数と金額) と `chart-grams` (グラム) の 2 つだけ。通貨はグラフを分けて表し、色で分けない。
- `components/bundle.css` は上の部品の CSS の参照 (クラス名と寸法)。プレビューはこれと同じものを埋め込んでいる。

## アイコン

- アプリアイコンは `assets/AppIcon/`。クラフト紙の地 (`#e6d5b8`) に `roast` の印: ドリッパーの一滴が記録の行に落ちる (brew と book)。`icon.svg` は角丸で透過、`icon-maskable.svg` は全面塗りで印を中央 80% に収める。`Icon-192.png` と `Icon-512.png` は `frontend/web/icons/` の同名ファイルを、`favicon-32.png` は `frontend/web/favicon.png` を置き換える用。`apple-touch-icon-180.png` は iOS 用。`manifest.json` の `background_color` と `theme_color` は `#f4ede2` にする。
- 印とワードマークは `assets/Marks/`。`mark.svg` は AppBar の左、ドロワーの頭、ログイン画面、記録が無いときの表示に、`wordmark.svg` は Plex Serif が読み込めない場所 (メール、README) に使う。ダークでは `mark-night.svg` と `wordmark-night.svg`。
- 画面内のアイコンは Material Icons の outlined (Flutter の `Icons.*_outlined`) を 24 px、`ink` か `ink-muted` で使う。塗りつぶしの版と混ぜない。プレビューの線画 (1.8 px の線、丸い端) は同じ調子の代用で、実装では Material Icons を使う。
- ハンバーガーは `Icons.menu_outlined`。記録の種類を表す印: 抽出は `Icons.format_list_bulleted_outlined`、購入は `Icons.shopping_bag_outlined`、商品は `Icons.spa_outlined`、店は `Icons.storefront_outlined`、統計は `Icons.bar_chart_outlined`、設定は `Icons.settings_outlined`。

## 状態

- 読み込み中は「読み込み中」の文字だけ。スケルトンやスピナーを足さない。
- 記録が無いときは `mark.svg` を `ink-faint` の濃さで置き、「記録がありません」と、次にすることを 1 文 (`caption`、`ink-muted`)。
- 通信エラーは `Feedback` のバナー (`signal-soft` の地、`signal` の文字、「再試行」)。フォームの検証エラーは項目の枠を `signal` にして、下に `caption` で理由を書く。
- 保存、アーカイブ、アーカイブ解除はスナックバー (`ink` の地に `paper` の文字)。取り消せる操作には `crema` の「元に戻す」を付けてよい。
- 取り消せない操作 (アカウント削除) は確認ダイアログを経る。確認のボタンだけを `signal` の塗りにし、キャンセルは文字ボタン (FR-15)。

## Flutter への落とし込み

| Flutter | トークン |
| --- | --- |
| `ColorScheme.surface` / `surfaceContainer` / `surfaceContainerLowest` | `paper` / `paper-raised` / `paper-sunken` |
| `ColorScheme.onSurface` / `onSurfaceVariant` | `ink` / `ink-muted` |
| `ColorScheme.primary` / `onPrimary` / `primaryContainer` | `roast` / `on-roast` / `roast-soft` |
| `ColorScheme.secondary` / `secondaryContainer` | `crema-ink` / `crema-soft` |
| `ColorScheme.error` / `errorContainer` / `onError` | `signal` / `signal-soft` / `on-signal` |
| `ColorScheme.outline` / `outlineVariant` | `line-strong` / `line` |
| `TextTheme.titleLarge` / `titleMedium` / `bodyLarge` / `labelMedium` / `bodySmall` | `title` / `heading` / `body` / `label` / `caption` |
| `FilledButton` / `OutlinedButton` / `TextButton` | `Button` の primary / secondary / text |
| `InputDecorationTheme` (filled、`radius-sm`、枠 `line-strong`) | `Field` |
| `ListTile` (`minTileHeight` 64) | `ListRow` |
| `FloatingActionButton.extended` (`radius-lg`) | `Button` の FAB |
| `NavigationDrawer` (`Scaffold.drawer`、840 px 以上は常設) | `AppBar` のドロワー、`WideLayout` |

- `ThemeData` は `useMaterial3: true`、`brightness` ごとに上の対応で `ColorScheme` を組む。`elevation` は FAB、メニュー、ダイアログ以外を 0 にする。
- `Divider` の色は `line`、太さ 1。`Card` は使わず、`paper-raised` の `Container` と `Divider` で組む。
