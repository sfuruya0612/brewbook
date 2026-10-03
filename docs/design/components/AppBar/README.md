# AppBar

画面の帯。高さ 56 px、地は `paper`、下端に `line` の罫線を引き、影は付けない。先頭からハンバーガーボタン、先頭の操作、アプリの印とアプリ名、その下の画面名、末尾の操作の順に置く。

- ハンバーガーボタンは記録、統計、設定の全ての画面と全ての幅で出す (0047)。押すとメニューの面がハンバーガーの下に左寄せで開く。メニューは抽出、購入、商品、店、統計、設定の 6 項目と、区切りの下にログアウトを置く。項目を押すと対応する経路へ移って閉じ、現在の経路と同じ項目を押したときは遷移せずメニューだけ閉じる。
- 認証前の画面 (登録) はハンバーガーを出さない。印とアプリ名と画面名だけを出す。
- アプリの印は `mark.svg` の 32 px。アプリ名は `appTitle` の「brewbook」を IBM Plex Serif の 18 px で組む。アプリ名を押すとホームへ戻る (ホームでは何もしない)。
- 画面名は ARB の `*Title` をアプリ名の下に `label` (13 px、`ink-muted`) で出す。ホームは「抽出」(`brewsLabel`)。
- 先頭の操作は 1 つ: 一覧と詳細は戻る (`arrow_back_ios_new`)、フォームは閉じる (`close`)。
- 末尾の操作は最大 2 つ。詳細はアーカイブと編集のアイコン、フォームは「保存」の文字ボタン (`crema-ink`)。
- Dioxus では `frontend/src/ui/app_bar.rs` の `AppBar` で `.appbar` の帯を組み、下端の罫線は `border-bottom: 1px solid var(--line)` で引く (影は付けない)。経路の操作は `menu` と `on_home` の prop で受け取り、`frontend/src/screens/app_nav.rs` の `ScreenAppBar` が `AppNav` と組む。ハンバーガーは `.nav`、印とアプリ名は `.app-name`、画面名は `.ttl`、先頭の操作は `.lead`、末尾の操作は `.iconbtn` と `.textbtn`。メニューの面は `.menu.left` で左寄せにする。
