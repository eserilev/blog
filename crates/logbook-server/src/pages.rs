//! Page routes. Every page is the same `index.html`; the web components pick the view.

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
};

use crate::AppState;

/// Serves `index.html`. Step 2 adds post titles to the `<head>` and returns 404
/// for posts that are not public (spec 6.3).
pub async fn index(State(state): State<AppState>) -> Html<String> {
    Html(state.index_html.to_string())
}

/// `GET /healthz`. Step 2 adds the database and replication checks (spec 6.12).
pub async fn healthz() -> &'static str {
    "ok"
}

/// Any path that no route matches.
pub async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "not found")
}
