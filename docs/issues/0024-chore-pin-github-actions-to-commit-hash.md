# GitHub Actions の uses をコミットハッシュに固定する

Created: 2026-09-26
Model: deepseek-v4p1-flash
対応 ADR: ADR-0011 (docs/adr/0011-github-actions-commit-hash-pinning.md)
関連 PRD: 運用 (CI)

## 背景

`.github/workflows/ci.yml` の `uses` は `actions/checkout@v7.0.1` と `jdx/mise-action@v4.3.0` のタグで固定されている。
Git のタグは書き換え可能で、同じタグが別のコミットを指すようになると、CI はそのまま別のコードを実行する。
ADR-0009 はツールの版を `mise.toml` で固定する方針を定めるが、action の固定方法は定めていない。
ADR-0011 でコミットハッシュで固定することを決めた。

## 対応方針

2 つの `uses` を 40 桁のコミットハッシュに置き換え、バージョンのコメントを残す。

- `actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1` (v7.0.1)
- `jdx/mise-action@c2a87611a18de5b3828c5652fe268e992400cb5c` (v4.3.0)

上の 2 つの値は 2026-09-26 に GitHub の API (`https://api.github.com/repos/<owner>/<repo>/git/ref/tags/<tag>`) で、タグの指すコミットとして確認した。
コメントにバージョンと確認した日を残す。
既存のコメント「action の版はタグで固定する (ADR-0009)」は ADR-0011 を参照する形に直す。
更新は所有者が手動で行い、Dependabot などは導入しない (ADR-0011)。

## 完了条件

- `ci.yml` の全ての `uses` が 40 桁のコミットハッシュで、バージョンのコメントを持つ。
- 2 つのハッシュが、`actions/checkout` の v7.0.1 と `jdx/mise-action` の v4.3.0 のタグの指すコミットと一致する。確認の方法と確認日を issue に記録する。
- `ci.yml` のコメントが ADR-0011 を参照する。
- `mise run check` が通過する。作業の前から失敗している検査がある場合は、同じ失敗だけであることを確認して issue に記録する。
- 変更を `main` に push した後の CI が成功する。push は所有者が行う (実装者は push しない。リポジトリにリモートが設定されていないため、CI の結果の確認は所有者に委ねることを issue に記録する)。
- `CHANGES.md` の `### misc` に `[UPDATE]` のエントリがある。
