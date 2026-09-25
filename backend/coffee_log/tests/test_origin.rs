//! 状態を変更するメソッドの判定 (src/origin.rs) の単体テスト。
//!
//! `is_same_origin` は `worker::Request` を必要とし、ネイティブのテストでは組み立てられないため、
//! wrangler dev を起動する結合テスト (`tests/wrangler_same_origin_api.rs`) で検証する。

use coffee_log::origin::changes_state;
use coffee_log_core::routes::Method;

#[test]
fn post_put_patch_delete_change_state() {
    // 状態を変更するメソッドは `Origin` の検証の対象になる (ADR-0005)。
    for method in [Method::Post, Method::Put, Method::Patch, Method::Delete] {
        assert!(changes_state(method), "{method:?} must change state");
    }
}

#[test]
fn get_does_not_change_state() {
    // GET は `Origin` の検証の対象外とする (ブラウザは同一オリジンの GET に `Origin` を付けない)。
    assert!(!changes_state(Method::Get));
}
