# Auth

ログインとパスキーの登録。入力は登録の名前だけで、利用者名もパスワードも置かない (FR-2)。

- ログイン: 画面の中央に `mark.svg` (72 px)、`wordmark` の「brewbook」、説明 (`body`、`ink-muted`)、主要ボタン「パスキーでログイン」。画面の下端に登録用リンクの案内を `caption` で。AppBar は無い。
- 登録 (`/register?token=`): AppBar「パスキーの登録」、説明、`Field`「パスキーの名前」(1 文字以上 50 文字以下の注記)、主要ボタン「登録する」。
- トークンの 404、409、410 はバナー (`Feedback`) にそれぞれの文言 (ARB の `registerToken*`) を出し、フォームを出さない。管理者に再発行を頼む案内を続ける。
- パスキーの操作の取り消しと非対応 (`passkeyCancelled`、`passkeyUnsupported`) もバナーで示し、ボタンは押せるまま残す。
