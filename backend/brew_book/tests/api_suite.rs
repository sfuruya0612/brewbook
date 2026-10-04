mod support;

use brew_book_core::routes::{Method, OkTest, Route};

/// 台帳とスイートの照合の検査を試すための経路。実際の経路は 0005 以降が追加する。
/// 5 つのメソッドと、認証と入力の 4 通りの組み合わせと、正常系を staging で確認する区分を含める。
fn sample_ledger() -> Vec<Route> {
    vec![
        Route {
            name: "shops_list",
            method: Method::Get,
            pattern: "/api/shops",
            auth_required: true,
            has_input: false,
            ok_test: OkTest::Ci,
        },
        Route {
            name: "shops_create",
            method: Method::Post,
            pattern: "/api/shops",
            auth_required: true,
            has_input: true,
            ok_test: OkTest::Ci,
        },
        Route {
            name: "shops_update",
            method: Method::Put,
            pattern: "/api/shops/:id",
            auth_required: true,
            has_input: true,
            ok_test: OkTest::Ci,
        },
        Route {
            name: "passkeys_rename",
            method: Method::Patch,
            pattern: "/api/passkeys/:id",
            auth_required: true,
            has_input: true,
            ok_test: OkTest::Ci,
        },
        Route {
            name: "passkeys_delete",
            method: Method::Delete,
            pattern: "/api/passkeys/:id",
            auth_required: true,
            has_input: false,
            ok_test: OkTest::Ci,
        },
        Route {
            name: "auth_challenge",
            method: Method::Post,
            pattern: "/api/auth/challenge",
            auth_required: false,
            has_input: false,
            ok_test: OkTest::Ci,
        },
        Route {
            name: "auth_verify",
            method: Method::Post,
            pattern: "/api/auth/login/verify",
            auth_required: false,
            has_input: true,
            ok_test: OkTest::Ci,
        },
        Route {
            // 正常系を CI で実行できない経路 (写真からの推測。FR-19。staging で確認する)。
            name: "purchase_suggestions",
            method: Method::Post,
            pattern: "/api/purchase-suggestions",
            auth_required: true,
            has_input: true,
            ok_test: OkTest::Staging,
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
            route: "passkeys_rename",
            kinds: &[OK, UNAUTH, INPUT],
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
        support::SuiteEntry {
            // 正常系を staging で確認する経路は、正常系の種別を持たない (FR-19)。
            route: "purchase_suggestions",
            kinds: &[UNAUTH, INPUT],
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
fn the_checker_rejects_an_ok_test_for_a_route_that_ci_cannot_run() {
    // 正常系を staging で確認する経路 (FR-19) に正常系の種別を付けると、CI で実行できない
    // テストを要求したことになるため検出する。
    let ledger = sample_ledger();
    let mut suite = sample_suite();
    let entry = suite
        .iter_mut()
        .find(|entry| entry.route == "purchase_suggestions")
        .expect("the sample suite must have purchase_suggestions");
    entry.kinds = &[
        support::KIND_OK,
        support::KIND_UNAUTHENTICATED_401,
        support::KIND_INVALID_INPUT_400,
    ];
    let error = support::covers(&ledger, &suite)
        .expect_err("an ok test for a manual route must be detected");
    assert!(
        error.contains("purchase_suggestions"),
        "the error must name the route: {error}"
    );
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
