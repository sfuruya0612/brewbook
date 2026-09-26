# TLA+ で認証の状態遷移をモデル検査する

Created: 2026-09-26
Model: deepseek-v4p1-flash
Completed: 2026-09-27
対応 ADR: ADR-0013 (docs/adr/0013-five-test-layers.md)
関連 PRD: FR-1、FR-2、FR-3、FR-4、セキュリティ、制約と前提 (テスト)
依存: 0023

本文のパスは、依存の 0023 の完了後の名前 (`backend/brew_book` など) で書く。

## 背景

PRD の「制約と前提」のテストは 2026-09-26 に 5 種 (E2E、PBT、Fuzzing、形式手法、単体) となり、ADR-0013 が形式手法の対象 (WebAuthn のチャレンジ、登録用トークン、セッション、パスキーの状態遷移) を定めた。
形式手法の成果物 (TLA+ の仕様と `formal/`) はまだ無い。

対象の実装は次のとおり (2026-09-26 に確認)。

- チャレンジの発行、検索、消費は `backend/brew_book/src/auth/mod.rs` の `issue_challenge`、`find_challenge`、`consume_challenge` にある。チャレンジは 1 回だけ消費できる (FR-1、FR-2)。
- セッションの発行、解決、ログアウトは `backend/brew_book/src/auth/session.rs` の `prepare`、`resolve`、`logout` にある。セッションはクッキーで保持し、有効期限は発行から 30 日である (FR-4、セキュリティ)。
- パスキーの追加と削除は `backend/brew_book/src/auth/passkeys.rs` の `complete` と `delete` にある。利用者のパスキーが 1 つしかないときの削除は 409 で拒否される (FR-3)。
- 登録用トークンは 1 回だけ使用でき、期限は発行から 24 時間である (FR-1、FR-17)。

## 目的

認証の状態遷移を TLA+ の仕様として書き、TLC のモデル検査で不変条件が常に成り立つことを確かめる。

## 設計判断

- 仕様は `formal/auth.tla`、設定は `formal/auth.cfg` とし、リポジトリのルートに `formal/` を作る。
- モデル化する状態は、利用者 (管理者が作成した直後はパスキーを持たない)、パスキー (登録、追加、削除)、チャレンジ (発行、消費、期限切れ)、登録用トークン (発行、使用、期限切れ)、セッション (発行、ログアウト、期限切れ) とする。
- 検査する不変条件は次の 4 つとする。
  - チャレンジは最大 1 回しか消費されない (FR-1、FR-2)。
  - 登録用トークンは最大 1 回しか使用されない (FR-1)。
  - パスキーを 1 つ以上持つ利用者は、削除によってパスキーが 0 個にならない (FR-3)。作成直後のパスキー 0 個の利用者は対象の状態として持つ。
  - ログアウトしたセッションは API の要求を通さない (FR-4)。
- 定数は小さくする (利用者 2、パスキー 2、チャレンジ 2、セッション 2)。状態空間を有限にし、TLC の実行を短時間で終わらせる。
- 道具は Temurin 25 (LTS) の JDK を `mise.toml` の `[tools]` で固定する (`java = "temurin-25.0.4+101.0.LTS"`。2026-09-26 に `mise ls-remote java` で確認)。
- `tla2tools.jar` は版 (v1.7.4) と SHA-256 (`936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`。2026-09-26 に取得して確認) をタスクに固定して取得する (mise のレジストリに tla2tools は無いため、`mise.toml` の `[tools]` では取得できない。ADR-0013)。jar はリポジトリに含めず、`formal/.cache/` を `.gitignore` に追加する。チェックサムが一致しない場合はタスクを失敗させる。
- `mise run formal` は、jar を取得して (存在すれば再取得しない) TLC を実行する。`mise run check` に `formal` を依存として追加する。
- ADR-0009 はタスクに独自のロジックを書かないと定める。jar の取得とチェックサムの検証は、既存の `verify-deploy` と `frontend:test-integration` と同じく、標準のコマンド (curl、`test`、sha256 の照合) の組み合わせでタスクに書く。Rust や Dart のコードは追加しない。
- 仕様と実装の対応 (どの不変条件がどの要求に対応するか) を `formal/README.md` に書く。
- 採らなかった案: Kani で Rust の実装を検証する (対象はデータベースの行をまたぐ状態遷移で、実装の詳細に依存する。ADR-0013)、Alloy で書く (所有者が TLA+ を選んだ。ADR-0013)、手元での実行だけにする (変更のたびに CI で検査できない。ADR-0013)。

## 完了条件

- `formal/auth.tla` と `formal/auth.cfg` があり、`mise run formal` が TLC を実行して不変条件の違反を報告しない (出力に `No error has been found` を含む)。
- 検査が機能していることを、1 つの不変条件を意図的に破る変更で確かめる (TLC が違反を報告する)。確認の方法と結果を issue に記録する。
- `mise run check` に `formal` が含まれ、通過する。TLC の実行時間が 2 分以内であることを issue に記録する。作業の前から失敗している検査がある場合は、同じ失敗だけであることを確認して issue に記録する。
- `mise ls --current java` が temurin-25.0.4+101.0.LTS を返す。
- `tla2tools.jar` のチェックサムが一致しない場合に `mise run formal` が失敗する (確認の方法と結果を issue に記録する)。
- `formal/.cache/` が `.gitignore` にあり、`tla2tools.jar` がリポジトリにコミットされていない。
- `formal/README.md` に、モデル化した状態、不変条件と要求 (FR-1、FR-2、FR-3、FR-4) の対応、実行方法がある。
- `CHANGES.md` の `### misc` に `[UPDATE]` のエントリがある。

## 解決方法

認証の状態遷移の TLA+ の仕様と、TLC によるモデル検査を追加した (ADR-0013)。

- `formal/auth.tla` に、利用者、パスキー、チャレンジ、登録用トークン、セッションの状態遷移を書いた。登録用トークンの発行先の利用者は、発行のときに状態として決まる。
- `formal/auth.cfg` に、定数 (利用者 2、パスキー 2、チャレンジ 2、登録用トークン 2、セッション 2) と検査の設定 (仕様、不変条件、デッドロックの検査) を書いた。
- 不変条件は 4 つである。チャレンジは登録とログインを合わせて最大 1 回しか結果を生まない (FR-1、FR-2)。結果は上書きせず蓄積し、同じチャレンジの二重消費を検出できるようにした。登録用トークンは最大 1 回しか使用されない (FR-1)。パスキーを 1 つ以上持った利用者は削除によって 0 個にならない (FR-3)。ログアウトしたセッションは再び有効にならない (FR-4)。`Inv` はこれらと型の不変条件 (`TypeOK`) の連言である。
- `mise.toml` の `[tools]` に `java = "temurin-25.0.4+101.0.LTS"` を追加し、`[tasks.formal]` を追加した。タスクは `tla2tools.jar` (v1.7.4) を GitHub のリリースから `formal/.cache/` に取得し (存在すれば再取得しない。接続と転送のタイムアウト付き)、SHA-256 (`936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`) を確認してから TLC を実行する。TLC のメタディレクトリは `-metadir formal/.cache/states` に置き、リポジトリの直下に `states/` を作らない。`[tasks.check]` の依存に `formal` を追加した。
- `.gitignore` に `/formal/.cache/` を追加し、jar と TLC の作業ディレクトリをリポジトリに含めないようにした。
- `formal/README.md` に、モデル化した状態、不変条件と要求の対応、実行方法、検査が機能していることの確認方法を書いた。
- `CHANGES.md` の `### misc` に `[UPDATE]` のエントリ (「認証の状態遷移の TLA+ の仕様と TLC の検査を追加する」) を追加した。

完了条件の検証:

- `mise run formal` が TLC を実行し、出力に `Model checking completed. No error has been found.` を含む (91,132 states generated、11,644 distinct states、Finished in 00s)。デッドロックは検出されなかった。
- 検査が機能していることを、リポジトリの外に置いた一時コピーへの 2 つの変更で確認した。1 つ目は `DeletePasskey` の `Cardinality(passkeys[u]) > 1` のガードを外す変更で、TLC は `Invariant Inv is violated.` を報告し、AddPasskey の後に DeletePasskey を実行してパスキーが 0 個になる利用者の反例を示した (終了コード 12)。2 つ目は `Register` の消費済みチェック (`c \in issuedChallenge \ consumedChallenge` の `\ consumedChallenge`) を外す変更で、同じチャレンジで 2 回登録でき、チャレンジの不変条件の違反を検出した (終了コード 12)。結果は蓄積する符号化にしたため、この二重消費を検出できる。
- `mise run check` に `formal` が含まれ、通過した (直列実行の 2 回目で 1779.41 秒)。ベースライン (作業前) の `mise run check` は通過していた (exit 0)。直列実行を選んだ理由は、既定の並列実行で起きる Chrome の起動失敗 (0029) を避けるためである。CI は既定の並列実行の `mise run check` を使うため、CI で通過することは未確認である。TLC の実行時間は約 1 秒で、2 分以内である。この後に TLC のメタディレクトリを `formal/.cache/states` に置く変更を加え、`mise run formal` を再実行して通過を確認した (91,132 states generated、11,644 distinct states)。
- `mise ls --current java` が `java temurin-25.0.4+101.0.LTS ~/apps/coffee/mise.toml temurin-25.0.4+101.0.LTS` を返す。
- チェックサムの検査が機能することを、`formal/.cache/tla2tools.jar` の末尾 1 バイトを変えて `mise run formal` を実行して確認した。タスクは `tla2tools.jar checksum mismatch: delete formal/.cache/tla2tools.jar and retry` で失敗した (終了コード 1)。その後に jar を戻し、`mise run formal` が再び通過することを確認した。チェックサムの確認に使う `shasum` が無い場合は、別のメッセージ (`shasum is required to verify tla2tools.jar but was not found`) で失敗する。
- `formal/.cache/` が `.gitignore` にあり、`tla2tools.jar` はコミットしていない (`git status` に現れない)。
- `formal/README.md` に、モデル化した状態、不変条件と要求 (FR-1、FR-2、FR-3、FR-4) の対応、実行方法、jar とメタディレクトリの扱い (TLC の作業ディレクトリは `-metadir formal/.cache/states` に置き、成功時は `-cleanup` で消える)、検査が機能していることの確認方法がある。
- `CHANGES.md` の `### misc` に `[UPDATE]` のエントリがある。

方針からの乖離: 無し。
補足: 設計判断の定数の列挙 (利用者 2、パスキー 2、チャレンジ 2、セッション 2) には登録用トークンが無い。モデル化の対象にトークンが含まれるため、登録用トークン 2 を加えた (状態空間を小さく保つ範囲の追加である)。
補足: 仕様の作成中に、TLC が 2 つの仕様の誤りを検出した。1 つは `Login` の `UNCHANGED` から `issuedChallenge` が抜けていたこと、1 つは `ExpireChallenge` と `ExpireToken` が消費済みのものも外せたことである。どちらも仕様側を直し、不変条件の検査で検出できることを確かめた。
補足: 登録用トークンの発行先は、当初は設定ファイルの定数関数 (`tokenUser`) で与える設計だったが、TLC の設定ファイルが関数の定義を受け付けなかったため、発行のときに状態 (`tokenOwner`) として決める形にした。方式 (不変条件と対象) は変えていない。
補足: チャレンジの結果の蓄積はパスキーの集合で行うため、同じチャレンジで同じパスキーの値が別の利用者に登録される場合 (パスキーは利用者に紐づく一意な資格情報であり、現実には起きない) は検出できない。異なるパスキーでの二重消費は検出できる (検査の確認の 2 つ目)。この限界はレビューの指摘 (低) として認識し、実害が無いため受け入れた。
