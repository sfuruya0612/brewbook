//! 利用者向けの Worker。`/api/*` を Router で処理し、リクエストごとにログを出す。

use coffee_log_core::error::ErrorCode;
use coffee_log_core::routes::{matched, Method, Route, NOT_FOUND_ROUTE, ROUTES};
use worker::{
    console_error, event, Context, Date, Env, Request, Response, Result, RouteContext, Router,
};

pub mod auth;
pub mod d1_check;
pub mod db;
pub mod logging;
pub mod random;
pub mod records;
pub mod respond;
pub mod test_page;

#[event(fetch)]
pub async fn main(mut req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let started_at_ms = Date::now().as_millis();
    let method = req.method().to_string();
    let path = req.path();

    let (route_name, status, response) = match matched(&method, &path) {
        Some(route) => {
            let (status, response) = run_router(req, env).await;
            (route.name, status, response)
        }
        // 台帳に無い経路は、テスト専用の経路だけを処理する。
        None => unmatched_route(&mut req, &env).await,
    };

    let duration_ms = Date::now().as_millis().saturating_sub(started_at_ms);
    logging::log_request(&logging::RequestLog::new(
        route_name,
        &method,
        status,
        duration_ms,
    ));
    Ok(response)
}

/// Router を実行し、失敗を PRD の形式のエラー応答に変換する。
async fn run_router(req: Request, env: Env) -> (u16, Response) {
    match build_router().run(req, env).await {
        Ok(response) => (response.status_code(), response),
        Err(error) => {
            console_error!("router failed: {error}");
            (
                ErrorCode::Internal.status(),
                respond::error(ErrorCode::Internal, "internal error"),
            )
        }
    }
}

/// 台帳に無い経路を処理する。テスト専用の経路だけが応答し、それ以外は 404 になる。
async fn unmatched_route(req: &mut Request, env: &Env) -> (&'static str, u16, Response) {
    if let Some(result) = test_page::run(req, env).await {
        return test_route_result(test_page::ROUTE_NAME, result);
    }
    if let Some(result) = d1_check::run(req, env).await {
        return test_route_result(d1_check::ROUTE_NAME, result);
    }
    (
        NOT_FOUND_ROUTE,
        ErrorCode::NotFound.status(),
        respond::error(ErrorCode::NotFound, "route not found"),
    )
}

/// テスト専用の経路の結果を、ログの経路名と状態コードと応答にする。
fn test_route_result(
    name: &'static str,
    result: Result<Response>,
) -> (&'static str, u16, Response) {
    match result {
        Ok(response) => (name, response.status_code(), response),
        Err(error) => {
            console_error!("the {name} route failed: {error}");
            (
                name,
                ErrorCode::Internal.status(),
                respond::error(ErrorCode::Internal, "internal error"),
            )
        }
    }
}

/// 台帳の経路から Router を組み立てる。
fn build_router() -> Router<'static, ()> {
    build_router_from(ROUTES)
}

/// 経路の並びから Router を組み立てる。テストは台帳以外の経路の並びを渡して登録を検証する。
///
/// 台帳の経路はどれも経路の名前で 1 つのハンドラ ([`route_handler`]) に渡すため、
/// 経路を追加しても Router の組み立ては変えない。
pub fn build_router_from(routes: &[Route]) -> Router<'static, ()> {
    let mut router = Router::new();
    for route in routes {
        router = register_route(router, route);
    }
    router
}

/// 経路 1 件を Router に登録する。ハンドラは経路の名前と認証の要否を受け取る。
fn register_route(router: Router<'static, ()>, route: &Route) -> Router<'static, ()> {
    let name = route.name;
    let auth_required = route.auth_required;
    let handler =
        move |req: Request, ctx: RouteContext<()>| route_handler(name, auth_required, req, ctx);
    match route.method {
        Method::Get => router.get_async(route.pattern, handler),
        Method::Post => router.post_async(route.pattern, handler),
        Method::Put => router.put_async(route.pattern, handler),
        Method::Patch => router.patch_async(route.pattern, handler),
        Method::Delete => router.delete_async(route.pattern, handler),
    }
}

/// 経路 1 件を処理する。認証が必要な経路はセッションを解決し、無効なら 401 を返す。
async fn route_handler(
    name: &'static str,
    auth_required: bool,
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let env = ctx.env.clone();
    let id = ctx.param("id").cloned();
    if auth_required {
        let Some(session) = auth::session::resolve(&req, &env).await? else {
            return Ok(auth::unauthorized());
        };
        return authenticated_route(name, &mut req, &env, &session, id.as_deref()).await;
    }
    unauthenticated_route(name, &mut req, &env).await
}

/// 認証が不要な 4 経路を処理する。それ以外の名前は 404 を返す。
async fn unauthenticated_route(name: &str, req: &mut Request, env: &Env) -> Result<Response> {
    match name {
        "auth_register_begin" => auth::register::begin(req, env).await,
        "auth_register_complete" => auth::register::complete(req, env).await,
        "auth_login_begin" => auth::login::begin(req, env).await,
        "auth_login_complete" => auth::login::complete(req, env).await,
        _ => Ok(not_implemented()),
    }
}

/// 認証が必要な経路を処理する。それ以外の名前は 404 を返す。
async fn authenticated_route(
    name: &str,
    req: &mut Request,
    env: &Env,
    session: &auth::session::Session,
    id: Option<&str>,
) -> Result<Response> {
    match name {
        "auth_logout" => auth::session::logout(env, session).await,
        "passkeys_list" => auth::passkeys::list(env, session).await,
        "passkeys_begin" => auth::passkeys::begin(req, env, session).await,
        "passkeys_complete" => auth::passkeys::complete(req, env, session).await,
        "passkeys_rename" => auth::passkeys::rename(req, env, session, id).await,
        "passkeys_delete" => auth::passkeys::delete(env, session, id).await,
        "shops_list" => records::shops::list(req, env, session).await,
        "shops_create" => records::shops::create(req, env, session).await,
        "shops_get" => records::shops::get(env, session, id).await,
        "shops_update" => records::shops::update(req, env, session, id).await,
        "shops_archive" => records::shops::archive(env, session, id, true).await,
        "shops_unarchive" => records::shops::archive(env, session, id, false).await,
        "products_list" => records::products::list(req, env, session).await,
        "products_create" => records::products::create(req, env, session).await,
        "products_get" => records::products::get(env, session, id).await,
        "products_update" => records::products::update(req, env, session, id).await,
        "products_archive" => records::products::archive(env, session, id, true).await,
        "products_unarchive" => records::products::archive(env, session, id, false).await,
        "flavor_tags_list" => records::tags::list(env, session).await,
        "purchases_list" => records::purchases::list(req, env, session).await,
        "purchases_create" => records::purchases::create(req, env, session).await,
        "purchases_get" => records::purchases::get(env, session, id).await,
        "purchases_update" => records::purchases::update(req, env, session, id).await,
        "purchases_archive" => records::purchases::archive(env, session, id, true).await,
        "purchases_unarchive" => records::purchases::archive(env, session, id, false).await,
        "brews_list" => records::brews::list(req, env, session).await,
        "brews_create" => records::brews::create(req, env, session).await,
        "brews_get" => records::brews::get(env, session, id).await,
        "brews_update" => records::brews::update(req, env, session, id).await,
        "brews_archive" => records::brews::archive(env, session, id, true).await,
        "brews_unarchive" => records::brews::archive(env, session, id, false).await,
        _ => Ok(not_implemented()),
    }
}

/// 台帳に載っているがハンドラが無い経路の応答。
fn not_implemented() -> Response {
    respond::error(ErrorCode::NotFound, "route not implemented")
}
