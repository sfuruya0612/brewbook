# PRD のディレクトリ名を docs/prd に直す

Created: 2026-09-26
Model: deepseek-v4p1-flash

## 背景

PRD は `docs/pdr/coffee-log.md` に置かれている。
`pdr` は `prd` (Product Requirements Document) の綴りの誤りである。
`docs/pdr` を指す箇所は `README.md` の 5 行目と `docs/adr/0001-backend-rust-on-cloudflare-workers.md` の 9 行目の 2 つである (2026-09-26 に `git grep -n 'docs/pdr'` で確認)。
ほかに ADR-0010 の背景と対応表にも `docs/pdr/coffee-log.md` があるが、これは改名の前の記録なので変更しない。

## 対応方針

- `git mv docs/pdr docs/prd` で移動する。
- `README.md` と `docs/adr/0001-backend-rust-on-cloudflare-workers.md` の参照を `docs/prd/coffee-log.md` に更新する。
- `CHANGES.md` の `### misc` に `[UPDATE] PRD のディレクトリ名を docs/prd に直す` を追加し、担当者を `@sfuruya0612` とする。

## 完了条件

- `docs/prd/coffee-log.md` が存在し、`docs/pdr` が存在しない。
- `git grep -n 'docs/pdr' -- ':!docs/issues' ':!docs/adr/0010-app-name-and-namespace-brewbook.md'` が何も出さない (issue のファイル (close 後は `docs/issues/closed/` に移る) と、ADR-0010 の改名の前の記録を除く)。
- `README.md` と `docs/adr/0001-backend-rust-on-cloudflare-workers.md` の参照が `docs/prd/coffee-log.md` を指す。
- `CHANGES.md` の `### misc` に `[UPDATE]` のエントリがある。
- `mise run check` が通過する。作業の前から失敗している検査がある場合は、同じ失敗だけであることを確認して issue に記録する。
