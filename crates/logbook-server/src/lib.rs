//! The Logbook server: a JSON API plus one `index.html` (spec 6).

pub mod auth;
pub mod checks;
pub mod config;
pub mod counter;
pub mod db;
pub mod export;
pub mod feed;
pub mod guard;
pub mod head;
pub mod headers;
pub mod now;
pub mod pages;
pub mod posts;
pub mod routes;
pub mod seed;
pub mod topic;

use std::{sync::Arc, time::Duration};

use axum::Router;
use ipnet::IpNet;
use sqlx::SqlitePool;
use tokio::sync::Notify;
use tower_http::{services::ServeDir, trace::TraceLayer};

pub use config::Config;

/// Shared state for all handlers.
#[derive(Clone)]
pub struct AppState {
    /// `static/index.html`, read once at startup.
    pub index_html: Arc<str>,
    pub pool: SqlitePool,
    /// Public origin, for absolute URLs and the CSRF check.
    pub origin: Arc<str>,
    /// Passkey ceremonies.
    pub auth: Arc<auth::Auth>,
    /// Rate limit for `/auth/*`.
    pub auth_limiter: Arc<guard::RateLimiter>,
    /// Proxies whose `X-Forwarded-For` header counts.
    pub trusted_proxies: Arc<[IpNet]>,
    /// Page loads not yet flushed.
    pub counter: Arc<counter::Counter>,
    /// Wakes the git export after a change to a public post. `None` without `EXPORT_REPO`.
    pub export_changed: Option<Arc<Notify>>,
}

impl AppState {
    /// Tells the git export that public content may have changed.
    pub fn content_changed(&self) {
        if let Some(n) = &self.export_changed {
            n.notify_one();
        }
    }
}

impl AppState {
    /// Reads `index.html` and checks its head markers.
    ///
    /// # Errors
    ///
    /// Fails if `index.html` cannot be read or has no `<!--head-->...<!--/head-->` block,
    /// or if WebAuthn cannot use the origin.
    pub fn new(config: &Config, pool: SqlitePool) -> Result<Self, String> {
        let path = config.static_dir.join("index.html");
        let index_html = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        match (index_html.find(head::START), index_html.find(head::END)) {
            (Some(a), Some(b)) if a < b => {}
            _ => {
                return Err(format!(
                    "{} has no {}...{} block",
                    path.display(),
                    head::START,
                    head::END
                ));
            }
        }
        Ok(Self {
            index_html: index_html.into(),
            pool,
            origin: config.origin.clone().into(),
            auth: auth::Auth::new(&config.origin)?,
            auth_limiter: Arc::new(guard::RateLimiter::new(
                config.auth_rate_limit,
                Duration::from_mins(1),
            )),
            trusted_proxies: config.trusted_proxies.clone().into(),
            counter: Arc::new(counter::Counter::default()),
            export_changed: config.git_export.as_ref().map(|_| Arc::new(Notify::new())),
        })
    }
}

/// Builds the full app: the route table, static files, the 404 fallback, and the
/// security headers on every response.
pub fn app(config: &Config, state: AppState) -> Router {
    let router = routes::router(&state)
        .nest_service("/static", ServeDir::new(&config.static_dir))
        .fallback(pages::not_found)
        .with_state(state);
    headers::apply(router).layer(TraceLayer::new_for_http())
}
