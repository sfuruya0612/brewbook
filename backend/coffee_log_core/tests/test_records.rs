//! `records` の単体テスト。入力の誤りと応答の対応を確認する。
//!
//! 値の扱いの性質 (前後の空白の除去、空白だけの値の拒否、タグ名の正規化) は PBT
//! (`prop_records.rs`) が担う。

use coffee_log_core::records::{NameError, TagNameError};

#[test]
fn every_input_error_maps_to_a_bad_request() {
    assert_eq!(NameError::Empty.code().status(), 400);
    assert_eq!(TagNameError::Empty.code().status(), 400);
    assert!(!NameError::Empty.message().is_empty());
    assert!(!TagNameError::Empty.message().is_empty());
}
