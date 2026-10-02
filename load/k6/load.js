// brewbook のローカルの負荷試験 (issue 0025、ADR-0012)。
//
// `mise run dev` が配信する wrangler dev に対して、VU 50 で 1 分間の負荷をかける。
// 対象は認証が不要な経路だけである (認証はパスキーだけで、k6 からセッションを作れない)。
// 本番は対象にしない。実行は `mise run load` で行う。

import http from 'k6/http';

// 対象のオリジン。既定は `mise run dev` のオリジン。末尾のスラッシュは落とす
// (付いたままだと `//assets/...` のようなパスになり、404 の原因が分かりにくい)。
const baseUrl = (__ENV.BASE_URL || 'http://localhost:8787').replace(/\/+$/, '');

export const options = {
  scenarios: {
    load: {
      executor: 'constant-vus',
      vus: 50,
      duration: '1m',
    },
  },
  // ローカルの目安 (ADR-0012)。本番の p95 の判定は Workers Logs の集計で行う。
  thresholds: {
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<500'],
  },
};

// 静的アセットのファイル名には dx がハッシュを付けるため、ビルドのたびに変わる。
// 負荷をかける前に index.html と JavaScript のローダーを 1 回だけ読み、実際のパスを解決する
// (setup のリクエストはシナリオの計測に含まれない)。
export function setup() {
  const index = http.get(`${baseUrl}/`);
  if (index.status !== 200 || !index.body) {
    throw new Error(`failed to load ${baseUrl}/: status ${index.status}`);
  }
  const loader = matchOrFail(
    index.body,
    /\/assets\/[A-Za-z0-9_.-]+\.js/,
    'the JavaScript loader in index.html',
  );
  const script = http.get(`${baseUrl}${loader}`);
  if (script.status !== 200 || !script.body) {
    throw new Error(`failed to load ${baseUrl}${loader}: status ${script.status}`);
  }
  const wasm = matchOrFail(
    script.body,
    /\/assets\/[A-Za-z0-9_.-]+\.wasm/,
    'the WebAssembly module in the JavaScript loader',
  );
  return { loader, wasm };
}

// 正規表現に一致した最初の文字列を返す。見つからなければ失敗させる (設定のずれを黙って通さない)。
function matchOrFail(body, pattern, what) {
  const match = body.match(pattern);
  if (!match) {
    throw new Error(`failed to find ${what}`);
  }
  return match[0];
}

export default function (data) {
  // 静的アセット (index.html、JavaScript のローダー、WebAssembly) と、入力を持たず
  // D1 に書き込むログインのチャレンジ発行を叩く。
  http.get(`${baseUrl}/`);
  http.get(`${baseUrl}${data.loader}`);
  http.get(`${baseUrl}${data.wasm}`);
  // 状態を変更する API は `Origin` を検証する (0017)。k6 は `Origin` を自動で付けないため明示する。
  http.post(`${baseUrl}/api/auth/login/begin`, null, {
    headers: { Origin: baseUrl },
  });
}
