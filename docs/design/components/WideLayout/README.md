# WideLayout

幅 840 px 以上の配置。ナビゲーションレールと、一覧 (400 px) と詳細の 2 段組。

- レールは 88 px、上に `mark.svg`、その下に抽出、購入、商品、店、統計、設定。選択中は `roast-soft` の地に `ink`、他は `ink-muted`。
- 一覧は幅 400 px で右端に `line` の罫線。選択中の行は `roast-soft`。FAB は一覧の右下。
- 詳細は残りの幅。内容は最大 720 px に収め、`Ledger` と `ReferenceTile` を 2 列に並べる。フォームも同じ位置に開く。
- 幅 840 px 未満では 1 列に戻り、一覧の行を押すと詳細を上に積む。判定は `design.css` のメディアクエリ (`@media (min-width: 840px)`) で行う。
- Dioxus では `frontend/src/ui/wide_layout.rs` の `NavigationRail`、`WideLayout`、`WidePage` で `.rail`、`.wide-layout`、`.wide-page` のクラスを組む。
