# TLA+ で認証の状態遷移をモデル検査する

Created: 2026-09-26
Model: deepseek-v4p1-flash
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
