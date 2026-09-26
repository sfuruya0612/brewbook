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

## 調査タスク

2026-09-25 に実装と自動テストを完了した (ローカルの `mise run check` は通過)。実装は 4eff521 (本体)、31d9550 (実装詳細の乖離) に記録し、その後のレビューの指摘への対応 (CI のジョブの上限を 60 分に延長、`Origin` を URL として正規化して比べる形と境界値の単体テスト、`r2_cors.rs` のコメントの修正) を加えた。

確認できた完了条件: 画面と API が同じオリジンから配信され、`/`、`/register`、`/brews/new`、`/stats` が `index.html` を 200 で返すことと `/api/*` が Worker の JSON を返すこと (`wrangler_same_origin_api.rs`)、別オリジンと `Origin` の無い変更が 403 になること (同テストと `test_origin.rs`)、既存の結合テストが同一オリジンの `Origin` 付きで成功すること、CORS のヘッダが無いこと、R2 の CORS に端末の開発オリジンと本番のオリジンが含まれること (`r2_cors.rs`)、ログインから抽出の保存までの統合テスト 1 本 (`wrangler_same_origin_e2e.rs`)、`mise run dev` の起動での同一オリジン配信 (`/register` が 200 の HTML、`/api/passkeys` が JSON)、初回ロードの転送量の測定と判断 (下の「## 初回ロードの転送量」)。

残るのは次の確認である。

- `mise run deploy` を実行し、`coffee-log.<アカウントのサブドメイン>.workers.dev` で画面と API が 1 回のデプロイで更新されることと、画面の表示、登録用リンクからのパスキーの登録、ログイン、抽出の保存、写真のアップロードを確認する。
- `mise run r2-setup` で実環境の R2 のバケットに CORS を適用し、アプリのオリジンから写真を PUT できることを確認する。
- リリース後に Workers Logs の保持期間の全量で p95 を集計し、成功指標 (一覧と単件は 200 ms、統計は 500 ms) を満たすことを確認する。
- デプロイの前に、`wrangler.toml` の `[vars]` の `RP_ID` と `ORIGIN`、`cors.json` の本番のオリジンを、実際の workers.dev のサブドメインの値へ置き換える (README のデプロイ手順 1)。

## 関連

- 0002 が `check` と CI の枠を作る。
- 0009 が R2 の CORS を設定する。
- 0013 から 0016 が画面を作る。
- 0018 が管理者 Worker のデプロイと、デプロイ後の確認のタスクを追加する。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. `wrangler.toml` に `[observability]` を有効にした (README の p95 の集計の前提。応答ログを出す既存の仕組みは変えていない)。
2. `Origin` の比較は、`[vars]` の `ORIGIN` ではなくリクエスト自身の URL のオリジンに対して行う (リクエストを受け取ったオリジンが正であり、`vars` の設定漏れで検証が緩まないため)。`ORIGIN` は WebAuthn の期待オリジンとして 0005 から使う。
3. テスト専用ページのパスを `/__test_page` から `/api/__test_page` に変えた (`/api/*` 以外のパスは Static Assets と SPA の `index.html` が返すため、Worker が処理するページは `/api/` の下に置く必要がある)。
4. `frontend/lib/main.dart` に `usePathUrlStrategy()` を入れた (パス形式の URL にして、SPA のパスを直接開けるようにするため。`flutter_web_plugins` を追加)。
5. テストの `ApiClient` は、状態を変更するメソッドに同一オリジンの `Origin` を既定で付ける (0003 と 0005 から 0012 の結合テストの呼び出しを書き換えずに同じ前提に揃えるため。別オリジンと `Origin` の無い呼び出しの検査は `with_origin` と `without_origin` で行う)。
6. テスト用の Static Assets の配信は、一時の wrangler の設定 (`--config`) で行う (`wrangler dev --assets` は `run_worker_first` と `not_found_handling` を引き継がず、本番と同じルーティングにならないため)。
7. テストコード入りの Web ビルドは `frontend:build-e2e` が `build/e2e-web` に作り、配布用の `build/web` と混ぜない (配布用のビルドを配信するとテストが時間切れになるまで気付けないため、Rust のハーネスがビルドにテスト固有の文字列があることを確かめる)。
8. `frontend:test-same-origin` を `mise run check` に含めた (既存の `frontend:test-integration` と同じ扱い)。
9. `backend:test-integration` に `frontend:build` の依存を足した (`wrangler.toml` の `[assets]` のディレクトリが無いと `wrangler dev` が起動しないため)。

## 初回ロードの転送量

`mise run frontend:build` の配布用のビルドを `mise run dev` の `wrangler dev` から配信し、`curl --compressed` で受け取った転送量を測った (2026-09-25。`wrangler dev` は `Accept-Encoding: br` に `Content-Encoding: br` を返すことを確認した)。

| 資産 | 転送量 |
| --- | --- |
| `index.html` | 780 バイト |
| `flutter_bootstrap.js` | 5.2 KB |
| `main.dart.js` | 1.05 MB |
| `canvaskit/canvaskit.js` | 27.7 KB |
| `canvaskit/canvaskit.wasm` | 2.94 MB |
| `assets/fonts/MaterialIcons-Regular.otf` (使うアイコンだけに削減済み) | 4.2 KB |
| `manifest.json`、`version.json`、`flutter_service_worker.js`、フォントのマニフェスト | 1.3 KB |
| 合計 (同一オリジン) | 約 4.03 MB |

CanvasKit は日本語の表示に必要な Noto Sans JP のサブセットを `fonts.gstatic.com` から追加で取得する (1 サブセットあたり約 43 KB。表示する文字により複数)。

判断: 初回の 1 回だけで、以後は Service Worker が同じ資産をキャッシュするため、CanvasKit のまま進める (skwasm への変更はしない)。

## pending にした理由

2026-09-25 に実装と自動テストを完了し、コミット 4eff521 と 31d9550 に記録した (その後のレビューの指摘への対応も反映済み)。完了条件のうち、画面と API の同一オリジン配信、`Origin` の検証、CORS のヘッダが無いこと、R2 の CORS の設定の検査、ログインから抽出の保存までの統合テスト、初回ロードの転送量の測定はローカルで確認済みである。

残るのは、Cloudflare のアカウントの資格情報が要る次の 3 つで、この環境では実行できないため pending にした。

- `mise run deploy` の実行と workers.dev の URL での動作
- 実環境の R2 のバケットへの CORS の適用と、ブラウザからの写真の PUT
- リリース後の p95 の集計

再開の条件: Cloudflare のアカウントの資格情報が使える環境で、所有者がデプロイ手順 (README) に従って `mise run deploy`、`mise run r2-setup`、p95 の集計を実行し、結果を本 issue に記録する。`CHANGES.md` の `[ADD]` エントリはこの pending への移動のコミットに含める (close のときに重ねて追記しない)。
