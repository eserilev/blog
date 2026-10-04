//! The Logbook server: a JSON API plus one `index.html` (spec 6).

pub mod checks;
pub mod config;
pub mod db;
pub mod head;
pub mod headers;
pub mod pages;
pub mod posts;
pub mod routes;
pub mod seed;
pub mod topic;

use std::sync::Arc;

use axum::Router;
use sqlx::SqlitePool;
use tower_http::{services::ServeDir, trace::TraceLayer};

pub use config::Config;

/// Shared state for all handlers.
#[derive(Clone)]
pub struct AppState {
    /// `static/index.html`, read once at startup.
    pub index_html: Arc<str>,
    pub pool: SqlitePool,
    /// Public origin, for absolute URLs.
    pub origin: Arc<str>,
}

impl AppState {
    /// Reads `index.html` and checks its head markers.
    ///
    /// # Errors
    ///
    /// Fails if `index.html` cannot be read or has no `<!--head-->...<!--/head-->` block.
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
        })
    }
}

/// Builds the full app: the route table, static files, the 404 fallback, and the
/// security headers on every response.
pub fn app(config: &Config, state: AppState) -> Router {
    let router = routes::router()
        .nest_service("/static", ServeDir::new(&config.static_dir))
        .fallback(pages::not_found)
        .with_state(state);
    headers::apply(router).layer(TraceLayer::new_for_http())
}
