# Ledger

詳細画面の項目名と値の表。項目名は左に `label` (`ink-muted`)、値は右揃えの `mono`、行ごとに `line` の罫線。

- 数値は単位を `caption` (`ink-muted`) で右に添える: 「15.0 g」「92.0 ℃」「165 秒」「1,980 JPY」。文字列 (抽出方法、挽き目、焙煎度) は `sans` で右揃え。
- 未設定の項目は行を消さず、「未設定」を `ink-faint` で出す (ARB の `unsetLabel`)。表の行数が画面ごとに変わらないようにする。
- 抽出の詳細: 豆の量、湯量、湯の温度、時間、抽出方法、挽き目、評価。購入の詳細: 購入日、焙煎度、焙煎日、価格、重量、写真 (差し替えと削除の文字ボタン)。
- 感想は表に入れず、表の下に「感想」の `label` と `body` の段落で置く。
- Dioxus では `frontend/src/ui/ledger.rs` で `.ledger` の grid (`max-content 1fr`) を組み、`.k` と `.v` の行に `line` の罫線を引く。値は右揃えで `tabular-nums` にする。
