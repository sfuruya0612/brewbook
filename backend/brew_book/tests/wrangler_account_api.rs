//! アカウントと全データの削除の結合テスト (HTTP)。
//!
//! FR-15 の受け入れ基準 (利用者に属する全行の物理削除、ログイン用のチャレンジの行の除外、
//! R2 の紐づけ済みと紐づけ前のオブジェクトの削除、削除後の API 呼び出しの 401) を検査する。
//! 想定規模 (購入 3,000 件、写真 3,000 件) の R2 のオブジェクトをカーソルで全件削除できることも
//! 検査する (R2 の一覧は 1 回で返る件数に上限がある)。
//!
//! 想定規模のオブジェクトの下ごしらえには、テスト専用の経路 (`/api/__r2_check`、`R2_CHECK` の
//! var で有効にする) を使う。`wrangler r2 object put` は 1 件につき 1 プロセスを要するためである。
//! 削除の確認には `wrangler r2 object get` も使い、バインディングの一覧に依らない経路でも
//! ページの境目を跨ぐ鍵が消えていることを確かめる。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。
//! テストは並行に走るため、削除するテストは利用者ごとに分ける。

mod support;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use brew_book_core::auth::{KIND_AUTHENTICATION, KIND_REGISTRATION};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, Seed};
use support::{DevServer, ServerLease};

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const CREATED: &str = "2026-09-01T00:00:00.000Z";
const FUTURE: &str = "2099-01-01T00:00:00.000Z";
/// 下ごしらえに使う購入日と抽出日時。
const D21: &str = "2026-09-21";
const B21: &str = "2026-09-21T10:00:00.000Z";

/// 写真のバケット名 (wrangler.toml の R2 バインディングと同じ)。
const PHOTO_BUCKET: &str = "brewbook-photos";
/// 置くオブジェクトの実体。中身は API が検査しない。
const OBJECT_BODY: &[u8] = b"\xff\xd8\xff\xe0account-delete";

/// 登録用のチャレンジの値 (利用者を持つ行が削除されることの確認に使う)。
const REGISTRATION_CHALLENGE: &str = "registration-challenge-marker-4e2a";
/// ログイン用のチャレンジの値 (利用者を持たない行が削除されないことの確認に使う)。
const LOGIN_CHALLENGE: &str = "login-challenge-marker-7c3d";

/// アカウント削除で行を消す 11 テーブルと、利用者を指す列の名前 (ADR-0006)。
/// 外部キーの参照元から先に消す順に並べる。テストは実装と独立にこの一覧を持つ。
const DELETED_TABLES: &[(&str, &str)] = &[
    ("product_flavor_tags", "user_id"),
    ("brews", "user_id"),
    ("purchases", "user_id"),
    ("flavor_tags", "user_id"),
    ("products", "user_id"),
    ("shops", "user_id"),
    ("sessions", "user_id"),
    ("passkey_credentials", "user_id"),
    ("webauthn_challenges", "user_id"),
    ("registration_tokens", "user_id"),
    ("users", "id"),
];

/// 他の利用者が行を持つテーブル (削除されないことの確認に使う)。
const OTHER_USER_TABLES: &[&str] = &[
    "users",
    "sessions",
    "passkey_credentials",
    "products",
    "purchases",
];

/// 想定規模の購入の件数 (PRD の想定規模)。
const SCALE_PURCHASE_COUNT: usize = 3_000;
/// 想定規模の紐づけ済みのオブジェクトの件数 (購入ごとに 1 枚)。
const SCALE_PHOTO_COUNT: usize = 3_000;
/// 想定規模の紐づけ前のオブジェクトの件数。
const SCALE_PENDING_COUNT: usize = 3_000;
/// 1 つの INSERT 文に入れる行数。D1 の 1 文の長さの上限 (100 KB) を超えないように分ける
/// (購入の行は写真のキーを持つため、想定規模のテストの 400 件では超える)。
const ROWS_PER_STATEMENT: usize = 200;
/// 一覧のページの境目を跨ぐ鍵の添字 (カーソルの繰り返しの検査に使う)。
const PAGE_BOUNDARIES: [i64; 6] = [0, 999, 1_000, 1_999, 2_000, 2_999];
/// 想定規模の削除の応答を待つ上限。1 件ずつの削除の時間が機械の負荷で伸びても待てるようにする。
const SCALE_TIMEOUT: Duration = Duration::from_secs(600);
/// 想定規模の下ごしらえ (1 リクエストで 1,000 件の put) の応答を待つ上限。
/// 既定の 30 秒では、並列の負荷の下で 1,000 件の put が 30 秒を超えることがある (0030)。
const PUT_TIMEOUT: Duration = Duration::from_secs(120);
/// 想定規模の下ごしらえの再試行の回数。接続断と 200 以外の応答で同じ範囲をやり直す (0030)。
const PUT_RETRIES: usize = 3;
/// 想定規模の下ごしらえの再試行の間隔 (0030)。
const PUT_RETRY_INTERVAL: Duration = Duration::from_secs(1);

/// 正常系のテストの購入の ID。
const OK_PURCHASE_ID: &str = "ok-purchase-00000";
/// 想定規模のテストの商品の ID。
const SCALE_PRODUCT_ID: &str = "scale-product-00000";

/// 正常系のテストの利用者の紐づけ済みのキー (ADR-0003 のキーの形)。
fn ok_photo_key(user: &str, index: usize) -> String {
    format!("users/{user}/purchases/{OK_PURCHASE_ID}/a1b2c3d4-0000-4000-8000-{index:012}.jpg")
}

/// 正常系のテストの利用者の紐づけ前のキー。
fn ok_pending_key(user: &str, index: usize) -> String {
    format!("pending/{user}/b1b2c3d4-0000-4000-8000-{index:012}.jpg")
}

/// 他の利用者の紐づけ済みのキー。
fn other_photo_key(user: &str) -> String {
    format!("users/{user}/purchases/other-purchase-00000/c1b2c3d4-0000-4000-8000-000000000001.jpg")
}

/// 他の利用者の紐づけ前のキー。
fn other_pending_key(user: &str) -> String {
    format!("pending/{user}/d1b2c3d4-0000-4000-8000-000000000001.jpg")
}

/// `wrangler r2 object` と、想定規模のバインディングの書き込みを同時に走らせないための錠。
///
/// `wrangler r2 object` は `wrangler dev` と同じ状態ディレクトリを別のプロセスで開くため、
/// 数百件をまとめて書き込む想定規模の下ごしらえと重なると 500 で失敗することがある
/// (確認した症状: `R2 error response does not contain the CF-R2-Error header.`)。
static R2_CLI_LOCK: Mutex<()> = Mutex::new(());

/// 錠を取る。毒された場合は中の値をそのまま使う (テストの失敗を他のテストに広げない)。
fn r2_cli_lock() -> MutexGuard<'static, ()> {
    R2_CLI_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    /// 共有のサーバーの起動時に先に流す下ごしらえの SQL。
    seed_sql: String,
    /// 正常系のテストの利用者と、そのセッション。
    user: String,
    session: String,
    /// その利用者の R2 のキー (紐づけ済みと紐づけ前)。
    photo_keys: Vec<String>,
    pending_keys: Vec<String>,
    /// 他の利用者と、そのセッション。
    other_user: String,
    other_session: String,
    /// 他の利用者の R2 のキー (削除されないことの確認に使う)。
    other_photo_key: String,
    other_pending_key: String,
    /// 想定規模のテストの利用者と、そのセッション。
    scale_user: String,
    scale_session: String,
    /// 想定規模の購入を入れる SQL。
    scale_sql: Vec<String>,
}

/// 下ごしらえを 1 回だけ組み立てる。
fn data() -> &'static TestData {
    static DATA: OnceLock<TestData> = OnceLock::new();
    DATA.get_or_init(build_data)
}

/// 共有のサーバーを借りる。想定規模の下ごしらえのため、テスト専用の R2 の経路を有効にする。
fn server() -> ServerLease {
    support::shared_server("main", || {
        support::DevServer::start_with(
            |_| vec![(brew_book::r2_check::VAR_NAME.to_owned(), "true".to_owned())],
            &data().seed_sql,
        )
    })
    .expect("wrangler dev must start")
}

/// 下ごしらえの SQL と、テストが使う値を作る。
fn build_data() -> TestData {
    let mut seed = Seed::new();

    // 正常系のテストの利用者。11 テーブルすべてに行を持つ。
    let user = user_id(1);
    seed.user(&user, "削除する利用者", CREATED);
    let session = seed.session(&user, FUTURE, CREATED);
    seed.passkey(&user, "削除するパスキー", CREATED, None);
    seed.registration_token(&user, FUTURE);
    seed.used_registration_token(&user, FUTURE, CREATED);
    seed.challenge(
        Some(&user),
        KIND_REGISTRATION,
        REGISTRATION_CHALLENGE,
        FUTURE,
    );
    // ログイン用のチャレンジの行は user_id を持たない (ADR-0006)。削除の対象外であることを検査する。
    seed.challenge(None, KIND_AUTHENTICATION, LOGIN_CHALLENGE, FUTURE);
    let shop = seed.shop(
        &user,
        "削除する店",
        Some("削除する住所"),
        CREATED,
        CREATED,
        None,
    );
    let product = seed.product(&user, "削除する豆", CREATED, CREATED, None);
    let tag = seed.flavor_tag(&user, "削除するタグ");
    seed.product_flavor_tag(&user, &product.id, &tag);
    let photo_keys = vec![ok_photo_key(&user, 0), ok_photo_key(&user, 1)];
    let pending_keys = vec![ok_pending_key(&user, 0), ok_pending_key(&user, 1)];
    // 購入は写真のキーを持ち、抽出はその購入を参照する。
    seed.raw(&format!(
        "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, roast, roast_date, \
         price_amount, price_currency, weight_grams, photo_key, created_at, updated_at, archived_at) \
         VALUES ('{OK_PURCHASE_ID}', '{user}', '{}', '{}', '{D21}', NULL, NULL, 1200, 'JPY', 200, \
         '{}', '{CREATED}', '{CREATED}', NULL)",
        product.id, shop.id, photo_keys[0]
    ));
    seed.brew(&user, OK_PURCHASE_ID, B21, CREATED, CREATED, None);

    // 他の利用者。行と R2 のオブジェクトが削除されないことを検査する。
    let other_user = user_id(2);
    seed.user(&other_user, "残る利用者", CREATED);
    let other_session = seed.session(&other_user, FUTURE, CREATED);
    seed.passkey(&other_user, "残るパスキー", CREATED, None);
    let other_product = seed.product(&other_user, "残る豆", CREATED, CREATED, None);
    seed.purchase(
        &other_user,
        &other_product.id,
        None,
        D21,
        CREATED,
        CREATED,
        None,
    );
    let other_photo = other_photo_key(&other_user);
    let other_pending = other_pending_key(&other_user);

    // 想定規模のテストの利用者。購入の行は共有のサーバーの起動後にまとめて入れる。
    let scale_user = user_id(3);
    seed.user(&scale_user, "想定規模の利用者", CREATED);
    let scale_session = seed.session(&scale_user, FUTURE, CREATED);
    let scale_sql = scale_statements(&scale_user);

    TestData {
        seed_sql: seed.sql(),
        user,
        session,
        photo_keys,
        pending_keys,
        other_user,
        other_session,
        other_photo_key: other_photo,
        other_pending_key: other_pending,
        scale_user,
        scale_session,
        scale_sql,
    }
}

/// 想定規模の行 (商品 1 件、購入 3,000 件) を入れる SQL を組み立てる。
///
/// 購入の `photo_key` には、下ごしらえの経路が R2 に置くのと同じ鍵 (`<利用者>/purchases/...`) を入れる。
fn scale_statements(user: &str) -> Vec<String> {
    let mut statements = vec![format!(
        "INSERT INTO products (id, user_id, name, producer, origin, region, process, variety, \
         created_at, updated_at, archived_at) VALUES ('{SCALE_PRODUCT_ID}', '{user}', '想定規模の豆', \
         NULL, NULL, NULL, NULL, NULL, '{CREATED}', '{CREATED}', NULL)"
    )];
    statements.extend(chunked_insert(
        "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, roast, roast_date, \
         price_amount, price_currency, weight_grams, photo_key, created_at, updated_at, archived_at)",
        (0..SCALE_PURCHASE_COUNT)
            .map(|index| {
                let index = index as i64;
                format!(
                    "('{}', '{user}', '{SCALE_PRODUCT_ID}', NULL, '{D21}', NULL, NULL, NULL, NULL, \
                     NULL, '{}', '{CREATED}', '{CREATED}', NULL)",
                    brew_book::r2_check::photo_purchase_id(index),
                    brew_book::r2_check::object_key(user, index)
                )
            })
            .collect(),
    ));
    statements
}

/// 行を `ROWS_PER_STATEMENT` 件ずつの INSERT 文にする。
fn chunked_insert(prefix: &str, rows: Vec<String>) -> Vec<String> {
    rows.chunks(ROWS_PER_STATEMENT)
        .map(|chunk| format!("{prefix} VALUES {}", chunk.join(", ")))
        .collect()
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

/// 応答のヘッダーを 1 つ読む。
fn header(response: &Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// 列が値に一致する行数を数える。
fn count_rows(server: &DevServer, table: &str, column: &str, value: &str) -> i64 {
    server
        .query_int(&format!(
            "SELECT COUNT(*) FROM {table} WHERE {column} = '{value}'"
        ))
        .unwrap_or_else(|error| panic!("the {table} count must be readable: {error}"))
}

/// 利用者に属する行数を、削除の対象の 11 テーブルについて数える。
fn counts(server: &DevServer, user: &str) -> Vec<(&'static str, i64)> {
    DELETED_TABLES
        .iter()
        .map(|entry| (entry.0, count_rows(server, entry.0, entry.1, user)))
        .collect()
}

/// テスト専用の経路で、オブジェクトを `count` 件置く。1 回の上限ごとに分けて呼ぶ。
///
/// miniflare の ProxyWorker と Worker の間の接続は失われることがある (接続の再利用と idle の
/// 接続の閉鎖の競合)。接続断と 5xx の応答は同じ範囲をやり直す。R2 の put は同じ鍵への上書き
/// なので、やり直しても結果は変わらない (0030)。
fn put_objects(lease: &ServerLease, user: &str, kind: &str, count: usize) {
    let base_url = lease.use_server(|server| server.base_url());
    // 1 リクエストで 1,000 件を置くため、既定の 30 秒では並列の負荷の下で足りないことがある。
    let client = ApiClient::with_timeout(&base_url, None, PUT_TIMEOUT);
    let limit = brew_book::r2_check::MAX_COUNT as usize;
    let mut start = 0;
    while start < count {
        let size = (count - start).min(limit);
        put_range(&client, user, kind, start, size);
        start += size;
    }
}

/// 1 つの範囲の put を送る。一時的な失敗 (送信の失敗と 5xx) は [`PUT_RETRY_INTERVAL`] の間隔で
/// 最大 [`PUT_RETRIES`] 回やり直す。4xx は入力の誤りなのでやり直さず、すぐに失敗させる。
/// 最後まで成功しなければ、最後の失敗の内容を添えて失敗にする (0030)。
fn put_range(client: &ApiClient, user: &str, kind: &str, start: usize, size: usize) {
    let body = json!({
        "action": "put",
        "user_id": user,
        "kind": kind,
        "start": start,
        "count": size,
    });
    let mut last_failure = None;
    for attempt in 0..=PUT_RETRIES {
        if attempt > 0 {
            thread::sleep(PUT_RETRY_INTERVAL);
        }
        match client.try_post_json(brew_book::r2_check::PATH, &body) {
            Ok(response) if response.status().as_u16() == 200 => {
                let body = assert_status(response, 200);
                assert_eq!(body["count"], json!(size), "{body}");
                return;
            }
            // 5xx は本文が JSON とは限らない (ProxyWorker の 500 はエラーの文面を返す)。
            // JSON として読まずに、そのまま再試行の材料にする。
            Ok(response) if response.status().as_u16() >= 500 => {
                let status = response.status().as_u16();
                let body = response
                    .text()
                    .unwrap_or_else(|error| format!("<the body was unreadable: {error}>"));
                last_failure = Some(format!("the response status was {status}: {body}"));
            }
            Ok(response) => {
                let status = response.status().as_u16();
                let body = response.text().unwrap_or_default();
                panic!(
                    "the put of {size} objects from {start} must not be retried: \
                     the response status was {status}: {body}"
                );
            }
            Err(error) => last_failure = Some(error.to_string()),
        }
    }
    panic!(
        "the put of {size} objects from {start} must succeed within {} attempts: {}",
        PUT_RETRIES + 1,
        last_failure.unwrap_or_else(|| "no attempt was made".to_owned())
    );
}

/// 再試行の経路の検査に使う、応答を差し替えられる小さな HTTP サーバー。
///
/// 指定した応答を順に返し、受け取ったリクエストの本文を返す。`Content-Length` の分だけ本文を
/// 読み、1 リクエストにつき 1 接続で応答の後に閉じる。
fn spawn_fake_server(
    responses: Vec<(u16, &'static str)>,
) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the fake server must bind");
    let address = listener
        .local_addr()
        .expect("the fake server must have an address");
    let handle = thread::spawn(move || {
        let mut bodies = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().expect("the fake server must accept");
            let mut buffer = Vec::new();
            let mut chunk = [0_u8; 1024];
            let header_end = loop {
                if let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                    break position + 4;
                }
                let read = stream
                    .read(&mut chunk)
                    .expect("the request must be readable");
                assert!(read > 0, "the request must have headers");
                buffer.extend_from_slice(&chunk[..read]);
            };
            let headers = String::from_utf8_lossy(&buffer[..header_end]).to_ascii_lowercase();
            let length: usize = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(0);
            while buffer.len() < header_end + length {
                let read = stream
                    .read(&mut chunk)
                    .expect("the request body must be readable");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
            }
            bodies.push(
                String::from_utf8_lossy(&buffer[header_end..header_end + length]).into_owned(),
            );
            let response = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("the response must be written");
        }
        bodies
    });
    (format!("http://{address}"), handle)
}

/// 再試行の経路: 5xx の後に成功したら、同じ範囲が再送されることを検査する (0030)。
#[test]
fn put_range_retries_the_same_range_after_a_failure() {
    let (base_url, server) = spawn_fake_server(vec![
        (500, "Error: Network connection lost."),
        (200, r#"{"count": 1000, "pages": 0}"#),
    ]);
    let client = ApiClient::with_timeout(&base_url, None, Duration::from_secs(5));
    put_range(&client, "user-1", "users", 0, 1000);
    let bodies = server.join().expect("the fake server must finish");
    assert_eq!(bodies.len(), 2, "the request must be sent twice");
    assert_eq!(bodies[0], bodies[1], "the same range must be sent again");
}

/// 再試行の上限: 5xx が続いたら、最後の失敗の内容を添えて失敗することを検査する (0030)。
#[test]
#[should_panic(expected = "the put of 1000 objects from 0 must succeed within 4 attempts")]
fn put_range_fails_after_the_retry_limit() {
    let (base_url, server) = spawn_fake_server(vec![
        (500, "Error: Network connection lost."),
        (500, "Error: Network connection lost."),
        (500, "Error: Network connection lost."),
        (500, "Error: Network connection lost."),
    ]);
    let client = ApiClient::with_timeout(&base_url, None, Duration::from_secs(5));
    put_range(&client, "user-1", "users", 0, 1000);
    let _ = server.join();
}

/// テスト専用の経路で、プレフィックスごとの残件数を数える。
/// 返り値は (件数, 一覧を引いた回数)。回数はカーソルの繰り返しが起きたことの確認に使う。
fn count_objects(lease: &ServerLease, user: &str, kind: &str) -> (usize, usize) {
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, None);
    let body = assert_status(
        client.post_json(
            brew_book::r2_check::PATH,
            &json!({ "action": "count", "user_id": user, "kind": kind }),
        ),
        200,
    );
    let count = body["count"]
        .as_u64()
        .unwrap_or_else(|| panic!("the count must be a number: {body}")) as usize;
    let pages = body["pages"]
        .as_u64()
        .unwrap_or_else(|| panic!("the pages must be a number: {body}")) as usize;
    (count, pages)
}

#[test]
fn wrangler_account_delete_ok() {
    let data = data();
    // `wrangler r2 object` を使うため、想定規模の下ごしらえと重ならないようにする。
    let _lock = r2_cli_lock();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, Some(&data.session));

    // 下ごしらえの R2 のオブジェクトを置く。削除の前後の比較に使う。
    lease.use_server(|server| {
        for key in data.photo_keys.iter().chain(data.pending_keys.iter()) {
            server
                .put_r2_object(PHOTO_BUCKET, key, OBJECT_BODY, "image/jpeg")
                .expect("the object of the deleting user must be placed");
        }
        for key in [&data.other_photo_key, &data.other_pending_key] {
            server
                .put_r2_object(PHOTO_BUCKET, key, OBJECT_BODY, "image/jpeg")
                .expect("the object of the other user must be placed");
        }
    });
    // 検査が空振りしないよう、利用者の行が 11 テーブルすべてにあることを先に確かめる。
    lease.use_server(|server| {
        for (table, count) in counts(server, &data.user) {
            assert!(count > 0, "the test data must have the {table} row");
        }
        assert_eq!(
            count_rows(server, "webauthn_challenges", "challenge", LOGIN_CHALLENGE),
            1,
            "the test data must have the login challenge row"
        );
    });

    let response = client.delete("/api/account");
    let status = response.status().as_u16();
    let cookie = header(&response, "set-cookie").expect("the response must clear the cookie");
    let body = response.text().expect("the response must be readable");
    assert_eq!(status, 204, "the deletion must answer 204");
    // 本体を持たず、セッションの Cookie を失効させる (FR-15)。
    assert!(
        body.is_empty(),
        "the 204 response must have no body: {body}"
    );
    assert!(
        cookie.contains("session=;") && cookie.contains("Max-Age=0"),
        "the session cookie must expire: {cookie}"
    );

    // 利用者に属する全行が物理削除される (FR-15)。
    lease.use_server(|server| {
        for (table, count) in counts(server, &data.user) {
            assert_eq!(count, 0, "the {table} rows of the user must be deleted");
        }
        // 利用者を持つチャレンジの行は消え、ログイン用の行 (user_id が NULL) は残る (ADR-0006)。
        assert_eq!(
            count_rows(
                server,
                "webauthn_challenges",
                "challenge",
                REGISTRATION_CHALLENGE
            ),
            0,
            "the registration challenge row must be deleted"
        );
        assert_eq!(
            count_rows(server, "webauthn_challenges", "challenge", LOGIN_CHALLENGE),
            1,
            "the login challenge row must remain"
        );
    });
    // 他の利用者の行は残る (FR-5)。
    lease.use_server(|server| {
        for table in OTHER_USER_TABLES {
            let column = if *table == "users" { "id" } else { "user_id" };
            let count = count_rows(server, table, column, &data.other_user);
            assert!(count > 0, "the other user's {table} row must remain");
        }
    });
    // R2 のオブジェクトは利用者の両方のプレフィックスで消え、他の利用者のものは残る (ADR-0003)。
    lease.use_server(|server| {
        for key in data.photo_keys.iter().chain(data.pending_keys.iter()) {
            assert_eq!(
                server.get_r2_object(PHOTO_BUCKET, key),
                Ok(None),
                "the object {key} must be deleted"
            );
        }
        for key in [&data.other_photo_key, &data.other_pending_key] {
            assert_eq!(
                server
                    .get_r2_object(PHOTO_BUCKET, key)
                    .map(|object| object.is_some()),
                Ok(true),
                "the object {key} of the other user must remain"
            );
        }
    });
    // 削除後は、同じ Cookie を使った API 呼び出しが 401 になる (FR-15)。
    assert_unauthorized(client.get("/api/shops"));
    assert_unauthorized(client.delete("/api/account"));
    // 他の利用者のセッションは有効なままである (FR-5)。
    let other = ApiClient::new(&base_url, Some(&data.other_session));
    let body = assert_status(other.get("/api/shops"), 200);
    assert!(
        body["shops"].is_array(),
        "the other user's session must stay valid: {body}"
    );
}

#[test]
fn wrangler_account_delete_unauthenticated_401() {
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());
    assert_unauthorized(anonymous(&base_url).delete("/api/account"));
}

#[test]
fn wrangler_account_delete_removes_the_assumed_scale_of_the_r2_objects_by_cursor() {
    let data = data();
    // 想定規模の書き込みが `wrangler r2 object` と重ならないようにする (R2_CLI_LOCK の説明)。
    let _lock = r2_cli_lock();
    let lease = server();
    let base_url = lease.use_server(|server| server.base_url());

    // 想定規模 (購入 3,000 件) の行を入れる。
    lease.use_server(|server| {
        server
            .execute_sql_file(&data.scale_sql)
            .expect("the scale rows must be inserted");
        assert_eq!(
            count_rows(server, "purchases", "user_id", &data.scale_user),
            SCALE_PURCHASE_COUNT as i64,
            "the test data must have the purchase rows"
        );
    });
    // 想定規模の R2 のオブジェクトを置く。一覧の 1 回の上限 (1,000 件) を跨ぐ件数にする。
    put_objects(&lease, &data.scale_user, "users", SCALE_PHOTO_COUNT);
    put_objects(&lease, &data.scale_user, "pending", SCALE_PENDING_COUNT);
    let (photos, photo_pages) = count_objects(&lease, &data.scale_user, "users");
    assert_eq!(
        photos, SCALE_PHOTO_COUNT,
        "the test data must have the objects under users/"
    );
    let (pending, pending_pages) = count_objects(&lease, &data.scale_user, "pending");
    assert_eq!(
        pending, SCALE_PENDING_COUNT,
        "the test data must have the objects under pending/"
    );
    // 一覧は 1 回で全件を返さない (削除がカーソルの繰り返しを要することを先に確かめる)。
    for (label, pages, expected) in [
        ("users/", photo_pages, SCALE_PHOTO_COUNT),
        ("pending/", pending_pages, SCALE_PENDING_COUNT),
    ] {
        assert!(
            pages >= expected.div_ceil(1_000),
            "the list of {label} must be paged but took {pages} calls"
        );
    }
    // 他の利用者のオブジェクトが消えないことを確かめるために置く。
    lease.use_server(|server| {
        server
            .put_r2_object(
                PHOTO_BUCKET,
                &data.other_photo_key,
                OBJECT_BODY,
                "image/jpeg",
            )
            .expect("the object of the other user must be placed");
    });

    // 削除する。想定規模のオブジェクトの削除を待つ。
    let client = ApiClient::with_timeout(&base_url, Some(&data.scale_session), SCALE_TIMEOUT);
    let started = Instant::now();
    let response = client.delete("/api/account");
    let elapsed = started.elapsed();
    assert_eq!(
        response.status().as_u16(),
        204,
        "the deletion must answer 204"
    );
    // 処理時間を記録する (本番の Workers Logs の `duration_ms` と比べる材料にする)。
    println!(
        "the deletion of {} purchase rows and {} objects took {} ms",
        SCALE_PURCHASE_COUNT,
        SCALE_PHOTO_COUNT + SCALE_PENDING_COUNT,
        elapsed.as_millis()
    );

    // 想定規模の全行が物理削除される (FR-15)。
    lease.use_server(|server| {
        for (table, count) in counts(server, &data.scale_user) {
            assert_eq!(count, 0, "the {table} rows of the user must be deleted");
        }
    });
    // R2 のオブジェクトはカーソルを繰り返して全件削除される (FR-15)。
    let (photos, _) = count_objects(&lease, &data.scale_user, "users");
    assert_eq!(photos, 0, "every object under users/ must be deleted");
    let (pending, _) = count_objects(&lease, &data.scale_user, "pending");
    assert_eq!(pending, 0, "every object under pending/ must be deleted");
    // ページの境目を跨ぐ鍵が消えていることを、バインディングの一覧に依らない経路でも確かめる。
    lease.use_server(|server| {
        for index in PAGE_BOUNDARIES {
            let key = brew_book::r2_check::object_key(&data.scale_user, index);
            assert_eq!(
                server.get_r2_object(PHOTO_BUCKET, &key),
                Ok(None),
                "the object {key} must be deleted"
            );
            let key = brew_book::r2_check::pending_object_key(&data.scale_user, index);
            assert_eq!(
                server.get_r2_object(PHOTO_BUCKET, &key),
                Ok(None),
                "the object {key} must be deleted"
            );
        }
        // 他の利用者のオブジェクトは残る (FR-5)。
        assert_eq!(
            server
                .get_r2_object(PHOTO_BUCKET, &data.other_photo_key)
                .map(|object| object.is_some()),
            Ok(true),
            "the object of the other user must remain"
        );
    });
}
