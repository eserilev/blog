//! The route table (spec 6.4). The router and the access tests are both built
//! from [`ROUTES`], so a route cannot exist without a stated access level.

use axum::{Router, routing::get};

use crate::{AppState, checks, pages, posts};

/// Who can call a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Anyone. The response never depends on the session.
    Public,
    /// The owner only. Step 3 enforces this with the session check.
    Owner,
}

/// What a route returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `index.html` with the default head.
    Page,
    /// `index.html` with the head of one post, or 404.
    PostPage,
    /// `index.html` for one topic, or 404.
    TopicPage,
    /// `GET /api/posts`.
    ApiPosts,
    /// `GET /api/posts/{slug}`.
    ApiPost,
    /// `GET /api/topics/{topic}`.
    ApiTopic,
    /// `GET /healthz`.
    Healthz,
}

/// One row of the route table. All routes are `GET` until step 3.
#[derive(Debug, Clone, Copy)]
pub struct Route {
    /// Path, in axum syntax.
    pub path: &'static str,
    pub access: Access,
    pub kind: Kind,
}

const fn public(path: &'static str, kind: Kind) -> Route {
    Route {
        path,
        access: Access::Public,
        kind,
    }
}

/// Every route of the app.
pub const ROUTES: &[Route] = &[
    public("/", Kind::Page),
    public("/about", Kind::Page),
    public("/posts/{slug}", Kind::PostPage),
    public("/topics/{topic}", Kind::TopicPage),
    public("/write", Kind::Page),
    public("/write/{id}", Kind::Page),
    public("/api/posts", Kind::ApiPosts),
    public("/api/posts/{slug}", Kind::ApiPost),
    public("/api/topics/{topic}", Kind::ApiTopic),
    public("/healthz", Kind::Healthz),
];

/// Builds the router from [`ROUTES`].
pub fn router() -> Router<AppState> {
    ROUTES.iter().fold(Router::new(), |router, route| {
        let handler = match route.kind {
            Kind::Page => get(pages::index),
            Kind::PostPage => get(pages::post),
            Kind::TopicPage => get(pages::topic),
            Kind::ApiPosts => get(posts::api_list),
            Kind::ApiPost => get(posts::api_get),
            Kind::ApiTopic => get(posts::api_topic),
            Kind::Healthz => get(checks::healthz),
        };
        router.route(route.path, handler)
    })
}
