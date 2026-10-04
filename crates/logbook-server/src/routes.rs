//! The route table (spec 6.4). The router and the access matrix test are both built
//! from [`ROUTES`], so a route cannot exist without a stated method and access level.

use std::collections::BTreeMap;

use axum::{
    Router,
    http::{HeaderValue, header},
    middleware,
    routing::{MethodRouter, delete, get, post},
};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::{AppState, auth, checks, guard, pages, posts};

/// HTTP method of a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Get,
    Post,
    Delete,
}

/// Who can call a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Anyone. The response never depends on the session.
    Public,
    /// Anyone. The response depends on the session, so it is `no-store`. Never 401.
    Session,
    /// Sign-in routes. Anyone, with a rate limit. `no-store`.
    Auth,
    /// The owner only: 401 without a valid session. `no-store`.
    Owner,
}

/// What a route does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Page,
    PostPage,
    TopicPage,
    ApiPosts,
    ApiPost,
    ApiTopic,
    Healthz,
    Me,
    RegisterStart,
    RegisterFinish,
    LoginStart,
    LoginFinish,
    Logout,
    Passkeys,
    DeletePasskey,
}

/// One row of the route table.
#[derive(Debug, Clone, Copy)]
pub struct Route {
    pub verb: Verb,
    /// Path, in axum syntax.
    pub path: &'static str,
    pub access: Access,
    pub kind: Kind,
}

const fn r(verb: Verb, path: &'static str, access: Access, kind: Kind) -> Route {
    Route {
        verb,
        path,
        access,
        kind,
    }
}

/// Every route of the app.
pub const ROUTES: &[Route] = &[
    r(Verb::Get, "/", Access::Public, Kind::Page),
    r(Verb::Get, "/about", Access::Public, Kind::Page),
    r(Verb::Get, "/setup", Access::Public, Kind::Page),
    r(Verb::Get, "/posts/{slug}", Access::Public, Kind::PostPage),
    r(
        Verb::Get,
        "/topics/{topic}",
        Access::Public,
        Kind::TopicPage,
    ),
    r(Verb::Get, "/write", Access::Public, Kind::Page),
    r(Verb::Get, "/write/{id}", Access::Public, Kind::Page),
    r(Verb::Get, "/api/posts", Access::Public, Kind::ApiPosts),
    r(
        Verb::Get,
        "/api/posts/{slug}",
        Access::Public,
        Kind::ApiPost,
    ),
    r(
        Verb::Get,
        "/api/topics/{topic}",
        Access::Public,
        Kind::ApiTopic,
    ),
    r(Verb::Get, "/healthz", Access::Public, Kind::Healthz),
    r(Verb::Get, "/api/me", Access::Session, Kind::Me),
    r(
        Verb::Post,
        "/auth/register/start",
        Access::Auth,
        Kind::RegisterStart,
    ),
    r(
        Verb::Post,
        "/auth/register/finish",
        Access::Auth,
        Kind::RegisterFinish,
    ),
    r(
        Verb::Post,
        "/auth/login/start",
        Access::Auth,
        Kind::LoginStart,
    ),
    r(
        Verb::Post,
        "/auth/login/finish",
        Access::Auth,
        Kind::LoginFinish,
    ),
    r(Verb::Post, "/auth/logout", Access::Auth, Kind::Logout),
    r(
        Verb::Get,
        "/api/owner/passkeys",
        Access::Owner,
        Kind::Passkeys,
    ),
    r(
        Verb::Delete,
        "/api/owner/passkeys/{id}",
        Access::Owner,
        Kind::DeletePasskey,
    ),
];

fn handler(kind: Kind) -> MethodRouter<AppState> {
    match kind {
        Kind::Page => get(pages::index),
        Kind::PostPage => get(pages::post),
        Kind::TopicPage => get(pages::topic),
        Kind::ApiPosts => get(posts::api_list),
        Kind::ApiPost => get(posts::api_get),
        Kind::ApiTopic => get(posts::api_topic),
        Kind::Healthz => get(checks::healthz),
        Kind::Me => get(auth::me),
        Kind::RegisterStart => post(auth::register_start),
        Kind::RegisterFinish => post(auth::register_finish),
        Kind::LoginStart => post(auth::login_start),
        Kind::LoginFinish => post(auth::login_finish),
        Kind::Logout => post(auth::logout),
        Kind::Passkeys => get(auth::list_passkeys),
        Kind::DeletePasskey => delete(auth::delete_passkey),
    }
}

/// Builds the router from [`ROUTES`]. Adds `no-store` to every non-public route,
/// the rate limit to `/auth/*`, and the CSRF check to every write.
pub fn router(state: &AppState) -> Router<AppState> {
    let mut by_path: BTreeMap<&str, MethodRouter<AppState>> = BTreeMap::new();
    for route in ROUTES {
        let mut mr = handler(route.kind);
        if route.access != Access::Public {
            mr = mr.layer(SetResponseHeaderLayer::overriding(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-store"),
            ));
        }
        if route.access == Access::Auth {
            mr = mr.layer(middleware::from_fn_with_state(
                state.clone(),
                guard::rate_limit,
            ));
        }
        let merged = match by_path.remove(route.path) {
            Some(existing) => existing.merge(mr),
            None => mr,
        };
        by_path.insert(route.path, merged);
    }
    by_path
        .into_iter()
        .fold(Router::new(), |router, (path, mr)| router.route(path, mr))
        .layer(middleware::from_fn_with_state(state.clone(), guard::csrf))
}
