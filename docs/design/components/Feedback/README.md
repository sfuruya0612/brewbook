# Feedback

エラーのバナー、スナックバー、確認ダイアログの 3 つ。色は `signal` と `ink` だけで、成功に色を足さない。

- バナー: `signal-soft` の地に `signal` の文字、`radius-md`。通信エラーと 401 以外の API エラーは右端に「再試行」(`signal` の 600)。フォームの検証は「入力の内容を確認してください。」を上に出し、項目ごとの理由は `Field` に書く。画面の上端、`space-4` の内側に置く。
- スナックバー: `ink` の地に `paper` の文字、`radius-md`、`shadow-float`。「保存しました。」「アーカイブしました。」「パスキーを追加しました。」。取り消せる操作には `crema` の「元に戻す」を右に付けてよい。画面に数えない (PRD の成功指標)。
- 確認ダイアログ: `paper-raised` の地、`radius-md`、内側 `space-6`、題は `title`、本文は `body` の `ink-muted`。操作は右揃えでキャンセル (text) と肯定。取り消せない操作の肯定だけ `signal` の塗り (FR-15)。
- Dioxus では `frontend/src/ui/feedback.rs` の `Banner`、`Snackbar`、`ConfirmDialog` で `.banner`、`.snack`、`.scrim` と `.dialog` のクラスを組む。
