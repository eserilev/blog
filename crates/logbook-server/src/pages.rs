//! Page routes. Every page is `index.html`; the web components pick the view.
//! The server only fills the `<head>` and picks the status (spec 6.3).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::{
    AppState,
    head::{self, Head, SITE_DESCRIPTION},
    posts, topic,
};

fn page(s: &AppState, status: StatusCode, h: &Head<'_>) -> Response {
    (
        status,
        Html(head::inject(&s.index_html, &head::head_tags(h))),
    )
        .into_response()
}

fn not_found_page(s: &AppState, path: &str) -> Response {
    let url = format!("{}{path}", s.origin);
    page(
        s,
        StatusCode::NOT_FOUND,
        &Head {
            title: Some("Not found"),
            description: "",
            url: &url,
            article: false,
            noindex: true,
        },
    )
}

/// `/`, `/about`, `/write`, `/write/{id}`: the default head.
pub async fn index(State(s): State<AppState>) -> Response {
    let url = format!("{}/", s.origin);
    page(
        &s,
        StatusCode::OK,
        &Head {
            title: None,
            description: SITE_DESCRIPTION,
            url: &url,
            article: false,
            noindex: false,
        },
    )
}

/// `/posts/{slug}`. A public post gets its title and summary in the head. Any other
/// slug gets 404, `noindex`, and no post data (spec 6.3).
pub async fn post(State(s): State<AppState>, Path(slug): Path<String>) -> Response {
    match posts::public_by_slug(&s.pool, &slug).await {
        Ok(Some(pp)) => {
            let p = pp.post();
            let title = String::from_utf8_lossy(&p.title);
            let summary = String::from_utf8_lossy(&p.summary);
            let url = format!("{}/posts/{}", s.origin, String::from_utf8_lossy(&p.slug));
            page(
                &s,
                StatusCode::OK,
                &Head {
                    title: Some(&title),
                    description: &summary,
                    url: &url,
                    article: true,
                    noindex: false,
                },
            )
        }
        Ok(None) => not_found_page(&s, "/posts/"),
        Err(e) => {
            tracing::error!("database error: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
        }
    }
}

/// `/topics/{topic}`. An unknown topic gets 404 and `noindex`.
pub async fn topic(State(s): State<AppState>, Path(t): Path<String>) -> Response {
    match topic::parse(&t) {
        Some(t) => {
            let url = format!("{}/topics/{}", s.origin, topic::slug(t));
            page(
                &s,
                StatusCode::OK,
                &Head {
                    title: Some(topic::name(t)),
                    description: SITE_DESCRIPTION,
                    url: &url,
                    article: false,
                    noindex: false,
                },
            )
        }
        None => not_found_page(&s, "/topics/"),
    }
}

/// Any path that no route matches.
pub async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "not found")
}
