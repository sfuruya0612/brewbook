# AppBar

画面の題を左に、操作を右に置く高さ 56 px の帯。地は `paper`、下端に `line` の罫線を引き、影は付けない。

- ホームだけ題を `wordmark` (IBM Plex Serif、24 px) の「brewbook」にする。他の画面は `title` で画面名 (ARB の `*Title`)。
- 先頭の操作は 1 つ: 一覧と詳細は戻る (`arrow_back_ios_new`)、フォームは閉じる (`close`)。
- 末尾の操作は最大 2 つ。詳細はアーカイブと編集のアイコン、フォームは「保存」の文字ボタン (`crema-ink`)。ホームはメニュー (`more_vert`) だけ。
- Dioxus では `frontend/src/ui/app_bar.rs` の `AppBar` で `.appbar` の帯を組み、下端の罫線は `border-bottom: 1px solid var(--line)` で引く (影は付けない)。題は左揃えの `.ttl`、ホームは `.ttl.wordmark`、先頭の操作は `.lead`、末尾の操作は `.iconbtn` と `.textbtn`。
