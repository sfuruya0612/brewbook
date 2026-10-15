//! 経路の台帳とルーターの単体テスト (ADR-0017)。
//!
//! 台帳の一覧は、移行前の Flutter 版 (frontend/lib/router/app_router.dart。0045 で削除) の
//! `AppRoutes` と同じ内容を固定した期待値と照合する。

use std::str::FromStr;

use brew_book_frontend::router::{fallback_destination, route_name, AppRoute, Route, APP_ROUTES};

/// 移行前の Flutter 版の 18 経路から商品の 3 経路を除いた 15 経路 (名前、パターン。ADR-0022)。
/// 名前はパターンと同じ。
const FLUTTER_ROUTES: [(&str, &str); 15] = [
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
    ("/shops", "/shops"),
    ("/shops/new", "/shops/new"),
    ("/shops/:id/edit", "/shops/:id/edit"),
    ("/stats", "/stats"),
];

#[test]
fn the_ledger_has_the_15_routes_of_flutter_without_the_products() {
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

/// 購入の登録の経路は、引き継ぐ元の購入の ID をクエリから取る (issue 0058)。
#[test]
fn the_purchase_new_route_takes_the_source_purchase_from_the_query() {
    assert_eq!(
        Route::from_str("/purchases/new?from=p1").expect("the route must be parsed"),
        Route::PurchaseNew {
            from: Some("p1".to_string())
        }
    );
    assert_eq!(
        Route::from_str("/purchases/new").expect("the route must be parsed"),
        Route::PurchaseNew { from: None }
    );
    // 値の無いクエリは空の文字になる (フォームの側で引き継ぐ元が無いものとして扱う)。
    assert_eq!(
        Route::from_str("/purchases/new?from=").expect("the route must be parsed"),
        Route::PurchaseNew {
            from: Some(String::new())
        }
    );
    assert_eq!(
        Route::PurchaseNew {
            from: Some("p1".to_string())
        }
        .to_string(),
        "/purchases/new?from=p1"
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

#[test]
fn every_route_has_the_pattern_as_its_data_route_name() {
    // 画面数の成功指標 (PRD の成功指標) で数える `data-route` の名前は、経路のパターンと同じに
    // する (0044)。動的な経路もパターンの形にする。
    let id = || "x".to_string();
    let routes = [
        (Route::Login {}, "/login"),
        (Route::Register { token: None }, "/register"),
        (Route::Register { token: Some(id()) }, "/register"),
        (Route::Home {}, "/"),
        (Route::Settings {}, "/settings"),
        (Route::BrewNew {}, "/brews/new"),
        (Route::BrewDetail { id: id() }, "/brews/:id"),
        (Route::BrewEdit { id: id() }, "/brews/:id/edit"),
        (Route::Purchases {}, "/purchases"),
        (Route::PurchaseNew { from: None }, "/purchases/new"),
        (Route::PurchaseNew { from: Some(id()) }, "/purchases/new"),
        (Route::PurchaseDetail { id: id() }, "/purchases/:id"),
        (Route::PurchaseEdit { id: id() }, "/purchases/:id/edit"),
        (Route::Shops {}, "/shops"),
        (Route::ShopNew {}, "/shops/new"),
        (Route::ShopEdit { id: id() }, "/shops/:id/edit"),
        (Route::Stats {}, "/stats"),
        (
            Route::NotFound {
                segments: vec!["unknown".to_string()],
            },
            "/:..segments",
        ),
    ];
    let mut names: Vec<&'static str> = Vec::new();
    for (route, expected) in routes {
        assert_eq!(route_name(&route), expected, "the data-route of {route:?}");
        names.push(expected);
    }
    // 台帳の 15 経路の名前が全て現れる。
    for route in APP_ROUTES {
        assert!(
            names.contains(&route.name),
            "the data-route {} must exist",
            route.name
        );
    }
}
