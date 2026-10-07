//! Page routes. Every page is `index.html`; the web components pick the view.
//! The server fills the `<head>`, the title section, the topic list, the author, and
//! the public post lists, and picks the status (spec 6.3, 6.16).

use std::net::SocketAddr;

use axum::{
    Extension,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};

use logbook_core::filter_public;

use crate::{
    AppState,
    head::{self, Article, Head, PageKind},
    posts::{self, ListItem},
    site, topic,
};

/// What a page shows in its head, without the site values.
struct Page<'a> {
    title: Option<&'a str>,
    /// `None` uses the site tagline.
    description: Option<&'a str>,
    path: &'a str,
    kind: PageKind<'a>,
    /// The topic of a topic page, for the post table.
    topic: Option<&'a str>,
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

/// The public posts, newest first, through `filter_public`.
async fn public_items(s: &AppState) -> Result<Vec<ListItem>, sqlx::Error> {
    let posts = posts::all(&s.pool).await?;
    let names = topic::names(&s.pool).await?;
    Ok(filter_public(&posts)
        .iter()
        .map(|pp| ListItem::new(pp, &names))
        .collect())
}

async fn page(s: &AppState, status: StatusCode, p: &Page<'_>) -> Response {
    let loaded = async {
        Ok::<_, sqlx::Error>((
            site::get(&s.pool).await?,
            topic::all(&s.pool).await?,
            public_items(s).await?,
        ))
    };
    let (site, topics, items) = match loaded.await {
        Ok(v) => v,
        Err(e) => return internal(&e),
    };
    let url = format!("{}{}", s.origin, p.path);
    let tags = head::head_tags(&Head {
        title: p.title,
        description: p.description.unwrap_or(&site.tagline),
        url: &url,
        origin: &s.origin,
        kind: p.kind,
        noindex: p.noindex,
        site: &site.title,
    });
    let html = site::fill(
        &head::inject(&s.index_html, &tags),
        &site::Fill {
            site: &site,
            topics: &topics,
            posts: &items,
            topic: p.topic,
            year: time::OffsetDateTime::now_utc().year(),
        },
    );
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
            kind: PageKind::Website,
            topic: None,
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
            kind: PageKind::Website,
            topic: None,
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
            let names = match topic::names(&s.pool).await {
                Ok(n) => n,
                Err(e) => return internal(&e),
            };
            let item = ListItem::new(&pp, &names);
            let modified = String::from_utf8_lossy(&pp.post().updated_at);
            let keywords: Vec<String> = std::iter::once(item.topic_name.clone())
                .chain(item.tags.iter().cloned())
                .collect();
            let path = format!("/posts/{}", item.slug);
            page(
                &s,
                StatusCode::OK,
                &Page {
                    title: Some(&item.title),
                    description: Some(&item.summary),
                    path: &path,
                    kind: PageKind::Post(Article {
                        published: item.published_at.as_deref().unwrap_or_default(),
                        modified: &modified,
                        keywords: &keywords,
                    }),
                    topic: None,
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
                    kind: PageKind::Topic,
                    topic: Some(&t.slug),
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
