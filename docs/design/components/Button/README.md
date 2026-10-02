# Button

主要 (`roast` の塗り) は 1 画面に 1 つ、他は次点 (枠だけ) と文字ボタンにする。

- primary: `roast` の塗り、`on-roast` の文字。フォームの「保存」、ログイン、登録、確認ダイアログの肯定に使う。
- secondary: 透明の地に `line-strong` の枠、`ink` の文字。「写真を選ぶ」「パスキーを追加」「エクスポートをダウンロード」など画面内の補助操作。
- text: `crema-ink` の文字だけ。AppBar の「保存」、ダイアログの「キャンセル」、「再試行」。
- danger: `signal` の塗りと `on-signal` の文字は確認ダイアログの「削除する」だけ。設定画面の入口は `signal` の枠と文字 (danger-outline) にして、押しただけでは何も消えないことを形で示す。
- 拡張 FAB (`radius-lg`、`shadow-float`) は一覧の右下に 1 つ。文言は動詞 (「抽出を記録」「購入を記録」「商品を登録」「店を登録」)。
- 高さは 48 px、小 (sm) は 36 px。文字は 15 px の 500。アイコンは 20 px で文字の左。
- 無効化は `paper-sunken` の地に `ink-faint` の文字。処理中は文言を変えず無効化だけする。
- Dioxus では `frontend/src/ui/button.rs` の `Button` と `Fab` で `.btn` と `.fab` のクラスを組む。種類は `ButtonVariant` (primary、secondary、text、danger、danger-text、danger-outline)、大きさは `ButtonSize` で指定する。
