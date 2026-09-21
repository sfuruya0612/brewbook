# アカウントと全データの削除を実装する

Created: 2026-09-21
Model: deepseek-v4p1-flash
対応 ADR: ADR-0006 (docs/adr/0006-data-model-and-archive.md)、ADR-0002 (docs/adr/0002-database-cloudflare-d1.md)、ADR-0003 (docs/adr/0003-photo-storage-r2-presigned-upload.md)
関連 PRD: FR-15、セキュリティ (セッションの失効)
依存: 0001, 0002, 0003, 0005, 0009

## 背景

ADR-0006 は、アカウント削除 (PRD の FR-15) だけを物理削除とし、利用者に属する全行と R2 の全オブジェクトを削除すると決めた。
削除は外部キー制約に従って子から順に行い、D1 の batch で 1 つのトランザクションにする (ADR-0002)。
webauthn_challenges のログイン用の行は `user_id` を持たないため対象外とし、有効期限で失効させる (ADR-0006)。
ADR-0003 は、R2 の `users/<利用者 ID>/` と `pending/<利用者 ID>/` の両方を削除すると決めた。
0009 で R2 の操作ができた。本 issue が削除を追加する。

## 目的

利用者が、自分のアカウントと全データ (写真を含む) を削除できるようにする。

## 設計判断

- 経路は `DELETE /api/account` とし、セッションを必須にする。
- users、registration_tokens、passkey_credentials、webauthn_challenges、sessions、shops、products、flavor_tags、product_flavor_tags、purchases、brews のうち、利用者に属する各行 (`user_id` がその利用者の行) を物理削除する。
  削除の順は、product_flavor_tags、brews、purchases、flavor_tags、products、shops、sessions、passkey_credentials、webauthn_challenges、registration_tokens、users とする (外部キーの参照元から先に消す)。
  D1 の batch で 1 つのトランザクションにする (ADR-0002)。
  webauthn_challenges のログイン用の行は `user_id` を持たないため対象外とする (ADR-0006)。
- R2 のオブジェクトは `users/<利用者 ID>/` と `pending/<利用者 ID>/` の両方を削除する。
  一覧は 1 回で返る件数に上限があるため、カーソルを繰り返して全件を削除する (想定規模では 1 利用者あたりの購入が 3,000 件まであり得る)。
- 応答は 204 とし、Cookie を失効させる。
  削除後に同じ Cookie で認証が必要な API を呼ぶと 401 になる (セッションの行が無いため)。
- 削除は元に戻せないため、確認はクライアントが確認ダイアログで行う (0016 がテストする)。
- 削除は、R2 のオブジェクトの削除を先に行い、成功したら D1 の batch を実行する。
  R2 の削除が失敗した場合はセッションが有効なうちに利用者が再試行でき、R2 の削除は同じキーに対して繰り返しても成功する。
  D1 の削除は batch で 1 トランザクションなので、途中の状態は残らない。
  採らない案: 非同期の削除ジョブにする (利用者が 20 人以下で 1 回の batch に収まる。ADR-0002 の batch の範囲で足りる)、論理削除にする (PRD の FR-15 は全データの削除を求める)、D1 の削除を先に行う (R2 の削除に失敗すると認証が失われて再試行できない)。

## 完了条件

- FR-15 の受け入れ基準を満たす。
  利用者に属する全行が物理削除され、R2 の `users/<利用者 ID>/` と `pending/<利用者 ID>/` のオブジェクトが削除され、削除後の API 呼び出しが 401 になることを自動テストで確認する。
  ログイン用のチャレンジの行 (user_id が NULL) が削除の対象外であることも確認する。
- 想定件数 (1 利用者あたり購入 3,000 件、写真も最大 3,000 件) の R2 のオブジェクトをカーソルで全件削除できることを自動テストで確認する。
- 経路の台帳 (0001) に本 issue の経路を追加し、正常系と未認証 401 のテストが揃っている。
- `mise run check` が通過する。

## 関連

- 0005 がセッションを作る。
- 0009 が R2 の操作を作る。
- 0016 が削除の確認ダイアログを作る。
