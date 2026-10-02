# Rating

1 から 5 の評価を `crema` の丸で示す。星は使わない。

- 一覧と詳細は 10 px の丸を 5 つ (`radius-full`)、評価の数だけ `crema`、残りは `line`。詳細と一覧の右端には `mono` で「4 / 5」を添えてよい (ARB の `ratingValue`)。
- 入力は 28 px の丸を 5 つ、未選択は `line-strong` の枠だけ。押した丸までを塗る。右に「4 / 5」を `value` の `ink-muted` で示す。
- 未評価は 5 つとも枠だけにし、「未評価」を `caption` の `ink-faint` で添える。評価は任意なので既定値を入れない (FR-11)。
- Dioxus では `frontend/src/ui/rating.rs` で `.rating` (入力は `.lg`) の丸を 5 つ並べる。ライブラリは足さない。
