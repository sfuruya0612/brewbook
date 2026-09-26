//! 利用者向けの Worker と管理者 Worker が共有するロジック。
//!
//! wasm に依存させず、ネイティブターゲットでテストする (ADR-0001、ADR-0002)。

pub mod auth;
pub mod base64url;
pub mod cbor;
pub mod cose;
pub mod cursor;
pub mod datetime;
pub mod error;
pub mod ids;
pub mod photo;
pub mod query;
pub mod records;
pub mod routes;
pub mod stats;
pub mod webauthn;
