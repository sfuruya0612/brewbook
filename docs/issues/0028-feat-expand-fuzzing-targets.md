# Fuzzing の対象を入力の文字列パーサに広げる

Created: 2026-09-26
Model: deepseek-v4p1-flash
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
