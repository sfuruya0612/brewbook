//! 経路の台帳とルーターの単体テスト (ADR-0017)。
//!
//! 台帳の一覧は Flutter の `frontend/lib/router/app_router.dart` の `AppRoutes` と照合する。

use std::str::FromStr;

use brew_book_frontend::router::{fallback_destination, AppRoute, Route, APP_ROUTES};

/// Flutter の `AppRoutes` と同じ 18 経路 (名前、パターン)。名前はパターンと同じ。
const FLUTTER_ROUTES: [(&str, &str); 18] = [
    ("/login", "/login"),
    ("/register", "/register"),
    ("/", "/"),
    ("/settings", "/settings"),
    ("/brews/new", "/brews/new"),
    ("/brews/:id", "/brews/:id"),
    ("/brews/:id/edit", "/brews/:id/edit"),
    ("/purchases", "/purchases"),
    ("/purchases/new", "/purchases/new"),
    ("/purchases/:id", "/purchases/:id"),
    ("/purchases/:id/edit", "/purchases/:id/edit"),
    ("/products", "/products"),
    ("/products/new", "/products/new"),
    ("/products/:id/edit", "/products/:id/edit"),
    ("/shops", "/shops"),
    ("/shops/new", "/shops/new"),
    ("/shops/:id/edit", "/shops/:id/edit"),
    ("/stats", "/stats"),
];

#[test]
fn the_ledger_has_the_same_18_routes_as_flutter() {
    assert_eq!(APP_ROUTES.len(), FLUTTER_ROUTES.len());
    for (index, (name, pattern)) in FLUTTER_ROUTES.iter().enumerate() {
        let route = APP_ROUTES[index];
        assert_eq!(route.name, *name, "the name of the route at {index}");
        assert_eq!(
            route.pattern, *pattern,
            "the pattern of the route at {index}"
        );
    }
}

#[test]
fn a_route_name_is_the_same_as_its_pattern() {
    for AppRoute { name, pattern } in APP_ROUTES {
        assert_eq!(name, pattern);
    }
}

#[test]
fn the_patterns_are_unique() {
    let mut patterns: Vec<&str> = APP_ROUTES.iter().map(|route| route.pattern).collect();
    patterns.sort_unstable();
    patterns.dedup();
    assert_eq!(patterns.len(), APP_ROUTES.len());
}

#[test]
fn every_static_route_is_registered_in_the_router() {
    for route in APP_ROUTES {
        if route.pattern.contains(':') {
            continue;
        }
        let parsed = Route::from_str(route.pattern)
            .unwrap_or_else(|_| panic!("{} must be registered", route.pattern));
        // Dioxus のルーターはクエリを持つ経路の表示に `?` を付ける (`/register?`)。
        // 台帳のパターンは Flutter と同じくクエリの印を除いた形にする。
        assert_eq!(parsed.to_string().trim_end_matches('?'), route.pattern);
    }
}

#[test]
fn a_dynamic_route_is_parsed_with_its_value() {
    assert_eq!(
        Route::from_str("/brews/abc").expect("the route must be parsed"),
        Route::BrewDetail {
            id: "abc".to_string()
        }
    );
    assert_eq!(
        Route::from_str("/brews/abc/edit").expect("the route must be parsed"),
        Route::BrewEdit {
            id: "abc".to_string()
        }
    );
    assert_eq!(
        Route::from_str("/purchases/abc").expect("the route must be parsed"),
        Route::PurchaseDetail {
            id: "abc".to_string()
        }
    );
    assert_eq!(
        Route::from_str("/purchases/abc/edit").expect("the route must be parsed"),
        Route::PurchaseEdit {
            id: "abc".to_string()
        }
    );
    assert_eq!(
        Route::from_str("/products/abc/edit").expect("the route must be parsed"),
        Route::ProductEdit {
            id: "abc".to_string()
        }
    );
    assert_eq!(
        Route::from_str("/shops/abc/edit").expect("the route must be parsed"),
        Route::ShopEdit {
            id: "abc".to_string()
        }
    );
}

#[test]
fn the_register_route_takes_the_token_from_the_query() {
    assert_eq!(
        Route::from_str("/register?token=xyz").expect("the route must be parsed"),
        Route::Register {
            token: Some("xyz".to_string())
        }
    );
    assert_eq!(
        Route::from_str("/register").expect("the route must be parsed"),
        Route::Register { token: None }
    );
}

#[test]
fn an_unknown_path_falls_back_to_the_home() {
    let route = Route::from_str("/unknown/path").expect("the fallback must accept any path");
    assert!(matches!(route, Route::NotFound { .. }));
    // 未知の経路の遷移先はホーム (Flutter の go_router の既定のエラー画面からの意図的な変更)。
    assert_eq!(fallback_destination(), Route::Home {});
}

#[test]
fn a_path_that_only_differs_in_the_last_segment_falls_back() {
    let route = Route::from_str("/brews/abc/unknown").expect("the fallback must accept any path");
    assert!(matches!(route, Route::NotFound { .. }));
}
