# ADR-0001: Backend を Rust (workers-rs) で Cloudflare Workers 上に実装する

Created: 2026-09-21
Model: Claude Fable 5.1
Status: Accepted

## 背景

コーヒー記録アプリ (PRD: `docs/prd/coffee-log.md`) の Backend は、Cloudflare Workers 上で動かし、データベースに D1、写真の保存先に R2 を使う。
当初の案は Python + FastAPI だった。

Cloudflare Workers で Python を動かす Python Workers は、Pyodide (WebAssembly 上の CPython) で実行される。
D1 と R2 へのアクセスは JS のバインディングを Pyodide の FFI 経由で呼ぶ形になり、SQLAlchemy などの一般的な Python の DB ドライバは使えない。
この制約は、筆者 (Claude Fable 5.1) が Cloudflare のドキュメント (https://developers.cloudflare.com/workers/languages/python/) に基づいて 2026-09-21 の会話で所有者に提示した。
所有者はこれを受けて Backend の言語を Rust に変更し、依存を可能な限り少なくすることを決めた。

## 決定

- Backend は Rust で書き、workers-rs (`worker` クレート) で wasm32-unknown-unknown 向けにビルドし、Cloudflare Workers にデプロイする。
- HTTP のルーティングは `worker` クレートの Router を使い、axum などの Web フレームワークを追加しない。
- D1 は `worker` クレートの `d1` feature で、R2 は `worker` クレートの R2 バインディングで扱う。
- JSON のシリアライズは serde と serde_json を使う。
  それ以外の依存は、ADR-0003 (shiguredo_s3) と ADR-0004 (WebAuthn の検証に必要な暗号クレート) で個別に決める。
- 依存クレートを追加するときは、その理由を ADR または issue に記録する。
- Rust の版と wasm32-unknown-unknown ターゲットはリポジトリのルートの `mise.toml` で固定し、ビルド、テスト、デプロイのコマンドラインは mise のタスクに定義する (ADR-0009)。
- Backend は Cargo のワークスペースとし、利用者向けの Worker (coffee_log)、管理者 Worker (coffee_log_admin、ADR-0008)、両方で使うロジック (D1 のクエリ、トークンのハッシュ、入力検証) を持つライブラリクレート (coffee_log_core) の 3 つに分ける。
  管理者 Worker のためだけの依存は追加しない。
- Frontend のビルド成果物は利用者向けの Worker の Static Assets として配信する (ADR-0005)。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| Python Workers + FastAPI | Pyodide 上の実行と、JS バインディング経由の D1 と R2 へのアクセスが制約になる。所有者が Rust への変更を決めた |
| Rust を Cloudflare Containers で動かす | D1 は Workers のバインディングからしか実用的に触れず、Worker をプロキシにするか DB を変える必要がある。有料プランが前提になる |
| Rust を Cloudflare 以外のサーバーで動かす | D1 が使えず、当初の Cloudflare 構成から外れる |
| TypeScript (Hono) | Workers では主流だが、所有者が Rust を選んだ |
| Rust + axum (workers-rs の http feature) | 依存が増える。Router の機能で足りる |

## 結果

- Rust の標準的な非同期ランタイム (tokio) は使えない。
  非同期処理は、workers-rs が提供する JS の Promise ベースのものに限られる。
- Backend のコードが Cloudflare 固有のバインディング API に依存する。
  移植性より依存の少なさを優先する。
- ネイティブの DB ドライバや OpenSSL に依存するクレートは使えない。
  WebAuthn の検証は自前で実装する (ADR-0004)。
- ローカル開発は `wrangler dev` で行い、ローカルの D1 (SQLite) と R2 を使う (ADR-0002)。
  管理者 Worker は利用者向けの Worker と同じローカルの D1 を使い、別のポートで動かす。
- wasm に依存しない純粋なロジック (入力検証、WebAuthn の検証、SQL の組み立て) は、ネイティブターゲットで単体テスト、PBT、Fuzzing に掛ける。
  バインディングに触る部分は、`wrangler dev` に対する結合テストで検証する。
- Worker はリクエストごとに経路名と処理時間を英語のログに出し、PRD の成功指標 (API の応答時間) の測定に使う。
- 2026-09-21 の改訂で、管理者用の CLI を別のバイナリターゲットとして作る決定を取り消し、管理者 Worker (ADR-0008) に置き換えた。
  所有者が管理者用の Web 画面を作ると決めたためである。
