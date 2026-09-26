//! 管理者向けの Worker (ADR-0008)。
//!
//! 利用者の作成と登録用トークンの発行 (FR-17) を、サーバー側で生成する HTML のフォームで行う
//! (`coffee-log-admin.<アカウントのサブドメイン>.workers.dev`)。Flutter と JavaScript は使わない。
//!
//! アプリ内の認証は持たず、安全性は Worker 単位の Cloudflare Access 保護だけに依存する。
//! Access が付ける JWT は検証しない (rsa 相当の依存を増やさないため。ADR-0008)。
//!
//! 経路は 1 か所の台帳 ([`routes::ROUTES`]) から Router を組み立てる (0017 と同じ仕組み)。
//! 状態を変更するフォームの送信は、`Origin` ヘッダがリクエスト自身のオリジンと一致しない場合に
//! 403 を返す (ADR-0005 と同じ規則)。

pub mod db;
pub mod html;
pub mod input;
pub mod origin;
pub mod queries;
pub mod random;
pub mod routes;
pub mod tokens;
pub mod users;

use coffee_log_core::error::ErrorCode;
use coffee_log_core::routes::{match_route, Method, Route};
use worker::{console_error, event, Context, Env, Request, Response, Result, RouteContext, Router};

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let method = req.method().to_string();
    let path = req.path();

    match match_route(routes::ROUTES, &method, &path) {
        Some(route) => match origin::check(route.method, &req) {
            Ok(true) => run_router(req, env).await,
            // 状態を変更するフォームの送信は、同一オリジン以外と `Origin` の無いリクエストを拒否する。
            Ok(false) => Ok(origin::forbidden()),
            Err(error) => {
                console_error!("failed to check the request origin: {error}");
                Ok(internal_error())
            }
        },
        // 台帳に無い経路は 404 を返す。
        None => Ok(not_found()),
    }
}

/// Router を実行し、失敗を 500 の HTML にする。
async fn run_router(req: Request, env: Env) -> Result<Response> {
    match build_router().run(req, env).await {
        Ok(response) => Ok(response),
        Err(error) => {
            console_error!("router failed: {error}");
            Ok(internal_error())
        }
    }
}

/// 台帳の経路から Router を組み立てる。
fn build_router() -> Router<'static, ()> {
    build_router_from(routes::ROUTES)
}

/// 経路の並びから Router を組み立てる。
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

/// 経路 1 件を Router に登録する。ハンドラは経路の名前を受け取る。
fn register_route(router: Router<'static, ()>, route: &Route) -> Router<'static, ()> {
    let name = route.name;
    let handler = move |req: Request, ctx: RouteContext<()>| route_handler(name, req, ctx);
    match route.method {
        Method::Get => router.get_async(route.pattern, handler),
        Method::Post => router.post_async(route.pattern, handler),
        Method::Put => router.put_async(route.pattern, handler),
        Method::Patch => router.patch_async(route.pattern, handler),
        Method::Delete => router.delete_async(route.pattern, handler),
    }
}

/// 経路 1 件を処理する。失敗は 500 の HTML にする。
async fn route_handler(
    name: &'static str,
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let env = ctx.env.clone();
    let id = ctx.param("id").cloned();
    match run_route(name, req, &env, id.as_deref()).await {
        Ok(response) => Ok(response),
        Err(error) => {
            console_error!("the {name} route failed: {error}");
            Ok(internal_error())
        }
    }
}

/// 経路の名前からハンドラを呼ぶ。台帳に載っていて実装の無い名前は 404 にする。
async fn run_route(name: &str, req: Request, env: &Env, id: Option<&str>) -> Result<Response> {
    match name {
        "users_list" => users::list(env).await,
        "users_create" => users::create(req, env).await,
        "tokens_create" => match id {
            Some(id) => tokens::create(id, env).await,
            // 台帳のパターンは `:id` を必ず埋めるため、ここには来ない。
            None => Ok(not_found()),
        },
        _ => Ok(not_found()),
    }
}

/// HTML の応答を組み立てる。
pub fn html_response(body: String) -> Result<Response> {
    Response::from_html(body)
}

/// HTML のエラー応答を組み立てる。
pub fn html_error(status: u16, message: &str) -> Response {
    match Response::from_html(html::error_page(message)) {
        Ok(response) => response.with_status(status),
        Err(error) => {
            // `Response::from_html` は文字列からしか作らないため、ここには来ない。
            console_error!("failed to build the error page: {error}");
            Response::error("internal error", ErrorCode::Internal.status())
                .expect("a plain error response is always constructible")
        }
    }
}

/// 台帳に無い経路の応答 (404)。
fn not_found() -> Response {
    html_error(ErrorCode::NotFound.status(), "route not found")
}

/// Worker の内部の失敗の応答 (500)。
fn internal_error() -> Response {
    html_error(ErrorCode::Internal.status(), "internal error")
}
