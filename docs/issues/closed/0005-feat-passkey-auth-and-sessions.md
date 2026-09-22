# パスキーの登録とログインとセッションとパスキー管理を実装する

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-23
対応 ADR: ADR-0004 (docs/adr/0004-passkey-only-authentication.md)、ADR-0006 (docs/adr/0006-data-model-and-archive.md)
関連 PRD: FR-1 から FR-5、セキュリティ (チャレンジとセッションの規約)、制約と前提 (CDP の仮想認証器)
依存: 0001, 0002, 0003, 0004

## 背景

ADR-0004 は、認証をパスキー (WebAuthn) だけにし、パスワードとメールリンクを持たないと決めた。
利用者はセルフサインアップせず、管理者が発行した有効期限 24 時間の登録用トークン (0018 の管理者画面) でパスキーを登録する。
セッションは 32 バイトの乱数トークンをハッシュで保存し、HttpOnly、Secure、SameSite=Lax の Cookie で渡し、有効期限は 30 日とする。
チャレンジは 32 バイトの乱数で、有効期限 5 分、使用は 1 回とする。
PRD の FR-1 から FR-5 は、登録用トークンによる登録、ログイン、パスキーの追加と名前の変更と削除、ログアウト、利用者間の分離を要求する。
0004 で WebAuthn の検証コアができた。本 issue が API を提供する。

## 目的

登録用トークンでパスキーを登録してセッションを発行でき、パスキーだけでログインでき、利用者ごとに分離されたセッションで API を呼べるようにする。

## 設計判断

### API の一覧

| 操作 | 経路 | 認証 | 入力 |
| --- | --- | --- | --- |
| 登録のチャレンジ発行 | `POST /api/auth/register/begin` | 不要 | トークン |
| 登録の検証 | `POST /api/auth/register/complete` | 不要 | トークン、パスキーの名前、credential |
| ログインのチャレンジ発行 | `POST /api/auth/login/begin` | 不要 | 無し |
| ログインの検証 | `POST /api/auth/login/complete` | 不要 | credential |
| ログアウト | `POST /api/auth/logout` | 必要 | 無し |
| パスキーの一覧 | `GET /api/passkeys` | 必要 | 無し |
| パスキーの追加のチャレンジ発行 | `POST /api/passkeys/begin` | 必要 | 無し |
| パスキーの追加の検証 | `POST /api/passkeys/complete` | 必要 | パスキーの名前、credential |
| パスキーの名前の変更 | `PATCH /api/passkeys/<ID>` | 必要 | 名前 |
| パスキーの削除 | `DELETE /api/passkeys/<ID>` | 必要 | 無し |

- 認証が不要なのは上の 4 経路だけとし、それ以外の利用者向けの API はセッションを必須にする (PRD のセキュリティ)。
- セッションの解決は 1 つのミドルウェアに置く。
  Cookie を取得してハッシュを照合し、有効期限を確認して `user_id` を取り出す。無効なら 401 を返す。
  経路の台帳 (0001) の認証の要否に従って適用する。

### 登録用トークン

- トークンは 32 バイトの乱数を base64url で符号化した文字列とし、D1 には SHA-256 のハッシュだけを保存する。
- 有効期限は発行から 24 時間とし、使用日時を記録する。
- 存在しないトークンは 404、使用済みは 409、期限切れは 410 を返す。
- 登録が成功したら、トークンを使用済みにし、パスキーを利用者に紐づけ、セッションを発行する。
- 既存の利用者への再発行は 0018 が行い、この経路では利用者を増やさない。

### チャレンジ

- チャレンジは 32 バイトの乱数を base64url で符号化した文字列とし、`webauthn_challenges` に保存する。
  有効期限は 5 分で、使用は 1 回とする。
- 登録のチャレンジは `user_id` と種別 (登録) を持ち、ログインのチャレンジは `user_id` を NULL にして種別 (ログイン) を持つ (ADR-0006)。
  ログインのチャレンジ発行は利用者を特定しないため、同じ種別の行が複数できる。
  検証では `clientDataJSON` から取り出したチャレンジ値と一致する行を、チャレンジ値と種別で引く (ADR-0006 の「チャレンジ値で引く」)。
  登録の検証では、トークンの利用者と種別 (登録) とチャレンジ値の 3 つで引く。
- 使用したチャレンジの行は削除する。
  認証不要のログインのチャレンジ発行で行が増え続けないよう、チャレンジを発行するときに有効期限を過ぎた行を削除する (定期実行の仕組みは作らない。利用者が 20 人以下の規模で、発行のたびの削除で足りる)。
  検証時の状態の扱いを次に定める。
  - チャレンジ値に一致する行が無い場合は 409 を返す (使用済みの再利用か、存在しない値かを区別しない)。
  - 行はあるが有効期限を過ぎている場合は 410 を返し、その行を削除する。
  - 行が有効期限内なら署名を検証し、失敗は 400、成功は行を削除して続ける。
- 採らない案: `used_at` 列を足して再利用だけを 409 にする (ADR-0006 の列の改訂が必要になる)。

### WebAuthn の登録とログイン

- Relying Party ID と Origin は環境変数 (vars) で持ち、本番は `coffee-log.<アカウントのサブドメイン>.workers.dev` とその `https` の URL、ローカルは `localhost` と `http://localhost:8787` にする。
- 作成のオプションは、`attestation: "none"`、`authenticatorSelection: { residentKey: "preferred", userVerification: "required" }`、`pubKeyCredParams` は ES256 (alg -7) だけとする。
  ログインのオプションは `userVerification: "required"` とし、`allowCredentials` は空にする (利用者名を受け取らない。PRD の FR-2)。
- `user.id` には利用者の UUID の 16 バイトを入れ、`user.name` には利用者の UUID の文字列を入れる。
  `user.displayName` は設定しない。
  採らない案: `user.name` に表示名を使う (表示名は管理者が利用者を識別するためだけのもので、パスキーのメタデータにも出さない。FR-17)。
- 署名カウンタの判定は 0004 の純粋な関数を使い、拒否なら 409 を返してログインを拒否し、受理なら保存値を更新する。
- ログインに成功したら、そのパスキーの最終使用日時を更新する。
- パスキーの名前は前後の空白を除いて 1 文字以上 50 文字以下とし、範囲外は 400 を返す。
  最後の 1 つは削除できず 409、他の利用者のパスキーの変更と削除は 404 を返す。
- パスキーの一覧は、名前、登録日時、最終使用日時を返す。

### セッション

- セッションのトークンは 32 バイトの乱数を base64url で符号化した文字列とし、D1 には SHA-256 のハッシュだけを保存する。
- Cookie は `session` という名前で、`HttpOnly`、`Secure`、`SameSite=Lax`、`Path=/`、有効期限 30 日とする。
- ログアウトではセッションの行を物理削除し、Cookie を失効させる。
  アカウント削除 (0012) も同じく行を物理削除する。
- セッションの有効期限の照合は、ISO 8601 UTC の文字列の辞書順比較で行う (0003 の形式)。
- 採らない案: Bearer トークンを併用する (Web の同一オリジンでは Cookie で足りる。iOS は配布形態の決定後に扱う。ADR-0005)。

### テスト

- パスキーを伴う結合テストは、Chrome DevTools Protocol の仮想認証器で行う (PRD の制約と前提)。
  Chrome を CDP で起動するテストハーネスをネイティブの Rust テストとして書き、dev-dependency に `headless_chrome` を追加する (依存を追加する理由はここに記す)。
  `WebAuthn.enable`、`WebAuthn.addVirtualAuthenticator`、`Runtime.evaluate` のコマンドを送れることを実装の最初に確認する。
  送れない場合は別の CDP クライアントに替えるか、CDP のクライアントを自前で実装するかを選び、理由を issue 本文に追記する。
  テストは `wrangler dev` のオリジンのページで `navigator.credentials` を呼び、登録、ログイン、パスキーの追加、名前の変更、削除を一連で検証する。
- 時間に依存する検証 (チャレンジ、登録用トークン、セッションの有効期限) は、環境変数で秒数を短縮できるようにし、期限切れの経路をテストする。
- 経路の台帳 (0001) に本 issue の経路を追加し、認証の要否に応じた種別 (正常系、未認証 401、入力不正 400) のテストをスイートに追加する。

## 未確定論点

- ログインのチャレンジ発行が入力を持たないため、`allowCredentials` を空にして discoverable なパスキーだけを対象にする。
  ADR-0004 は `residentKey: preferred` とするため、discoverable でないパスキーを登録した利用者はログインできない。
  実装はこの前提で進め、discoverable でないパスキーが実際に現れた場合は、`residentKey` を `required` にするかログインに credential ID を渡すかを所有者に諮る。

## 完了条件

- FR-1 の受け入れ基準のうち API の分を満たす。
  存在しないトークンで 404、使用済みで 409、期限切れで 410 を返し、成功したらトークンを使用済みにしてセッションを発行し、名前が無いか範囲外なら 400 を返す (画面のウィジェットテストは 0013 が持つ)。
- FR-2 の受け入れ基準を満たす。
  チャレンジ発行の API が入力を持たず、検証の失敗で 400、期限切れで 410、再利用で 409 を返し、署名カウンタの検査と最終使用日時の更新を行う。
- FR-3 の受け入れ基準を満たす。
  追加、名前の変更、削除ができ、最後の 1 つの削除は 409、他の利用者のパスキーの変更と削除は 404 を返し、一覧が名前と登録日時と最終使用日時を返す。
- FR-4 の受け入れ基準を満たす。
  ログアウト後に同じセッションで認証が必要な API を呼ぶと 401 になり、セッションの行が削除されている。
- FR-5 の受け入れ基準のうちパスキーに関する分を満たす (他の利用者のパスキーの変更と削除が 404)。
- CDP の仮想認証器を使った結合テストで、登録、ログイン、パスキーの追加、名前の変更、削除、最後の 1 つの削除の 409、署名カウンタの後退の 409、ログアウト後の 401、使用済みチャレンジの 409、期限切れチャレンジの 410 が確認できる。
- セッションの有効期限を短縮した設定で、期限切れのセッションによる API 呼び出しが 401 になる。
- 期限切れのチャレンジの行が、次のチャレンジの発行で削除される。
- 認証が不要な 4 経路と認証が必要な経路のテストが、0001 の経路の台帳と照合して揃っている。
- `mise run check` が通過する。

## 関連

- 0003 が認証の 5 テーブルを作る。
- 0004 が WebAuthn の検証コアを作る。
- 0013 がログイン、登録、ログアウトの画面を作る。
- 0016 がパスキーの管理の画面を作る。
- 0018 が登録用トークンを発行する管理者画面を作る。
- 0017 が状態を変更する API の Origin の検証を追加し、既存の結合テストを更新する。

## CDP クライアントの確認

2026-09-23 に `headless_chrome` 1.0.22 で `WebAuthn.enable`、`WebAuthn.addVirtualAuthenticator`、`Runtime.evaluate` を送れることを実装の最初に確認した。
仮想認証器の ID を取得して `navigator.credentials.create()` が成功し、`getCredentials`、`removeCredential`、`addCredential` も動いた。
別の CDP クライアントへの差し替えは不要だった。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. `mise.toml` の `backend:test-integration` に `--test wrangler_auth_api --test wrangler_passkey_flow` を追加した。新しい結合テストを `mise run check` の対象にするためである。
2. テスト専用ページ `/__test_page` を `coffee_log/src/test_page.rs` に追加した。CDP のテストは `wrangler dev` のオリジンのページで `navigator.credentials` を呼ぶ必要がある。既存の `d1_check` と同じく台帳に載せず、`TEST_PAGE` の var の値が `true` のときだけ応答する (var が無ければ 404 になることを `wrangler_test_page_is_disabled_without_its_var` が確認する)。
3. テストページの `creationOptions` は `user.displayName` に `user.name` と同じ UUID の文字列を入れる。`PublicKeyCredentialUserEntity` の `displayName` は必須メンバーだが、サーバーのオプションには含めない (ADR-0004)。クライアント側 (画面を持つ 0013 とテストページ) が必須メンバーを埋める。管理者が使う表示名は含めない (FR-17)。
4. 登録用トークンの有効期限を短縮する var は作らなかった。トークンの発行は 0018 が行い、本 issue の経路は発行済みの行を検証するだけである。期限切れの経路 (410) は、テストが期限切れの行を D1 に入れて検証する。チャレンジとセッションは `CHALLENGE_TTL_SECONDS` と `SESSION_TTL_SECONDS` の var で短縮する。
5. 結合テストの下ごしらえは `wrangler d1 execute` で利用者、登録用トークン、セッション、パスキー、チャレンジを直接 D1 に入れる (0018 の管理者画面が無いため)。ハーネスの `DevServer::start_with` は vars をポートから組み立て、下ごしらえを `wrangler dev` の起動前に実行する (実行中の D1 への同時の書き込みを避ける)。
6. `respond.rs` を追加してエラー応答と Cookie 付き応答の組み立てを共通化し、`lib.rs` にあった同等の実装を置き換えた。新しい経路と共有するためである (応答の形式は変えていない)。
7. 仮想認証器のクレデンシャル ID は CDP が標準 base64 (パディングあり)、API が base64url (パディングなし) で返すため、署名カウンタの入れ替えの照合で正規化する。
8. Chrome は sandbox なし、`--disable-dev-shm-usage` 付き、idle の停止を 600 秒にして起動する (CI のコンテナで落ちないため)。
9. 登録の検証では、トークンの使用済みの印を付けた後に、パスキーとセッションの行を入れる。両者を 1 つの batch にまとめると「未使用のときだけ印を付ける」排他 (変更行数による判定) ができなくなるため、印は別の文にした。印を付けた後に保存が失敗すると、トークンは使用済みのままで登録をやり直せない (0018 の再発行で回復する)。利用者の操作と D1 の一時障害が重なったときだけ発生する。
10. 本番の `RP_ID` と `ORIGIN` の vars の設定は、0017 のデプロイ設定に委ねる (値はアカウントの `workers.dev` のホスト名で決まるため、この時点では確定できない)。vars が無いときはローカルの既定値 (`localhost` と `http://localhost:8787`) を使う。0017 が vars を設定しないままデプロイすると、WebAuthn の検証がオリジンの不一致で失敗する。
11. チャレンジの使い切り (`consume_challenge`) とパスキーの削除 (`DELETE_PASSKEY_IF_NOT_LAST`) は、削除の変更行数が 1 のときだけ続けることで「1 回だけ成立する」ことを保証する。署名カウンタの更新 (`UPDATE_SIGN_COUNT_IF_GREATER`) は、今回の値が保存値より大きいときだけ書くことで、保存値が後退しないことを保証する (2 本の同時のログインが両方成立することはある)。この排他の分岐 (削除や更新が 0 行になる経路) は、2 本のリクエストが検証の途中で重なったときだけ通るため、逐次のテストでは到達しない。排他はコードの読み取りで担保し、同時に送ったときの結果はテストで固定していない。
12. 登録の検証 (`register/complete`) とパスキーの追加 (`passkeys/complete`) の署名検証失敗の 400 は、有効な attestation を伴わない入力を組み立てる必要があるため、テストしていない (ログインの検証失敗の 400 は `wrangler_auth_login_complete_failed_verification_400` が確認する)。

## 解決方法

`coffee_log` に認証の 10 経路を追加し、`coffee_log_core` に認証の共通部品とテストを追加した。

- `backend/coffee_log_core/src/auth.rs` を新設した。秘密値 (32 バイトの乱数) の base64url 符号化 (`encode_secret`)、SHA-256 のハッシュ (`hash_secret`)、有効期限の計算と辞書順の照合 (`expiry_from`、`is_expired`)、パスキーの名前の検証 (`validate_passkey_name`)、セッションの Cookie の組み立てと取り出し (`session_cookie`、`expired_session_cookie`、`session_token`)、種別と有効期限の定数を持つ。`backend/coffee_log_core/src/lib.rs` に `pub mod auth;` を追加して公開した。
- `backend/coffee_log_core/src/webauthn.rs` に `client_data_challenge` を追加した。署名の検証の前に `clientDataJSON` からチャレンジ値を読み、対象のチャレンジ行を引けるようにするためである。
- `backend/coffee_log_core/src/ids.rs` に `uuid_bytes` を追加した。`user.id` に入れる UUID の 16 バイトを取り出すためである (ADR-0004)。
- `backend/coffee_log_core/src/routes.rs` の `ROUTES` に認証の 10 経路を追加した。認証が不要なのは登録とログインの begin と complete の 4 経路である。
- `backend/coffee_log/src/auth/` を新設した。`mod.rs` (vars の読取 `Config`、チャレンジの発行 `issue_challenge`、照合 `find_challenge`、使い切り `consume_challenge`、D1 の文の組み立てと一括実行、応答の組み立て、パスキーの応答の形)、`session.rs` (Cookie の解決 `resolve`、セッションの発行 `prepare`、ログアウト `logout`)、`register.rs` (登録の begin と complete)、`login.rs` (ログインの begin と complete)、`passkeys.rs` (一覧、追加の begin と complete、名前の変更、削除) を持つ。
- 同時のリクエストの扱いは、削除または更新の変更行数で判定する形にした。チャレンジは `consume_challenge` で 1 行だけ削除できたときだけ続け、パスキーの削除は `DELETE_PASSKEY_IF_NOT_LAST` (利用者に他のパスキーが残っているときだけ削除する 1 つの文) で行い、署名カウンタの更新は `UPDATE_SIGN_COUNT_IF_GREATER` (保存値より大きいときだけ更新する) で行う。削除で 0 行になった場合は、行の有無を引き直して 404 (同時に消えた) と 409 (最後の 1 つ) を区別する。
- `backend/coffee_log/src/lib.rs` は、台帳の経路名で 1 つのハンドラに振り分け、認証が必要な経路だけ `auth::session::resolve` を通し、無効なら 401 を返す形にした。`respond.rs` を新設してエラー応答と Cookie 付き応答を共通化し、`test_page.rs` にテスト専用のページ (`TEST_PAGE` の var の値が `true` のときだけ応答) を追加した。
- `backend/coffee_log/Cargo.toml` に dev-dependency `headless_chrome` (1.0.22) を追加した。結合テストの CDP クライアントに使う (依存を追加する理由は issue の「設計判断 / テスト」にある)。`backend/Cargo.lock` が追随した。
- `backend/coffee_log/src/random.rs` に `bytes_32` を追加した。チャレンジ、登録用トークン、セッションのトークンの 32 バイトの乱数を作る (乱数を取れるのは wasm の Worker の実行時だけで、ネイティブターゲットではエラーを返す)。
- `backend/coffee_log/tests/wrangler_auth_api.rs` (35 テスト) と `wrangler_passkey_flow.rs` (7 テスト) を追加した。`support/cdp.rs` は `headless_chrome` で仮想認証器を操作し、`support/seed.rs` は利用者と登録用トークンなどを投入し、`support/http.rs` はリクエストの補助を持つ。`support/mod.rs` の `SUITE` に 10 経路の種別を追加し、`DevServer::start_with` (vars をポートから組み立て、下ごしらえを先に実行) と、テストファイルごとに 1 つのサーバーを共有する `shared_server` を追加した。
- `backend/coffee_log_core/tests/test_auth.rs` (14 テスト) と `backend/pbt/tests/prop_auth.rs` (5 テスト) を追加し、`test_ids.rs` と `pbt/tests/prop_ids.rs` に `uuid_bytes` の検査を追加した。
- `mise.toml` の `backend:test-integration` に新しい 2 つの結合テストを追加した。

完了条件の検証:

- FR-1 の API の分: `wrangler_auth_register_begin_missing_token_404`、`wrangler_auth_register_begin_used_token_409`、`wrangler_auth_register_begin_expired_token_410`、`wrangler_auth_register_begin_invalid_input_400`、`wrangler_auth_register_complete_invalid_input_400`、`wrangler_auth_register_complete_missing_token_404`、`wrangler_auth_register_complete_used_token_409`、`wrangler_auth_register_complete_expired_token_410`、`wrangler_auth_register_complete_challenge_of_another_user_409`、`wrangler_auth_register_complete_ok_and_logout_401` が確認した。
- FR-2: `wrangler_auth_login_begin_ok` (本体なしで 200、`allowCredentials` は空)、`wrangler_auth_login_complete_invalid_input_400`、`wrangler_auth_login_complete_failed_verification_400`、`wrangler_auth_login_complete_unknown_challenge_409`、`wrangler_auth_login_complete_challenge_bound_to_another_user_or_kind_409`、`wrangler_auth_login_complete_reused_challenge_409_and_bad_signature_400`、`wrangler_auth_login_complete_expired_challenge_410_and_deleted_on_issue`、`wrangler_auth_login_complete_regressed_sign_count_409` (100 に上げた後の受理で、アサーションの署名カウンタが保存値になることも確認する)、`wrangler_auth_login_complete_ok_and_last_used_at_is_updated` が確認した。
- FR-3: `wrangler_passkeys_list_ok`、`wrangler_passkeys_complete_ok_and_the_passkey_is_managed` (追加と変更と削除と最後の 1 つの 409)、`wrangler_passkeys_rename_ok`、`wrangler_passkeys_delete_last_conflict_409` が確認した。
- FR-4: `wrangler_auth_logout_ok` (返る `Set-Cookie` が `session=;` と `Max-Age=0` を持つ)、`wrangler_auth_logout_deletes_the_session_row_401` (`sessions` の件数 0) が確認した。
- FR-5 のパスキーの分: `wrangler_passkeys_rename_other_user_404`、`wrangler_passkeys_delete_other_user_404` が確認した (対象が変わっていないことも確認する)。チャレンジの利用者と種別の束縛は、`wrangler_passkeys_complete_challenge_of_another_user_409`、`wrangler_auth_register_complete_challenge_of_another_user_409`、`wrangler_auth_login_complete_challenge_bound_to_another_user_or_kind_409` が確認した。
- CDP の仮想認証器: `wrangler_passkey_flow.rs` の 7 テストが、登録、ログイン、追加、名前の変更、削除、最後の 1 つの 409、署名カウンタの後退の 409、ログアウト後の 401、使用済みチャレンジの 409、期限切れチャレンジの 410 を確認した。
- セッションの期限切れ: `wrangler_auth_api.rs` の `wrangler_auth_session_expired_401` がサーバーの期限照合の根拠である (期限切れのセッションの行を下ごしらえで入れ、Cookie を付けて 401 になることを確認する)。`wrangler_passkey_flow.rs` の同名テスト (`SESSION_TTL_SECONDS=5` のサーバーで登録して待ち、401 を確認する) が短縮した var の一連の動作の根拠である。この 2 つでは、`SESSION_TTL_SECONDS` が DB の `expires_at` と Cookie の `Max-Age` の両方に効くことを区別できない (ブラウザが `Max-Age` の切れた Cookie を送らない場合も 401 になる)。
- 期限切れのチャレンジの削除: `wrangler_auth_login_complete_expired_challenge_410_and_deleted_on_issue` が、期限切れの 410 と、そのチャレンジ値の行が 0 件になることを確認し、次の発行のたびの削除で古いチャレンジ値が 409 になることも確認する。
- 台帳とスイートの照合: `the_ledger_and_the_suite_match` (api_suite) が 10 経路の種別を照合した。
- `mise run check`: exit 0 (ベースラインからの新たな失敗は無し)。

方針からの乖離 (方式は変えていない):

1. `mise.toml` の `backend:test-integration` に新しい 2 つの結合テストを追加した (テストコマンドの定義の更新)。
2. テスト専用ページ `/__test_page` を追加した。CDP は `wrangler dev` のオリジンのページを要するためである (`TEST_PAGE` の var の値が `true` のときだけ応答する)。
3. テストページの `creationOptions` は `user.displayName` をクライアント側で補う。サーバーは返さず (ADR-0004)、管理者が使う表示名も含めない (FR-17)。
4. 登録用トークンの有効期限を短縮する var は作らない。発行は 0018 が行い、この経路は発行済みの行を検証するだけである。期限切れの 410 は、期限切れの行を下ごしらえで入れて検証する。チャレンジとセッションは `CHALLENGE_TTL_SECONDS` と `SESSION_TTL_SECONDS` で短縮する。
5. 結合テストの下ごしらえは `wrangler d1 execute` で直接 D1 に入れる (0018 の管理者画面が無いため)。`DevServer::start_with` は下ごしらえを起動前に実行する。
6. `respond.rs` を追加して応答の組み立てを共通化し、`lib.rs` の同等の実装を置き換えた (応答の形式は変えていない)。
7. CDP が返す標準 base64 のクレデンシャル ID と、API の base64url の相違をテストで正規化する。
8. Chrome は sandbox なし、`--disable-dev-shm-usage` 付き、idle の停止を 600 秒にして起動する。
9. トークンの使用済みの印は、パスキーとセッションの保存と別の文にする。1 つの batch にまとめると「未使用のときだけ印を付ける」排他ができなくなるためである。印の後に保存が失敗すると、トークンは使用済みのまま残る (0018 の再発行で回復する)。
10. 本番の `RP_ID` と `ORIGIN` の vars の設定は 0017 のデプロイ設定に委ねる。値がアカウントの `workers.dev` のホスト名で決まるためである。vars が無いときはローカルの既定値を使う。0017 が vars を設定しないままデプロイすると、WebAuthn の検証がオリジンの不一致で失敗する。
11. チャレンジの使い切りとパスキーの削除は、削除の変更行数が 1 のときだけ続けることで「1 回だけ成立する」ことを保証し、署名カウンタの更新は、今回の値が保存値より大きいときだけ書くことで保存値が後退しないことを保証する (2 本の同時のログインが両方成立することはある)。この排他の 0 行の分岐は、2 本のリクエストが検証の途中で重なったときだけ通るため、逐次のテストでは到達しない。排他はコードの読み取りで担保し、同時に送ったときの結果はテストで固定していない。
12. 登録の検証とパスキーの追加の署名検証失敗の 400 は、有効な attestation を伴わない入力を組み立てる必要があるため、テストしていない (ログインの検証失敗の 400 は `wrangler_auth_login_complete_failed_verification_400` が確認する)。
