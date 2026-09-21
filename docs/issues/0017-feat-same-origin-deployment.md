# Frontend と Backend を同一オリジンの 1 つの Worker で配信する

Created: 2026-09-21
Model: deepseek-v4p1-flash
対応 ADR: ADR-0005 (docs/adr/0005-same-origin-deployment.md)、ADR-0003 (docs/adr/0003-photo-storage-r2-presigned-upload.md)、ADR-0004 (docs/adr/0004-passkey-only-authentication.md)、ADR-0009 (docs/adr/0009-mise-toolchain-and-tasks.md)
関連 PRD: 運用 (Static Assets とデプロイ手順)、セキュリティ (同一オリジンと Origin の検証)、制約と前提 (workers.dev)
依存: 0001, 0002, 0003, 0005, 0006, 0007, 0008, 0009, 0010, 0011, 0012, 0013, 0014, 0015, 0016

## 背景

ADR-0005 は、Flutter Web のビルド成果物を利用者向けの Worker の Static Assets として同梱し、1 つの Worker を `coffee-log.<アカウントのサブドメイン>.workers.dev` で配信すると決めた。
`/api/*` は Rust の処理に渡し、それ以外は Static Assets から返す。
既定では一致する静的ファイルが Worker より先に返されるため、`run_worker_first` を `["/api/*"]` に設定する。
Flutter のルーティングで使うパスは `not_found_handling` を `single-page-application` にして `index.html` を 200 で返すことで扱う。
Backend は CORS を許可せず、状態を変更する API は `Origin` ヘッダを検証して同一オリジン以外を 403 で拒否する。
ローカル開発では `flutter build web` の成果物を `wrangler dev` の Static Assets から配信し、Flutter の開発サーバーは使わない (別オリジンになり、CORS を許可しない決定と `Origin` の検証に反するため)。
現状は Worker の設定が API だけを想定しており、ビルド成果物の配信と Origin の検証が無い。

## 目的

アプリの画面と API が同じオリジンから配信され、同じ 1 回のデプロイで更新され、デプロイ手順がタスクで再現できるようにする。

## 設計判断

- `backend/coffee_log/wrangler.toml` に次を設定する。
  - Worker の名前は `coffee-log` とし、`workers_dev` を有効にする。
  - `[assets]` の `directory` を Flutter のビルド成果物のディレクトリにする。
    `wrangler.toml` は `backend/coffee_log/` にあり、`directory` は設定ファイルのあるディレクトリからの相対で解決されるため、`../../frontend/build/web` とする。
  - `run_worker_first` を `["/api/*"]`、`not_found_handling` を `single-page-application` にする。
  - 除外パターンは置かない。iOS 向けの `/.well-known/*` は iOS 対応時に Static Assets から返す。
- 独自ドメインは取得しない (PRD のやらないこと)。
  workers.dev のサブドメインはアカウントに 1 つで、Relying Party ID になる (ADR-0004、ADR-0005)。
  後で独自ドメインに移す場合は全利用者が登録用トークンで再登録する。
- 状態を変更する API (POST、PUT、PATCH、DELETE) の `Origin` ヘッダを検証し、同一オリジン以外は 403 を返す。
  `Origin` が無いリクエストも 403 にする (検証できないため)。
  この検証を既存の統合テストに足すと、`Origin` を付けていない呼び出しが 403 になる。
  0003 と 0005 から 0012 の結合テストのうち状態を変更する経路の呼び出しに同一オリジンの `Origin` を付ける更新を本 issue で行う。
  これにより 0003 以降の結合テストと 0018 の管理者 Worker の検証が同じ前提で動く。
  iOS ネイティブから API を使うときは `Origin` が付かない。
  これは本 issue が提起する論点であり、iOS 対応時に署名の検証の扱いと合わせて決める (PRD の未確定論点は iOS の Cookie と Bearer トークンの扱いを挙げるのみである)。
- CORS の応答ヘッダは付けない。
  R2 の S3 互換エンドポイントへの PUT だけは別オリジンになるため、R2 バケットの CORS でアプリのオリジンからの PUT を許可する (0009 の設定にアプリのオリジンを反映する)。
- セッションは HttpOnly Cookie で渡す (0005)。同一オリジンなのでブラウザが管理する。
- ローカル開発は `mise run dev` とする。
  タスクは `flutter build web` を実行してから `wrangler dev` を起動する。
  Flutter の開発サーバーは使わず、Backend に開発時だけの許可を入れない。
- デプロイは `mise run deploy` とする。
  タスクは `flutter build web` を先に実行し、その成果物を Static Assets として `wrangler deploy` する。
  管理者 Worker のデプロイは 0018 が `deploy-admin` として追加する。
  デプロイのタスクは `mise run check` に含めない (0002 の規則)。
  スキーマ変更を含むリリースでは、デプロイの前に `mise run db-migrate-remote` を実行する。
  この手順をデプロイ手順 (README) に含め、`deploy` の `depends` にはしない (本番への適用は意識して実行する)。
- 実バックエンドと仮想認証器を使う統合テストを 1 本追加する。
  0005 の CDP の仮想認証器、0013 の `flutter drive` のハーネス、`wrangler dev` を組み合わせ、ログインから抽出の保存までを実行する。
  利用者はテストの準備としてローカルの D1 に直接投入する (登録用トークンを含む。管理者画面の経路は 0018 が別に検証する)。
- リリース後に Workers Logs の保持期間の全量で応答時間の p95 (一覧と単件は 200 ms、統計は 500 ms) を集計する手順を README に書き、デプロイ後の確認に含める。
  保持期間が 3 日から 7 日であるため、集計はリリースごとに所有者が行う。
- 初回ロードの転送量を測り、issue 本文に追記する。
  CanvasKit で問題がある場合は skwasm への変更を ADR-0007 の改訂として提案する (ADR-0007 の結果)。
- 採らない案: 別ホスト名の Pages と Workers (CORS とトークンの管理が必要になり、Relying Party ID も一致しない。ADR-0005)、Worker から Pages をプロキシする (Static Assets で足りる。ADR-0005)、開発時だけ CORS を許可する (本番と構成が変わる)。

## 完了条件

- `mise run dev` で起動した環境で、`http://localhost:8787/` の画面と `/api/*` の API が同じオリジンから配信される。
  Flutter のルーティングのパス (`/register` など) を直接開くと `index.html` が 200 で返り、画面が表示されることをテストで確認する。
- `mise run deploy` が Flutter のビルドを先に実行し、1 回のデプロイで画面と API を更新する。
  デプロイ先は `coffee-log.<アカウントのサブドメイン>.workers.dev` である。
- デプロイ手順に、スキーマ変更時の `mise run db-migrate-remote` の実行と、リリース後の p95 の集計が含まれている。
- 状態を変更する API に別オリジンの `Origin` を付けたリクエストが 403 になり、`Origin` の無いリクエストも 403 になることを自動テストで確認する。
- 0003 と 0005 から 0012 の結合テストのうち状態を変更する経路の呼び出しが、同一オリジンの `Origin` 付きで成功する。
- API の応答に CORS のヘッダが含まれない。
- R2 の CORS の設定にアプリのオリジンが含まれ、ブラウザからの写真の PUT が成功する。
- ログインから抽出の保存までの統合テストが 1 本成功する。
- 初回ロードの転送量の測定結果が issue 本文に追記され、CanvasKit のまま進めるかの判断が書かれている。
- `mise run check` が通過する。

## 関連

- 0002 が `check` と CI の枠を作る。
- 0009 が R2 の CORS を設定する。
- 0013 から 0016 が画面を作る。
- 0018 が管理者 Worker のデプロイと、デプロイ後の確認のタスクを追加する。
