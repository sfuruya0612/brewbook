//! D1 の実行と、現在時刻と UUID の共通の補助。
//!
//! SQL の組み立ては `crate::queries` が持ち、ここは文の実行と、値の D1 への変換だけを行う。
//! 利用者向けの Worker (`brew_book::db`) と同じ処理である。両者は別のクレートで、共有すると
//! `brew_book_core` が `worker` に依存してしまうため、同じ実装をここに持つ。

use brew_book_core::auth;
use brew_book_core::datetime::format_epoch_millis;
use brew_book_core::ids::uuid_v4_from_bytes;
use worker::d1::{D1Database, D1PreparedStatement, D1Type};
use worker::{console_error, Date, Env, Error, Result};

use crate::random;

/// `DB` のバインディングを取る。
pub fn database(env: &Env) -> Result<D1Database> {
    env.d1("DB")
}

/// 現在時刻の ISO 8601 UTC の固定長文字列 (ADR-0002)。
pub fn now_text() -> Result<String> {
    format_epoch_millis(Date::now().as_millis() as i64)
        .map_err(|error| Error::RustError(format!("failed to format the current time: {error:?}")))
}

/// 現在時刻から有効期限の時刻の文字列を作る。
pub fn expiry_text(ttl_seconds: i64) -> Result<String> {
    auth::expiry_from(Date::now().as_millis() as i64, ttl_seconds)
        .map_err(|error| Error::RustError(format!("failed to format the expiry: {error:?}")))
}

/// UUID v4 の文字列を作る。
pub fn new_id() -> Result<String> {
    Ok(uuid_v4_from_bytes(random_bytes_16()?))
}

/// 1 件の文を組み立て、値を束縛する。
pub fn statement(d1: &D1Database, sql: &str, values: &[D1Type<'_>]) -> Result<D1PreparedStatement> {
    d1.prepare(sql).bind_refs(values.iter())
}

/// 文の列を 1 つのまとまりとして実行する (D1 の batch は 1 つのトランザクションで実行する)。
/// どれかが失敗したら内部エラーにする。
pub async fn execute_batch(d1: &D1Database, statements: Vec<D1PreparedStatement>) -> Result<()> {
    let results = d1.batch(statements).await?;
    for result in &results {
        if !result.success() {
            let error = result.error().unwrap_or_default();
            console_error!("a batched statement failed: {error}");
            return Err(Error::RustError("a batched statement failed".to_owned()));
        }
    }
    Ok(())
}

/// 乱数の失敗を Worker のエラーにする。
pub fn random_bytes_16() -> Result<[u8; 16]> {
    random::bytes_16().map_err(Error::RustError)
}

/// 乱数の失敗を Worker のエラーにする。
pub fn random_bytes_32() -> Result<[u8; 32]> {
    random::bytes_32().map_err(Error::RustError)
}
