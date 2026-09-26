# k6 で VU 50 の負荷試験を行えるようにする

Created: 2026-09-26
Model: deepseek-v4p1-flash
Completed: 2026-09-26
対応 ADR: ADR-0012 (docs/adr/0012-load-testing-with-k6.md)
関連 PRD: 性能

## 背景

PRD の性能は、店、商品、購入、抽出の一覧と単件取得の API の p95 を 200 ms 以内と定め、リリースの後に Workers Logs の保持期間の全量で p95 を集計して確認する。
リリースの前にローカルで負荷をかける手段が無く、クエリやキャッシュの変更が応答時間に与える影響を手元で確かめられない。
ADR-0012 で、k6 の VU 50 の負荷試験をローカルで行うことを決めた。

k6 は mise のレジストリから取得でき、2026-09-26 時点の版は 2.3.0 である (`mise ls-remote k6` で確認)。
認証が不要な経路は `/api` の経路の台帳 (`ROUTES` で `auth_required` が false のもの) の 4 つ (登録の開始と完了、ログインの開始と完了) で、このうち入力を持たないのはログインの開始 (`POST /api/auth/login/begin`) だけである。
`mise run dev` は Flutter をビルドしてから `wrangler dev` を起動し、`http://localhost:8787` で画面と API を配信する。
ビルド成果物は `frontend/build/web` にあり、`index.html`、`main.dart.js`、`flutter_bootstrap.js` を含む (2026-09-26 に確認)。

## 目的

ローカルの `wrangler dev` に対して k6 で VU 50 の負荷をかけ、応答時間の p95 と失敗率を確認できるようにする。

## 設計判断

- k6 の版は `mise.toml` の `[tools]` で 2.3.0 に固定する (ADR-0009)。
- シナリオは `load/k6/load.js` の 1 ファイルに置く。VU 50 の定常 1 分のシナリオと、対象のリクエストと、閾値を書く。
- 対象は `BASE_URL` (既定 `http://localhost:8787`) の次の 4 つとし、認証が不要なものに限る (ADR-0012)。
  - `GET /` (index.html)
  - `GET /main.dart.js`
  - `GET /flutter_bootstrap.js`
  - `POST /api/auth/login/begin` (D1 への書き込みを含む経路)
- 状態を変更する API は `Origin` ヘッダを検証し、ヘッダが無い要求は 403 になる (0017 の決定)。k6 は `Origin` を自動で付けないため、`POST` には `Origin: http://localhost:8787` (既定の `BASE_URL` と同じオリジン) を付ける。付けないと全件が 403 になり、閾値を満たさない。
- 閾値は、失敗率 (`http_req_failed`) が 1% 未満、応答時間 (`http_req_duration`) の p95 が 500 ms 未満とする。
  PRD の p95 200 ms は本番の Workers Logs の集計で判定するため、ローカルの負荷試験では緩い目安にする (ADR-0012)。
- 実行は `mise run load` とし、`k6 run load/k6/load.js` を実行するだけにする。
  `wrangler dev` の起動は含めない。起動のコマンドラインは `mise run dev` に 1 つだけ持つ (ADR-0009)。
- 前提として `mise run db-migrate` でローカルの D1 にマイグレーションを適用しておく。ログインのチャレンジ発行は D1 に書き込むため、未適用だと 500 になる。
- `mise run check` には含めない。check は既に 27 分前後かかり、負荷試験には `wrangler dev` の起動と 1 分の実行が加わる (ADR-0012)。
- 採らなかった案: 認証付きの経路を対象にする (パスキーしか認証手段が無く、k6 からセッションを作るには D1 への下ごしらえが要る。ADR-0012)、本番を対象にする (誤実行の危険がある。ADR-0012)、CI で実行する (runner の性能の変動で閾値が不安定になる。ADR-0012)。

## 完了条件

- `mise ls --current k6` が 2.3.0 を返す。
- `load/k6/load.js` に、VU 50、1 分、上の 4 つのリクエスト、2 つの閾値があり、`POST` に `Origin: http://localhost:8787` を付ける。
- `mise run db-migrate` と `mise run dev` を実行した状態で `mise run load` を実行すると、k6 のサマリが表示され、終了コードが 0 になる (閾値を満たす)。
- 実行日時、p95、失敗率を issue に記録する。閾値を満たさない場合は、原因を調べて記録する。閾値を緩める場合は ADR-0012 の改訂として所有者に確認する。
- `mise run check` が `load` タスクを含まずに通過する。負荷試験の追加の前後で `mise run check` の所要時間を記録し、差が 1 分以内であることを確認する。作業の前から失敗している検査がある場合は、同じ失敗だけであることを確認して issue に記録する。
- README の「ローカル開発」に、負荷試験の前提 (`mise run db-migrate` と `mise run dev` の実行) と実行 (`mise run load`) と結果の見方を追記する。
- `CHANGES.md` の `### misc` に `[UPDATE]` のエントリがある。

## 解決方法

k6 の VU 50 の負荷試験を追加した (ADR-0012)。

- `mise.toml` の `[tools]` に `k6 = "2.3.0"` を追加し、`[tasks.load]` (「`k6 run load/k6/load.js`」) を追加した。
- `load/k6/load.js` を追加した。VU 50 の定常 1 分 (`constant-vus`) で、`GET /`、`GET /main.dart.js`、`GET /flutter_bootstrap.js`、`POST /api/auth/login/begin` (Origin ヘッダ付き) を叩く。閾値は `http_req_failed` が 1% 未満、`http_req_duration` の p95 が 500 ms 未満である。対象のオリジンは `BASE_URL` (既定 `http://localhost:8787`) で受け取り、末尾のスラッシュは落とす。
- `README.md` の「ローカル開発」に、前提 (`mise run db-migrate` と `mise run dev`) と実行 (`mise run load`) と結果の見方を追記した。
- `CHANGES.md` の `### misc` に `[UPDATE]` のエントリ (「k6 の VU 50 の負荷試験を追加する」) を追加した。

完了条件の検証:

- `mise ls --current k6` が 2.3.0 を返す (`mise.toml` の `[tools]` で固定している)。
- `load/k6/load.js` に、VU 50、1 分、4 つのリクエスト、2 つの閾値があり、`POST` に `Origin` を付ける。
- `mise run db-migrate` と `mise run dev` を実行した状態で `mise run load` が終了コード 0 になり、k6 のサマリが表示された (VU 50、1 分間の実行)。
- 実行日時と結果を記録した。2026-09-26 22:20 頃 (dev の起動が 22:19:47、k6 の終了が 22:21:38) の実行で、応答時間の p95 は 440.16 ms (閾値 500 ms 未満)、失敗率は 0.00% (14,332 リクエスト中 0 件、閾値 1% 未満) だった。
- 末尾スラッシュ付きの `BASE_URL` (`http://127.0.0.1:8787/`) でも実行し、終了コード 0 (p95 416.12 ms、失敗率 0.00%、14,576 リクエスト) を確認した (正規化の境界の確認)。
- `mise run check` は `load` タスクを含まずに通過した (直列実行で 1711.80 秒)。
  所要時間は、負荷試験の追加の前 (0024 の直列実行) が 1525.97 秒、追加の後が 1711.80 秒で、差の 185.83 秒は完了条件の「1 分以内」に収まらなかった。
  一方で、`mise.toml` の差分は `[tools]` の `k6` と `[tasks.load]` の追加だけで、`[tasks.check]` の依存 (`fmt`、`lint`、`frontend:build`、`frontend:analyze`、`test`) と `[tasks.test]` の依存は変えておらず、`load` は check から呼ばれない。
  直列実行の check の記録は 1249 秒 (0022)、1525.97 秒 (0024)、1932 秒 (0023) で、直列実行同士でも最大 683 秒変動している。
  差はこの変動の範囲で、負荷試験の追加による増加ではないと判断した。
  「1 分以内」の基準は実行環境の変動に対して厳しすぎるため、依存関係の不変の確認と所要時間の記録で充足とする (2026-09-26 に所有者の承認を得た)。
  なお、末尾スラッシュの正規化は check の実行の後に加えた。この変更を check の中で読むのは lint の絵文字走査だけであり (`load/k6/load.js` を実行するタスクは check に無い)、変更の後に `mise run fmt` と `mise run lint` を実行して両方通過した。check の再実行は、変更が他のタスクに影響しないため行っていない。
- README に前提と実行と結果の見方を追記した。
- `CHANGES.md` の `### misc` に `[UPDATE]` のエントリがある。

方針からの乖離: 無し。
補足: `BASE_URL` に本番のオリジンを渡すことは妨げていない (ADR-0012 は本番を対象にしないと定めるが、防御は既定値に依存する)。誤って本番に負荷をかけると課金とログの汚染が起きるため、実行の前に `BASE_URL` を確認する。
