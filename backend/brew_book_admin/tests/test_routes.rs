//! 管理者 Worker の経路の台帳 (src/routes.rs) の単体テスト (PRD の成功指標)。
//!
//! 台帳と結合テストのスイート (`tests/wrangler_admin_api.rs`) の識別子を照合し、全経路に正常系、
//! 入力を持つ経路に入力不正 400 のテストがあることを検査する。照合の仕組みが働くことも確かめる。

mod support;

use brew_book_admin::routes::ROUTES;
use brew_book_core::routes::{Method, Route};
use support::{SuiteEntry, KIND_INVALID_INPUT_400, KIND_OK};

/// 照合の検査を試すための経路。実際の経路は `ROUTES` が持つ。
fn sample_ledger() -> Vec<Route> {
    vec![
        Route {
            name: "users_list",
            method: Method::Get,
            pattern: "/",
            auth_required: false,
            has_input: false,
        },
        Route {
            name: "users_create",
            method: Method::Post,
            pattern: "/users",
            auth_required: false,
            has_input: true,
        },
    ]
}

#[test]
fn the_ledger_and_the_suite_match() {
    support::admin_suite_covers_ledger().expect("the ledger and the suite must match");
}

#[test]
fn the_ledger_has_the_three_admin_routes_without_input_on_the_post_routes() {
    let names: Vec<&str> = ROUTES.iter().map(|route| route.name).collect();
    assert_eq!(names, ["users_list", "users_create", "tokens_create"]);
    // 管理者 API はアプリ内の認証を持たない (ADR-0008)。
    assert!(ROUTES.iter().all(|route| !route.auth_required));
    // 入力を持つのは利用者の作成だけである (表示名)。
    let with_input: Vec<&str> = ROUTES
        .iter()
        .filter(|route| route.has_input)
        .map(|route| route.name)
        .collect();
    assert_eq!(with_input, ["users_create"]);
}

#[test]
fn the_checker_detects_a_missing_test_kind() {
    let ledger = sample_ledger();
    let suite = vec![
        SuiteEntry {
            route: "users_list",
            kinds: &[KIND_OK],
        },
        SuiteEntry {
            route: "users_create",
            kinds: &[KIND_OK],
        },
    ];
    let error = support::covers(&ledger, &suite)
        .expect_err("a route without the invalid_input_400 test must be detected");
    assert!(
        error.contains("users_create"),
        "the error must name the route: {error}"
    );
}

#[test]
fn the_checker_detects_a_route_present_on_one_side_only() {
    let ledger = sample_ledger();
    let suite = vec![
        SuiteEntry {
            route: "users_list",
            kinds: &[KIND_OK],
        },
        SuiteEntry {
            route: "unknown_route",
            kinds: &[KIND_OK, KIND_INVALID_INPUT_400],
        },
    ];
    let error = support::covers(&ledger, &suite).expect_err("a route mismatch must be detected");
    assert!(
        error.contains("users_create") && error.contains("unknown_route"),
        "the error must list the mismatched routes: {error}"
    );
}

#[test]
fn the_router_registration_does_not_panic() {
    // ネイティブでは wasm のディスパッチを実行できないため、ここで確認できるのは
    // 台帳の経路の登録がパニックしないことまで。ディスパッチは結合テストが確認する。
    let _router = brew_book_admin::build_router_from(ROUTES);
}
