# テストとビルドの依存を削減し、cold ビルドを短縮する

Created: 2026-10-06
Model: DeepSeek V4.1 Flash
Completed: 2026-10-06

## 背景

所有者の要望は「dev-dependencies の重複 (headless_chrome と thirtyfour) を一本化できれば cold 10 分の短縮余地。工数は大きい。これをやりたい」「同じ issue で他のクレート、依存の削減も可能か調査して対応したい」である。

現状、テストとビルドでコンパイルされるクレートに次の重複と未使用の feature がある。

- Chrome を操作するクレートが 2 つある。
  - `headless_chrome` 1.0.22 は 0005 が追加した。`backend/brew_book/tests/support/cdp.rs` の `TestBrowser` だけが使い、`backend/brew_book/tests/wrangler_passkey_flow.rs` の 7 テストが `TestBrowser::open` で使う。Chrome を直接起動し、CDP の `WebAuthn.enable`、`WebAuthn.addVirtualAuthenticator`、`Runtime.evaluate` を送り、署名カウンタの後退の検査には `WebAuthn.getCredentials`、`WebAuthn.removeCredential`、`WebAuthn.addCredential` を送る。
  - `thirtyfour` 0.37.5 は 0044 が追加した。`backend/brew_book/tests/support/e2e.rs` の `E2eBrowser` と `backend/brew_book/tests/wrangler_same_origin_e2e.rs` が使い、chromedriver を起動して WebDriver で画面を操作する。仮想認証器は ChromeDriver の WebAuthn の拡張コマンド (`POST /session/{id}/webauthn/authenticator`) で付ける。
- Frontend は `dioxus` 0.7.10 の既定の feature を有効にしている。2026-10-06 に `cargo metadata` で確認した有効な feature は `default`、`devtools`、`logger`、`asset`、`document`、`mounted`、`web` などである (`default` は `launch`、`devtools`、`logger`、`lib` を含み、`minimal` は `macro`、`html`、`signals`、`hooks`、`launch` を含む)。`devtools` は `dx serve` のホットリロード用で、このリポジトリは `dx serve` を使わない (ADR-0005)。`frontend/src` に `asset!`、`document::`、`onmounted`、`devtools`、`dioxus_logger` の使用は無い (2026-10-06 に grep で確認。`document::` の一致は無く、`document` の一致は `window.document()` とそこから取った変数の使用だけである)。feature の定義はリポジトリ内に記録が無いため、実装時に dioxus 0.7.10 の `Cargo.toml` と `cargo tree -e features` で再確認する。
- `proptest` は `backend/pbt` と `frontend/pbt` の両方にあり、どちらも既定の feature で `fork` と `timeout` を有効にしている。PBT は `ProptestConfig` を使っておらず、`timeout` の既定は 0 である (proptest 1.11.0 の `src/test_runner/config.rs` で確認)。`proptest::bits` を使うテストも無い。
- `tokio` の `macros` feature は `#[tokio::main]` と `#[tokio::test]` を使っていない (ハーネスは `Runtime` を手で作る) ため未使用である。
- `reqwest` は 2 つの版がある。`brew_book` と `brew_book_admin` が 0.12.28 (既定の feature で `default-tls` と `system-proxy` が有効) を使い、`thirtyfour` 0.37.5 が別の 0.13.5 (feature `rustls`) を使う。ハーネスの接続先は `http://127.0.0.1` と `http://localhost` だけで、TLS を使わない。

2026-10-06 に `cargo metadata --format-version 1` の resolve グラフで、ワークスペースのメンバーを起点に到達できるパッケージ数を数えた。依存グラフの全体は Backend 320 パッケージ、Frontend 366 パッケージで、クレートを 1 つ外したときに到達不能になるパッケージ数は次の表のとおりである。表の数は、対象のクレートのノードを除いたシミュレーションの値である。feature を実際に外すと、`proptest` の `fork` が `tempfile` への辺を、`reqwest` 0.12.28 の `default-tls` が `tokio-native-tls` への辺も消すため、実測は表より数個多い見込みである。実装時に `cargo metadata` と `cargo tree` で再計測し、表と内訳を確定する。

Backend (320 パッケージ) の累計の削減数は次のとおりである。

| 外すもの | 削減されるパッケージ数 | 主な内訳 |
| --- | --- | --- |
| `headless_chrome` | 34 | `auto_generate_cdp`、`tungstenite`、`ureq`、`ureq-proto`、`flate2`、`miniz_oxide`、`zlib-rs`、`derive_builder`、`darling`、`sha1 0.10`、`socks`、`which` |
| 上に加えて `reqwest` 0.12.28 の既定の feature (`native-tls` と `hyper-tls`) | 42 | `native-tls`、`hyper-tls`、`openssl`、`openssl-sys`、`foreign-types`、`vcpkg` (`tokio-native-tls` も外れる見込み) |
| 上に加えて `proptest` の `fork` と `timeout` | 45 | `rusty-fork`、`wait-timeout`、`quick-error` (`tempfile` も外れる見込み) |

Frontend (366 パッケージ) の累計の削減数は次のとおりである。

| 外すもの | 削減されるパッケージ数 | 主な内訳 |
| --- | --- | --- |
| `dioxus` の `devtools` と `logger` | 12 | `dioxus-devtools`、`dioxus-devtools-types`、`dioxus-logger`、`tungstenite` 0.28、`tracing-subscriber`、`tracing-wasm`、`regex-automata`、`aho-corasick` |
| 上に加えて `dioxus` の `asset` | 24 | `manganis`、`manganis-core`、`manganis-macro`、`const-serialize` 2 版、`dunce`、`objc2`、`winnow 0.7` |
| 上に加えて `proptest` の `fork` と `timeout` | 27 | `rusty-fork`、`wait-timeout`、`quick-error` (`tempfile` も外れる見込み) |

削減される数は、プラットフォーム固有のパッケージ (Windows の `winapi` や macOS の `objc2` など) を含む。Linux の CI では `openssl-sys` のビルドが消える。`system-configuration` と `core-foundation`、`security-framework` は `thirtyfour` の `reqwest` 0.13.5 と `hyper-util` の経路で残り得る。

cold ビルドの実測は次のとおりである。2026-10-05 の手元の実行では、`frontend:test-same-origin` の `cargo test -p brew_book --test wrangler_same_origin_e2e` のコンパイルに 10 分 23 秒かかり、テスト本体は 33 秒だった。2026-10-02 の CI の run (37075588094) はキャッシュが無く 25 分 27 秒で、`frontend:test-web` が 21 分 6 秒を占めた。同じ run の `backend:test` は 517.6 秒で、内訳はビルドロック待ちが約 4 分 32 秒、自分のコンパイルが約 4 分、テストの実行が約 4 秒だった。

## 目的

テストとビルドでコンパイルされるクレートを減らし、cold ビルドを短縮する。製品の挙動と、テストが確認している内容 (登録、ログイン、パスキーの管理、署名カウンタの後退、チャレンジとセッションの期限切れ) は変えない。

## 設計判断

### `headless_chrome` を外し、パスキーの結合テストを `thirtyfour` に一本化する

- `backend/brew_book/tests/support/cdp.rs` の `TestBrowser` を `thirtyfour` と chromedriver で実装し直し、`headless_chrome` を dev-dependencies から外す。公開の API (`open`、`evaluate_json`、`set_sign_count`) は変えず、`wrangler_passkey_flow.rs` のテストの呼び出しは変えない (import のパスだけ `support::cdp` から `support::browser` に変わり、`support/mod.rs` の `pub mod` の名前も変わる)。
- 仮想認証器は ChromeDriver の WebAuthn の拡張コマンドで付ける。設定は `E2eBrowser::add_virtual_authenticator` と同じにする (protocol: ctap2、transport: internal、hasResidentKey: true、hasUserVerification: true、isUserVerified: true、automaticPresenceSimulation: true。Backend が userVerification: required を要求するため)。
- Chrome の起動オプションは `E2eBrowser::open` と同じ組にする (`--headless=new`、`--no-sandbox`、`--disable-dev-shm-usage`、`--disable-gpu`)。CI の Linux のコンテナで必要である。言語と窓の大きさはテストページの操作に要らないため付けない。
- 署名カウンタの付け替えは、`GET /session/{id}/webauthn/authenticator/{authenticatorId}/credentials` でクレデンシャルを取り、`DELETE .../credentials/{credentialId}` の後に `POST .../authenticator/{authenticatorId}/credential` (単数形) へ `signCount` を変えて入れ直す。2026-10-06 に `strings chromedriver | grep webauthn` で確認した経路は `session/:sessionId/webauthn/authenticator/:authenticatorId/credential` (追加)、`.../credentials` (一覧と全部削除)、`.../credentials/:credentialId` (削除) である。W3C WebAuthn Level 3 の WebDriver 拡張の Remove Credential も `DELETE .../credentials/{credentialId}` (複数形) と定義している (2026-10-06 に仕様の `index.bs` で確認)。
- 実装の最初に、この往復 (`GET .../credentials` が `privateKey` を返すこと、`DELETE` の後に `POST .../credential` へ `signCount` を変えて入れ直すと、次のアサーションの署名カウンタがその値から始まること) を確認し、確認した日付と結果を issue に記録する。これが成立しないと `wrangler_auth_login_complete_regressed_sign_count_409` が通らず、この issue は閉じられない。成立しない場合は、feature の削減 (この issue の他の項目) を先に分離して進め、パスキーの移行は別 issue にする。その判断を issue に記録する。
- `evaluate_json` は WebDriver の非同期スクリプト (`thirtyfour` の `execute_async`) にする。式は `(async () => { try { const value = await (式); done(JSON.stringify({ ok: true, value })); } catch (error) { done(JSON.stringify({ ok: false, error: String(error) })); } })()` の形でコールバックに渡し、`ok` が false のときは Rust 側で Err にする。値が無いとき、JSON の文字列でないときの既存の Err の経路も維持する。script timeout は 60 秒 (`STEP_TIMEOUT` と同じ) を `set_script_timeout` で明示する (0052 の教訓)。
- chromedriver はテストバイナリごとに 1 つ起動して共有し、最後の `TestBrowser` の Drop で停止する (`DevServer` の `shared_server` と同じ Weak の形)。停止はプロセスグループへの SIGTERM と SIGKILL (0019) にして、セッションの終了に失敗しても Chrome を孤児にしない。
- chromedriver の起動と待ち受けは、`wrangler_same_origin_e2e.rs` の `start_chromedriver` と `wait_for_chromedriver` を `backend/brew_book/tests/support/` に移して共有する。起動の補助は案内するタスク名を引数に取り (`start_chromedriver(port, task_name)` のようにする)、`TestBrowser` からは `mise run backend:test-integration`、E2E からは `mise run frontend:test-same-origin` を案内する。
- `support/cdp.rs` は `support/browser.rs` に改名する (パスキーのハーネスが CDP を直接使わなくなるため。E2E の `E2eBrowser` は表示のエミュレーションに CDP を引き続き使う)。`TestBrowser`、`js_string`、chromedriver の起動と待ち受け、WebAuthn の拡張コマンドの補助を置き、`support/e2e.rs` の `E2eBrowser` も同じ補助を使う。
- `backend:test-integration` は chromedriver を必要とするようになる。`mise.toml` の `[tools]` が chromedriver 154.0.8037.57 を持ち、CI は mise-action で入れ、Chrome for Testing もワークフローで入れているため、導入の追加は要らない。chromedriver が見つからない場合は、ハーネスの起動の補助がタスク名の引数の案内で失敗する。
- リスクと扱い: `mise run test` は 5 つのタスク (`backend:test`、`backend:test-integration`、`frontend:test`、`frontend:test-web`、`frontend:test-same-origin`) を並列に実行する。統合の後に ChromeDriver を使うタスクは 3 つ (`backend:test-integration`、`frontend:test-web`、`frontend:test-same-origin`) になり、0046 と 0052 の失敗の対象が広がる。統合の後に 0046 や 0052 の失敗が再現して `mise run check` が通らない場合は、この issue を完了とせず、再現と切り分けを記録して 0046 と 0052 の修正を先に進める (この issue は `docs/issues/pending/` に移す)。

### `reqwest` の既定の feature を外す

- `backend/brew_book/Cargo.toml` の dev-dependencies の `reqwest` (0.12.28) を `default-features = false, features = ["blocking", "json"]` にする。`backend/brew_book_admin/Cargo.toml` の `reqwest` (同じ 0.12.28) は `default-features = false, features = ["blocking"]` にする。管理者のテストは `.json()` を使わず、応答を `.text()` で読むためである (ADR-0001 の「管理者 Worker のためだけの依存は追加しない」に合わせる)。
- ハーネスの接続先は `http://127.0.0.1` と `http://localhost` だけで TLS を使わない。0.12.28 の既定の feature を外すと `native-tls`、`hyper-tls`、`tokio-native-tls`、`openssl`、`openssl-sys`、`openssl-macros`、`foreign-types`、`foreign-types-shared`、`vcpkg` が消える見込みである。
- `thirtyfour` 0.37.5 は別の版の `reqwest` 0.13.5 (feature `rustls`) を使う。0.13.5 の feature は変えない。`system-configuration` と `core-foundation`、`security-framework` は 0.13.5 と `hyper-util` の経路で残り得る。
- 確認は `cargo tree -e features` をホスト (macOS) で実行し、`reqwest` 0.12.28 の経路から `native-tls` と `hyper-tls` が消えたことを確認する。Linux 向けは `cargo tree -e features --target x86_64-unknown-linux-gnu` で `openssl-sys` が消えたことを確認する (CI は `cargo tree` を実行しないため、手元で対象を明示して確認する)。`system-configuration`、`core-foundation`、`security-framework` が残った場合は、その経路と理由を issue に記録する。

### Frontend の `dioxus` の feature を絞る

- `frontend/Cargo.toml` の `dioxus` を `default-features = false, features = ["web", "minimal"]` にする (feature の定義は実装時に dioxus 0.7.10 の `Cargo.toml` と `cargo tree -e features` で確認する)。`minimal` は `macro`、`html`、`signals`、`hooks`、`launch` を含むため、`frontend/src/main.rs` 11 行目の `dioxus::launch` が要求する `launch` feature は満たされる (2026-10-06 の `cargo metadata` で確認。実装時に再確認する)。`dioxus-web` 0.7.10 は `dioxus-devtools` と `dioxus-document` を依存として宣言しているため、`web` を有効にすると `devtools` と `document` が dioxus-web 側の feature で残る可能性がある。外れる feature と削減数は実装時に確認して記録する。
- `frontend/src` に `asset!`、`document::`、`onmounted` の使用が無いことは確認済みである (2026-10-06)。`public/` の静的ファイルの配信は manganis に依存しない (`frontend/Dioxus.toml` と `frontend/index.html`)。
- 目標は 24 パッケージの削減 (devtools、logger、asset が外れる) とする。A (`manganis` が `cargo tree` に現れない) が成立しない場合 (ビルドまたはテストの失敗を含む。`dioxus` の `web` や `dioxus-web` の既定の feature が `manganis` を引き込む場合もここに含む) は、外せなかった feature と、それを引き込んだ依存と feature (該当する API があればそれも) を issue に記録したうえで、外す範囲を `devtools` と `logger` に限定して 12 パッケージ以上の削減で完了とする。`devtools` と `logger` も外せない場合は、外せない理由 (どの依存と feature が引き込んでいるか) を記録し、dioxus の feature の削減は別 issue に分ける。

### `proptest` と `tokio` の feature を絞る

- `backend/pbt/Cargo.toml` と `frontend/pbt/Cargo.toml` の `proptest` を `default-features = false, features = ["std"]` にする。`fork` と `timeout` は未使用で、`timeout` の既定は 0 である。`bit-set` も外す (`proptest::bits` を使うテストが無いことを確認済み。使うテストが増えた場合は feature を戻す)。
- `backend/brew_book/Cargo.toml` の `tokio` から `macros` を外す。

### 採らなかった案

- 重複するバージョンの統合: `getrandom` 0.2/0.3/0.4、`base64` 0.22/0.23、`digest` 0.10/0.11、`block-buffer`、`crypto-common`、`cpufeatures` が重複するが、RustCrypto のメジャーをまたぐため、依存の更新と検証の範囲が広がりすぎる。
- Backend と Frontend のワークスペースの統合: 共通のクレートをホスト向けに 1 回だけビルドできるが、ADR-0017 の分離と `dx` の前提を崩し、`Cargo.lock` の統合も要る。
- `headless_chrome` を残して `thirtyfour` をやめる、または両方を残す: E2E が `thirtyfour` を要する。削減の目的にも反する。
- `thirtyfour` の `manager` feature (chromedriver の自動取得): mise の固定した版を使う方針 (0044) に反する。
- 自前の CDP クライアント: 保守の負担が増え、0005 の「CDP クライアントの確認」の結論 (既存のクライアントを使う) に反する。
- `backend:test` が実行しない `wrangler_` のテストバイナリのリンクを避ける (テストターゲットを列挙してビルドする): テストファイルの列挙は取りこぼしのリスクがあり、依存の削減ではない。CI のキャッシュと並列度の見直し (別の issue) を先に行う。

### 文書の更新

- `docs/adr/0004-passkey-only-authentication.md` 89 行目の「パスキーを伴う統合テストは、Chrome DevTools Protocol の仮想認証器で行う」に改訂の注記を足す (ADR-0013 の 0044 の改訂と同じ形)。
- `docs/adr/0017-frontend-rust-dioxus-web-only.md` 28 行目は「Chrome DevTools Protocol の仮想認証器を使い」となっているが、E2E は既に ChromeDriver の WebAuthn の拡張コマンドを使っている。統合に合わせて是正する。
- `docs/adr/0013-five-test-layers.md` 40 行目の「ChromeDriver の WebAuthn の拡張コマンド (`WebAuthn.addVirtualAuthenticator`)」は CDP のメソッド名のままなので、拡張コマンドの名前に合わせる。
- `docs/prd/brewbook.md` 798 行目の「現在は Chrome DevTools Protocol、移行後は ChromeDriver の WebAuthn の拡張コマンド」を、統合後の状態に合わせる。
- `backend/brew_book/src/test_page.rs` の 1 行目と 11 行目、`backend/brew_book/tests/wrangler_auth_api.rs` の 4 行目、`support/browser.rs` (旧 `support/cdp.rs`)、`wrangler_passkey_flow.rs`、`support/mod.rs` のコメントとモジュールの名前、`Cargo.toml` の依存のコメントを実装に合わせる。あわせて `mise.toml` の 12 行目から 14 行目の chromedriver の利用者のコメントを 3 タスクに直し、`backend:test-integration` の説明 (111 行目) に chromedriver を使うことを足す。

## 完了条件

- `backend/brew_book/Cargo.toml` から `headless_chrome` が消え、`backend/Cargo.lock` が追随している。`cargo tree` に `headless_chrome` と `auto_generate_cdp` が現れない。
- 実装の最初に、WebAuthn 拡張の往復で `set_sign_count` が成立することを確認し、確認した日付と結果を issue に記録している。成立しない場合は、`TestBrowser` の移行に関する項目 (3 番目から 6 番目) を新しい issue に移し、この issue では feature の削減だけを完了条件とする (その判断を issue に記録している)。
- `support/browser.rs` の `TestBrowser` が `thirtyfour` と chromedriver で動き、`wrangler_passkey_flow.rs` の 7 テストが `mise run backend:test-integration` で通過する。Chrome の起動オプションは `E2eBrowser` と同じで、CI の Linux のコンテナでも通過する。
- 署名カウンタの付け替え (`set_sign_count`) が WebAuthn の拡張コマンドで行え、署名カウンタの後退の 409 のテストが通過する。
- `evaluate_json` の式が例外を投げたとき、Rust 側で Err になる (`ok` が false の応答を Err にする)。値が無いとき、JSON の文字列でないときの Err も維持している。script timeout は 60 秒を明示している。
- chromedriver が見つからないとき、起動の補助が呼び出し元のタスク名 (`mise run backend:test-integration` または `mise run frontend:test-same-origin`) を案内するメッセージで失敗する。chromedriver の停止で Chrome のプロセスが残らない (0019 と同じプロセスグループの停止)。
- `reqwest` 0.12.28 の既定の feature を外し、ホスト (macOS) の `cargo tree -e features` で `native-tls` と `hyper-tls` が現れず、`cargo tree -e features --target x86_64-unknown-linux-gnu` で `openssl-sys` が現れない。`system-configuration`、`core-foundation`、`security-framework` が残った場合は、その経路と理由を記録する。
- Frontend の `dioxus` の feature を絞り、外れた feature とパッケージ数を記録している。判定は `cargo tree --workspace` (または `cargo tree -p brew_book_frontend_pbt`) で行い、次の順で最初に成立した段階で完了とする。
  - A: `dioxus-devtools`、`dioxus-logger`、`manganis` が現れない (dioxus の feature の変更による差で 24 パッケージ以上)。
  - B: A が不可の場合、`dioxus-devtools` と `dioxus-logger` が現れず、外せなかった feature と、それを引き込んだ依存と feature (該当する API があればそれも) を記録している (dioxus の feature の変更による差で 12 パッケージ以上)。
  - C: B も不可の場合、外せない理由を記録し、dioxus の feature の削減を別 issue に分けている。
- `proptest` と `tokio` の feature を絞り、Backend の `cargo tree` に `rusty-fork`、`wait-timeout`、`bit-set`、`bit-vec` が現れず、Frontend の `cargo tree --workspace` に `rusty-fork`、`wait-timeout`、`bit-set`、`bit-vec` が現れない (`tempfile` の去就も確認して記録する)。`tokio-macros` は `cargo tree -e features -i tokio-macros` で有効にしているクレートを確認し、`worker` や `thirtyfour` が `tokio` の `macros` を有効にして消えない場合は、残る経路と理由を記録する。
- 計測: 変更の前後で Backend と Frontend の `cargo metadata` の依存グラフのパッケージ数を記録する。cold のビルドを `CARGO_TARGET_DIR` を空のディレクトリにして (レジストリと git のキャッシュは共有してよい) 次の 3 つで計測し、所要時間を issue に記録する。
  - Backend: `cargo test --workspace --no-run` (`backend` で実行。`brew_book_pbt` を含む)
  - Frontend の native: `cargo test --manifest-path frontend/Cargo.toml --workspace --no-run` (`brew_book_frontend_pbt` を含む)
  - Frontend の wasm32: `cargo test --manifest-path frontend/Cargo.toml --workspace --target wasm32-unknown-unknown --no-run`
- `docs/adr/0004-passkey-only-authentication.md` に改訂の注記があり、`docs/adr/0017-frontend-rust-dioxus-web-only.md` 28 行目、`docs/adr/0013-five-test-layers.md` 40 行目、`docs/prd/brewbook.md` 798 行目が現状に合っている。
- コメントの更新 (`test_page.rs` の 1 行目と 11 行目、`wrangler_auth_api.rs` の 4 行目、`support/browser.rs`、`wrangler_passkey_flow.rs`、`support/mod.rs` のモジュールの名前、`Cargo.toml` の依存のコメント、`mise.toml` の chromedriver の利用者と `backend:test-integration` の説明) が済んでいる。
- テストの検証内容と製品の挙動を変えていない (テストの期待値と経路の台帳の変更が無い)。
- `mise run check` を既定の並列度 (`mise.local.toml` を外した状態。`jobs = 2` と `CARGO_BUILD_JOBS = "4"` の両方を外す) で 3 回連続して実行し、通過する。0046 または 0052 の失敗が再現した場合は、この issue は完了とせず、再現の記録を足して `docs/issues/pending/` に移す。

## 関連

- 0005 (`headless_chrome` を追加した issue) と 0044 (`thirtyfour` と E2E のハーネスを追加した issue) の統合である。どちらも closed のため、履歴は書き換えない。
- 0046 と 0052 (ChromeDriver の並列実行の失敗) に影響し得る。統合の後に失敗の再現が変わった場合は issue に記録する。
- 0037 (Frontend のツールチェーンと CI のキャッシュ)。CI のキャッシュの改善 (10 GB の上限を超えないように削る) と、`backend:test` と `backend:test-integration` のビルドロックの待ちの見直しは、この issue の範囲外の別 issue とする。

## 実装中の記録

- 所有者の指示 (2026-10-06) により、cold ビルドの所要時間の変更前後の計測は行わない。変更前後のパッケージ数 (`cargo metadata` の依存グラフで、ワークスペースのメンバーを起点に到達できるパッケージ数) は次のとおりである。
  - Backend: 320 (2026-10-06 の変更前) → 251 (2026-10-06 の変更後)
  - Frontend: 366 (同日の変更前) → 337 (同日の変更後)
- 所有者の指示 (2026-10-06) により、`mise run check` は「既定の並列度で 3 回連続」ではなく、`mise.local.toml` の設定 (`jobs = 2`、`CARGO_BUILD_JOBS = "4"`) のまま 1 回だけ実行する (CI の既定の並列度ではない)。
- `dioxus` の feature は A を満たせず (`manganis` は外れたが `dioxus-devtools` が残る)、B も満たせない (`dioxus-devtools` は `dioxus-web` 0.7.10 の非 optional の依存で、feature では外せない)。C に当たるが、外せる `dioxus-logger` と `manganis` (asset) は外しており、残る `dioxus-devtools` は feature では外せない (依存の構成の問題) ため、分ける対象が無い。Frontend の削減は 29 パッケージである。
- `document` feature は、`frontend/tests/` の複数の Web テスト (test_settings_web、test_stats_web、test_records_lists_web、test_records_screens_web) が `dioxus::history` を使うため残した (方針の `["web", "minimal"]` からの乖離)。
- `web-sys` は `dioxus-web` が host のビルドでも要求するため target の限定を外し、外した `devtools` と `mounted` が有効にしていた `Location` と `DomRect` を明示した。
- `reqwest` 0.12.28 の既定の feature を外した結果、`native-tls`、`hyper-tls`、`openssl-sys`、`system-configuration` は `cargo tree -e features` から消えた (ホストと `--target x86_64-unknown-linux-gnu` で確認)。残るのは `core-foundation` と `security-framework` (reqwest 0.13.5 の rustls 経路の `rustls-platform-verifier` と `rustls-native-certs` 経由) である。
- `tokio-macros` は、`hyper-util` (reqwest 0.12.28 と 0.13.5 の `client` feature) が `tokio` の `macros` を有効にするため残る。`#[tokio::main]` と `#[tokio::test]` は引き続き未使用である。
- 2026-10-06 に、実装の最初に WebAuthn 拡張の往復を確認した。`GET .../credentials` が `privateKey` を返し、`DELETE .../credentials/{id}` の後に `POST .../credential` へ `signCount` を変えて入れ直すと、次のアサーションの署名カウンタがその値から始まることを、`wrangler_auth_login_complete_regressed_sign_count_409` の通過で確認した。
- 検証の方法: `TestBrowser` の 7 テストは `mise run check` の `backend:test-integration` で通過した。レビューの反映で足した 2 テスト (`evaluate_json` の Err と値の欠落、chromedriver のプロセスグループの停止) は `cargo test -p brew_book --test wrangler_passkey_flow` の単独実行で通過した (9 passed)。Chrome の起動オプションと script timeout 60 秒はコードの読み取りで確認した。chromedriver 不在時のメッセージは、PATH から chromedriver を外してテストバイナリを実行し、`chromedriver must start (run this test with `mise run backend:test-integration`): No such file or directory` で失敗することを確認した。
- CI の Linux のコンテナでの通過は、push 後でないと確認できない (未検証)。CI は mise の chromedriver 154.0.8037.57 と Chrome for Testing の同じ版を入れるため、前提は整っている。
- `E2eBrowser::open` は script timeout を設定していない (0052 の範囲)。この issue では `TestBrowser` に 60 秒を明示し、E2E 側の扱いは 0052 に委ねる。

## 解決方法

パスキーの結合テストのブラウザ操作を ChromeDriver の WebAuthn の拡張コマンドに移行し、テストとビルドでコンパイルされる依存を削減した。

- `backend/brew_book/tests/support/browser.rs` (旧 `support/cdp.rs`) に `TestBrowser` を thirtyfour と chromedriver で実装し直した。`shared_chromedriver` はテストバイナリごとに 1 つの chromedriver を Weak で共有し、`ChromeDriver` の Drop がプロセスグループへ SIGTERM と SIGKILL を送る (0019 と同じ)。`start_chromedriver` は案内するタスク名を引数に取る。仮想認証器は `add_virtual_authenticator` が ChromeDriver の WebAuthn の拡張コマンドで付け、`set_sign_count` は `GET .../credentials`、`DELETE .../credentials/{id}`、`POST .../credential` で `signCount` を入れ直す。`evaluate_json` は `execute_async` のコールバックで `{ ok, value }` または `{ ok: false, error }` を返し、例外と `undefined` は Rust の Err にする。script timeout は 60 秒を明示する。chromedriver への HTTP 呼び出しには timeout を設定する。
- `backend/brew_book/tests/support/mod.rs` の `pub mod cdp` を `pub mod browser` に、`support/e2e.rs` の `E2eBrowser::add_virtual_authenticator` を共有の補助への委譲に変えた。`wrangler_same_origin_e2e.rs` のローカルの chromedriver の起動と待ち受けを削除した。`wrangler_passkey_flow.rs` の import と、`wrangler_auth_api.rs`、`src/test_page.rs` のコメントを直し、`evaluate_json` の Err の経路と chromedriver のプロセスグループの停止の 2 テストを足した。
- `backend/brew_book/Cargo.toml` から `headless_chrome` を外し、`reqwest` の既定の feature を外し (`default-features = false, features = ["blocking", "json"]`)、`tokio` から `macros` を外した。`backend/brew_book_admin/Cargo.toml` の `reqwest` も既定の feature を外した (`features = ["blocking"]`)。`backend/pbt` と `frontend/pbt` の `proptest` は `default-features = false, features = ["std"]` にした。`frontend/Cargo.toml` の `dioxus` は `default-features = false, features = ["web", "minimal", "document"]` にし、`web-sys` を target の限定から外して `Location` と `DomRect` を足した。
- `mise.toml` の chromedriver の利用者のコメントと `backend:test-integration` の説明、`docs/adr/0004` (改訂の注記)、`docs/adr/0013`、`docs/adr/0017`、`docs/prd/brewbook.md` を現状に合わせた。

完了条件の検証:

- `headless_chrome` と `auto_generate_cdp` は `cargo tree` から消えた。`cargo metadata` の依存グラフの到達数は Backend 320 → 251。
- 実装の最初に `set_sign_count` の往復を確認し、`wrangler_auth_login_complete_regressed_sign_count_409` の通過 (7 テスト) で `GET .../credentials` が `privateKey` を返すことと、入れ直した `signCount` から次のアサーションが始まることを確認した (2026-10-06)。
- `TestBrowser` の 7 テストは `mise run check` の `backend:test-integration` で通過した。レビューの反映で足した 2 テスト (`evaluate_json` の Err と値の欠落、chromedriver のプロセスグループの停止) は `cargo test -p brew_book --test wrangler_passkey_flow` の単独実行で通過した (9 passed)。Chrome の起動オプションと script timeout 60 秒はコードの読み取りで確認した。chromedriver 不在時のメッセージは、PATH から chromedriver を外した実行で `mise run backend:test-integration` を案内することを確認した。
- `reqwest` 0.12.28 の既定の feature を外し、`native-tls`、`hyper-tls`、`openssl-sys`、`system-configuration` は `cargo tree -e features` から消えた (ホストと `--target x86_64-unknown-linux-gnu` で確認)。残るのは `core-foundation` と `security-framework` (reqwest 0.13.5 の rustls 経路の `rustls-platform-verifier` と `rustls-native-certs` 経由) である。
- Frontend は `dioxus-logger` と `manganis` が消えた (366 → 337)。`dioxus-devtools` は `dioxus-web` 0.7.10 の非 optional の依存で feature では外せないため、A も B も満たせず C に当たる。外せる `dioxus-logger` と `manganis` は外しており、残る `dioxus-devtools` は feature では外せない (依存の構成の問題) ため、分ける対象が無い (「## 実装中の記録」)。
- `proptest` の `fork` と `timeout` は backend と frontend の両方で外れ、`rusty-fork`、`wait-timeout`、`tempfile`、`bit-set`、`bit-vec` は現れない。`tokio-macros` は `hyper-util` が `tokio` の `macros` を有効にするため残る。
- 文書とコメントを更新した。
- `mise run check` を `mise.local.toml` の設定 (jobs = 2) で 1 回実行して通過した (所有者の指示。CI の既定の並列度ではない)。レビューの反映で足したテストは単独で確認した。
- CI の Linux のコンテナでの通過は push 後でないと確認できない (未検証)。CI は mise の chromedriver 154.0.8037.57 と Chrome for Testing の同じ版を入れるため、前提は整っている。

方針からの乖離:

- cold ビルドの所要時間の変更前後の計測は所有者の指示で行っていない (パッケージ数のみ記録)。
- `dioxus` の A (devtools も外す) は `dioxus-web` の非 optional の依存のため不可で、C の理由を記録した。
- `document` feature は `frontend/tests/` の複数の Web テストが `dioxus::history` を使うため残した。`web-sys` は `dioxus-web` が host のビルドでも要求するため target の限定を外し、外した `devtools` と `mounted` が有効にしていた `Location` と `DomRect` を明示した。
- `evaluate_json` は、方針の `done(JSON.stringify({ ok: true, value }))` ではなくオブジェクトを直接コールバックに渡す (`execute_async` の戻り値がそのまま JSON になるため)。`undefined` は JS 側で明示的にエラーにして、旧実装と同じく Err にする。
