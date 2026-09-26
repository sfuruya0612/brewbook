use coffee_log_core::routes::{
    match_route, pattern_matches, test_requirements, Method, Route, TestRequirements, ROUTES,
};

fn sample_routes() -> Vec<Route> {
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
            name: "shops_show",
            method: Method::Get,
            pattern: "/api/shops/:id",
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
            ok: true,
            unauthenticated_401: true,
            invalid_input_400: false,
        }
    );
    assert_eq!(
        test_requirements(&routes[1]),
        TestRequirements {
            ok: true,
            unauthenticated_401: true,
            invalid_input_400: true,
        }
    );
    // 認証が不要で入力を持たない経路 (PRD のログインのチャレンジ発行) は正常系のテストだけを持つ。
    assert_eq!(
        test_requirements(&routes[3]),
        TestRequirements {
            ok: true,
            unauthenticated_401: false,
            invalid_input_400: false,
        }
    );
    // 認証が不要で入力を持つ経路 (PRD の登録用トークンによる登録とログインの検証) は
    // 正常系と入力不正 400 のテストを持つ。
    assert_eq!(
        test_requirements(&routes[4]),
        TestRequirements {
            ok: true,
            unauthenticated_401: false,
            invalid_input_400: true,
        }
    );
}
