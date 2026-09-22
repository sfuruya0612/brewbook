# WebAuthn の検証コアと CBOR と COSE のパーサを実装する

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-22
対応 ADR: ADR-0004 (docs/adr/0004-passkey-only-authentication.md)
関連 PRD: 制約と前提 (WebAuthn の自前実装とテスト)
依存: 0001

## 背景

ADR-0004 は、WebAuthn の検証を外部クレートを使わず自前で実装し、範囲を ES256 と attestation none に限定すると決めた。
暗号は RustCrypto の `p256` (ecdsa feature) と `sha2` を使い、CBOR のデコードは WebAuthn が使う範囲だけを自前で実装する。
検証ロジックはバインディングに依存しない純粋な関数として書き、公開されているテストベクタを使った単体テストと、CBOR と COSE のパーサに対する PBT と Fuzzing を持つ (PRD の制約と前提)。
現状は `coffee_log_core` の枠 (0001) だけで、検証のロジックが無い。
検証コアは、認証の API (0005) から独立して実装と検証ができるため、本 issue に分ける。

## 目的

`coffee_log_core` に、WebAuthn の登録とログインの検証を純粋な関数として実装する。

## 設計判断

- 実装する検証は次に限定する (ADR-0004)。
  - `clientDataJSON` の `type` (登録は `webauthn.create`、ログインは `webauthn.get`)、`challenge`、`origin` の一致。
  - `authenticatorData` の `rpIdHash` (RP ID の SHA-256)、フラグ (`UP` と `UV`、登録時は `AT`)、署名カウンタ。
  - attestation object の CBOR デコード、`fmt: none` の受け入れ、COSE キー (EC2、P-256) の取り出し。
  - ES256 の署名検証 (署名の対象は `authenticatorData` と `clientDataJSON` の SHA-256)。
- CBOR のデコーダは map、bytes、text、integer、array だけを扱う。
  indefinite length は受け付けず (CTAP2 の canonical CBOR は indefinite length を禁止する)、余分なバイトが残る入力も拒否する。
  パーサは入力の長さに比例して動き、入力のサイズの上限 (64 KiB) を超える場合は拒否する。
- COSE キーは EC2 (kty 2)、alg -7、crv P-256 (1)、x と y が 32 バイトであることだけを受け付ける。
- 検証の関数は、入力を構造体 (base64url の文字列、バイト列) で受け取り、エラーを型で返す。
  時刻、乱数、D1 に依存しない。
- 署名カウンタの判定は ADR-0004 の規則 (両方 0 なら省略、今回が保存値以下なら拒否、大きければ更新) を純粋な関数にし、呼び出し側 (0005) が更新する。
  拒否の理由を呼び出し側が区別できるよう、エラーの型で表す (署名の検証失敗、カウンタの後退など)。
- 公開されている WebAuthn のテストベクタを使い、出典をテストのコメントに書く。
  登記の成功、attestation none 以外の拒否、ES256 以外の拒否、origin の不一致、challenge の不一致、UP と UV の欠落、署名の不一致を検証する。
- CBOR と COSE のパーサは PBT を持つ。
  PBT はテスト用の最小のエンコーダを書いて、生成した CBOR 構造のデコードの往復と、値の型ごとの不変条件を検証する。
- Fuzzing は `cargo fuzz` のターゲット (`backend/coffee_log_core/fuzz/`) を置き、任意のバイト列で panic しないことを検証する。
  `cargo fuzz` は nightly を要するため CI では実行せず、手元で 60 秒実行した結果を issue 本文に追記する (nightly を CI に足すと ADR-0009 の版固定と二重管理になる。この扱いは「## 未確定論点」に記す)。
  CI のクラッシュ耐性は PBT が担う。PBT は生成と検証のプロパティ、Fuzzing は任意入力のクラッシュ耐性という役割の分担にする (所有者の Rust のテスト規約)。
- 依存に `p256` (ecdsa feature) と `sha2` を追加する (ADR-0004 が定める。依存を追加する理由はここに記す)。
  `webauthn-rs`、`ciborium`、`serde_cbor` は追加しない。
- 採らない案: `webauthn-rs` (OpenSSL 依存で wasm32 ではビルドできない見込み。ADR-0004)、RS256 の追加 (`rsa` クレートが増える。ADR-0004)、CBOR の汎用クレート (WebAuthn が使う型だけで足り、依存を増やさない方針に合う。ADR-0004)、常にカウンタの後退を拒否する (同期パスキーでログインできなくなる。ADR-0004)。

## 未確定論点

- PRD は Fuzzing を持つことと CI が全ての自動テストを実行することを求める (制約と前提)。
  `cargo fuzz` は nightly を要するため CI では実行せず、手元で実行した結果を issue 本文に記録する。
  この解釈が所有者の意図と合わない場合は、fuzzing 用の nightly を CI に足す案を ADR-0009 の改訂として提案する。

## 完了条件

- 検証の各経路 (成功、attestation の不一致、ES256 以外、origin と challenge の不一致、UP と UV の欠落、署名の不一致、カウンタの後退) が公開テストベクタを含む単体テストで確認できる。
- CBOR と COSE のパーサの PBT が通る (往復、境界値、巨大な入力の拒否)。
- `cargo fuzz` のターゲットがリポジトリにあり、手元で 60 秒実行して panic が無いことを確認し、実行結果を issue 本文に追記する。
- `cargo test -p coffee_log_core` と `cargo clippy --all-targets -- -D warnings` が通る。
- `mise run check` が通過する。

## 関連

- 0005 がこの検証コアを使って認証の API を実装する。

## Fuzzing の実行結果

2026-09-22 に `backend/coffee_log_core` で `cargo +nightly fuzz run parse -- -max_total_time=60` を実行した。

- 結果: `Done 1569601 runs in 61 second(s)`、最終行 `#1569601 DONE   cov: 540 ft: 2117 corp: 506/39Kb lim: 4096 exec/s: 25731 rss: 481Mb`
- 終了コード 0、クラッシュ無し (`fuzz/artifacts/parse/` は空)
- ターゲットは `clientDataJSON` の challenge を入力の符号化と一致させて組み立て、ログインの `authenticatorData` は RP ID のハッシュで始めるため、検証の関数は `clientDataJSON` の照合と rpIdHash の検査を越えて、フラグ、 `authenticatorData`、 attestation object の解析に到達する (署名の検証は有効な署名が要るため到達しない)

## 未確定論点の扱い

2026-09-22 に所有者が「CI では実施せず、ローカルで実行する」ことを確認した。

Fuzzing は CI では実行せず、 手元で `cargo +nightly fuzz run parse -- -max_total_time=60` として実行し、 結果を上の「## Fuzzing の実行結果」に記録する。 CI のクラッシュ耐性は PBT が担う。 ターゲットは `backend/coffee_log_core/fuzz/` にあり、 `mise run lint` が fuzz クレートの型検査を行う。

## 解決方法

`coffee_log_core` に WebAuthn の登録とログインの検証を純粋な関数として実装し、CBOR と COSE のパーサ、base64url の復号、テスト、PBT、Fuzzing のターゲットを追加した。

- `backend/coffee_log_core/src/lib.rs` に `base64url`、 `cbor`、 `cose`、 `webauthn` のモジュールを追加した。 `backend/coffee_log_core/Cargo.toml` に `p256` (`default-features = false`、 `features = ["ecdsa"]`) と `sha2` を追加した。 `std` feature を外すのは、 `getrandom` を wasm32 の依存ツリーに入れないためである。
- `src/base64url.rs` は復号 (`decode`) と符号化 (`encode`、 0005 のチャレンジと登録用トークンの発行に使う) を持つ。 0003 のカーソルもこの実装を使う。 `src/cbor.rs` は map、 bytes、 text、 integer、 array だけを扱うデコーダ (`decode`、 `decode_prefix`、 `MAX_INPUT_LEN`、 `MAX_DEPTH`、 indefinite length と余分なバイトの拒否) を持つ。 `src/cose.rs` は EC2 (kty 2)、 alg -7、 crv P-256 (1)、 x と y が 32 バイトの鍵だけを受け付ける (`parse`、 `from_value`)。 `src/webauthn.rs` は `clientDataJSON` の type、 challenge、 origin、 `authenticatorData` の rpIdHash、 フラグ (UP、 UV、 登録時の AT)、 attestation の `fmt: none`、 CBOR と COSE の取り出し、 ES256 の署名検証、 署名カウンタ (`check_sign_count`、 `SignCounter::Skipped` / `Updated(u32)`、 `Error::SignCounterRegressed`) を持つ。 登録は `verify_registration`、 ログインは `verify_authentication` で、 時刻、 乱数、 D1 に依存しない。 challenge の照合 (`check_challenge`) は base64url の文字列で行い、 非正準の符号化も不一致として拒否する (W3C WebAuthn Level 3 の 7.1 と 7.2)。 ED フラグが立った authData の拡張は中身を扱わないが、 CBOR の map であることは確認する。
- `tests/test_webauthn.rs` は W3C WebAuthn Level 3 の 16 節の公開テストベクタ (出典をファイル冒頭に記載) を使い、 登録成功、 attestation none 以外の拒否、 ES256 以外の拒否、 origin と challenge の不一致 (登録とログインの両方)、 rpIdHash の不一致 (両方)、 非正準の challenge の拒否、 UP、 UV、 AT の欠落、 署名の不一致、 カウンタの後退、 ED フラグ付き拡張データの受理と map でない拡張の拒否を確認する。 `tests/test_base64url.rs`、 `tests/test_cbor.rs`、 `tests/test_cose.rs` は境界値とエラーパスを確認する (長さの上限の境界は、 上限ちょうどの入力の受理と 1 バイト超過の拒否を確認する)。
- `backend/pbt/tests/prop_base64url.rs`、 `prop_cbor.rs`、 `prop_cose.rs` は往復、 型ごとの不変条件、 区切りの省略の拒否、 長さの上限の拒否、 任意のバイト列で panic しないことを検証する (`support/mod.rs` のテスト用エンコーダを使うのは `prop_cbor.rs` と `prop_cose.rs`)。
- `backend/coffee_log_core/fuzz/` に `cargo fuzz` のターゲット `parse` を置いた。 `fuzz_targets/parse.rs` は base64url、 CBOR、 COSE と、 登録とログインの検証に任意のバイト列を与える。 `clientDataJSON` は JSON のバイト列を base64url で符号化した形で渡し、 challenge を入力の符号化と一致させ、 ログインの `authenticatorData` は RP ID のハッシュで始めるため、 検証は challenge の照合と rpIdHash の検査を越えてフラグ、 `authenticatorData`、 attestation object の解析に到達する。 `target/`、 `corpus/`、 `artifacts/`、 `coverage/` は `fuzz/.gitignore` で除外する。 `fuzz/Cargo.toml` に rpIdHash の組み立てのための `sha2` を追加した。 `mise.toml` の `lint` に fuzz クレートの `cargo check` を追加し、 ワークスペース外のターゲットの型検査を `mise run check` に含めた。

完了条件の検証:

- 検証の各経路を `tests/test_webauthn.rs` の 41 テストが確認した。 公開テストベクタの出典は同ファイル冒頭のコメントにある (W3C WebAuthn Level 3 16.2、 16.3、 16.4、 16.8)。
- CBOR と COSE の PBT が通った (prop_cbor 6、 prop_cose 8)。 往復は `every_generated_structure_round_trips` と `a_generated_es256_key_is_parsed`、 境界値は `nesting_is_bounded` と `a_coordinate_of_the_wrong_length_is_rejected`、 巨大入力の拒否は `an_input_over_the_limit_is_rejected` (CBOR と COSE の両方)、 クラッシュ耐性は `arbitrary_input_does_not_panic` である。
- ターゲットは `backend/coffee_log_core/fuzz/fuzz_targets/parse.rs` にあり、 手元で 60 秒実行してクラッシュが無い (1569601 runs、 `cov: 540`、 終了コード 0)。 実行結果は issue の「## Fuzzing の実行結果」に追記した。
- `cargo test -p coffee_log_core` の 83 テストと `cargo clippy --workspace --all-targets -- -D warnings` が通った。
- `mise run check` が通過した (本 issue の検証時点で 112 テスト。 0003 を含む統合後の作業ツリーでは 154 テスト)。 本 issue の追加は 91 テストである。

方針からの乖離 (方式は変えていない):

- `p256` を `default-features = false, features = ["ecdsa"]` で追加した。 方針は `p256` の ecdsa feature のみを求めており、 既定の `std` が `getrandom` を引き込んで wasm32 のビルドに追加設定を要するため無効化した。
- 方針が名指ししない補助を追加した。 `base64url` モジュール、 `cbor::decode_prefix`、 `cbor::MAX_INPUT_LEN` と `MAX_DEPTH`、 `cose::from_value`、 fuzz クレートの `sha2` と `cargo check`。 いずれも方式を変えない。
- 署名カウンタは `check_sign_count` が `Skipped` と `Updated(u32)` を返し、 後退は `Error::SignCounterRegressed` で表す形にした。 更新は方針どおり 0005 が行う。
- challenge の照合を base64url の文字列の一致で行うことを明示した (非正準の符号化を拒否する。 W3C WebAuthn Level 3 の 7.1 と 7.2 に合わせる)。
- ED フラグ付きの拡張は中身を扱わないが、 CBOR の map であることは確認する (不正な CBOR を無検査で受理しない)。
- Fuzzing のターゲットは、 検証の関数のうち署名の検証には到達しない (有効な署名が要るため)。 到達するのは challenge の照合、 rpIdHash とフラグの検査、 `authenticatorData` と attestation object の解析、 CBOR、 COSE、 base64url のデコーダである。
- 「## 未確定論点」の Fuzzing の扱いは、 2026-09-22 に所有者が「CI では実施せず、 ローカルで実行する」ことを確認した。 扱いは issue の「## 未確定論点の扱い」に記録した。
