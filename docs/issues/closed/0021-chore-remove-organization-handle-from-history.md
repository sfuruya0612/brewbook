# 組織名を含むハンドルと個人のパスを Git の履歴から取り除く

Created: 2026-09-26
Model: deepseek-v4p1-flash
Completed: 2026-09-26

## 背景

`CHANGES.md` の全エントリ (19 行) の担当者は、個人名と所属組織を推測させるハンドル (`@` で始まる) である。
この行は現在のファイルにあり、ハンドルが入ったコミット以降の全コミットの `CHANGES.md` にも含まれる (2026-09-26 時点で 32 コミット)。ほかに `docs/issues/closed/0015-feat-flutter-stats-screen.md` の過去の版 1 つにも含まれる。

本 issue には個人名と組織名を書かない。
置換する文字列は、実装のときに `CHANGES.md` の担当者の行と、`docs/issues/0020-bug-worker-build-race-in-the-shared-build-directory.md` (close 済みなら `docs/issues/closed/` の同じファイル) の再現ログの絶対パスから取る。
後者はローカルの利用者名を含む (2026-09-26 時点で `/Users/<利用者名>/apps/coffee/backend/coffee_log/build/.tmp/package.json`)。

2026-09-26 の調査では、次のとおりである。

- 担当者のハンドルを含むコミットは 19 である。18 は `CHANGES.md` を、1 つ (`90ec45c`) は `docs/issues/closed/0015-feat-flutter-stats-screen.md` の当該の行を変更したコミットで、同ファイルの過去の版にもこの行があった。置換は全履歴の全ファイルを対象にする (`git log --all -S '<ハンドル>' --format='%h'` と `git show --name-only` で確認)。
- コミットメッセージにはハンドルも利用者名も含まれない (`git log --format='%B'` で確認)。
- reachable な全コミットの author と committer は `sfuruya0612` と同一のメールアドレスである (`git log --format='%an <%ae>%n%cn <%ce>' | sort -u` で確認)。所有者の指定 (2026-09-26) により、この 2 つは変更しない。
- 到達不能なオブジェクトが残っている。`git fsck --unreachable --no-reflogs` は、2026-09-26 時点で amend 前のコミット `5650c9fe` を報告する。この commit オブジェクトと `.git/logs/HEAD` の最初の行には、所属組織のドメインのメールアドレスを含む旧い identity がある。到達不能なオブジェクトは、後述の掃除で消す。
- リポジトリにリモートは設定されておらず、ブランチは `main` だけである。書き換えの影響を受ける他のクローンは無い。
- git 管理外のビルド成果物 (`backend/target/`、`backend/*/build/`、`frontend/build/` など) にも個人名を含むパスがある。これらはローカルで再生成されるたびに現在の利用者名が入るもので、コミットにも CI の成果物にも含まれない。本 issue の対象外とし、所有者が必要ならローカルで削除する。

## 対応方針

全履歴のファイルの内容とコミットメッセージを置換する。道具は `git filter-repo` を使う。

- 採らなかった案: `git filter-branch` (全履歴の書き換えに時間がかかり、置換と掃除の手順を自前で組む必要がある)、BFG Repo-Cleaner (Java の導入が要り、置換できる対象が限られる)、直近のコミットだけを書き換える (対象の文字列は 19 コミットに含まれるため消えない)、履歴を残して現在のファイルだけ直す (過去のコミットから文字列を取得できてしまう)。
- 置換は対応表のファイルで渡す。対応表は書き換えのときだけ作業ディレクトリの外に置き、リポジトリに含めない (旧い文字列を含むため)。
  - `CHANGES.md` の担当者のハンドルを `@sfuruya0612` に置換する。`--replace-text` を使う。
  - 0020 の再現ログの絶対パスのうち、利用者名の部分を `user` に置換する (`/Users/<利用者名>/apps/coffee` を `/Users/user/apps/coffee` にする)。同じ対応表に入れる。0020 のファイルが `docs/issues/closed/` にある場合は、その版のパスを取る。
  - コミットメッセージにも同じ置換を適用する。メッセージは `--replace-message` を別に渡さないと置換されない。現状は該当が無いが、取りこぼしを防ぐ。
- `git filter-repo` は 2026-09-26 時点で入っていないため、`brew install git-filter-repo` または `pipx install git-filter-repo` で入れる。入れた版を issue に記録する。
- リモートを持たない既存のリポジトリでは、`git filter-repo` は `--force` を要求する。`--force` を付けて実行する。
- 実行の前に `git bundle create ../coffee-before-rewrite.bundle --all` でバックアップを取る。バックアップはリポジトリの外に置き、書き換えの確認が終わったら削除する (バックアップ自体に旧い文字列が含まれるため)。
- 実行の前に、`git status --porcelain` が空であることを確認する (ADR-0010 から ADR-0013 と PRD の 2026-09-26 の決定がコミット済みであること。未コミットの変更があると書き換えをやり直すことになる)。
- git 管理外のビルド成果物の個人名のパスは対象外とする (背景の最後の項目)。

`--replace-text` はコミットの木の中のファイルを置換する。
`git filter-repo` は既定で、書き換えの最後に作業ツリーを書き換え後の HEAD に合わせ (`git reset --hard`)、reflog を期限切れにし、到達不能なオブジェクトを掃除する (`git gc --prune=now`)。
そのため実行の直後は `git status` がクリーンで、reflog と到達不能なオブジェクトも掃除されている。掃除の結果は完了条件で確認する。

## 履歴の書き換え後の作業

書き換えで、内容も親も変わらないコミット (ルートコミットなど) を除いて、全コミットのハッシュが変わる。
`git filter-repo` が残す `.git/filter-repo/commit-map` (旧ハッシュから新ハッシュへの対応表。変わらないコミットは同じ値が並ぶ) を使い、pending の issue にあるコミットハッシュの参照を新しい値に更新する。

| issue | 旧ハッシュ |
| --- | --- |
| `docs/issues/pending/0009-feat-photo-r2-presigned-upload.md` | 2d1c2ee |
| `docs/issues/pending/0010-feat-stats-api.md` | b9d1d55 |
| `docs/issues/pending/0012-feat-account-deletion.md` | 5805484 |
| `docs/issues/pending/0017-feat-same-origin-deployment.md` | 6e48c94、3f2f593 |
| `docs/issues/pending/0018-feat-admin-worker-cloudflare-access.md` | ea37b23、71ff292 |

closed の issue は当時の記録として変更しない。closed の issue が持つ 7 桁の数字 (`docs/issues/closed/0004-feat-webauthn-verification-core.md` の `1569601`) は Fuzzing の実行回数で、コミットハッシュではない。
旧ハッシュから新ハッシュへの対応表は、close のときに「## 解決方法」に記録する。
`CHANGES.md` の `### misc` に `[UPDATE]` のエントリを追加する。

## 完了条件

- 実行の前に `../coffee-before-rewrite.bundle` のバックアップを取り、書き換えの確認が終わった後に削除した (`test ! -e ../coffee-before-rewrite.bundle` が真) ことを issue に記録する。
- 実行の前に `git status --porcelain` が空であったことを issue に記録する。
- `git log --all --format='%H' | wc -l` が、書き換えの直前に測った値と同じである (コミットが欠けていない)。起票と PRD と ADR のコミットで数は増えているため、直前の値を基準にする。
- 全履歴の `CHANGES.md` と `docs/issues/closed/0015-feat-flutter-stats-screen.md` の担当者の行が `@sfuruya0612` だけである。`git grep -h -E '^  - @' $(git rev-list --all) | sort -u` が `  - @sfuruya0612` の 1 行になる (CHANGES.md の現在の 19 行と、closed の 0015 の過去の版の 1 行のいずれも、この 1 行に置き換わる)。
- 現在の `CHANGES.md` の担当者の行は、書き換えの直前に測った行数と本 issue の `[UPDATE]` のエントリの 1 行の合計になる (`git grep -c -h -E '^  - @' -- CHANGES.md` がその合計の数値を返す)。
- 全履歴の `/Users/` で始まるパスの利用者名が `user` だけである。対象を 0020 のファイル (close 済みなら `docs/issues/closed/0020-*.md`) に絞り、念のため CHANGES.md も加えて確認する。`git grep -h -o -E '/Users/[^/]+' $(git rev-list --all) -- CHANGES.md 'docs/issues/0020-*.md' 'docs/issues/closed/0020-*.md' | sort -u` が `/Users/user` の 1 行になる。
- `docs/issues/0020-bug-worker-build-race-in-the-shared-build-directory.md` (close 済みなら `docs/issues/closed/` の同じファイル) のパスが `/Users/user/apps/coffee/backend/coffee_log/build/.tmp/package.json` になっている。
- 対応表の左辺のそれぞれの文字列が、書き換え後の全履歴に現れない。確認のコマンドと結果を issue に記録する (コマンドは対応表から組み立て、本 issue には旧い文字列を書かない)。
- `git log --format='%an <%ae>|%cn <%ce>' | sort -u` が 1 行だけである (author と committer が全コミットで同一で、書き換えの前後で変わっていない)。
- 書き換えの前に `git fsck --unreachable --no-reflogs` が報告した到達不能なオブジェクト (2026-09-26 時点では commit `5650c9fe`) が、書き換え後に報告されない。`git fsck --unreachable --no-reflogs` が何も出さない。
- `.git/logs/HEAD` に、書き換え前の identity を含む行が無い (書き換えの前に行を控えて比較する)。
- pending の issue の 7 つのハッシュ参照 (上の表) が新しい値になっている。
- `CHANGES.md` の `### misc` に `[UPDATE]` のエントリがある。
- 入れた `git filter-repo` の版、`--replace-text` と `--replace-message` と `--force` を使ったこと、旧ハッシュから新ハッシュへの対応表を issue に記録する。
- `mise run check` が通過する。作業の前から失敗している検査がある場合は、同じ失敗だけであることを確認して issue に記録する。

## 解決方法

全履歴のファイルの内容とコミットメッセージを `git filter-repo` 2.47.0 (`git filter-repo --version` の出力は `a40bce548d2c`) で置換した。
実行の前に、作業ツリーがクリーンであること (`git status --porcelain` が空) と、直前のコミット数 36 を記録した。
`git bundle create ../coffee-before-rewrite.bundle --all` でバックアップを取った。
書き換えと検証が終わった後にバックアップを削除した (`test ! -e ../coffee-before-rewrite.bundle` が真)。
置換の対応表は作業ディレクトリの外に置き、書き換えの後に削除した (リポジトリに含めていない)。

変更の内容:

- 旧い担当者のハンドル (`CHANGES.md` の `  - @` の行の値) を `@sfuruya0612` に置換した (`--replace-text`)。
  全履歴の 19 コミット分が対象になった (`CHANGES.md` の 18 コミットと、`docs/issues/closed/0015-feat-flutter-stats-screen.md` の当該の行を変更した `90ec45c`)。
- `docs/issues/0020-bug-worker-build-race-in-the-shared-build-directory.md` の再現ログの絶対パスを `/Users/user/apps/coffee/backend/coffee_log/build/.tmp/package.json` に置換した (`--replace-text`)。
- コミットメッセージにも同じ置換を適用した (`--replace-message`)。該当は無かった。
- リモートを持たないリポジトリのため `--force` を付けて実行した。
- `.git/filter-repo/commit-map` に基づき、pending の issue のコミットハッシュ参照 7 件を更新した。

| issue | 旧ハッシュ | 新ハッシュ |
| --- | --- | --- |
| pending/0009 | 2d1c2ee | 928d9c6 |
| pending/0010 | b9d1d55 | fbdff32 |
| pending/0012 | 5805484 | 1dd2bb3 |
| pending/0017 | 6e48c94 | 4eff521 |
| pending/0017 | 3f2f593 | 31d9550 |
| pending/0018 | ea37b23 | edc3ac1 |
| pending/0018 | 71ff292 | 735681a |

- `CHANGES.md` の `### misc` に `[UPDATE]` のエントリ (「組織名を含むハンドルと個人のパスを Git の履歴から取り除く」) を追加した。
- 背景が参照する `90ec45c` は書き換えで `85c145d` になった。
  到達不能だった amend 前のコミット `5650c9fe` は掃除され、`git fsck --unreachable --no-reflogs` は何も報告しない。
- author と committer は所有者の指定どおり変更していない (`git log --format='%an <%ae>|%cn <%ce>' | sort -u` が 1 行のまま)。

完了条件の検証:

- 実行前の `git status --porcelain` が空であったことと、バックアップの作成と削除を確認した。
- `git log --all --format='%H' | wc -l` は 36 で、書き換えの直前の値と同じ (コミットは欠けていない)。
- `git grep -h -E '^  - @' $(git rev-list --all) | sort -u` が `  - @sfuruya0612` の 1 行になり、全履歴の担当者の行が `@sfuruya0612` だけになった。
- `git grep -c -h -E '^  - @' -- CHANGES.md` が 20 を返す (既存の 19 行と本 issue の `[UPDATE]` の 1 行)。
- `git grep -h -o -E '/Users/[^/]+' $(git rev-list --all) -- CHANGES.md 'docs/issues/0020-*.md' 'docs/issues/closed/0020-*.md' | sort -u` が `/Users/user` の 1 行になり、0020 のパスは置換後の値になった。
- 対応表の 2 行の左辺のそれぞれについて、全履歴に現れないことを確認した。`left=$(sed -n 'Np' <対応表> | sed 's/==>.*//'); git grep -l -F "$left" $(git rev-list --all) | wc -l` を N=1 と N=2 で実行し、両方 0 件になった (2 行目の左辺 (0020 のパスの利用者名の部分) は、完了条件の `/Users/` の検査でも重ねて確認した。対応表は既に削除した)。
- `git log --format='%an <%ae>|%cn <%ce>' | sort -u` が 1 行で、書き換えの前後で変わっていない。
- `git fsck --unreachable --no-reflogs` が何も報告しない。`.git/logs/HEAD` に書き換え前の identity を含む行は無い (書き換えの前に行を控えて比較した)。
- pending の issue の 7 件のハッシュ参照を上の表のとおり更新した。
- `mise run check` が通過した (1222 秒)。ベースラインも通過しており、新たな失敗は無い。

方針からの乖離: 無し。
方針が挙げた採らなかった案 (git filter-branch、BFG Repo-Cleaner、直近のコミットだけの書き換え、履歴を残す) は採用しなかった。

補足: 背景の「2026-09-26 時点で 32 コミット」は、起票と PRD と ADR のコミットを加える前の調査時点の値である。書き換えの後の現在は、`CHANGES.md` に担当者の行があるコミットは 34 である (増えた 2 コミットは、ハンドルが `@sfuruya0612` に置換された後の `CHANGES.md` を持つ)。
