# Fuzzing の対象を入力の文字列パーサに広げる

Created: 2026-09-26
Model: deepseek-v4p1-flash
Completed: 2026-09-27
対応 ADR: ADR-0013 (docs/adr/0013-five-test-layers.md)
関連 PRD: 制約と前提 (テスト)
依存: 0023

対象の関数のパスは、依存の 0023 の完了後の名前 (`backend/brew_book_core` など) で書く。

## 背景

fuzz のクレートの `fuzz_targets/parse.rs` は CBOR、COSE、base64url、WebAuthn の検証を対象にする。
他の入力のパーサは、主に単体テストで、一部は PBT で境界値を確認しているが、任意入力に対するパニック安全性 (Fuzzing) の対象になっていない。
ADR-0013 で、Fuzzing を入力の文字列のパーサ全体に広げることを決めた。

対象になる関数は次のとおり (2026-09-26 に共有のクレートの `src` で確認)。

- `cursor.rs` の `parse_page_size`
- `datetime.rs` の `parse_epoch_millis`、`is_valid_date`
- `ids.rs` の `uuid_bytes`
- `query.rs` の `parse_include_archived`、`parse_suggestion_field`
- `stats.rs` の `parse_granularity`、`parse_offset_minutes`、`parse_period`

## 目的

任意の文字列に対するパーサのパニック安全性を確かめ、クラッシュの余地を減らす。

## 設計判断

- fuzz のクレートに新しいターゲット `parse_strings.rs` を追加する。UTF-8 の入力文字列を上の 9 つの関数に与える。既存の `parse.rs` はバイト列の入力 (CBOR、COSE、base64url、WebAuthn) の対象として残す。
- 入力の長さの上限は設けない。実行時間の上限は手元の実行で `-max_total_time=60` として与える。
- 実行は手元で行い、CI では対象の型検査だけを行う (nightly を要するため。0004 の決定を維持し、ADR-0013 に記録する)。nightly は `mise.toml` に追加しない。stable の版固定 (ADR-0009) と二重管理になるためである。
- 実行には cargo-fuzz が必要である。cargo-fuzz は `mise.toml` の `[tools]` に無いため、README に `cargo install cargo-fuzz` の導入を書く。
- 発見したパニックは修正し、回帰テスト (PBT か単体テスト) を追加する。
- 採らなかった案: 文字列のパーサを既存の `parse.rs` に足す (バイト列の入力と UTF-8 の入力の意図が混ざる)、CI で Fuzzing を実行する (nightly の導入と実行時間が加わる。0004 の決定に反する)、nightly を `mise.toml` に追加する (stable の版固定と二重管理になる。ADR-0009)。

## 完了条件

- `backend/brew_book_core/fuzz/fuzz_targets/parse_strings.rs` が追加され、上の 9 つの関数全てに任意の文字列を与える。
- `cargo check --manifest-path backend/brew_book_core/fuzz/Cargo.toml --bins` が通過する (`mise run lint` に含まれる)。
- `backend/brew_book_core` で `cargo +nightly fuzz run parse_strings -- -max_total_time=60` を実行してクラッシュが無い。実行日と結果 (runs、cov、終了コード) を issue に記録する。クラッシュが出た場合は修正し、回帰テストを追加して再実行する。
- README に、cargo-fuzz の導入、nightly の導入、実行の作業ディレクトリ (`backend/brew_book_core`)、`cargo +nightly fuzz run` の使い方、対象のターゲット (`parse`、`parse_strings`) を書く。
- `mise run check` が通過する。作業の前から失敗している検査がある場合は、同じ失敗だけであることを確認して issue に記録する。
- `CHANGES.md` の `### misc` に `[ADD]` のエントリがある。

## 解決方法

Fuzzing の対象を入力の文字列のパーサに広げた (ADR-0013)。

- `backend/brew_book_core/fuzz/fuzz_targets/parse_strings.rs` を追加した。任意の UTF-8 の入力文字列を、`cursor::parse_page_size`、`datetime::parse_epoch_millis`、`datetime::is_valid_date`、`ids::uuid_bytes`、`query::parse_include_archived`、`query::parse_suggestion_field`、`stats::parse_granularity`、`stats::parse_offset_minutes`、`stats::parse_period` (開始と終了は入力の分割した別々の文字列) の 9 つの関数に与える。
  あわせて、レビューで指摘された同じ共有クレートの文字列入力のパーサ (`webauthn::client_data_challenge`、`cursor::CursorKey::decode`、`auth::session_token`、`auth::validate_passkey_name`、`photo::parse_pending_key`、`routes::pattern_matches`、`routes::match_route`、`records::validate_name`、`validate_day`、`validate_timestamp`、`validate_currency`、`validate_decimal`、`validate_flavor_notes`) にも入力を与える。
  任意の入力では成功側の分岐に届きにくいため、形式に合う代表的な入力 (`SEEDS`: 日時の形式、日付、`day` と `month`、`true` と `false`、ページサイズ、サジェストの 8 項目、UUID、期間の 2 つの日付、カーソルと clientDataJSON の base64url、`pending/` のキー、Cookie ヘッダ、名前、通貨、数の形式、経路のパターンとパス) も毎回与える。`photo::parse_pending_key` には利用者 ID を、`routes::match_route` にはメソッドを固定して渡し、成功側の分岐にも届くようにする。
- `backend/brew_book_core/fuzz/Cargo.toml` にターゲット `parse_strings` を追加した (既存の `parse` は、CBOR、COSE、base64url、WebAuthn のバイト列の対象として残す)。
- `README.md` に Fuzzing の節を追加し、nightly と cargo-fuzz の導入、実行の作業ディレクトリ (`backend/brew_book_core`)、2 つのターゲットの実行方法 (`cargo +nightly fuzz run <ターゲット> -- -max_total_time=60`) を書いた。CI では実行せず、対象の型検査だけを行う。
- `CHANGES.md` の `### misc` に `[ADD]` のエントリ (「Fuzzing の対象を入力の文字列パーサに広げる」) を追加した。

完了条件の検証:

- `backend/brew_book_core/fuzz/fuzz_targets/parse_strings.rs` が、上の 9 つの関数全てに任意の文字列を与える (あわせて、同じクレートの文字列入力のパーサ 13 にも与える)。
- `cargo check --manifest-path backend/brew_book_core/fuzz/Cargo.toml --bins` が通過した (`mise run lint` に含まれる)。
- `backend/brew_book_core` で `cargo +nightly fuzz run parse_strings -- -max_total_time=60` を実行してクラッシュが無かった (2026-09-27 に 204,121 runs、cov: 1719、ft: 3958、実行 61 秒、終了コード 0)。`fuzz/artifacts/parse_strings/` にクラッシュの入力は無い (毎回の代表入りの実行で 1 回あたりの処理が増えるため、実行回数は前の版より少ない)。
  対象を広げた最初の実行では、ターゲット自身の `text.split_at(text.len() / 2)` が文字の境界を外れて panic した (入力は 2 バイトの UTF-8 文字 1 つ。パニックはターゲットの `parse_strings.rs` の分割で起きており、`brew_book_core` のパーサではない)。分割の位置を文字の境界に合わせて直し、再実行でクラッシュが無いことを確認した。パーサ側の不具合ではないため、回帰テストは追加していない (fuzz クレートは workspace の外にあり、`mise run check` の `cargo test --workspace` では実行されないため、回帰テストを置いても再発は防げない)。
- README に、cargo-fuzz の導入、nightly の導入、実行の作業ディレクトリ、`cargo +nightly fuzz run` の使い方、対象のターゲット (`parse`、`parse_strings`) がある。
- `mise run check` が通過した (直列実行で 1698.48 秒)。最終のコードでは 5 回実行し、4 回は 0030 に登録したスケールテスト (miniflare の接続断、確率約 50%) が失敗しただけで、他の全タスク (fmt、lint、frontend の 4 種、backend の統合テスト、formal など) は毎回成功し、5 回目で通過した (所有者の承認を得て、0030 の失敗だけを記録して充足とした)。既定の並列実行は Chrome の起動失敗 (0029) を避けるため使わなかった。
- check の後に、成功側の分岐に届くように fuzz ターゲットを直した (利用者 ID とメソッドの固定、シードの追加)。このファイルを型検査する `mise run lint` を再実行して通過した (check の他のタスクはこのファイルを実行しないため、影響しない)。
- `CHANGES.md` の `### misc` に `[ADD]` のエントリがある。

方針からの乖離: 無し。
補足: クラッシュはターゲット自身の不具合であり、回帰テストは追加していない (fuzz クレートは workspace の外にあり、check では型検査だけを行う)。
補足: fuzz クレートは `mise run check` の fmt と clippy の対象外である (既存の `parse` と同じ。今回のターゲットには手元で `cargo fmt` を掛けた)。
