//! 状態を変更するメソッドの判定と、`Origin` の比較 (src/origin.rs) の単体テスト。
//!
//! `is_same_origin` は `worker::Request` を必要とし、ネイティブのテストでは組み立てられないため、
//! ヘッダと URL の文字列を受け取る `is_same_origin_value` を直接検査する。
//! `wrangler dev` の結合テスト (`tests/wrangler_same_origin_api.rs`) は、実際のリクエストでの検証を担う。

use coffee_log::origin::{changes_state, is_same_origin_value};
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

#[test]
fn same_origin_value_passes() {
    // scheme、host、port がすべて同じなら同一オリジンである。
    assert!(is_same_origin_value(
        Some("http://localhost:8787"),
        "http://localhost:8787/api/shops"
    )
    .unwrap());
    // 既定のポートは `Origin` にも URL にも表記されない。
    assert!(is_same_origin_value(
        Some("https://coffee-log.example.workers.dev"),
        "https://coffee-log.example.workers.dev/api/shops"
    )
    .unwrap());
}

#[test]
fn another_scheme_or_host_or_port_is_not_same_origin() {
    // scheme が違えば別のオリジンである (http と https は別)。
    assert!(!is_same_origin_value(
        Some("https://localhost:8787"),
        "http://localhost:8787/api/shops"
    )
    .unwrap());
    // host が違えば別のオリジンである。
    assert!(!is_same_origin_value(
        Some("https://evil.example"),
        "https://coffee-log.example.workers.dev/api/shops"
    )
    .unwrap());
    // 同じ host でも port が違えば別のオリジンである。
    assert!(!is_same_origin_value(
        Some("http://localhost:1"),
        "http://localhost:8787/api/shops"
    )
    .unwrap());
}

#[test]
fn the_default_port_is_normalized() {
    // 既定のポート (https の 443) は、表記に含めても含めなくても同じオリジンである。
    assert!(is_same_origin_value(
        Some("https://example.com:443"),
        "https://example.com/api/shops"
    )
    .unwrap());
    assert!(is_same_origin_value(
        Some("https://example.com"),
        "https://example.com:443/api/shops"
    )
    .unwrap());
    // 既定でないポートは省略できない (省略すると別のオリジンになる)。
    assert!(!is_same_origin_value(
        Some("https://example.com"),
        "https://example.com:8443/api/shops"
    )
    .unwrap());
}

#[test]
fn missing_origin_is_not_same_origin() {
    // `Origin` が無いリクエストは、同じオリジンからのものかを検証できないため一致しない扱いにする。
    assert!(
        !is_same_origin_value(None, "https://coffee-log.example.workers.dev/api/shops").unwrap()
    );
}

#[test]
fn an_invalid_origin_is_not_same_origin() {
    // `Origin` が URL として読めない場合は、一致しない扱いにする (403)。
    assert!(!is_same_origin_value(Some("not a url"), "https://example.com/api/shops").unwrap());
}

#[test]
fn an_invalid_request_url_is_an_error() {
    // リクエストの URL が URL として読めない場合は、検証の失敗として扱う (呼び出し元が 500 にする)。
    assert!(is_same_origin_value(Some("https://example.com"), "not a url").is_err());
}
