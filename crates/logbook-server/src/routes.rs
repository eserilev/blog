//! The route table (spec 6.4). The router and the access matrix test are both built
//! from [`ROUTES`], so a route cannot exist without a stated method and access level.

use std::collections::BTreeMap;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, header},
    middleware,
    routing::{MethodRouter, delete, get, post, put},
};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::{
    AppState, auth, checks, counter, export, feed, guard, media, now, pages, posts, seo, site,
    surf, topic,
};

/// HTTP method of a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Get,
    Post,
    Put,
    Delete,
}

/// Who can call a route. The type is in `logbook-core`, with the access decision
/// [`logbook_core::authorize`] (theorem T16).
pub use logbook_core::Access;

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
    OwnerPosts,
    OwnerPost,
    CreatePost,
    SavePost,
    SetPostState,
    DeletePost,
    ApiNow,
    OwnerNow,
    SaveNow,
    Visitors,
    Feed,
    ExportZip,
    Media,
    Upload,
    Surf,
    ApiTopics,
    CreateTopic,
    SaveTopics,
    DeleteTopic,
    ApiSite,
    OwnerSite,
    SaveSite,
    Sitemap,
    Robots,
}

/// Every route kind. The test `every_kind_is_routed` checks that each one is in
/// [`ROUTES`]; `_all_kinds_listed` fails to compile when a new kind is missing here.
pub const ALL_KINDS: &[Kind] = &[
    Kind::Page,
    Kind::PostPage,
    Kind::TopicPage,
    Kind::ApiPosts,
    Kind::ApiPost,
    Kind::ApiTopic,
    Kind::Healthz,
    Kind::Me,
    Kind::RegisterStart,
    Kind::RegisterFinish,
    Kind::LoginStart,
    Kind::LoginFinish,
    Kind::Logout,
    Kind::Passkeys,
    Kind::DeletePasskey,
    Kind::OwnerPosts,
    Kind::OwnerPost,
    Kind::CreatePost,
    Kind::SavePost,
    Kind::SetPostState,
    Kind::DeletePost,
    Kind::ApiNow,
    Kind::OwnerNow,
    Kind::SaveNow,
    Kind::Visitors,
    Kind::Feed,
    Kind::ExportZip,
    Kind::Media,
    Kind::Upload,
    Kind::Surf,
    Kind::ApiTopics,
    Kind::CreateTopic,
    Kind::SaveTopics,
    Kind::DeleteTopic,
    Kind::ApiSite,
    Kind::OwnerSite,
    Kind::SaveSite,
    Kind::Sitemap,
    Kind::Robots,
];

const fn _all_kinds_listed(k: Kind) {
    match k {
        Kind::Page
        | Kind::PostPage
        | Kind::TopicPage
        | Kind::ApiPosts
        | Kind::ApiPost
        | Kind::ApiTopic
        | Kind::Healthz
        | Kind::Me
        | Kind::RegisterStart
        | Kind::RegisterFinish
        | Kind::LoginStart
        | Kind::LoginFinish
        | Kind::Logout
        | Kind::Passkeys
        | Kind::DeletePasskey
        | Kind::OwnerPosts
        | Kind::OwnerPost
        | Kind::CreatePost
        | Kind::SavePost
        | Kind::SetPostState
        | Kind::DeletePost
        | Kind::ApiNow
        | Kind::OwnerNow
        | Kind::SaveNow
        | Kind::Visitors
        | Kind::Feed
        | Kind::ExportZip
        | Kind::Media
        | Kind::Upload
        | Kind::Surf
        | Kind::ApiTopics
        | Kind::CreateTopic
        | Kind::SaveTopics
        | Kind::DeleteTopic
        | Kind::ApiSite
        | Kind::OwnerSite
        | Kind::SaveSite
        | Kind::Sitemap
        | Kind::Robots => {}
    }
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
    r(
        Verb::Get,
        "/api/owner/posts",
        Access::Owner,
        Kind::OwnerPosts,
    ),
    r(
        Verb::Post,
        "/api/owner/posts",
        Access::Owner,
        Kind::CreatePost,
    ),
    r(
        Verb::Get,
        "/api/owner/posts/{id}",
        Access::Owner,
        Kind::OwnerPost,
    ),
    r(
        Verb::Put,
        "/api/owner/posts/{id}",
        Access::Owner,
        Kind::SavePost,
    ),
    r(
        Verb::Delete,
        "/api/owner/posts/{id}",
        Access::Owner,
        Kind::DeletePost,
    ),
    r(
        Verb::Post,
        "/api/owner/posts/{id}/state",
        Access::Owner,
        Kind::SetPostState,
    ),
    r(Verb::Get, "/api/now", Access::Public, Kind::ApiNow),
    r(Verb::Get, "/api/owner/now", Access::Owner, Kind::OwnerNow),
    r(Verb::Put, "/api/owner/now", Access::Owner, Kind::SaveNow),
    r(Verb::Get, "/api/visitors", Access::Public, Kind::Visitors),
    r(Verb::Get, "/feed.xml", Access::Public, Kind::Feed),
    r(
        Verb::Get,
        "/api/owner/export.zip",
        Access::Owner,
        Kind::ExportZip,
    ),
    r(Verb::Get, "/media/{key}", Access::Public, Kind::Media),
    r(
        Verb::Post,
        "/api/owner/uploads",
        Access::Owner,
        Kind::Upload,
    ),
    r(Verb::Get, "/api/surf", Access::Public, Kind::Surf),
    r(Verb::Get, "/api/topics", Access::Public, Kind::ApiTopics),
    r(
        Verb::Post,
        "/api/owner/topics",
        Access::Owner,
        Kind::CreateTopic,
    ),
    r(
        Verb::Put,
        "/api/owner/topics",
        Access::Owner,
        Kind::SaveTopics,
    ),
    r(
        Verb::Delete,
        "/api/owner/topics/{topic}",
        Access::Owner,
        Kind::DeleteTopic,
    ),
    r(Verb::Get, "/api/site", Access::Public, Kind::ApiSite),
    r(Verb::Get, "/api/owner/site", Access::Owner, Kind::OwnerSite),
    r(Verb::Put, "/api/owner/site", Access::Owner, Kind::SaveSite),
    r(Verb::Get, "/sitemap.xml", Access::Public, Kind::Sitemap),
    r(Verb::Get, "/robots.txt", Access::Public, Kind::Robots),
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
        Kind::OwnerPosts => get(posts::owner_list),
        Kind::OwnerPost => get(posts::owner_get),
        Kind::CreatePost => post(posts::owner_create),
        Kind::SavePost => put(posts::owner_save),
        Kind::SetPostState => post(posts::owner_set_state),
        Kind::DeletePost => delete(posts::owner_delete),
        Kind::ApiNow => get(now::api_now),
        Kind::OwnerNow => get(now::owner_now),
        Kind::SaveNow => put(now::save_now),
        Kind::Visitors => get(counter::api_visitors),
        Kind::Feed => get(feed::feed),
        Kind::ExportZip => get(export::owner_zip),
        Kind::Media => get(media::serve),
        Kind::Upload => {
            post(media::upload).layer(DefaultBodyLimit::max(media::UPLOAD_MAX + 64 * 1024))
        }
        Kind::Surf => get(surf::api_surf),
        Kind::ApiTopics => get(topic::api_topics),
        Kind::CreateTopic => post(topic::create),
        Kind::SaveTopics => put(topic::save),
        Kind::DeleteTopic => delete(topic::delete),
        Kind::ApiSite => get(site::api_site),
        Kind::OwnerSite => get(site::owner_site),
        Kind::SaveSite => put(site::save_site),
        Kind::Sitemap => get(seo::sitemap_xml),
        Kind::Robots => get(seo::robots_txt),
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
