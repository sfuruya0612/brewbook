# 管理者 Worker と Cloudflare Access の保護を作る

Created: 2026-09-21
Model: deepseek-v4p1-flash
対応 ADR: ADR-0008 (docs/adr/0008-admin-worker-cloudflare-access.md)、ADR-0001 (docs/adr/0001-backend-rust-on-cloudflare-workers.md)、ADR-0002 (docs/adr/0002-database-cloudflare-d1.md)
関連 PRD: FR-17、セキュリティ (Access)、運用 (デプロイ後の確認)、成功指標 (テスト網羅)
依存: 0003, 0005, 0017

## 背景

ADR-0008 は、利用者の作成と登録用トークンの発行 (PRD の FR-17) を管理者用の Worker (管理者 Worker) で行い、`coffee-log-admin.<アカウントのサブドメイン>.workers.dev` で配信すると決めた。
管理者 Worker は Worker 単位の Cloudflare Access 保護で全体を守り、ポリシーは所有者のメールアドレスだけを許可する。
管理者 Worker は Access の JWT を検証しない (依存を増やさないため。所有者が決定)。
管理者画面は管理者 Worker がサーバー側で生成する HTML のフォームとし、Flutter と JavaScript を使わない。
機能は、利用者の一覧、利用者の作成、登録用トークンの発行の 3 つとする。
管理者 Worker は利用者向けの Worker と同じ D1 データベースをバインディングで参照し、users と registration_tokens を読み書きし、passkey_credentials を読む。
現状は利用者向けの Worker とスキーマと認証までができており、利用者を作成する手段が無い。

## 目的

管理者が、Cloudflare Access で保護された管理者画面で、利用者を作成し、登録用トークンを発行して登録用リンクを利用者に渡せるようにする。

## 設計判断

- `backend/coffee_log_admin/` に管理者 Worker を実装し、`wrangler.toml` の Worker の名前を `coffee-log-admin` とする。
  D1 のバインディングは利用者向けの Worker と同じ `database_id` を参照し、マイグレーションを持たない (ADR-0002)。
  利用者向けの Worker のオリジン (登録用リンクに使う) は環境変数で持つ。
- 管理者画面はサーバー側で生成する HTML とし、テンプレートエンジンを追加せず Rust の文字列テンプレートで組み立てる。
  利用者の表示名と発行したリンクは HTML エスケープして出力する。
- 経路は次の 3 つとする。
  - `GET /` で利用者の一覧 (表示名、作成日時、パスキーの数) を表示する。
  - `POST /users` で表示名 (前後の空白を除いて 1 文字以上 50 文字以下) を入力して利用者を作成する。範囲外は 400 を返す。
  - `POST /users/<ID>/tokens` で登録用トークンを発行する。
- 管理者 Worker も経路の台帳を 1 か所に持ち、Router は台帳から組み立てる。
  テストの識別子と台帳を照合し、全経路に正常系、入力を持つ経路に入力不正 400 のテストがあることを検査する (PRD の成功指標は両方の Worker に同じ仕組みを求める)。
- 再発行では、その利用者の未使用の登録用トークンを削除してから新しいトークンを発行する (紛失時の再発行で古いリンクが残らないようにする)。
- トークンの発行の応答は、登録用リンク (`/register?token=<トークン>` の完全な URL) を表示する HTML を直接返す。
  303 のリダイレクトにしない (生のトークンを保存しないため、一覧を含む他の応答では再表示できない)。
  存在しない利用者の ID は 404 を返す。
  有効期限は 24 時間とし、D1 にはハッシュだけを保存する (0005 と同じ)。
- 状態を変更するフォームの送信は、`Origin` ヘッダが管理者 Worker のオリジンと一致しない場合に 403 を返す (`Origin` が無い場合も 403。0017 と同じ規則)。
- SQL は 1 か所で組み立て、SQL に現れるテーブル名が users、registration_tokens、passkey_credentials の 3 つだけであることを単体テストで照合する (FR-17)。
  それ以外のテーブルは読み書きしない。
- 利用者向けの Worker の応答に表示名が含まれないことを、テーブルに表示名を入れた状態での結合テストで確認する (FR-17)。
  利用者向けの SQL を列挙して `display_name` が現れないことと、`SELECT *` を使っていないことも単体テストで確認する (列を明示しない取得では検査をすり抜けるため)。
  本 issue の時点で利用者向けの SQL は出揃うため、このテストは本 issue が持つ。
- Cloudflare Access は Worker 単位の保護を設定し、ポリシーで所有者のメールアドレスだけを許可する。
  認証は Access の One-time PIN (メール) を使う。
  設定は Access の管理画面で行い、手順をデプロイ手順 (README) に書く。
  Zero Trust の Free プラン (50 ユーザーまで、契約にクレジットカードの登録が必要) を使う。
- デプロイ後の確認のタスク (`mise run verify-deploy`) を追加し、認証なしで管理者画面の URL を取得して Access のログイン画面へリダイレクトされる (302) ことを検証する。
  状態を変更する経路 (`POST /users`) への未認証リクエストも 302 になることを確認する。
  302 でなければタスクを失敗させる。
  `deploy-admin` タスクで管理者 Worker をデプロイする (デプロイのタスクは `mise run check` に含めない。0002 の規則)。
- ローカル開発では wrangler の `access.dev` の設定を使わない (管理者 Worker は識別情報を使わない。ADR-0008)。
- 管理者 Worker の自動テストは、HTML の生成とフォームの入力の検証を純粋な関数として単体テストし、D1 を触る部分は `wrangler dev` に対する結合テストで検証する (ADR-0008)。
  結合テストは 0001 のハーネスと同じ方式で、管理者 Worker の `wrangler dev` をテストが起動する。
- 採らない案: 管理者用の CLI (所有者が Web 画面を選んだ。ADR-0008)、利用者向けの Worker の `/admin/*` を Access で保護する (workers.dev ではパス単位の保護ができない。ADR-0008)、Access の JWT の検証 (rsa 相当の依存が増える。ADR-0008)、Basic 認証 (パスワードの管理が必要になる。ADR-0008)。

## 調査タスク

2026-09-26 に実装と自動テストを完了した (ローカルの `mise run check` は通過)。実装は edc3ac1 (本体)、735681a (実装詳細の乖離) と、レビューの指摘への対応 (管理者側の `SELECT *` の検査、既定ポートの境界値の検査、利用者向けの応答の状態コードの検査、`mise run dev-admin` の追加) に記録した。

確認できた完了条件:

- FR-17 の受け入れ基準: `wrangler_admin_api.rs` の 7 件が、一覧 (表示名、作成日時、パスキーの数)、表示名を入力した利用者の作成 (前後の空白を除く)、範囲外の表示名の 400、トークンの発行 (有効期限 24 時間、ハッシュだけの保存)、再発行で古いリンクが無効になること、存在しない利用者の 404、リンクが発行直後の応答に 1 回だけ含まれることを確認した。`test_html.rs` が一覧とリンクの HTML エスケープを、`test_input.rs` が表示名の検証 (1 文字以上 50 文字以下、メッセージは英語) を確認した。
- 管理者 Worker が扱うテーブルと SQL: `backend/coffee_log_admin/tests/test_queries.rs` の 5 件が、SQL の列挙から users、registration_tokens、passkey_credentials の 3 つだけであることと、取得 (`SELECT`) に `SELECT *` を使わないことを確認した。利用者向けの SQL は `backend/coffee_log/tests/test_queries.rs` の 6 件が、取得 (SELECT) に表示名 (`display_name`) が現れないことと `SELECT *` を使わないことを確認した (`d1_check` の検証用の INSERT は表示名を入れるため、検査は取得に限る)。
- 経路の台帳とテストの照合: `test_routes.rs` が台帳とテストの識別子を照合し、全経路に正常系、入力を持つ経路に入力不正 400 のテストがあることを検査する (成功指標の管理者 Worker の分)。
- `Origin` の検証: `wrangler_admin_api.rs` の `wrangler_admin_origin_403` が別オリジンと `Origin` の無いフォームの送信で 403 を、`backend/coffee_log_admin/tests/test_origin.rs` の 8 件が境界値 (scheme、host、port、既定のポート) を確認した。
- 利用者向けの応答に表示名が含まれないこと: `wrangler_display_name.rs` が表示名を持つ利用者で利用者向けの全経路を呼び、どの応答の本文にも `display_name` と表示名の値が含まれないことと、セッションを消す経路以外は有効なセッションで到達できること (401 と 500 でないこと) を確認した。
- 依存: 管理者 Worker の依存は `serde` と `worker` (d1)、開発依存は `reqwest` と `serde_json` で、利用者向けの Worker と同じ組である (ADR-0001)。
- `deploy-admin` と `verify-deploy` のタスクを実装した (`verify-deploy` は `ADMIN_ORIGIN` の GET / と POST /users の 302 を検査し、302 でなければ失敗する)。ローカルの確認用に `mise run dev-admin` を追加した。
- README に Access の設定手順を書いた (Worker 単位の保護、所有者のメールアドレスのポリシー、One-time PIN、利用者向けの Worker には設定しないこと、アカウント全体の Worker を既定で保護する設定を使わないこと)。
- `mise run check`: exit 0 (ベースラインからの新たな失敗は無し)。

残るのは、Cloudflare のアカウントの資格情報が要る次の確認である。

- デプロイの前に、`backend/coffee_log_admin/wrangler.toml` の `[vars]` の `APP_ORIGIN` を実際の利用者向けの Worker のオリジンへ置き換える。
- `mise run deploy-admin` を実行し、`coffee-log-admin.<アカウントのサブドメイン>.workers.dev` で管理者画面が配信されることを確認する。
- Cloudflare の管理画面で Worker 単位の Cloudflare Access の保護と、所有者のメールアドレスだけを許可するポリシーを設定する (Zero Trust の Free プラン、One-time PIN)。**デプロイした直後に設定する** (設定が終わるまで管理者画面の URL を共有しない。Access の設定だけが管理者 Worker の安全性を担う)。
- `ADMIN_ORIGIN=https://coffee-log-admin.<サブドメイン>.workers.dev mise run verify-deploy` を実行し、認証なしの `GET /` と `POST /users` が Access のログイン画面へ 302 でリダイレクトされることを確認する (302 でなければ失敗する)。
- Access を設定した後、所有者がログインして利用者の作成と登録用トークンの発行を行い、利用者向けの画面から登録できることを確認する。

## 関連

- 0005 が登録用トークンの検証とパスキーの登録を作る。
- 0017 が利用者向けの Worker のデプロイとオリジンの検証を作る。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. 乱数の取得 (`backend/coffee_log_admin/src/random.rs`) は 0005 の利用者向けの実装と同じものを置いた (別クレートで、共有すると `coffee_log_core` が `worker` に依存してしまうため。追加する依存は増やしていない)。
2. 管理者 Worker の依存は `serde` と `worker` (d1)、開発依存は `reqwest` と `serde_json` とし、利用者向けの Worker と同じ組に限った (ADR-0001)。PBT は `coffee_log_admin` を開発依存として参照する。
3. FR-17 の「表示名を読まない」「`SELECT *` を使わない」「扱うテーブルは 3 つだけ」の検査のため、`backend/coffee_log/src/queries.rs` を新設して利用者向けの SQL 文を 1 か所に列挙し、`auth/*` と `d1_check.rs` の SQL の定数を `pub(crate)` にした (文の内容は不変)。
4. HTML の生成と入力の検証は純関数として `html.rs` と `input.rs` に置き、単体テストに加えて PBT (`backend/pbt/tests/prop_admin_html.rs`、`prop_admin_input.rs`) で性質を検査した (既存のテストの方針に合わせた)。
5. `mise run verify-deploy` は、対象の管理者 Worker の URL を環境変数 `ADMIN_ORIGIN` で受け取る (デプロイ先はアカウントごとに異なり、リポジトリに実値を含めないため)。
6. 経路の台帳の `POST /users/:id/tokens` は `has_input` を false にした (`:id` の経路のパラメータは入力に数えない。入力を持たない `:id` の経路 (`shops_get` など) と同じ扱い)。
7. `Origin` の検証は 0017 と同じ規則を管理者 Worker にも置いた (URL として正規化して比較し、`Origin` が無ければ 403。`backend/coffee_log_admin/src/origin.rs`)。
8. 登録用リンクのオリジンは `[vars]` の `APP_ORIGIN` で持ち、`wrangler.toml` が常に値を与える。`tokens.rs` の既定値 (`http://localhost:8787`) は var が未設定のときのフォールバックであり、ローカルの確認は `mise run dev-admin` (`--var APP_ORIGIN:http://localhost:8787`) を使う。本番の値はデプロイの前に置き換える。
9. `mise run dev-admin` を追加した (ローカルの管理者画面の確認で、登録用リンクが利用者向けの `mise run dev` のオリジンを指すようにするため)。

## pending にした理由

2026-09-26 に実装と自動テストを完了し、コミット edc3ac1 と 735681a、レビューの指摘への対応のコミットに記録した。完了条件のうち、FR-17 の受け入れ基準、SQL の検査 (3 テーブルだけ、`SELECT *` を使わない、表示名を読まない)、経路の台帳とテストの照合、`Origin` の検証、利用者向けの応答に表示名が含まれないこと、依存が利用者向けと同じ組であること、タスクと README の実装はローカルで確認済みである。

残るのは、Cloudflare のアカウントの資格情報が要る次の 5 つで、この環境では実行できないため pending にした。

- `[vars]` の `APP_ORIGIN` の、実際の利用者向けの Worker のオリジンへの置き換え
- `mise run deploy-admin` の実行
- Cloudflare の管理画面での Access の設定 (Worker 単位の保護とポリシー。デプロイの直後に行い、設定が終わるまで URL を共有しない)
- `mise run verify-deploy` の実環境での実行 (302 の確認)
- 所有者による、利用者の作成、登録用トークンの発行、利用者向けの画面からの登録の確認

再開の条件: Cloudflare のアカウントの資格情報が使える環境で、所有者がデプロイ手順 (README) に従って上記の 5 つを実行し、結果を本 issue に記録する。`CHANGES.md` の `[ADD]` エントリはこの pending への移動のコミットに含める (close のときに重ねて追記しない)。
