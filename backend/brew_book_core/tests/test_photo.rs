//! `photo` の単体テスト。キーの組み立てと検証、申告サイズの境界、署名付き URL の条件を確認する。
//!
//! キーの往復の性質は PBT (`prop_photo.rs`) が担う。

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use brew_book_core::photo::{
    parse_pending_key, pending_key, photo_key, presign_put_url, system_time_from_millis,
    validate_declared_size, SigningConfig, SizeError, CONTENT_TYPE, DEFAULT_URL_EXPIRES_SECONDS,
    MAX_BYTES,
};

/// 署名に使う固定の時刻 (署名を決定的にするため)。2023-11-14T22:13:20Z。
fn now() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_700_000_000)
}

/// 署名の設定。エンドポイントと資格情報はテスト専用の値にする。
fn config() -> SigningConfig<'static> {
    SigningConfig {
        endpoint: "https://test-account.r2.cloudflarestorage.com",
        bucket: "brewbook-photos",
        access_key_id: "test-access-key-id",
        secret_access_key: "test-secret-access-key",
    }
}

/// テストに使う UUID (小文字の 16 進)。
const UUID: &str = "00000000-0000-4000-8000-000000000001";

/// 署名付き URL のクエリを名前と値の組にする。値は URL エンコードのまま返す。
fn query_pairs(url: &str) -> Vec<(String, String)> {
    let (_, query) = url
        .split_once('?')
        .expect("the presigned url must have a query");
    query
        .split('&')
        .map(|pair| {
            pair.split_once('=')
                .expect("a query pair must have a name and a value")
        })
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

/// クエリの名前から値を引く。
fn query_value<'a>(pairs: &'a [(String, String)], name: &str) -> &'a str {
    pairs
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
        .unwrap_or_else(|| panic!("the query must have {name}"))
}

/// クエリの名前と値をエンコードする前の形で返す。
fn decoded_query_value(pairs: &[(String, String)], name: &str) -> String {
    query_value(pairs, name)
        .replace("%3B", ";")
        .replace("%2F", "/")
}

#[test]
fn the_pending_key_and_the_photo_key_have_the_documented_shape() {
    assert_eq!(
        pending_key("user-1", UUID),
        "pending/user-1/00000000-0000-4000-8000-000000000001.jpg"
    );
    assert_eq!(
        photo_key("user-1", "purchase-1", UUID),
        "users/user-1/purchases/purchase-1/00000000-0000-4000-8000-000000000001.jpg"
    );
}

#[test]
fn a_pending_key_of_the_caller_parses_to_its_uuid() {
    assert_eq!(
        parse_pending_key("user-1", &pending_key("user-1", UUID)),
        Some(UUID)
    );
    // 大文字の 16 進も UUID として受け付ける (`ids::uuid_bytes` と同じ)。
    let uppercase = UUID.to_uppercase();
    assert_eq!(
        parse_pending_key("user-1", &format!("pending/user-1/{uppercase}.jpg")),
        Some(uppercase.as_str())
    );
}

#[test]
fn a_key_of_another_user_or_another_shape_does_not_parse() {
    // 他の利用者の `pending/` のキーは受け付けない (FR-10)。
    assert_eq!(
        parse_pending_key("user-1", &pending_key("user-2", UUID)),
        None
    );
    // 紐づけ済みのキー。
    assert_eq!(
        parse_pending_key("user-1", &photo_key("user-1", "purchase-1", UUID)),
        None
    );
    // プレフィックスが違う。
    assert_eq!(parse_pending_key("user-1", UUID), None);
    // 区切りが無い。
    assert_eq!(parse_pending_key("user-1", "pending/user-1"), None);
    // 拡張子が無い、または位置が違う。
    assert_eq!(
        parse_pending_key("user-1", &format!("pending/user-1/{UUID}")),
        None
    );
    assert_eq!(
        parse_pending_key("user-1", &format!("pending/user-1/{UUID}.jpg/extra.jpg")),
        None
    );
    // UUID の部分が UUID でない。
    assert_eq!(
        parse_pending_key("user-1", "pending/user-1/not-a-uuid.jpg"),
        None
    );
    assert_eq!(
        parse_pending_key(
            "user-1",
            "pending/user-1/00000000000000000000000000000001.jpg"
        ),
        None
    );
    // 上位のディレクトリへ移動する並び。
    assert_eq!(
        parse_pending_key("user-1", &format!("pending/user-1/../user-2/{UUID}.jpg")),
        None
    );
}

#[test]
fn every_size_error_maps_to_a_bad_request() {
    assert_eq!(SizeError::Negative.code().status(), 400);
    assert_eq!(SizeError::TooLarge.code().status(), 400);
    assert!(!SizeError::Negative.message().is_empty());
    assert!(!SizeError::TooLarge.message().is_empty());
}

#[test]
fn the_size_validation_accepts_zero_and_the_maximum() {
    assert_eq!(MAX_BYTES, 5_000_000);
    assert_eq!(validate_declared_size(0), Ok(0));
    assert_eq!(validate_declared_size(1), Ok(1));
    assert_eq!(validate_declared_size(MAX_BYTES), Ok(MAX_BYTES));
    assert_eq!(
        validate_declared_size(MAX_BYTES + 1),
        Err(SizeError::TooLarge)
    );
    assert_eq!(validate_declared_size(-1), Err(SizeError::Negative));
}

#[test]
fn the_presigned_url_signs_the_content_type_and_the_content_length() {
    assert_eq!(CONTENT_TYPE, "image/jpeg");
    let key = pending_key("user-1", UUID);
    let url = presign_put_url(&config(), &key, 1234, DEFAULT_URL_EXPIRES_SECONDS, now())
        .expect("the presign must succeed");

    // 署名の対象に Content-Length と Content-Type が含まれる (ADR-0003)。
    // 値は URL エンコードされるため、区切りの `;` は `%3B` になる。
    let pairs = query_pairs(&url);
    assert_eq!(
        decoded_query_value(&pairs, "X-Amz-SignedHeaders"),
        "content-length;content-type;host"
    );
    assert_eq!(query_value(&pairs, "X-Amz-Algorithm"), "AWS4-HMAC-SHA256");
    assert_eq!(
        decoded_query_value(&pairs, "X-Amz-Credential"),
        "test-access-key-id/20231114/auto/s3/aws4_request"
    );
    assert_eq!(query_value(&pairs, "X-Amz-Date"), "20231114T221320Z");
    let signature = query_value(&pairs, "X-Amz-Signature");
    assert_eq!(
        signature.len(),
        64,
        "the signature must be a sha256: {signature}"
    );
    assert!(
        signature.chars().all(|ch| ch.is_ascii_hexdigit()),
        "the signature must be hexadecimal: {signature}"
    );

    // パスは path 形式 (バケット名をパスに入れる) で、キーのパスを指す。
    let (path, _) = url.split_once('?').expect("the url must have a query");
    assert_eq!(
        path,
        format!("https://test-account.r2.cloudflarestorage.com/brewbook-photos/{key}")
    );
}

#[test]
fn the_presigned_url_uses_the_configured_expiry() {
    let key = pending_key("user-1", UUID);
    // 有効期限の値 (X-Amz-Expires) が設定値と一致する (FR-10)。
    for seconds in [DEFAULT_URL_EXPIRES_SECONDS, 1, 900] {
        let url = presign_put_url(&config(), &key, 1234, seconds, now())
            .expect("the presign must succeed");
        let pairs = query_pairs(&url);
        let expected = seconds.to_string();
        assert_eq!(query_value(&pairs, "X-Amz-Expires"), expected.as_str());
    }
}

#[test]
fn the_presigned_url_changes_with_the_declared_size() {
    let key = pending_key("user-1", UUID);
    let first = presign_put_url(&config(), &key, 1234, DEFAULT_URL_EXPIRES_SECONDS, now())
        .expect("the presign must succeed");
    let second = presign_put_url(&config(), &key, 1235, DEFAULT_URL_EXPIRES_SECONDS, now())
        .expect("the presign must succeed");
    // 申告サイズは署名対象のヘッダの値のため、署名だけが変わる (URL からは値が見えない)。
    let without_signature = |url: &str| {
        let (path, _) = url.split_once('?').expect("the url must have a query");
        let pairs: Vec<(String, String)> = query_pairs(url)
            .into_iter()
            .filter(|(name, _)| name != "X-Amz-Signature")
            .collect();
        (path.to_owned(), pairs)
    };
    assert_eq!(without_signature(&first), without_signature(&second));
    assert_ne!(first, second, "the declared size must be signed");
}

#[test]
fn the_presigned_url_is_deterministic_for_a_fixed_time() {
    let key = pending_key("user-1", UUID);
    let first = presign_put_url(&config(), &key, 1234, DEFAULT_URL_EXPIRES_SECONDS, now())
        .expect("the presign must succeed");
    let second = presign_put_url(&config(), &key, 1234, DEFAULT_URL_EXPIRES_SECONDS, now())
        .expect("the presign must succeed");
    assert_eq!(first, second);
}

#[test]
fn the_system_time_is_the_epoch_for_a_negative_millis() {
    // wasm32-unknown-unknown では `SystemTime::now()` を使えないため、Worker は `Date` の値を渡す。
    assert_eq!(system_time_from_millis(0), UNIX_EPOCH);
    assert_eq!(system_time_from_millis(-1), UNIX_EPOCH);
    assert_eq!(
        system_time_from_millis(1_700_000_000_000),
        UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    );
    assert_eq!(
        system_time_from_millis(1_234),
        UNIX_EPOCH + Duration::from_millis(1_234)
    );
}
