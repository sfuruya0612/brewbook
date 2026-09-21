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
pub const ROUTES: &[Route] = &[];

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
