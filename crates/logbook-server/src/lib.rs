//! The Logbook server: a JSON API plus one `index.html` (spec 6).

pub mod config;
pub mod headers;
pub mod pages;
pub mod routes;

use std::sync::Arc;

use axum::Router;
use tower_http::{services::ServeDir, trace::TraceLayer};

pub use config::Config;

/// Shared state for all handlers.
#[derive(Clone)]
pub struct AppState {
    /// `static/index.html`, read once at startup.
    pub index_html: Arc<str>,
}

/// Builds the full app: the route table, static files, the 404 fallback,
/// and the security headers on every response.
///
/// # Errors
///
/// Fails if `index.html` cannot be read from the static folder.
pub fn app(config: &Config) -> std::io::Result<Router> {
    let index_html = std::fs::read_to_string(config.static_dir.join("index.html"))?;
    let state = AppState {
        index_html: index_html.into(),
    };
    let router = routes::router()
        .nest_service("/static", ServeDir::new(&config.static_dir))
        .fallback(pages::not_found)
        .with_state(state);
    Ok(headers::apply(router).layer(TraceLayer::new_for_http()))
}
