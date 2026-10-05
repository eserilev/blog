//! Page routes. Every page is `index.html`; the web components pick the view.
//! The server fills the `<head>`, the title section, and the topic list, and picks
//! the status (spec 6.3).

use std::net::SocketAddr;

use axum::{
    Extension,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};

use crate::{
    AppState,
    head::{self, Head},
    posts, site, topic,
};

/// What a page shows in its head, without the site values.
struct Page<'a> {
    title: Option<&'a str>,
    /// `None` uses the site tagline.
    description: Option<&'a str>,
    path: &'a str,
    article: bool,
    noindex: bool,
}

/// The peer address, when the server runs with connect info (tests do not).
type Peer = Option<Extension<ConnectInfo<SocketAddr>>>;

/// Counts the page load for the visitor counter (spec 4.8).
fn count_visit(s: &AppState, headers: &HeaderMap, peer: &Peer) {
    let peer = peer.as_ref().map(|Extension(ConnectInfo(a))| a.ip());
    let ip = crate::guard::client_ip(headers, peer, &s.trusted_proxies);
    s.counter.hit(headers, ip);
}

fn internal(e: &sqlx::Error) -> Response {
    tracing::error!("database error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
}

async fn page(s: &AppState, status: StatusCode, p: &Page<'_>) -> Response {
    let (site, topics) = match (site::get(&s.pool).await, topic::all(&s.pool).await) {
        (Ok(site), Ok(topics)) => (site, topics),
        (Err(e), _) | (_, Err(e)) => return internal(&e),
    };
    let url = format!("{}{}", s.origin, p.path);
    let tags = head::head_tags(&Head {
        title: p.title,
        description: p.description.unwrap_or(&site.tagline),
        url: &url,
        article: p.article,
        noindex: p.noindex,
        site: &site.title,
    });
    let html = site::fill(&head::inject(&s.index_html, &tags), &site, &topics);
    (status, Html(html)).into_response()
}

async fn not_found_page(s: &AppState, path: &str) -> Response {
    page(
        s,
        StatusCode::NOT_FOUND,
        &Page {
            title: Some("Not found"),
            description: Some(""),
            path,
            article: false,
            noindex: true,
        },
    )
    .await
}

/// `/`, `/about`, `/write`, `/write/{id}`: the default head.
pub async fn index(State(s): State<AppState>, peer: Peer, headers: HeaderMap) -> Response {
    count_visit(&s, &headers, &peer);
    page(
        &s,
        StatusCode::OK,
        &Page {
            title: None,
            description: None,
            path: "/",
            article: false,
            noindex: false,
        },
    )
    .await
}

/// `/posts/{slug}`. A public post gets its title and summary in the head. Any other
/// slug gets 404, `noindex`, and no post data (spec 6.3).
pub async fn post(
    State(s): State<AppState>,
    Path(slug): Path<String>,
    peer: Peer,
    headers: HeaderMap,
) -> Response {
    count_visit(&s, &headers, &peer);
    match posts::public_by_slug(&s.pool, &slug).await {
        Ok(Some(pp)) => {
            let p = pp.post();
            let title = String::from_utf8_lossy(&p.title);
            let summary = String::from_utf8_lossy(&p.summary);
            let path = format!("/posts/{}", String::from_utf8_lossy(&p.slug));
            page(
                &s,
                StatusCode::OK,
                &Page {
                    title: Some(&title),
                    description: Some(&summary),
                    path: &path,
                    article: true,
                    noindex: false,
                },
            )
            .await
        }
        Ok(None) => not_found_page(&s, "/posts/").await,
        Err(e) => internal(&e),
    }
}

/// `/topics/{topic}`. An unknown topic gets 404 and `noindex`.
pub async fn topic(
    State(s): State<AppState>,
    Path(t): Path<String>,
    peer: Peer,
    headers: HeaderMap,
) -> Response {
    count_visit(&s, &headers, &peer);
    match topic::get(&s.pool, &t).await {
        Ok(Some(t)) => {
            let path = format!("/topics/{}", t.slug);
            page(
                &s,
                StatusCode::OK,
                &Page {
                    title: Some(&t.name),
                    description: None,
                    path: &path,
                    article: false,
                    noindex: false,
                },
            )
            .await
        }
        Ok(None) => not_found_page(&s, "/topics/").await,
        Err(e) => internal(&e),
    }
}

/// Any path that no route matches.
pub async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "not found")
}
