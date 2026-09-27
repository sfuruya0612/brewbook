# mise run dev-admin のローカル D1 が利用者向けの Worker と共有されない

Created: 2026-09-27
Model: deepseek-v4p1-flash
Completed: 2026-09-27

## 症状

`mise run dev-admin` (管理者 Worker、`backend/brew_book_admin` で実行する) は、wrangler のローカル D1 を `backend/brew_book_admin/.wrangler/state` に作る。`mise run dev` (利用者向けの Worker、`backend/brew_book` で実行する) は `backend/brew_book/.wrangler/state` を使うため、2 つの Worker は同じ `database_id` (`00000000-0000-0000-0000-000000000000`) を参照していても別のデータベースを見る。`mise run db-migrate` がマイグレーションを適用するのは `backend/brew_book` の状態だけである。

このため、既定のタスクで起動した管理者画面 (`GET /`) は次のエラーで 500 になる (2026-09-27 に確認)。

```
[ERROR] the users_list route failed: Error: D1_ERROR: no such table: users: SQLITE_ERROR
```

利用者の作成 (INSERT INTO users) と登録用トークンの発行も、管理者 Worker のローカル D1 に `users` と `registration_tokens` のテーブルが無いため同じエラーになり、管理者画面から利用者を登録できない。仮に行が保存できたとしても、保存先は管理者 Worker のローカル D1 に限られるため、利用者向けの `http://localhost:8787/register?token=...` は 404 (the registration token does not exist) になる。

`mise run dev-admin --port 8788 --persist-to ../brew_book/.wrangler/state` を実行したときだけ、利用者向けの Worker と同じローカル D1 を使える (回避策。2026-09-27 に確認)。

## 再現手順

1. リポジトリのルートで `mise run db-migrate` を実行する。
2. ターミナル 1 で `mise run dev` を実行し、`http://localhost:8787/` が表示されることを確認する。
3. ターミナル 2 で `mise run dev-admin` を実行する。wrangler は使用中の 8787 を避けて 8788 で起動し、`Ready on http://localhost:8788` と表示する (どのポートで起動したかはタスクの出力でしか分からない)。
4. `http://localhost:8788/` を開く。500 になり、wrangler のログに `no such table: users` が出る (期待: 利用者一覧が表示され、利用者を作成して登録用リンクを発行できる)。
5. `mise run dev-admin --port 8788 --persist-to ../brew_book/.wrangler/state` を実行してから `http://localhost:8788/` を開くと、利用者一覧が表示される (回避策)。

環境: macOS、wrangler 4.135.0 (`mise.toml`)、ローカルの `wrangler dev` と D1 のバインディング。

## 原因

wrangler のローカル D1 は、設定ファイルのあるディレクトリの `.wrangler/state` に SQLite ファイルを作る。`mise.toml` の `dev` は `backend/brew_book`、`dev-admin` は `backend/brew_book_admin` を `dir` に指定しているため、保存先が分かれる。

## 完了条件

- `mise run dev` の起動中に `mise run dev-admin` を実行すると、`http://localhost:8788/` で利用者一覧が表示され、利用者の作成と登録用リンクの発行ができる。
- 管理者画面で発行した登録用リンク (`http://localhost:8787/register?token=...`) からパスキーを登録できる。
- `mise run check` が通過する。

## 解決方法

- `mise.toml` の `dev-admin` の `run` を `wrangler dev --port 8788 --persist-to ../brew_book/.wrangler/state --var APP_ORIGIN:http://localhost:8787` に変えた。`--persist-to` で利用者向けの Worker (`mise run dev`、`backend/brew_book`) と同じローカル状態を参照する (2 つの `wrangler.toml` の `database_id` が同じであることが前提。ADR-0002)。`--port` で利用者向けの 8787 と分けたポートを固定し、起動するポートを出力から読み取らなくてよいようにした。
- `README.md` のローカル開発に、管理者画面が `http://localhost:8788/` で配信され、ローカルの D1 を `mise run dev` の利用者向けと共有することを追記した。
- 再現確認: `mise run dev` の起動中に `mise run dev-admin` を実行し、修正前は `GET http://localhost:8788/` が 500 (`the users_list route failed: Error: D1_ERROR: no such table: users: SQLITE_ERROR`) になることを確認した。修正後は同じ起動で `GET /` が 200 になり、利用者一覧にローカルの D1 の利用者 (`ふるしょ`、作成 2026-09-27T02:25:40.760Z) が表示されることを確認した。
- 完了条件の確認:
  - 利用者一覧の表示は、修正後の起動の `GET /` が 200 になることで確認した。利用者の作成と登録用リンクの発行、登録用リンクからのパスキーの登録は、所有者が 2026-09-27 に同じ `--persist-to` を使う起動で確認済みである。
  - `mise run check` が終了コード 0 で通過した (1452 秒)。
