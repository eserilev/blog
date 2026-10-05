//! The RSS feed (spec 4.7). Public posts only, through `filter_public`.

use axum::{
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use logbook_core::filter_public;
use rss::{ChannelBuilder, GuidBuilder, ItemBuilder};
use time::{
    OffsetDateTime,
    format_description::well_known::{Rfc2822, Rfc3339},
};

use crate::{AppState, head, posts, topic};

/// RFC 3339 → RFC 2822, the date format of RSS.
fn rss_date(rfc3339: &str) -> Option<String> {
    OffsetDateTime::parse(rfc3339, &Rfc3339)
        .ok()?
        .format(&Rfc2822)
        .ok()
}

/// Root-relative links and images become absolute, so feed readers can follow them.
#[must_use]
pub fn absolute_urls(html: &str, origin: &str) -> String {
    html.replace("href=\"/", &format!("href=\"{origin}/"))
        .replace("src=\"/", &format!("src=\"{origin}/"))
}

/// `GET /feed.xml`.
pub async fn feed(State(s): State<AppState>) -> Response {
    let posts = match posts::all(&s.pool).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("database error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    let items: Vec<rss::Item> = filter_public(&posts)
        .iter()
        .map(|pp| {
            let p = pp.post();
            let link = format!("{}/posts/{}", s.origin, text(&p.slug));
            ItemBuilder::default()
                .title(Some(text(&p.title)))
                .link(Some(link))
                .description(Some(text(&p.summary)))
                .content(Some(absolute_urls(&text(&p.body_html), &s.origin)))
                .categories(vec![rss::Category {
                    name: topic::name(p.topic).into(),
                    domain: None,
                }])
                .guid(Some(
                    GuidBuilder::default()
                        .value(format!("logbook-post-{}", p.id))
                        .permalink(false)
                        .build(),
                ))
                .pub_date(p.published_at.as_deref().and_then(|d| rss_date(&text(d))))
                .build()
        })
        .collect();
    let channel = ChannelBuilder::default()
        .title(head::SITE_NAME)
        .link(format!("{}/", s.origin))
        .description(head::SITE_DESCRIPTION)
        .language(Some("en".into()))
        .items(items)
        .build();
    (
        [
            (header::CONTENT_TYPE, "application/rss+xml; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        channel.to_string(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_convert() {
        assert_eq!(
            rss_date("2026-09-28T14:02:00Z").as_deref(),
            Some("Mon, 28 Sep 2026 14:02:00 +0000")
        );
        assert_eq!(rss_date("nope"), None);
    }

    #[test]
    fn root_relative_urls_become_absolute() {
        let html =
            r#"<a href="/posts/x">x</a><img src="/media/k.png"><a href="https://e.com/">e</a>"#;
        assert_eq!(
            absolute_urls(html, "https://logbook.test"),
            r#"<a href="https://logbook.test/posts/x">x</a><img src="https://logbook.test/media/k.png"><a href="https://e.com/">e</a>"#
        );
    }
}
