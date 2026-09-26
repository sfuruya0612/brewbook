//! 購入の写真の API の結合テスト (HTTP)。
//!
//! アップロード用 URL の発行、アップロード完了の通知、取得、削除を検査する。
//! 写真の実体は `wrangler dev` のローカルの R2 に `wrangler r2 object` で置き、
//! Worker がバインディングで `pending/` の確認、移動、削除を行うことを確かめる。
//! 署名付き URL は R2 の実サービスに送らない (資格情報とネットワークが要るため)。
//! 代わりに、署名の条件 (署名対象ヘッダと有効期限) を URL のクエリで検査する。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。
//! テストは並行に走るため、記録を書き換えるテストは利用者ごとに分ける。

mod support;

use std::sync::OnceLock;

use reqwest::blocking::Response;
use serde_json::{json, Value};
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, Seed, SeededPurchase};
use support::ServerLease;

/// テストが注入する R2 の設定 (vars と Secret)。実値はリポジトリに含めない (ADR-0003)。
const R2_ENDPOINT: &str = "https://test-account.r2.cloudflarestorage.com";
const ACCESS_KEY_ID: &str = "test-access-key-id";
const SECRET_ACCESS_KEY: &str = "test-secret-access-key";
/// 署名付き URL の有効期限 (秒)。既定値 (300) と違う値を注入し、vars が読まれることを確かめる。
const URL_EXPIRES_SECONDS: &str = "180";
/// バケット名。`wrangler.toml` のバインディングと同じにする (既定値)。
const BUCKET: &str = "brewbook-photos";
/// 写真の Content-Type。
const CONTENT_TYPE: &str = "image/jpeg";

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const CREATED: &str = "2026-09-01T00:00:00.000Z";
const FUTURE: &str = "2099-01-01T00:00:00.000Z";
/// 下ごしらえに使う購入日。
const D21: &str = "2026-09-21";

/// 紐づけ前のキーの UUID。テストごとに別の値を使い、オブジェクトの置き場を分ける。
const PENDING_UUID: &str = "00000000-0000-4000-8000-000000000101";
const REPLACED_UUID: &str = "00000000-0000-4000-8000-000000000102";
const OTHER_PENDING_UUID: &str = "00000000-0000-4000-8000-000000000103";

/// 写真の実体に使うバイト列。JPEG の先頭の並びを含める (中身は API が検査しない)。
const PHOTO_BYTES: &[u8] = b"\xff\xd8\xff\xe0photo-complete";
const REPLACED_BYTES: &[u8] = b"\xff\xd8\xff\xe0photo-replaced";
const OTHER_PENDING_BYTES: &[u8] = b"\xff\xd8\xff\xe0photo-other";

/// 下ごしらえした利用者と、その利用者の購入。
struct SeededUser {
    user_id: String,
    session: String,
    purchase: SeededPurchase,
}

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// アップロード用 URL の発行のテストの利用者。
    upload: SeededUser,
    /// 完了通知のテストの利用者。
    complete: SeededUser,
    /// 完了通知の確認に失敗するテストの利用者。
    mismatch: SeededUser,
    /// 他の利用者の `pending/` のキーを渡すテストの利用者。
    steal: SeededUser,
    /// 取得のテストの利用者。
    get: SeededUser,
    /// 写真が無い購入の取得のテストの利用者。
    missing: SeededUser,
    /// 削除のテストの利用者。
    delete: SeededUser,
    /// 差し替えのテストの利用者。
    replace: SeededUser,
    /// 他の利用者 (404 の検査に使う)。
    other: SeededUser,
}

/// 下ごしらえを 1 回だけ組み立てる。
fn data() -> &'static TestData {
    static DATA: OnceLock<TestData> = OnceLock::new();
    DATA.get_or_init(build_data)
}

/// 共有のサーバーを借りる。R2 の設定を vars と Secret として注入する。
fn server() -> ServerLease {
    support::shared_server("main", || {
        support::DevServer::start_with(
            |_| {
                [
                    ("R2_ENDPOINT", R2_ENDPOINT),
                    ("R2_ACCESS_KEY_ID", ACCESS_KEY_ID),
                    ("R2_SECRET_ACCESS_KEY", SECRET_ACCESS_KEY),
                    ("PHOTO_URL_EXPIRES_SECONDS", URL_EXPIRES_SECONDS),
                ]
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect()
            },
            &data().seed_sql,
        )
    })
    .expect("wrangler dev must start")
}

/// 下ごしらえの SQL と、テストが使う値を作る。
fn build_data() -> TestData {
    let mut seed = Seed::new();
    let upload = seeded_user(&mut seed, 1);
    let complete = seeded_user(&mut seed, 2);
    let mismatch = seeded_user(&mut seed, 3);
    let steal = seeded_user(&mut seed, 4);
    let get = seeded_user(&mut seed, 5);
    let missing = seeded_user(&mut seed, 6);
    let delete = seeded_user(&mut seed, 7);
    let replace = seeded_user(&mut seed, 8);
    let other = seeded_user(&mut seed, 9);
    TestData {
        seed_sql: seed.sql(),
        upload,
        complete,
        mismatch,
        steal,
        get,
        missing,
        delete,
        replace,
        other,
    }
}

/// 購入を 1 件持つ利用者を下ごしらえする。テストごとに別の利用者にして、並行に書き換えても衝突しないようにする。
fn seeded_user(seed: &mut Seed, index: u32) -> SeededUser {
    let user_id = user_id(index);
    seed.user(&user_id, "写真の利用者", CREATED);
    let session = seed.session(&user_id, FUTURE, CREATED);
    let product = seed.product(&user_id, "写真の豆", CREATED, CREATED, None);
    let purchase = seed.purchase(&user_id, &product.id, None, D21, CREATED, CREATED, None);
    SeededUser {
        user_id,
        session,
        purchase,
    }
}

/// セッションを持たないクライアント。
fn anonymous(base_url: &str) -> ApiClient {
    ApiClient::new(base_url, None)
}

/// 応答の状態コードと本体を確かめる。
fn assert_status(response: Response, expected: u16) -> Value {
    let (status, body) = read(response);
    assert_eq!(status, expected, "the response body was {body}");
    body
}

/// 未認証の呼び出しが 401 になることを確かめる。
fn assert_unauthorized(response: Response) {
    let body = assert_status(response, 401);
    assert_eq!(error_code(&body), Some("unauthorized"), "{body}");
}

/// 入力不正の呼び出しが 400 になることを確かめる。
fn assert_bad_request(response: Response) {
    let body = assert_status(response, 400);
    assert_eq!(error_code(&body), Some("bad_request"), "{body}");
}

/// 対象が無い呼び出しが 404 になることを確かめる。
fn assert_not_found(response: Response) {
    let body = assert_status(response, 404);
    assert_eq!(error_code(&body), Some("not_found"), "{body}");
}

/// 紐づけ前のキー (`pending/<利用者 ID>/<UUID>.jpg`)。テストは組み立てを独立に書く (FR-10)。
fn pending_key(user_id: &str, uuid: &str) -> String {
    format!("pending/{user_id}/{uuid}.jpg")
}

/// 紐づけ後のキー (`users/<利用者 ID>/purchases/<購入 ID>/<UUID>.jpg`)。
fn users_key(user_id: &str, purchase_id: &str, uuid: &str) -> String {
    format!("users/{user_id}/purchases/{purchase_id}/{uuid}.jpg")
}

/// アップロード用 URL の経路。
fn upload_url_path(purchase_id: &str) -> String {
    format!("/api/purchases/{purchase_id}/photo/upload-url")
}

/// 完了通知と取得と削除の経路。
fn photo_path(purchase_id: &str) -> String {
    format!("/api/purchases/{purchase_id}/photo")
}

/// 署名付き URL をパスとクエリの組に分ける。値は URL エンコードのまま返す。
fn split_url(url: &str) -> (String, Vec<(String, String)>) {
    let (path, query) = url
        .split_once('?')
        .expect("the presigned url must have a query");
    let pairs = query
        .split('&')
        .map(|pair| {
            pair.split_once('=')
                .expect("a query pair must have a name and a value")
        })
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
    (path.to_owned(), pairs)
}

/// クエリの名前から値を引く。
fn query_value<'a>(pairs: &'a [(String, String)], name: &str) -> &'a str {
    pairs
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
        .unwrap_or_else(|| panic!("the query must have {name}"))
}

/// 実体を GET で読み、状態コードと応答を返す。
fn get_photo(client: &ApiClient, purchase_id: &str) -> (u16, Vec<u8>) {
    let response = client.get(&photo_path(purchase_id));
    let status = response.status().as_u16();
    let bytes = response
        .bytes()
        .expect("the photo response must be readable")
        .to_vec();
    (status, bytes)
}

/// `pending/` のオブジェクトをローカルの R2 に置き、完了通知を送る。
///
/// 署名付き URL への PUT は R2 の実サービスを要するため行わず、クライアントが PUT した後の
/// 状態を `wrangler r2 object put` で作る。返り値は紐づけ後のキー。
fn complete_photo(
    server: &support::DevServer,
    client: &ApiClient,
    user_id: &str,
    purchase_id: &str,
    uuid: &str,
    bytes: &[u8],
) -> String {
    let key = pending_key(user_id, uuid);
    server
        .put_r2_object(BUCKET, &key, bytes, CONTENT_TYPE)
        .expect("the pending object must be placed");
    let body = assert_status(
        client.post_json(
            &photo_path(purchase_id),
            &json!({ "key": key, "size": bytes.len() }),
        ),
        200,
    );
    body["photo_key"]
        .as_str()
        .expect("the photo_key must be a string")
        .to_owned()
}

mod upload_url {
    //! アップロード用 URL の発行の経路のテスト (FR-10、ADR-0003)。

    use super::*;

    #[test]
    fn wrangler_purchases_photo_upload_url_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.upload.session));
        let body = assert_status(
            client.post_json(
                &upload_url_path(&data.upload.purchase.id),
                &json!({ "size": 1234 }),
            ),
            200,
        );

        // キーは `pending/<利用者 ID>/<UUID>.jpg` で、UUID の部分は UUID の形をしている (FR-10)。
        let key = body["key"].as_str().expect("the key must be a string");
        let uuid = key
            .strip_prefix(&format!("pending/{}/", data.upload.user_id))
            .and_then(|rest| rest.strip_suffix(".jpg"))
            .unwrap_or_else(|| panic!("the key must be a pending key of the caller: {key}"));
        assert!(
            brew_book_core::ids::uuid_bytes(uuid).is_some(),
            "the key must contain a uuid: {key}"
        );

        // URL は設定したエンドポイントとバケットに、キーのパスを付けたものになる。
        let url = body["url"].as_str().expect("the url must be a string");
        let (without_query, query) = split_url(url);
        assert_eq!(
            without_query,
            format!("{R2_ENDPOINT}/{BUCKET}/{key}"),
            "the url must use the path style with the bucket: {url}"
        );

        // 署名の条件: Content-Length と Content-Type が署名対象のヘッダに含まれる (ADR-0003)。
        // 値は URL エンコードされるため、区切りの `;` は `%3B` になる。
        assert_eq!(
            query_value(&query, "X-Amz-SignedHeaders"),
            "content-length%3Bcontent-type%3Bhost"
        );
        // 有効期限は設定値 (vars) と一致する。
        assert_eq!(query_value(&query, "X-Amz-Expires"), URL_EXPIRES_SECONDS);
        assert_eq!(query_value(&query, "X-Amz-Algorithm"), "AWS4-HMAC-SHA256");
        let credential = query_value(&query, "X-Amz-Credential");
        assert!(
            credential.starts_with("test-access-key-id%2F")
                && credential.ends_with("%2Fauto%2Fs3%2Faws4_request"),
            "the credential must use the configured access key and the auto region: {credential}"
        );
        assert!(
            !query_value(&query, "X-Amz-Date").is_empty(),
            "the url must have the signing time"
        );
        let signature = query_value(&query, "X-Amz-Signature");
        assert_eq!(
            signature.len(),
            64,
            "the signature must be a sha256: {signature}"
        );
        assert!(
            signature.chars().all(|ch| ch.is_ascii_hexdigit()),
            "the signature must be hexadecimal: {signature}"
        );

        // 申告サイズの上限 (5,000,000 バイト) は受け付ける。
        let body = assert_status(
            client.post_json(
                &upload_url_path(&data.upload.purchase.id),
                &json!({ "size": 5_000_000 }),
            ),
            200,
        );
        assert!(
            body["url"]
                .as_str()
                .is_some_and(|url| url.contains("X-Amz-Expires=")),
            "the url for the maximum size must be presigned: {body}"
        );
    }

    #[test]
    fn wrangler_purchases_photo_upload_url_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.upload.session));
        // 5 MB を超える申告は 400 で URL を発行しない (ADR-0003)。
        assert_bad_request(client.post_json(
            &upload_url_path(&data.upload.purchase.id),
            &json!({ "size": 5_000_001 }),
        ));
        // 負のサイズは 400。
        assert_bad_request(client.post_json(
            &upload_url_path(&data.upload.purchase.id),
            &json!({ "size": -1 }),
        ));
        // サイズの欠落、型の違い、受け取らない項目は 400。
        assert_bad_request(
            client.post_json(&upload_url_path(&data.upload.purchase.id), &json!({})),
        );
        assert_bad_request(client.post_json(
            &upload_url_path(&data.upload.purchase.id),
            &json!({ "size": "1234" }),
        ));
        assert_bad_request(client.post_json(
            &upload_url_path(&data.upload.purchase.id),
            &json!({ "size": 100, "key": "pending/x.jpg" }),
        ));
        // 本体が JSON でない場合も 400。
        assert_bad_request(client.post_json(
            &upload_url_path(&data.upload.purchase.id),
            &json!("no-body"),
        ));
    }

    #[test]
    fn wrangler_purchases_photo_upload_url_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post_json(
            &upload_url_path(&data.upload.purchase.id),
            &json!({ "size": 1234 }),
        ));
    }
}

mod complete {
    //! アップロード完了の通知の経路のテスト (FR-10、ADR-0003)。

    use super::*;

    #[test]
    fn wrangler_purchases_photo_complete_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.complete.session));
        let key = lease.use_server(|server| {
            complete_photo(
                server,
                &client,
                &data.complete.user_id,
                &data.complete.purchase.id,
                PENDING_UUID,
                PHOTO_BYTES,
            )
        });

        // 紐づけ後のキーは `users/` から始まり、`pending/` ではない (FR-10)。
        assert_eq!(
            key,
            users_key(
                &data.complete.user_id,
                &data.complete.purchase.id,
                PENDING_UUID
            )
        );

        lease.use_server(|server| {
            // `pending/` のオブジェクトは移した後に消える (ADR-0003)。
            assert_eq!(
                server.get_r2_object(BUCKET, &pending_key(&data.complete.user_id, PENDING_UUID)),
                Ok(None),
                "the pending object must be deleted"
            );
            // 紐づけ後のキーのオブジェクトが実体を持つ。
            assert_eq!(
                server.get_r2_object(BUCKET, &key),
                Ok(Some(PHOTO_BYTES.to_vec())),
                "the object must be moved to the users key"
            );
            // 購入の photo_key が更新される (D1)。
            let matched = server
                .query_int(&format!(
                    "SELECT COUNT(*) FROM purchases WHERE id = '{}' AND photo_key = '{key}'",
                    data.complete.purchase.id
                ))
                .expect("the count must be readable");
            assert_eq!(matched, 1, "the purchase must refer to the photo");
        });

        // 取得は同じ内容を返す。
        let (status, bytes) = get_photo(&client, &data.complete.purchase.id);
        assert_eq!(status, 200);
        assert_eq!(bytes, PHOTO_BYTES);
    }

    #[test]
    fn wrangler_purchases_photo_complete_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.mismatch.session));
        let purchase_id = &data.mismatch.purchase.id;
        let user_id = &data.mismatch.user_id;

        // 申告サイズが 5 MB を超える場合は 400 (オブジェクトに触れない)。
        assert_bad_request(client.post_json(
            &photo_path(purchase_id),
            &json!({
                "key": pending_key(user_id, PENDING_UUID),
                "size": 5_000_001,
            }),
        ));
        // キーとサイズの欠落、受け取らない項目は 400。
        assert_bad_request(client.post_json(&photo_path(purchase_id), &json!({})));
        assert_bad_request(client.post_json(
            &photo_path(purchase_id),
            &json!({ "key": pending_key(user_id, PENDING_UUID) }),
        ));
        assert_bad_request(client.post_json(
            &photo_path(purchase_id),
            &json!({ "key": pending_key(user_id, PENDING_UUID), "size": 1, "photo_key": "x" }),
        ));
        // 形式の違うキーは 400 (オブジェクトに触れない)。
        assert_bad_request(client.post_json(
            &photo_path(purchase_id),
            &json!({ "key": "users/x.jpg", "size": 1 }),
        ));
        assert_bad_request(client.post_json(
            &photo_path(purchase_id),
            &json!({ "key": format!("pending/{user_id}/not-a-uuid.jpg"), "size": 1 }),
        ));
        assert_bad_request(client.post_json(
            &photo_path(purchase_id),
            &json!({ "key": pending_key(user_id, PENDING_UUID).trim_end_matches(".jpg"), "size": 1 }),
        ));
        // 存在しないオブジェクトは 400。
        assert_bad_request(client.post_json(
            &photo_path(purchase_id),
            &json!({ "key": pending_key(user_id, PENDING_UUID), "size": 1 }),
        ));

        // 申告サイズと実体のサイズが違う場合は 400 で、置かれたオブジェクトを削除する (ADR-0003)。
        let key = pending_key(user_id, "00000000-0000-4000-8000-000000000201");
        lease.use_server(|server| {
            server
                .put_r2_object(BUCKET, &key, b"short", CONTENT_TYPE)
                .expect("the pending object must be placed");
        });
        assert_bad_request(
            client.post_json(&photo_path(purchase_id), &json!({ "key": key, "size": 6 })),
        );
        lease.use_server(|server| {
            assert_eq!(
                server.get_r2_object(BUCKET, &key),
                Ok(None),
                "the mismatched object must be deleted"
            );
        });

        // 実体のサイズが 5 MB を超える場合も 400 で削除する (申告は 5 MB 以下でも)。
        let key = pending_key(user_id, "00000000-0000-4000-8000-000000000202");
        let large = vec![0u8; 5_000_001];
        lease.use_server(|server| {
            server
                .put_r2_object(BUCKET, &key, &large, CONTENT_TYPE)
                .expect("the pending object must be placed");
        });
        assert_bad_request(client.post_json(
            &photo_path(purchase_id),
            &json!({ "key": key, "size": 5_000_000 }),
        ));
        lease.use_server(|server| {
            assert_eq!(
                server.get_r2_object(BUCKET, &key),
                Ok(None),
                "the oversized object must be deleted"
            );
        });

        // Content-Type が `image/jpeg` でない場合は 400 で削除する (ADR-0003)。
        let key = pending_key(user_id, "00000000-0000-4000-8000-000000000203");
        lease.use_server(|server| {
            server
                .put_r2_object(BUCKET, &key, b"plain", "text/plain")
                .expect("the pending object must be placed");
        });
        assert_bad_request(
            client.post_json(&photo_path(purchase_id), &json!({ "key": key, "size": 5 })),
        );
        lease.use_server(|server| {
            assert_eq!(
                server.get_r2_object(BUCKET, &key),
                Ok(None),
                "the object with the wrong content type must be deleted"
            );
        });

        // 確認に失敗した後は、購入に写真が紐づかない。
        let body = assert_status(client.get(&photo_path(purchase_id)), 404);
        assert_eq!(error_code(&body), Some("not_found"), "{body}");
    }

    #[test]
    fn wrangler_purchases_photo_complete_does_not_touch_another_users_pending_object() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.steal.session));
        // 他の利用者の `pending/` のキーを渡すと 400 になり、そのオブジェクトは削除されない (FR-10)。
        let key = pending_key(&data.other.user_id, OTHER_PENDING_UUID);
        lease.use_server(|server| {
            server
                .put_r2_object(BUCKET, &key, OTHER_PENDING_BYTES, CONTENT_TYPE)
                .expect("the pending object must be placed");
        });
        assert_bad_request(client.post_json(
            &photo_path(&data.steal.purchase.id),
            &json!({ "key": key, "size": OTHER_PENDING_BYTES.len() }),
        ));
        lease.use_server(|server| {
            assert_eq!(
                server.get_r2_object(BUCKET, &key),
                Ok(Some(OTHER_PENDING_BYTES.to_vec())),
                "the other user's pending object must not be deleted"
            );
        });
        // 購入には写真が紐づかない。
        assert_not_found(client.get(&photo_path(&data.steal.purchase.id)));
        // 持ち主は同じキーで完了通知を送れる (オブジェクトが壊れていない)。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.other.session)).post_json(
                &photo_path(&data.other.purchase.id),
                &json!({ "key": key, "size": OTHER_PENDING_BYTES.len() }),
            ),
            200,
        );
        assert_eq!(
            body["photo_key"],
            users_key(
                &data.other.user_id,
                &data.other.purchase.id,
                OTHER_PENDING_UUID
            )
        );
    }

    #[test]
    fn wrangler_purchases_photo_complete_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).post_json(
            &photo_path(&data.complete.purchase.id),
            &json!({
                "key": pending_key(&data.complete.user_id, PENDING_UUID),
                "size": PHOTO_BYTES.len(),
            }),
        ));
    }

    #[test]
    fn wrangler_purchases_photo_replace_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.replace.session));
        let purchase_id = &data.replace.purchase.id;
        let user_id = &data.replace.user_id;

        // 1 枚目を紐づける。
        let first = lease.use_server(|server| {
            complete_photo(
                server,
                &client,
                user_id,
                purchase_id,
                PENDING_UUID,
                PHOTO_BYTES,
            )
        });
        assert_eq!(first, users_key(user_id, purchase_id, PENDING_UUID));

        // 2 枚目を紐づけると差し替えになり、古いオブジェクトは紐づけの後に消える (FR-10)。
        let second = lease.use_server(|server| {
            complete_photo(
                server,
                &client,
                user_id,
                purchase_id,
                REPLACED_UUID,
                REPLACED_BYTES,
            )
        });
        assert_eq!(second, users_key(user_id, purchase_id, REPLACED_UUID));
        lease.use_server(|server| {
            assert_eq!(
                server.get_r2_object(BUCKET, &first),
                Ok(None),
                "the replaced object must be deleted"
            );
            assert_eq!(
                server.get_r2_object(BUCKET, &second),
                Ok(Some(REPLACED_BYTES.to_vec())),
                "the new object must stay"
            );
        });

        // 取得は差し替え後の実体を返す。
        let (status, bytes) = get_photo(&client, purchase_id);
        assert_eq!(status, 200);
        assert_eq!(bytes, REPLACED_BYTES);
    }
}

mod get {
    //! 写真の取得の経路のテスト (FR-10、ADR-0003)。

    use super::*;

    #[test]
    fn wrangler_purchases_photo_get_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.get.session));
        lease.use_server(|server| {
            complete_photo(
                server,
                &client,
                &data.get.user_id,
                &data.get.purchase.id,
                PENDING_UUID,
                PHOTO_BYTES,
            );
        });

        // 取得は `image/jpeg` で実体を返し、共用の端末のキャッシュに残さない (UC-12)。
        let response = client.get(&photo_path(&data.get.purchase.id));
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some(CONTENT_TYPE)
        );
        assert_eq!(
            response
                .headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("private, no-store")
        );
        let bytes = response
            .bytes()
            .expect("the photo response must be readable");
        assert_eq!(&bytes[..], PHOTO_BYTES);
    }

    #[test]
    fn wrangler_purchases_photo_get_not_found_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.missing.session));
        // 写真が紐づいていない購入は 404 を返す (資源が無いものとして扱う)。
        assert_not_found(client.get(&photo_path(&data.missing.purchase.id)));
    }

    #[test]
    fn wrangler_purchases_photo_get_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get(&photo_path(&data.get.purchase.id)));
    }
}

mod delete {
    //! 写真の削除の経路のテスト (FR-10、ADR-0003)。

    use super::*;

    #[test]
    fn wrangler_purchases_photo_delete_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.delete.session));
        let purchase_id = &data.delete.purchase.id;
        let user_id = &data.delete.user_id;
        let key = lease.use_server(|server| {
            complete_photo(
                server,
                &client,
                user_id,
                purchase_id,
                PENDING_UUID,
                PHOTO_BYTES,
            )
        });

        // 削除は購入の `photo_key` を NULL にし、オブジェクトも消す (FR-10)。
        let body = assert_status(client.delete(&photo_path(purchase_id)), 200);
        assert_eq!(body["photo_key"], Value::Null, "{body}");
        lease.use_server(|server| {
            assert_eq!(
                server.get_r2_object(BUCKET, &key),
                Ok(None),
                "the object must be deleted"
            );
            let remaining = server
                .query_int(&format!(
                    "SELECT COUNT(*) FROM purchases WHERE id = '{purchase_id}' AND photo_key IS NOT NULL"
                ))
                .expect("the count must be readable");
            assert_eq!(remaining, 0, "the photo_key must be null");
        });

        // 削除の後の取得は 404 になる。
        assert_not_found(client.get(&photo_path(purchase_id)));
        // 写真が無い購入の削除も 404 になる。
        assert_not_found(client.delete(&photo_path(purchase_id)));
    }

    #[test]
    fn wrangler_purchases_photo_delete_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).delete(&photo_path(&data.delete.purchase.id)));
    }
}

mod other_user {
    //! 経路の共通の検査 (FR-5)。存在しない購入と他の利用者の購入は区別せず 404 になる。

    use super::*;

    #[test]
    fn wrangler_purchases_photo_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.upload.session));
        let missing = "no-such-id";

        // 4 経路とも、他の利用者の購入では 404 になる。
        assert_not_found(client.post_json(
            &upload_url_path(&data.other.purchase.id),
            &json!({ "size": 1234 }),
        ));
        assert_not_found(client.post_json(
            &photo_path(&data.other.purchase.id),
            &json!({
                "key": pending_key(&data.upload.user_id, PENDING_UUID),
                "size": PHOTO_BYTES.len(),
            }),
        ));
        assert_not_found(client.get(&photo_path(&data.other.purchase.id)));
        assert_not_found(client.delete(&photo_path(&data.other.purchase.id)));

        // 存在しない購入の ID も同じ 404 になる。
        assert_not_found(client.post_json(&upload_url_path(missing), &json!({ "size": 1234 })));
        assert_not_found(client.post_json(
            &photo_path(missing),
            &json!({ "key": pending_key(&data.upload.user_id, PENDING_UUID), "size": 1 }),
        ));
        assert_not_found(client.get(&photo_path(missing)));
        assert_not_found(client.delete(&photo_path(missing)));
    }
}
