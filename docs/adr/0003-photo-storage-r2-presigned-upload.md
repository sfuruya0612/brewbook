# ADR-0003: 写真を Cloudflare R2 に保存し、署名付き URL で直接アップロードする

Created: 2026-09-21
Model: Claude Fable 5.1
Status: Accepted

## 背景

購入には 1 枚のパッケージ写真を添付する (PRD の FR-10)。
写真はクライアントで JPEG に変換して長辺 2048 px 以下に縮小し、サイズの上限を 5 MB とする。
本 ADR と PRD では、5 MB を 5,000,000 バイトとして扱う。
所有者は、R2 のクライアントに shiguredo/s3-rs (`shiguredo_s3` クレート) を使うことと、Content-Type とサイズを署名付き URL の条件で制限することを求めた。

`shiguredo_s3` は Sans I/O 設計の S3 クライアントで、リクエストの組み立てと署名 (SigV4) だけを行い、HTTP 通信は呼び出し側が持つ。
この説明は 2026-09-21 時点の GitHub リポジトリ (https://github.com/shiguredo/s3-rs) の README に基づき、同日に筆者がクレート 2026.1.0-canary.7 のソース (`PresignedRequest` と `S3Request` を返し、HTTP クライアントを持たない) で確認した。
使うクレートの版は `=2026.1.0-canary.7` で固定する (「## 結果」)。
R2 は S3 互換 API を提供し、`shiguredo_s3` は R2 をカスタムエンドポイントとパス形式のアクセスで明示的にサポートしている。
一方、workers-rs の R2 バインディングは Worker 内からオブジェクトを操作できるが、署名付き URL は生成できない。

## 決定

- 写真は R2 の 1 つのバケットに保存する。
  紐づけ済みのオブジェクトキーは `users/<利用者 ID>/purchases/<購入 ID>/<UUID>.jpg`、紐づけ前のキーは `pending/<利用者 ID>/<UUID>.jpg` とする。
- クライアントは変換後のサイズを申告してアップロード用 URL を要求する。
  Backend は申告サイズが 5 MB を超えていれば 400 を返す。
- Backend は `shiguredo_s3` で署名付き PUT URL を生成し、クライアントが直接アップロードする。
  署名の対象ヘッダに Content-Type (`image/jpeg`) と Content-Length (申告サイズ) を含め、有効期限は 5 分とする。
- アップロード完了後、クライアントが Backend に通知する。
  Backend は R2 バインディングで `pending/` のオブジェクトの存在、サイズ (申告サイズと一致し 5 MB 以下)、Content-Type を確認し、オブジェクトを取得して紐づけ済みのキーに保存し、`pending/` のオブジェクトを削除し、購入に紐づける。
  R2 バインディングの操作は head、get、put、delete、list、createMultipartUpload、resumeMultipartUpload の 7 つで複製と移動の操作は無い (2026-09-21 に Workers R2 API リファレンスで確認) ため、取得と保存の 2 操作で移す。
  写真は 5 MB 以下なので Worker のメモリに収まる。
  条件を満たさないオブジェクトは削除して 400 を返す。
- `pending/` プレフィックスには R2 のライフサイクルルールを設定し、1 日後に削除する。
  完了通知が来なかったオブジェクトはこれで消える。
- 写真の取得は Backend の `/api/purchases/<購入 ID>/photo` が R2 バインディングで読み出して返す。
  認証と利用者の分離は Backend が行い、R2 のバケットは公開しない。
- 写真の差し替えと削除、アカウント削除に伴う一括削除 (`users/<利用者 ID>/` と `pending/<利用者 ID>/` の両方) は R2 バインディングで行う。
- `shiguredo_s3` の用途は署名付き PUT URL の生成に限定する。
  Worker 内からの R2 操作にはバインディングを使う。
- 署名に必要な R2 の API トークン (アクセスキーとシークレット) は Workers の Secret に置く。
- R2 バケットの CORS 設定で、アプリのオリジンからの PUT だけを許可する。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| Worker 経由でアップロード (バインディングで put) | Worker のリクエスト本文に写真が乗り、Worker の CPU 時間と帯域を使う。所有者が `shiguredo_s3` の採用を求めた |
| R2 の全操作を `shiguredo_s3` + `worker::Fetch` で行う | バインディングより経路が長く、全操作で API トークンを使うことになる。`pending/` からの移動も取得と保存で足りるため、S3 API に置き換える理由が無い |
| 取得も署名付き GET URL にする | URL を知る者が認証なしで取得できるため、PRD の「写真の取得は認証済みの利用者だけ」と両立しない。写真は 5 MB 以下で件数も少ないため Worker 経由で返す |
| サイズの上限をアップロード完了後の確認だけで強制する | 所有者が署名付き URL の条件での制限を求めた。Content-Length を署名対象に含める方法が R2 で検証できない場合の代替案として残す |
| S3 の POST Policy (content-length-range) | R2 は S3 の POST Object をサポートしていない |
| サーバー側で画像の変換と縮小 | wasm 上の画像処理は CPU 時間と依存が増える。所有者がクライアント側の変換を選んだ |

## 結果

- crates.io の `shiguredo_s3` には安定版が無く、2026-09-21 時点の最新は 2026.1.0-canary.7 (MSRV 1.93) で、全ての版がプレリリースである。
  所有者は 2026-09-21 にプレリリース版を版固定で採用することを決め、使う版は `=2026.1.0-canary.7` とする。
  Rust の版は mise で固定し (ADR-0009)、MSRV の 1.93 以上を選ぶ。
- `shiguredo_s3` 2026.1.0-canary.7 が wasm32-unknown-unknown でビルドできることを、2026-09-21 に所有者が mise で入れた Rust 1.98.1 で筆者が確認した。
  依存は base64ct、crc-fast、hmac、md-5、sha1、sha2、xml と digest 系の純 Rust のクレートだけで、OpenSSL や ring を含まない。
  既定の feature (`rust-crypto`) をそのまま使い、`aws_lc_rs` feature は有効にしない。
  ビルドできない場合の代替案 (署名付き URL の生成だけを sha2 と hmac で自前実装する) は所有者が 2026-09-21 に承認済みだが、確認が取れたため使わない。
  版を上げてビルドできなくなった場合に限り、この代替案に切り替えて本 ADR を改訂する。
- R2 のドキュメントは、Content-Type を署名対象に含めると異なる Content-Type の PUT を 403 (SignatureDoesNotMatch) で拒否すると明記している (2026-09-21 に確認)。
  `shiguredo_s3` の `put_object` のビルダーに `content_type` と `content_length` を与えて `presigned` を呼ぶと、`X-Amz-SignedHeaders=content-length;content-type;host` の PUT 用 URL が得られることを、2026-09-21 に筆者が手元の実行で確認した。
  Content-Length については R2 のドキュメントが触れていないため、Content-Length を署名対象に含めた URL に申告と異なるサイズの PUT を送ると R2 が 403 で拒否することを、実装の最初に検証する。
  検証できない場合は、サイズの上限を完了後の確認だけで強制する案を本 ADR の改訂として所有者に提案する (PRD の未確定論点)。
- サイズと Content-Type は、署名の条件と完了後の確認の両方で強制する。
  どちらか一方が働かなくても上限を超えたオブジェクトは紐づかない。
- クライアントは、アップロード用 URL の要求、PUT、完了通知の 3 回の呼び出しを行う。
- エクスポート (PRD の FR-14) には写真の実体を含めず、`photo_key` と写真取得 API のパスだけを含める。
  署名付き GET URL は使わない。
