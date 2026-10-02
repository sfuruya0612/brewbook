# ADR-0005: Frontend と Backend を 1 つの Worker から同一オリジンで配信する

Created: 2026-09-21
Model: Claude Fable 5.1
Status: Accepted
改訂: 2026-10-02 (issue 0045。Frontend を Dioxus の Web だけにする決定 (ADR-0017) に合わせて、Flutter と iOS の記述を Web だけの対象に改めた)

## 背景

Frontend (Web の静的ファイル) と Backend (Rust の Worker) を別のホスト名にすると、CORS の設定が必要になり、Cookie を使えないためクライアントでトークンを管理することになる。
パスキーの Relying Party ID はホスト名に固定され、後から変えると登録済みのパスキーが無効になる (ADR-0004)。

初版では独自ドメインを 1 つ取り、Frontend を Cloudflare Pages で配信し、`/api/*` だけを Workers にルーティングする案を置いていた。
2026-09-21 に所有者が、独自ドメインを取得せず Cloudflare が割り当てる workers.dev のホスト名で始めることを決めた。
筆者は、Workers Routes は自分のゾーンにしか設定できず `pages.dev` のホスト名から `/api/*` を Worker に振り分けられないこと、Workers Static Assets を使えば 1 つの Worker で静的ファイルと API を同じホスト名から配信できることを説明し、所有者は Static Assets を選んだ。
この 2 点は 2026-09-21 に Cloudflare のドキュメントで確認した。
Workers Routes はアカウントの有効なゾーンと Cloudflare でプロキシした DNS レコードが前提であり、Static Assets のルーティングのドキュメントは `run_worker_first` に `["/api/*", "!/api/docs/*"]` のようなパターンの配列を与える例を示している。

## 決定

- Frontend のビルド成果物を利用者向けの Worker (ADR-0001) の Static Assets として同梱し、1 つの Worker を `coffee-log.<アカウントのサブドメイン>.workers.dev` で配信する。
- `/api/*` へのリクエストは Rust の処理に渡し、それ以外は Static Assets から返す。
  既定では一致する静的ファイルがあれば Worker より先に返されるため、`run_worker_first` を `["/api/*"]` に設定する (2026-09-21 に所有者が決定)。
  除外パターンは置かない。
- 画面のルーティングで使うパス (`/register` など) は、`not_found_handling` を `single-page-application` にし、存在しないファイルへのリクエストに `index.html` を 200 で返すことで扱う。
- Backend は CORS を許可しない。
  状態を変更するリクエストは `Origin` ヘッダを検証し、同一オリジン以外からのものを 403 で拒否する。
- セッションは HttpOnly Cookie で渡す (ADR-0004)。
- 写真のアップロード先 (R2 の S3 互換エンドポイント) だけは別オリジンになるため、R2 バケットの CORS 設定でアプリのオリジンからの PUT を許可する (ADR-0003)。
- 管理者画面は別の Worker で別のホスト名から配信する (ADR-0008)。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| 独自ドメインを取得し、Pages と Workers Routes で分ける (初版の案) | ドメインの取得と DNS の管理が増える。所有者が workers.dev で始めることを選んだ |
| Pages (`pages.dev`) と Workers (`workers.dev`) を別ホスト名にする | 別オリジンになり、CORS 設定と Bearer トークンのクライアント側での保存が必要になる。Relying Party ID も Pages 側になり、API と一致しない |
| Pages Functions で Backend を書く | Pages Functions は JavaScript と TypeScript が対象で、Rust の Worker を置けない |
| Worker から Pages のアセットをプロキシする | Worker の外に Pages のプロジェクトが増え、Static Assets で足りる |

## 結果

- Relying Party ID は `coffee-log.<アカウントのサブドメイン>.workers.dev` になる。
  後で独自ドメインに移すと登録済みのパスキーは全て無効になり、全利用者が登録用トークンで再登録する。
  所有者はこの条件を受け入れた。
- workers.dev のサブドメイン名は Cloudflare のアカウントに 1 つで、後から変えると同じく Relying Party ID が変わる。
- 1 回のデプロイで Frontend と Backend の両方が更新される。
  デプロイの mise のタスク (ADR-0009) は Frontend のビルドを先に実行し、その成果物のディレクトリを Static Assets に指定する。
- ローカル開発では、Frontend のビルド成果物を `wrangler dev` の Static Assets から配信し、本番と同じ同一オリジンにする (2026-09-21 に所有者が決定)。
  Dioxus の開発サーバー (`dx serve`) は別オリジンになり、CORS を許可しない決定と `Origin` の検証に反するため使わない。
  Backend に開発時だけの許可を含めない。
- 2026-09-21 の改訂で、独自ドメインと Cloudflare Pages を前提にした初版の決定を、workers.dev と Workers Static Assets に置き換えた。
- 2026-10-02 の改訂で、iOS 対応を要件から落として Frontend を Dioxus の Web だけにした (ADR-0017) ことに伴い、iOS 向けの設定ファイルの配信と iOS ネイティブの Cookie と Bearer トークンの記述を消した。
