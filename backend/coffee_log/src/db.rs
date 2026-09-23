//! D1 の実行と、現在時刻と UUID の共通の補助。
//!
//! 認証 (0005) と記録の API (0006) が共有する。SQL の組み立ては `coffee_log_core::query` が持ち、
//! ここは組み立てた文の実行と、値の D1 への変換だけを行う。

use coffee_log_core::datetime::format_epoch_millis;
use coffee_log_core::ids::uuid_v4_from_bytes;
use coffee_log_core::query::{Statement, Value};
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

/// UUID v4 の文字列を作る。
pub fn new_id() -> Result<String> {
    Ok(uuid_v4_from_bytes(random_bytes_16()?))
}

/// 1 件の文を組み立て、値を束縛する。
pub fn statement(d1: &D1Database, sql: &str, values: &[D1Type<'_>]) -> Result<D1PreparedStatement> {
    d1.prepare(sql).bind_refs(values.iter())
}

/// 組み立てたクエリを、値を束縛した D1 の文にする。
pub fn prepared(d1: &D1Database, query: &Statement) -> Result<D1PreparedStatement> {
    let values = d1_values(&query.params)?;
    statement(d1, &query.sql, &values)
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

/// 1 件の文を実行し、変更した行数を返す。
pub async fn execute_changes(d1: &D1Database, sql: &str, values: &[D1Type<'_>]) -> Result<usize> {
    let result = statement(d1, sql, values)?.run().await?;
    Ok(result.meta()?.and_then(|meta| meta.changes).unwrap_or(0))
}

/// 乱数の失敗を Worker のエラーにする。
pub fn random_bytes_16() -> Result<[u8; 16]> {
    random::bytes_16().map_err(Error::RustError)
}

/// 組み立てた値の並びを D1 の値にする。
fn d1_values(values: &[Value]) -> Result<Vec<D1Type<'_>>> {
    values.iter().map(d1_value).collect()
}

/// 組み立てた値を D1 の値にする。D1 の整数は 32 ビットまでとする。
fn d1_value(value: &Value) -> Result<D1Type<'_>> {
    Ok(match value {
        Value::Null => D1Type::Null,
        Value::Text(text) => D1Type::Text(text),
        Value::Integer(number) => D1Type::Integer(
            i32::try_from(*number)
                .map_err(|_| Error::RustError(format!("the value {number} does not fit in D1")))?,
        ),
    })
}
