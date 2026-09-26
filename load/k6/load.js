// brewbook のローカルの負荷試験 (issue 0025、ADR-0012)。
//
// `mise run dev` が配信する wrangler dev に対して、VU 50 で 1 分間の負荷をかける。
// 対象は認証が不要な経路だけである (認証はパスキーだけで、k6 からセッションを作れない)。
// 本番は対象にしない。実行は `mise run load` で行う。

import http from 'k6/http';

// 対象のオリジン。既定は `mise run dev` のオリジン。末尾のスラッシュは落とす
// (付いたままだと `//main.dart.js` のようなパスになり、404 の原因が分かりにくい)。
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

export default function () {
  // 静的アセットと、入力を持たず D1 に書き込むログインのチャレンジ発行を叩く。
  http.get(`${baseUrl}/`);
  http.get(`${baseUrl}/main.dart.js`);
  http.get(`${baseUrl}/flutter_bootstrap.js`);
  // 状態を変更する API は `Origin` を検証する (0017)。k6 は `Origin` を自動で付けないため明示する。
  http.post(`${baseUrl}/api/auth/login/begin`, null, {
    headers: { Origin: baseUrl },
  });
}
