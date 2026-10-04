# Field

項目名を上に `label` (`ink-muted`) で置き、48 px の枠 (`paper-sunken` の地、`line-strong` の枠、`radius-sm`) に値を入れる。

- 数値 (豆の量、湯量、湯の温度、時間、価格、重量) と日付は `mono` で組み、単位 (g、℃、秒) を `caption` で右端に添える。実装ではキーボードを数値にする。
- 日付はブラウザ標準の date input、時刻は time input で選ぶ (`frontend/src/ui/field.rs` の `TextFieldKind::Date` と `TextFieldKind::Time`)。値は `YYYY-MM-DD` と `HH:MM` のまま持ち、表示の形式はブラウザの言語に従う。ピッカーはブラウザが持つため `icon` と `placeholder` を使わず、形式の案内は `help` に `Key::DayFormatHint` (「YYYY-MM-DD」) と `Key::TimeFormatHint` (「HH:MM」) で出す。抽出日時は日付と時刻の 2 欄を横に並べる。
- 必須は項目名の横に「必須」を 400 で添える。任意の項目には何も付けない。
- 自由記述の 8 項目 (生産者、生産国、地域、精製方法、品種、焙煎度、抽出方法、挽き目) は入力中に候補を最大 20 件、枠の直下に `paper-raised` の一覧で出し、一致した先頭部分を `crema-ink` の 600 で示す (FR-13)。候補に無い値もそのまま入力できる。
- フォーカスは枠を `crema-ink` にして `focus-ring` を付ける。エラーは枠を `signal` にし、下に理由を `caption` の `signal` で書く (ARB の `validation*`)。
- 感想 (複数行) は最小 88 px で、行数に合わせて伸ばす。
- Dioxus では `frontend/src/ui/field.rs` の `Field` で `.field` と `.box` のクラスを組む。地は `paper-sunken`、枠は `line-strong`、角は `radius-sm`、内側は 12 px。項目名は上に別に置く (浮動ラベルにしない)。フォーカスは `.focus`、エラーは `.error` を付ける。
