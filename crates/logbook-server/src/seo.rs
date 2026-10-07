//! `/sitemap.xml` and `/robots.txt` (spec 6.16). The sitemap lists public posts
//! only, through `filter_public`.

use std::collections::BTreeSet;

use axum::{
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use logbook_core::{Post, filter_public};

use crate::{AppState, head::escape, posts, topic::Topic};

/// The sitemap: the home page, every public post, and every topic with a public
/// post. A post `lastmod` is the date of its `updated_at`.
#[must_use]
pub fn sitemap(origin: &str, posts: &[Post], topics: &[Topic]) -> String {
    use std::fmt::Write as _;
    let public = filter_public(posts);
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    let _ = writeln!(out, "<url><loc>{}/</loc></url>", escape(origin));
    let mut used = BTreeSet::new();
    for pp in &public {
        let p = pp.post();
        let slug = String::from_utf8_lossy(&p.slug);
        let updated = String::from_utf8_lossy(&p.updated_at);
        let date = updated.get(..10).unwrap_or_default();
        let _ = writeln!(
            out,
            "<url><loc>{}</loc><lastmod>{}</lastmod></url>",
            escape(&format!("{origin}/posts/{slug}")),
            escape(date)
        );
        used.insert(p.topic.clone());
    }
    for t in topics.iter().filter(|t| used.contains(t.slug.as_bytes())) {
        let _ = writeln!(
            out,
            "<url><loc>{}</loc></url>",
            escape(&format!("{origin}/topics/{}", t.slug))
        );
    }
    out.push_str("</urlset>\n");
    out
}

/// The robots file. Crawlers skip the owner pages and the owner API.
#[must_use]
pub fn robots(origin: &str) -> String {
    format!(
        "User-agent: *\n\
Allow: /\n\
Disallow: /write\n\
Disallow: /setup\n\
Disallow: /api/owner/\n\
\n\
Sitemap: {origin}/sitemap.xml\n"
    )
}

/// `GET /sitemap.xml`.
pub async fn sitemap_xml(State(s): State<AppState>) -> Response {
    let loaded = async {
        Ok::<_, sqlx::Error>((
            posts::all(&s.pool).await?,
            crate::topic::all(&s.pool).await?,
        ))
    };
    let (posts, topics) = match loaded.await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("database error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };
    (
        [
            (header::CONTENT_TYPE, "application/xml; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        sitemap(&s.origin, &posts, &topics),
    )
        .into_response()
}

/// `GET /robots.txt`.
pub async fn robots_txt(State(s): State<AppState>) -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        robots(&s.origin),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robots_points_to_the_sitemap() {
        assert_eq!(
            robots("https://e.com"),
            "User-agent: *\nAllow: /\nDisallow: /write\nDisallow: /setup\nDisallow: /api/owner/\n\nSitemap: https://e.com/sitemap.xml\n"
        );
    }
}
