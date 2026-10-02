//! brewbook の Frontend (ADR-0017)。
//!
//! 画面の基盤 (経路の台帳、API クライアント、値の変換、翻訳、起動時のセッション確認) を持つ。
//! デザインシステムは 0039、画面の実装は 0040 以降が入れる。

pub mod api;
pub mod app;
pub mod auth;
pub mod i18n;
pub mod records;
pub mod router;
pub mod screens;
pub mod ui;
