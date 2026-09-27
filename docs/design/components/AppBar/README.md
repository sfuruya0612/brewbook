# AppBar

画面の題を左に、操作を右に置く高さ 56 px の帯。地は `paper`、下端に `line` の罫線を引き、影は付けない。

- ホームだけ題を `wordmark` (IBM Plex Serif、24 px) の「brewbook」にする。他の画面は `title` で画面名 (ARB の `*Title`)。
- 先頭の操作は 1 つ: 一覧と詳細は戻る (`Icons.arrow_back_ios_new_outlined`)、フォームは閉じる (`Icons.close_outlined`)。
- 末尾の操作は最大 2 つ。詳細はアーカイブと編集のアイコン、フォームは「保存」の文字ボタン (`crema-ink`)。ホームはメニュー (`Icons.more_vert_outlined`) だけ。
- Flutter では `AppBar` に `elevation: 0`、`scrolledUnderElevation: 0`、`bottom` に `PreferredSize` で 1 px の `Divider` を置く。`centerTitle: false`。
