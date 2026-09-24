//! `/api` の経路の台帳。
//!
//! 経路名はログとテストの識別子に使う (PRD の成功指標)。
//! PRD の成功指標の照合は、この台帳を入力にしたデータ駆動の結合テストで行う。

/// 台帳が持つ HTTP メソッド。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    /// 台帳とログで使う大文字の表記。
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
        }
    }
}

/// `/api` の経路 1 件のメタデータ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    /// ログとテストの識別子に使う経路名。
    pub name: &'static str,
    pub method: Method,
    /// `:name` のセグメントをパラメータとして扱うパス。例: `/api/shops/:id`
    pub pattern: &'static str,
    /// 認証が必要な経路は、正常系と未認証 401 のテストを持つ (PRD の成功指標)。
    pub auth_required: bool,
    /// 入力を持つ経路は、入力不正 400 のテストを持つ (PRD の成功指標)。
    pub has_input: bool,
}

/// 経路の台帳。経路は 0005 以降が追加する。
///
/// 認証が不要なのは、登録のチャレンジ発行と検証、ログインのチャレンジ発行と検証の 4 経路だけとし、
/// それ以外の利用者向けの経路はセッションを必須にする (PRD のセキュリティ)。
/// 入力を持つ経路とは、JSON の本体かクエリパラメータを読む経路を指す
/// (ログインのチャレンジ発行は入力を持たない)。
pub const ROUTES: &[Route] = &[
    Route {
        name: "auth_register_begin",
        method: Method::Post,
        pattern: "/api/auth/register/begin",
        auth_required: false,
        has_input: true,
    },
    Route {
        name: "auth_register_complete",
        method: Method::Post,
        pattern: "/api/auth/register/complete",
        auth_required: false,
        has_input: true,
    },
    Route {
        name: "auth_login_begin",
        method: Method::Post,
        pattern: "/api/auth/login/begin",
        auth_required: false,
        has_input: false,
    },
    Route {
        name: "auth_login_complete",
        method: Method::Post,
        pattern: "/api/auth/login/complete",
        auth_required: false,
        has_input: true,
    },
    Route {
        name: "auth_logout",
        method: Method::Post,
        pattern: "/api/auth/logout",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "passkeys_list",
        method: Method::Get,
        pattern: "/api/passkeys",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "passkeys_begin",
        method: Method::Post,
        pattern: "/api/passkeys/begin",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "passkeys_complete",
        method: Method::Post,
        pattern: "/api/passkeys/complete",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "passkeys_rename",
        method: Method::Patch,
        pattern: "/api/passkeys/:id",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "passkeys_delete",
        method: Method::Delete,
        pattern: "/api/passkeys/:id",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "shops_list",
        method: Method::Get,
        pattern: "/api/shops",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "shops_create",
        method: Method::Post,
        pattern: "/api/shops",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "shops_get",
        method: Method::Get,
        pattern: "/api/shops/:id",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "shops_update",
        method: Method::Patch,
        pattern: "/api/shops/:id",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "shops_archive",
        method: Method::Post,
        pattern: "/api/shops/:id/archive",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "shops_unarchive",
        method: Method::Post,
        pattern: "/api/shops/:id/unarchive",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "products_list",
        method: Method::Get,
        pattern: "/api/products",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "products_create",
        method: Method::Post,
        pattern: "/api/products",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "products_get",
        method: Method::Get,
        pattern: "/api/products/:id",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "products_update",
        method: Method::Patch,
        pattern: "/api/products/:id",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "products_archive",
        method: Method::Post,
        pattern: "/api/products/:id/archive",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "products_unarchive",
        method: Method::Post,
        pattern: "/api/products/:id/unarchive",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "flavor_tags_list",
        method: Method::Get,
        pattern: "/api/flavor-tags",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "purchases_list",
        method: Method::Get,
        pattern: "/api/purchases",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "purchases_create",
        method: Method::Post,
        pattern: "/api/purchases",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "purchases_get",
        method: Method::Get,
        pattern: "/api/purchases/:id",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "purchases_update",
        method: Method::Patch,
        pattern: "/api/purchases/:id",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "purchases_archive",
        method: Method::Post,
        pattern: "/api/purchases/:id/archive",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "purchases_unarchive",
        method: Method::Post,
        pattern: "/api/purchases/:id/unarchive",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "purchases_photo_upload_url",
        method: Method::Post,
        pattern: "/api/purchases/:id/photo/upload-url",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "purchases_photo_complete",
        method: Method::Post,
        pattern: "/api/purchases/:id/photo",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "purchases_photo_get",
        method: Method::Get,
        pattern: "/api/purchases/:id/photo",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "purchases_photo_delete",
        method: Method::Delete,
        pattern: "/api/purchases/:id/photo",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "brews_list",
        method: Method::Get,
        pattern: "/api/brews",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "brews_create",
        method: Method::Post,
        pattern: "/api/brews",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "brews_get",
        method: Method::Get,
        pattern: "/api/brews/:id",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "brews_update",
        method: Method::Patch,
        pattern: "/api/brews/:id",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "brews_archive",
        method: Method::Post,
        pattern: "/api/brews/:id/archive",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "brews_unarchive",
        method: Method::Post,
        pattern: "/api/brews/:id/unarchive",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "suggestions_list",
        method: Method::Get,
        pattern: "/api/suggestions/:field",
        auth_required: true,
        // 入力中の文字列 (`q`) を読む (項目名は経路のパラメータであり、入力には数えない)。
        has_input: true,
    },
    Route {
        name: "stats_brews",
        method: Method::Get,
        pattern: "/api/stats/brews",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "stats_purchases",
        method: Method::Get,
        pattern: "/api/stats/purchases",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "stats_brew_ratings",
        method: Method::Get,
        pattern: "/api/stats/brew-ratings",
        auth_required: true,
        has_input: true,
    },
    Route {
        name: "purchases_rating_history",
        method: Method::Get,
        pattern: "/api/purchases/:id/rating-history",
        auth_required: true,
        has_input: false,
    },
    Route {
        name: "export_get",
        method: Method::Get,
        pattern: "/api/export",
        auth_required: true,
        has_input: false,
    },
];

/// 経路が一致しなかったリクエストのログに使う経路名。
pub const NOT_FOUND_ROUTE: &str = "not_found";

/// パスがパターンに一致するかを判定する。
/// `:` で始まるセグメントは空でない 1 セグメントに一致し、それ以外は文字列が一致する必要がある。
pub fn pattern_matches(pattern: &str, path: &str) -> bool {
    let pattern_segments: Vec<&str> = pattern.split('/').collect();
    let path_segments: Vec<&str> = path.split('/').collect();
    if pattern_segments.len() != path_segments.len() {
        return false;
    }
    pattern_segments
        .iter()
        .zip(path_segments.iter())
        .all(|(pattern_segment, path_segment)| {
            if let Some(_parameter) = pattern_segment.strip_prefix(':') {
                !path_segment.is_empty()
            } else {
                pattern_segment == path_segment
            }
        })
}

/// メソッドとパスに一致する経路を台帳から探す。一致が複数ある場合は先頭を返す。
pub fn match_route<'a>(routes: &'a [Route], method: &str, path: &str) -> Option<&'a Route> {
    routes
        .iter()
        .find(|route| route.method.as_str() == method && pattern_matches(route.pattern, path))
}

/// 台帳全体からメソッドとパスに一致する経路を探す。
pub fn matched(method: &str, path: &str) -> Option<&'static Route> {
    match_route(ROUTES, method, path)
}

/// 経路が持つべきテストの種別。PRD の成功指標の照合に使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TestRequirements {
    /// 正常系のテスト。全ての経路が持つ。
    pub ok: bool,
    /// 未認証 401 のテスト。認証が必要な経路だけが持つ。
    pub unauthenticated_401: bool,
    /// 入力不正 400 のテスト。入力を持つ経路だけが持つ。
    pub invalid_input_400: bool,
}

/// 成功指標に基づき、経路が持つべきテストの種別を返す。
pub fn test_requirements(route: &Route) -> TestRequirements {
    TestRequirements {
        ok: true,
        unauthenticated_401: route.auth_required,
        invalid_input_400: route.has_input,
    }
}
