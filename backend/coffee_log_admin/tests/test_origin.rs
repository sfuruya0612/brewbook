//! 管理者 Worker の `Origin` の検証 (src/origin.rs) の単体テスト。
//!
//! `is_same_origin` は `worker::Request` を必要とし、ネイティブのテストでは組み立てられないため、
//! ヘッダと URL の文字列を受け取る `is_same_origin_value` を直接検査する。
//! `wrangler dev` の結合テスト (`tests/wrangler_admin_api.rs`) は、実際のリクエストでの検証を担う。

use coffee_log_admin::origin::{changes_state, is_same_origin_value};
use coffee_log_core::routes::Method;

#[test]
fn post_put_patch_delete_change_state() {
    // 状態を変更するメソッドは `Origin` の検証の対象になる (ADR-0005 と同じ規則)。
    for method in [Method::Post, Method::Put, Method::Patch, Method::Delete] {
        assert!(changes_state(method), "{method:?} must change state");
    }
}

#[test]
fn get_does_not_change_state() {
    // GET は `Origin` の検証の対象外とする (ブラウザは同一オリジンの GET に `Origin` を付けない)。
    assert!(!changes_state(Method::Get));
}

#[test]
fn same_origin_value_passes() {
    assert!(
        is_same_origin_value(Some("http://127.0.0.1:8787"), "http://127.0.0.1:8787/users").unwrap()
    );
    // 既定のポートは `Origin` にも URL にも表記されない。
    assert!(is_same_origin_value(
        Some("https://coffee-log-admin.example.workers.dev"),
        "https://coffee-log-admin.example.workers.dev/users"
    )
    .unwrap());
}

#[test]
fn another_scheme_or_host_or_port_is_not_same_origin() {
    // scheme が違えば別のオリジンである (http と https は別)。
    assert!(!is_same_origin_value(
        Some("https://127.0.0.1:8787"),
        "http://127.0.0.1:8787/users"
    )
    .unwrap());
    assert!(!is_same_origin_value(
        Some("https://evil.example"),
        "https://coffee-log-admin.example.workers.dev/users"
    )
    .unwrap());
    // 同じ host でも port が違えば別のオリジンである。
    assert!(
        !is_same_origin_value(Some("http://127.0.0.1:1"), "http://127.0.0.1:8787/users").unwrap()
    );
}

#[test]
fn missing_origin_is_not_same_origin() {
    // `Origin` が無いリクエストは、同じオリジンからのものかを検証できないため一致しない扱いにする。
    assert!(
        !is_same_origin_value(None, "https://coffee-log-admin.example.workers.dev/users").unwrap()
    );
}

#[test]
fn an_invalid_origin_is_not_same_origin() {
    // `Origin` が URL として読めない場合は、一致しない扱いにする (403)。
    assert!(!is_same_origin_value(Some("not a url"), "https://example.com/users").unwrap());
}

#[test]
fn an_invalid_request_url_is_an_error() {
    // リクエストの URL が URL として読めない場合は、検証の失敗として扱う (呼び出し元が 500 にする)。
    assert!(is_same_origin_value(Some("https://example.com"), "not a url").is_err());
}
