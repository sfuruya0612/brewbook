# WebAuthn の検証コアと CBOR と COSE のパーサを実装する

Created: 2026-09-21
Model: deepseek-v4p1-flash
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
