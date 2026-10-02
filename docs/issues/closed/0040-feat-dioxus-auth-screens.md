# 認証の画面を Dioxus で実装する

Created: 2026-10-02
Model: deepseek-v4p1-flash
Completed: 2026-10-02
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

## 解決方法

認証の画面 (ログイン、登録) とパスキーの呼び出しを実装した (ADR-0017、ADR-0004)。

- `frontend/src/auth/base64url.rs` に base64url の符号化と復号 (パディング無し) を追加した。標準の `=` と空白は拒否し、URL 安全な文字だけを扱う。
- `frontend/src/auth/passkey.rs` にサーバーのオプションの JSON の型 (`CreationOptions`、`RequestOptions`)、失敗の型 (`PasskeyError`、`PasskeyErrorKind`) と失敗の種類の判定 (`PasskeyError::kind_of_error_name`。`NotAllowedError` と `AbortError` は取り消し、`NotSupportedError` は非対応、ほかはその他。Flutter の `kindOf` と同じ) を追加した。base64url の復号はここで行い、`PasskeyClient` は型付きのオプションを受け取る。
- `frontend/src/auth/passkey_web.rs` に `navigator.credentials` の呼び出しを追加した (`web-sys` の `CredentialsContainer`、`PublicKeyCredential` とオプションの辞書の型。`js-sys` の `Reflect` は JS の例外の `name` を読むためだけに使い、`serde-wasm-bindgen` は使わない)。オプションのバイト列 (チャレンジ、利用者の ID、`allowCredentials` の ID) は `Uint8Array` にコピーして渡す。
- `frontend/src/auth.rs` に `AuthServices` と `login`、`register`、`logout`、`passkey_name_for_request` (名前を前後の空白を除いて 1 文字以上 50 文字以下に制限する。Rust の `char` の数で数える) と、文言のキー (`message_key`、`register_error_key`、`login_error_key`、`is_invalid_registration_token`) を追加した。
- `frontend/src/screens/login.rs` と `frontend/src/screens/register.rs` にログインと登録の画面を追加した。登録の画面はトークンが無いときと、API が 404 と 409 と 410 を返したときの表示を分ける。送信の前の判定 (名前の検証とトークンの有無) は `register_submit` として切り出した。
- `frontend/src/router.rs` の `/login` と `/register?:token` を新しい画面に配線した。`frontend/src/app.rs` に `guard_destination` を追加し、`AppShell` の effect が経路とセッションの変化のたびに評価して `navigator.replace` する (未認証ではログインと登録以外をログイン画面へ、ログイン済みでログインと登録を開いたときはホームへ、確認中と確認できなかったときは遷移しない)。401 の callback はセッションが失われたことを記録し、遷移はこの判定が行う。
- `frontend/src/screens/mod.rs` に `login` と `register` のモジュールの配線と再公開を、`frontend/src/lib.rs` にモジュールの説明の更新を追加した。
- `frontend/tests/support/mod.rs` に偽のパスキーの実装 (`FakePasskeyClient`) を追加した (認証のテストの前提)。
- `frontend/Cargo.toml` の web-sys に WebAuthn の型の feature を追加した (新しいクレートは増やしていない)。
- `frontend/tests/` に `test_auth.rs` (18 件)、`test_base64url.rs` (6 件)、`test_passkey.rs` (8 件)、`test_passkey_web.rs` (wasm 6 件)、`test_register.rs` (3 件) を、`frontend/pbt/tests/prop_base64url.rs` に base64url の PBT (4 性質) を追加した。
- `docs/adr/0004-passkey-only-authentication.md` の iOS の記述 (Cloudflare Access の行の iOS の理由と、結果の Flutter Web と iOS ネイティブの記述) を、Web だけを対象とする形に改訂した。

完了条件の検証:

- `/register?token=<トークン>` の登録とホームへの遷移、`/login` のログインとホームへの遷移、ログアウトと以後の 401: 0044 の E2E で確認する (issue の記載どおり。この issue では実装と native のテストまで)。
- 名前の制限: `tests/test_auth.rs` の `a_passkey_name_is_trimmed_and_limited_to_fifty_characters` (50 文字ちょうどは可、51 文字は不可、サロゲートペアも 1 文字と数える) と `an_empty_passkey_name_is_rejected`、画面の送信の前の判定は `tests/test_register.rs` の `an_out_of_range_name_is_rejected_before_the_api` と `a_missing_token_is_not_sent` で確認した (範囲外とトークン無しでは API を呼ばない)。
- base64url の単体テストと PBT: `tests/test_base64url.rs` (6 件。パディング無しの復号、パディング無しの符号化、URL 安全な文字、`=` と空白の拒否、長さの拒否) と `pbt/tests/prop_base64url.rs` (4 性質。往復、アルファベット、復号の成否、正規化の一貫性) で確認した。
- 未認証で保護された画面を開くとログイン画面へ遷移: `tests/test_auth.rs` の `a_signed_out_session_is_sent_to_the_login` (ホーム、設定、統計 → ログイン)、`the_login_and_the_register_stay_open_while_signed_out`、`a_signed_in_session_returns_to_the_home_from_the_login_and_the_register`、`the_check_and_the_unknown_states_do_not_navigate` で確認した。
- パスキーの呼び出し: `tests/test_passkey.rs` (8 件。オプションの JSON の読みと失敗の種類の判定) と `tests/test_passkey_web.rs` (wasm 6 件。オプションがブラウザの辞書になること、JS の例外の名前から失敗の種類を決めること、空の `rpId` を設定しないこと、`attestation` と `userVerification` の列挙の写像) で確認した。文言の対応は `tests/test_auth.rs` の `the_error_messages_follow_the_flutter_mapping`、`the_registration_errors_use_the_token_messages`、`the_login_errors_use_the_retry_message` で確認した。
- ADR-0004 の改訂: 上記のとおり。
- `mise run dioxus:lint` と `mise run dioxus:test`: 作業ツリーで成功した (lint は clippy と fmt とも警告なし、native は 127 件)。`mise run dioxus:test-web` も成功した (26 件)。`mise run dioxus:build` も成功した。
- `mise run check` が通過する: 所有者の指示により、移行の全 issue の完了後に 1 回だけ実行するため、この issue では未実行 (最終確認に委ねる)。

方針を保った実装詳細の乖離:

- 方針は「web-sys の型で足りない options の辞書は `js-sys` の `Reflect` で組み立てる」とするが、web-sys 0.3.106 が WebAuthn の辞書の型を持っていたため、辞書は web-sys の型で組み立てた (`Reflect` は例外名の読み取りだけ)。新しいクレートを増やさない点と `serde-wasm-bindgen` を使わない点は方針どおり。
- `PasskeyClient` の引数は生の JSON ではなく、パース済みの `CreationOptions` と `RequestOptions` にした (base64url の変換を純粋な関数として切り出して native のテストと PBT で守る方針はそのまま)。
- ログアウトの操作の UI はホーム (0041) と設定 (0043) が置く。この issue では `logout` と、`SignedOut` になったときの `/login` への遷移を実装した。

レビューの指摘を受けて変えたもの (方式は変えていない):

- テストの入力のサロゲートペアの文字をエスケープ表記 (`\u{1F44D}`) にした (リポジトリの絵文字禁止の lint に一致しないようにするため。値は同じ)。
- 使われなくなった `unauthorized_route` とそのテストを削除し、401 の callback の説明を実装 (callback はセッションが失われたことを記録し、遷移は `guard_destination` が行う) に合わせた。
- 登録の画面の送信の前の判定を `register_submit` として切り出し、`tests/test_register.rs` で守るようにした (完了条件「範囲外は画面で拒否する」を画面の経路でも確認するため)。
- `tests/test_passkey_web.rs` に、JS の例外の名前から失敗の種類を決めること、空の `rpId` を設定しないこと、`attestation` と `userVerification` の列挙の写像の検査を足した。
- オプションのバイト列を `Uint8Array` にコピーして渡すように直した。web-sys の `*_with_u8_slice` は wasm のメモリへのビューを JS の辞書に保持させるため、その後の確保でチャレンジの中身が変わる (追加したテストが実際に検出した。パスキーの検証が失敗し得る不具合だった)。
