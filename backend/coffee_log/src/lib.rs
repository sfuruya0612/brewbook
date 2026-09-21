//! 利用者向けの Worker。`/api/*` を Router で処理し、リクエストごとにログを出す。

use coffee_log_core::error::{envelope, ErrorCode};
use coffee_log_core::routes::{matched, Method, Route, NOT_FOUND_ROUTE, ROUTES};
use worker::{
    console_error, event, Context, Date, Env, Request, Response, Result, RouteContext, Router,
};

pub mod logging;

/// ルーティング以外で Worker が失敗したときの応答のメッセージ。
const INTERNAL_ERROR_MESSAGE: &str = "internal error";

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let started_at_ms = Date::now().as_millis();
    let method = req.method().to_string();
    let path = req.path();

    let (route_name, status, response) = match matched(&method, &path) {
        Some(route) => {
            let (status, response) = run_router(req, env).await;
            (route.name, status, response)
        }
        None => (
            NOT_FOUND_ROUTE,
            ErrorCode::NotFound.status(),
            error_response(ErrorCode::NotFound, "route not found"),
        ),
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
                error_response(ErrorCode::Internal, INTERNAL_ERROR_MESSAGE),
            )
        }
    }
}

/// 台帳の経路から Router を組み立てる。
fn build_router() -> Router<'static, ()> {
    build_router_from(ROUTES)
}

/// 経路の並びから Router を組み立てる。テストは台帳以外の経路の並びを渡して登録を検証する。
pub fn build_router_from(routes: &[Route]) -> Router<'static, ()> {
    let mut router = Router::new();
    for route in routes {
        router = register_route(router, route);
    }
    router
}

/// 経路 1 件を Router に登録する。
///
/// 経路のハンドラは、経路を追加する issue (0005 以降) が実装する。
/// 台帳に載っているがハンドラが未実装の経路は、ここで 404 を返す。
fn register_route(router: Router<'static, ()>, route: &Route) -> Router<'static, ()> {
    match route.method {
        Method::Get => router.get(route.pattern, handle_not_implemented),
        Method::Post => router.post(route.pattern, handle_not_implemented),
        Method::Put => router.put(route.pattern, handle_not_implemented),
        Method::Patch => router.patch(route.pattern, handle_not_implemented),
        Method::Delete => router.delete(route.pattern, handle_not_implemented),
    }
}

fn handle_not_implemented(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    Ok(error_response(ErrorCode::NotFound, "route not implemented"))
}

/// PRD の形式のエラー応答を組み立てる。応答の構築に失敗した場合は 500 の平文に落とす。
fn error_response(code: ErrorCode, message: &str) -> Response {
    match Response::from_json(&envelope(code, message)) {
        Ok(response) => response.with_status(code.status()),
        Err(error) => {
            console_error!("failed to build the error response: {error}");
            Response::error(INTERNAL_ERROR_MESSAGE, ErrorCode::Internal.status())
                .expect("a plain error response is always constructible")
        }
    }
}
