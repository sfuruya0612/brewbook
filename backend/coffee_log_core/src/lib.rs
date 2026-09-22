//! 利用者向けの Worker と管理者 Worker が共有するロジック。
//!
//! wasm に依存させず、ネイティブターゲットでテストする (ADR-0001、ADR-0002)。

pub mod cursor;
pub mod datetime;
pub mod error;
pub mod ids;
pub mod query;
pub mod routes;
