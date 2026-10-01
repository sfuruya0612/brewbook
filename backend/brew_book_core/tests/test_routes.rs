use brew_book_core::routes::{
    match_route, pattern_matches, test_requirements, Method, OkTest, Route, TestRequirements,
    ROUTES,
};

fn sample_routes() -> Vec<Route> {
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
            name: "shops_show",
            method: Method::Get,
            pattern: "/api/shops/:id",
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
            name: "purchase_suggestions",
            method: Method::Post,
            pattern: "/api/purchase-suggestions",
            auth_required: true,
            has_input: true,
            ok_test: OkTest::Manual,
        },
    ]
}

#[test]
fn a_parameter_segment_does_not_match_an_empty_segment() {
    assert!(!pattern_matches("/api/shops/:id", "/api/shops/"));
}

#[test]
fn match_route_uses_both_method_and_path() {
    let routes = sample_routes();
    assert_eq!(
        match_route(&routes, "GET", "/api/shops").map(|route| route.name),
        Some("shops_list")
    );
    assert_eq!(
        match_route(&routes, "POST", "/api/shops").map(|route| route.name),
        Some("shops_create")
    );
    assert_eq!(
        match_route(&routes, "GET", "/api/shops/42").map(|route| route.name),
        Some("shops_show")
    );
    assert_eq!(match_route(&routes, "DELETE", "/api/shops"), None);
    assert_eq!(match_route(&routes, "GET", "/api/unknown"), None);
}

#[test]
fn the_ledger_route_names_are_unique() {
    let mut names: Vec<&str> = ROUTES.iter().map(|route| route.name).collect();
    names.sort_unstable();
    let count = names.len();
    names.dedup();
    assert_eq!(names.len(), count);
}

#[test]
fn the_ledger_patterns_start_with_api() {
    for route in ROUTES {
        assert!(
            route.pattern.starts_with("/api/"),
            "route {} has a pattern outside /api: {}",
            route.name,
            route.pattern
        );
    }
}

#[test]
fn test_requirements_follow_the_auth_and_input_flags() {
    let routes = sample_routes();
    assert_eq!(
        test_requirements(&routes[0]),
        TestRequirements {
            ok: OkTest::Ci,
            unauthenticated_401: true,
            invalid_input_400: false,
        }
    );
    assert_eq!(
        test_requirements(&routes[1]),
        TestRequirements {
            ok: OkTest::Ci,
            unauthenticated_401: true,
            invalid_input_400: true,
        }
    );
    // 認証が不要で入力を持たない経路 (PRD のログインのチャレンジ発行) は正常系のテストだけを持つ。
    assert_eq!(
        test_requirements(&routes[3]),
        TestRequirements {
            ok: OkTest::Ci,
            unauthenticated_401: false,
            invalid_input_400: false,
        }
    );
    // 認証が不要で入力を持つ経路 (PRD の登録用トークンによる登録とログインの検証) は
    // 正常系と入力不正 400 のテストを持つ。
    assert_eq!(
        test_requirements(&routes[4]),
        TestRequirements {
            ok: OkTest::Ci,
            unauthenticated_401: false,
            invalid_input_400: true,
        }
    );
    // 正常系を CI で実行できない経路 (写真からの推測。FR-19) は、手元で確認する区分になる。
    assert_eq!(
        test_requirements(&routes[5]),
        TestRequirements {
            ok: OkTest::Manual,
            unauthenticated_401: true,
            invalid_input_400: true,
        }
    );
}

#[test]
fn the_ledger_marks_the_photo_suggestion_route_as_a_manual_check() {
    // 写真からの推測の正常系は Workers AI の推論を要するため、CI の照合の対象外にする (FR-19)。
    let route = ROUTES
        .iter()
        .find(|route| route.name == "purchase_suggestions")
        .expect("the ledger must have the purchase suggestions route");
    assert_eq!(route.ok_test, OkTest::Manual);
    assert_eq!(route.pattern, "/api/purchase-suggestions");
    assert_eq!(route.method, Method::Post);
    assert!(route.auth_required);
    assert!(route.has_input);
    // それ以外の経路は CI で正常系を実行する。
    for route in ROUTES {
        if route.name != "purchase_suggestions" {
            assert_eq!(
                route.ok_test,
                OkTest::Ci,
                "route {} must run its ok test in CI",
                route.name
            );
        }
    }
}
