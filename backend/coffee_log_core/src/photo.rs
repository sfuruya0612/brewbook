//! 購入の写真のオブジェクトキーと署名付き PUT URL (ADR-0003、FR-10)。
//!
//! キーの組み立てと検証、申告サイズの検証、`shiguredo_s3` による署名付き URL の生成を持つ。
//! R2 の操作は Worker 側 (`coffee_log::records::photos`) がバインディングで行う。
//! `shiguredo_s3` は Sans I/O のため、ここでは HTTP 通信を行わない。

use std::time::{Duration, SystemTime};

use shiguredo_s3::{Client, Config, Credentials};

use crate::error::ErrorCode;
use crate::ids;

/// 写真の Content-Type。署名の条件に含める (ADR-0003)。
pub const CONTENT_TYPE: &str = "image/jpeg";
/// 申告サイズの上限。5 MB を 5,000,000 バイトとして扱う (ADR-0003)。
pub const MAX_BYTES: i64 = 5_000_000;
/// 署名付き URL の有効期限の既定 (秒)。5 分 (ADR-0003)。
pub const DEFAULT_URL_EXPIRES_SECONDS: u64 = 300;
/// 紐づけ前のオブジェクトキーのプレフィックス (ADR-0003)。
pub const PENDING_PREFIX: &str = "pending/";
/// 紐づけ済みのオブジェクトキーのプレフィックス (ADR-0003)。
pub const USERS_PREFIX: &str = "users/";
/// SigV4 の署名に使うリージョン。R2 は `auto` を受け付ける。
pub const REGION: &str = "auto";
/// オブジェクトキーの拡張子。
const SUFFIX: &str = ".jpg";

/// 申告サイズの検証の誤り (FR-10)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeError {
    /// 0 未満。
    Negative,
    /// 5 MB を超える。
    TooLarge,
}

impl SizeError {
    /// 応答の `code`。
    pub fn code(self) -> ErrorCode {
        ErrorCode::BadRequest
    }

    /// 応答のメッセージ (英語)。
    pub fn message(self) -> &'static str {
        match self {
            SizeError::Negative => "the declared size must be 0 or greater",
            SizeError::TooLarge => "the declared size must not exceed 5000000 bytes",
        }
    }
}

/// クライアントが申告したサイズを検証する。0 以上で 5 MB 以下だけを受け付ける (FR-10)。
pub fn validate_declared_size(size: i64) -> Result<i64, SizeError> {
    if size < 0 {
        return Err(SizeError::Negative);
    }
    if size > MAX_BYTES {
        return Err(SizeError::TooLarge);
    }
    Ok(size)
}

/// 紐づけ前の `pending/<利用者 ID>/<UUID>.jpg` のキーを組み立てる (ADR-0003)。
pub fn pending_key(user_id: &str, uuid: &str) -> String {
    format!("{PENDING_PREFIX}{user_id}/{uuid}{SUFFIX}")
}

/// 紐づけ済みの `users/<利用者 ID>/purchases/<購入 ID>/<UUID>.jpg` のキーを組み立てる (ADR-0003)。
pub fn photo_key(user_id: &str, purchase_id: &str, uuid: &str) -> String {
    format!("{USERS_PREFIX}{user_id}/purchases/{purchase_id}/{uuid}{SUFFIX}")
}

/// `pending/<利用者 ID>/<UUID>.jpg` のキーから UUID を取り出す。形式が違えば None を返す。
///
/// キーはクライアントから戻される信頼できない入力である。利用者 ID の一致と、UUID の部分が
/// 16 進とハイフンだけであることまで確認し、他の利用者の `pending/` のオブジェクトを
/// 取り込めないようにする (FR-10)。
pub fn parse_pending_key<'a>(user_id: &str, key: &'a str) -> Option<&'a str> {
    let rest = key.strip_prefix(PENDING_PREFIX)?;
    let (key_user_id, rest) = rest.split_once('/')?;
    if key_user_id != user_id {
        return None;
    }
    let uuid = rest.strip_suffix(SUFFIX)?;
    ids::uuid_bytes(uuid)?;
    Some(uuid)
}

/// ミリ秒単位の UNIX 時刻を `SystemTime` にする。
///
/// wasm32-unknown-unknown では `SystemTime::now()` を使えないため、Worker は `Date` の値を
/// ここに渡す。UNIX エポックより前の値はエポックに丸める。
pub fn system_time_from_millis(millis: i64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_millis(millis.max(0) as u64)
}

/// 署名付き URL の生成の設定。
pub struct SigningConfig<'a> {
    /// R2 の S3 互換エンドポイント (例: `https://<アカウント ID>.r2.cloudflarestorage.com`)。
    pub endpoint: &'a str,
    /// バケット名。
    pub bucket: &'a str,
    /// R2 の API トークンのアクセスキー。
    pub access_key_id: &'a str,
    /// R2 の API トークンのシークレット。
    pub secret_access_key: &'a str,
}

/// Content-Type を `image/jpeg`、Content-Length を申告サイズ、有効期限を指定の秒数として
/// 署名した PUT 用の URL を生成する (FR-10、ADR-0003)。
///
/// `shiguredo_s3` は Sans I/O のため通信を行わない。`now` を渡すのは署名を決定的にするためで、
/// 有効期限の判定は R2 が行い、Backend は関与しない (FR-10)。
pub fn presign_put_url(
    config: &SigningConfig<'_>,
    key: &str,
    size: i64,
    expires_seconds: u64,
    now: SystemTime,
) -> Result<String, shiguredo_s3::Error> {
    let credentials = Credentials::new(
        config.access_key_id,
        config.secret_access_key,
        None,
        None,
        "coffee-log",
    );
    let client_config = Config::builder()
        .region(REGION)
        .credentials_provider(credentials)
        .endpoint(config.endpoint)
        // R2 の S3 互換 API はパス形式のアクセスを使う (バケット名をパスに入れる)。
        .force_path_style(true)
        .build()?;
    let request = Client::from_conf(client_config)
        .put_object()
        .bucket(config.bucket)
        .key(key)
        .content_type(CONTENT_TYPE)
        .content_length(size)
        .presigned(expires_seconds, now)?;
    Ok(request.url)
}
