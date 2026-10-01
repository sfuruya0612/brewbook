# 認証の画面を Dioxus で実装する

Created: 2026-10-02
Model: deepseek-v4p1-flash
対応 ADR: ADR-0017、ADR-0004 (docs/adr/0004-passkey-only-authentication.md)、ADR-0005
関連 PRD: FR-1、FR-2、FR-4、FR-16
依存: 0038, 0039

## 背景

本 issue は、2026-10-01 の所有者の決定 (ADR-0017) から起票した。

Flutter の認証は `frontend/lib/auth/passkey_client_web.dart` が `dart:js_interop` と `package:web` で `navigator.credentials` を呼び、`frontend/lib/screens/login_screen.dart`、`register_screen.dart`、`home_screen.dart` (ログアウト) が画面である。
サーバーのオプションは JSON で受け渡し、`frontend/lib/auth/passkey_name.dart` が名前の長さ (1 文字以上 50 文字以下) を Unicode のスカラー値で数える。
セッションは HttpOnly の Cookie でブラウザが管理する (ADR-0005)。
パスキーを伴う検証は Chrome DevTools Protocol の仮想認証器で行う (ADR-0013)。

## 目的

`/login` と `/register?token=<トークン>` の画面でパスキーの登録とログインができ、ログアウトでき、未認証のときにログイン画面へ遷移する。

## 設計判断

- パスキーは `web-sys` の `PublicKeyCredential` と `CredentialsContainer` を使う。
  `web-sys` の型で足りない options の辞書は `js-sys` の `Reflect` で組み立て、`serde-wasm-bindgen` は使わない (依存を増やさない。ADR-0001 の規約)。
- サーバーのオプションの JSON の base64url の変換は純粋な関数として切り出し、native の単体テストと PBT で守る (Flutter の `test/web/passkey_interop_test.dart` の検査を写す)。
- 名前の長さは Rust の `char` の数で数える (Flutter の `String.runes` と同じ数え方。Backend の `validate_passkey_name` に合わせる)。
- 登録の画面は、トークンが無いときと、API が 404 と 409 と 410 を返したときの表示を分ける (Flutter と同じ)。
- 画面の部品は 0039 のものを使い、文言は 0038 の翻訳から取る。
- 採らない案: `wasm-webauthn` クレートを使う (2026-10-02 時点の crates.io では 2023 年から更新が無く、`web-sys` で足りる)、`webauthn-rs` のクライアント側の型を使う (サーバー側のクレートで、wasm の対応が未確認)。

## 完了条件

- `/register?token=<トークン>` を開くと登録の画面が表示され、パスキーを登録するとセッションが発行されてホームへ遷移する (0044 の E2E で確認する)。
- `/login` でパスキーを選ぶとログインでき、ホームへ遷移する (0044 の E2E で確認する)。
- ログアウトするとログイン画面へ遷移し、以後の API 呼び出しが 401 になる (0044 の E2E で確認する)。
- 登録するパスキーの名前が 1 文字以上 50 文字以下で、範囲外は画面で拒否する (native のテストで確認する)。
- base64url の変換の単体テストと PBT がある。
- 未認証で保護された画面を開くとログイン画面へ遷移する (native のテストで確認する)。
- ADR-0004 の iOS の記述 (パスキーのネイティブ連携と選択肢の表) を、Web だけを対象とする形に改訂する。
- `mise run dioxus:lint` と `mise run dioxus:test` が成功する。
- `mise run check` が通過する。

## 関連

- 0038 が基盤を、0039 が部品を作る。
- 0044 が E2E のハーネスを用意する (この issue の E2E の完了条件は 0044 の後で確認する)。

---
