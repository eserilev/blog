//! The route table (spec 6.4). The router and the access matrix test are both
//! built from [`ROUTES`], so a route cannot exist without a stated access level.

use axum::{Router, routing::get};

use crate::{AppState, pages};

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
    /// `index.html`.
    Page,
    /// The health check.
    Healthz,
}

/// One row of the route table.
#[derive(Debug, Clone, Copy)]
pub struct Route {
    /// Path, in axum syntax.
    pub path: &'static str,
    /// Access level.
    pub access: Access,
    /// Handler.
    pub kind: Kind,
}

const fn page(path: &'static str) -> Route {
    Route {
        path,
        access: Access::Public,
        kind: Kind::Page,
    }
}

/// Every route of the app. All routes are `GET` in step 1.
pub const ROUTES: &[Route] = &[
    page("/"),
    page("/about"),
    page("/posts/{slug}"),
    page("/topics/{topic}"),
    page("/write"),
    page("/write/{id}"),
    Route {
        path: "/healthz",
        access: Access::Public,
        kind: Kind::Healthz,
    },
];

/// Builds the router from [`ROUTES`].
pub fn router() -> Router<AppState> {
    ROUTES.iter().fold(Router::new(), |router, route| {
        let handler = match route.kind {
            Kind::Page => get(pages::index),
            Kind::Healthz => get(pages::healthz),
        };
        router.route(route.path, handler)
    })
}
