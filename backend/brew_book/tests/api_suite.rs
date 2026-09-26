mod support;

use brew_book_core::routes::{Method, Route};

/// 台帳とスイートの照合の検査を試すための経路。実際の経路は 0005 以降が追加する。
/// 5 つのメソッドと、認証と入力の 4 通りの組み合わせを含める。
fn sample_ledger() -> Vec<Route> {
    vec![
        Route {
            name: "shops_list",
            method: Method::Get,
            pattern: "/api/shops",
            auth_required: true,
            has_input: false,
        },
        Route {
            name: "shops_create",
            method: Method::Post,
            pattern: "/api/shops",
            auth_required: true,
            has_input: true,
        },
        Route {
            name: "shops_update",
            method: Method::Put,
            pattern: "/api/shops/:id",
            auth_required: true,
            has_input: true,
        },
        Route {
            name: "shops_archive",
            method: Method::Patch,
            pattern: "/api/shops/:id",
            auth_required: true,
            has_input: false,
        },
        Route {
            name: "passkeys_delete",
            method: Method::Delete,
            pattern: "/api/passkeys/:id",
            auth_required: true,
            has_input: false,
        },
        Route {
            name: "auth_challenge",
            method: Method::Post,
            pattern: "/api/auth/challenge",
            auth_required: false,
            has_input: false,
        },
        Route {
            name: "auth_verify",
            method: Method::Post,
            pattern: "/api/auth/login/verify",
            auth_required: false,
            has_input: true,
        },
    ]
}

/// `sample_ledger` の全経路に必要な種別を過不足なく持つスイート。
fn sample_suite() -> Vec<support::SuiteEntry> {
    use support::{
        KIND_INVALID_INPUT_400 as INPUT, KIND_OK as OK, KIND_UNAUTHENTICATED_401 as UNAUTH,
    };
    vec![
        support::SuiteEntry {
            route: "shops_list",
            kinds: &[OK, UNAUTH],
        },
        support::SuiteEntry {
            route: "shops_create",
            kinds: &[OK, UNAUTH, INPUT],
        },
        support::SuiteEntry {
            route: "shops_update",
            kinds: &[OK, UNAUTH, INPUT],
        },
        support::SuiteEntry {
            route: "shops_archive",
            kinds: &[OK, UNAUTH],
        },
        support::SuiteEntry {
            route: "passkeys_delete",
            kinds: &[OK, UNAUTH],
        },
        support::SuiteEntry {
            route: "auth_challenge",
            kinds: &[OK],
        },
        support::SuiteEntry {
            route: "auth_verify",
            kinds: &[OK, INPUT],
        },
    ]
}

#[test]
fn the_ledger_and_the_suite_match() {
    support::suite_covers_ledger().expect("the ledger and the suite must match");
}

#[test]
fn the_checker_accepts_a_complete_suite() {
    support::covers(&sample_ledger(), &sample_suite())
        .expect("a suite with every required kind must be accepted");
}

#[test]
fn the_checker_detects_a_missing_test_kind() {
    let ledger = sample_ledger();
    let mut suite = sample_suite();
    let entry = suite
        .iter_mut()
        .find(|entry| entry.route == "shops_create")
        .expect("the sample suite must have shops_create");
    entry.kinds = &[support::KIND_OK, support::KIND_UNAUTHENTICATED_401];
    let error = support::covers(&ledger, &suite)
        .expect_err("a route without the invalid_input_400 test must be detected");
    assert!(
        error.contains("shops_create"),
        "the error must name the route: {error}"
    );
}

#[test]
fn the_checker_detects_a_route_present_on_one_side_only() {
    let ledger = sample_ledger();
    // 台帳に無い経路をスイートだけが持つ場合も検出する。
    let suite = vec![
        support::SuiteEntry {
            route: "auth_challenge",
            kinds: &[support::KIND_OK],
        },
        support::SuiteEntry {
            route: "unknown_route",
            kinds: &[support::KIND_OK],
        },
    ];
    let error = support::covers(&ledger, &suite).expect_err("a route mismatch must be detected");
    assert!(
        error.contains("shops_list") && error.contains("unknown_route"),
        "the error must list the mismatched routes: {error}"
    );
}

#[test]
fn the_router_registration_does_not_panic_for_a_ledger_with_every_method() {
    // ネイティブでは wasm のディスパッチを実行できないため、ここで確認できるのは
    // 5 つのメソッドの登録がパニックしないことまで。ディスパッチは 0005 以降の結合テストが確認する。
    let ledger = sample_ledger();
    let _router = brew_book::build_router_from(&ledger);
}
