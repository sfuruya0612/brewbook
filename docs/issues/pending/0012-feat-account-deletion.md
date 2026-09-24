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

## 調査タスク

2026-09-23 に実装と自動テストを完了した (ローカルの `mise run check` は通過)。残るのは次の確認である。

- 本番の Workers のサブリクエスト上限 (有料 1,000、無料 50) の対象に R2 バインディングの操作が含まれるかを確認する。含まれる場合、購入 3,000 件の利用者の削除は `users/` と `pending/` の合計 6,000 オブジェクトで上限を超えるため、削除の分割 (複数リクエスト、Queue など) か上限の許容を決める。
- 決定に応じて実装を直し、本番相当の規模で削除が成功することを確認する。
- R2 の削除の実サービスでの動作 (資格情報が要る) と、`mise run deploy` 後の確認は行っていない。

## 関連

- 0005 がセッションを作る。
- 0009 が R2 の操作を作る。
- 0016 が削除の確認ダイアログを作る。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. `backend/coffee_log/src/account.rs` に削除のハンドラを追加し、R2 の削除は `backend/coffee_log/src/records/photos.rs` の `delete_user_objects` (`users/<利用者 ID>/` と `pending/<利用者 ID>/` をカーソルで全件) に置いた (0009 の写真の操作と共有するため)。
2. `backend/coffee_log/src/r2_check.rs` にテスト専用の経路を追加した (結合テストが R2 のオブジェクトの有無と、他の利用者のオブジェクトが残ることを確かめるため。`TEST_R2_CHECK` の var があるときだけ応答する)。
3. `backend/coffee_log_core/src/query.rs` に `account_delete` の 11 文を追加し、`db::execute_batch` (D1 の batch、1 トランザクション) で実行する。並びは外部キーの参照元から先に消す順とした。
4. `mise.toml` の `backend:test-integration` に新しい結合テストを追加した。
5. 想定規模の R2 の削除テストは、購入 3,000 行と `users/` 3,000 件 + `pending/` 3,000 件を用意し、削除前に一覧が 1 回で全件返らないこと (ページ数 3 以上) を確認してから、全件削除とページ境界のキーの消滅を確かめる。ローカル `wrangler dev` で 8,159 ms だった。

## pending にした理由

2026-09-23 に実装と自動テストを完了し、コミット 5805484 に記録した。完了条件はローカルで満たしているが、本番の Workers のサブリクエスト上限 (有料 1,000、無料 50) の対象に R2 バインディングの操作が含まれるかをこの環境では確認できず、含まれる場合は想定規模 (購入 3,000 件、`users/` と `pending/` の合計 6,000 オブジェクト) で上限を超えるため、所有者の判断 (削除の分割、上限の許容、設計の見直し) を要する。所有者の指示により close せず pending に置く。`CHANGES.md` の `[ADD]` エントリは実装のコミットに含める。
