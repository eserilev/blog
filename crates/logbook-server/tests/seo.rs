//! Search engine tests (spec 6.16): head tags, JSON-LD, the sitemap, robots.txt, the
//! author name, and the post lists in the HTML.

mod common;

use axum::http::{StatusCode, header};
use common::*;
use serde_json::{Value, json};

/// The JSON of every `<script type="application/ld+json">` element. Fails on any
/// other inline script.
fn json_ld(html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let lower = html.to_ascii_lowercase();
    for (i, _) in lower.match_indices("<script") {
        let tag_end = i + lower[i..].find('>').unwrap();
        let tag = &lower[i..tag_end];
        if tag.contains(" src=") {
            continue;
        }
        assert_eq!(
            tag, "<script type=\"application/ld+json\"",
            "an inline script that is not JSON-LD"
        );
        let end = tag_end + lower[tag_end..].find("</script>").unwrap();
        out.push(serde_json::from_str(&html[tag_end + 1..end]).expect("JSON-LD parses"));
    }
    out
}

/// The `@graph` node with this `@type`.
fn node<'a>(ld: &'a Value, ty: &str) -> Option<&'a Value> {
    ld["@graph"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["@type"] == ty)
}

#[tokio::test]
async fn the_rendered_page_has_no_inline_code_but_json_ld() {
    let f = fixture().await;
    for path in [
        "/".to_string(),
        format!("/posts/{PUBLIC_SLUG}"),
        "/topics/rust".into(),
        "/posts/missing".into(),
    ] {
        let r = f.get(&path).await;
        assert_eq!(json_ld(&r.body).len(), 1, "{path}");
    }
}

#[tokio::test]
async fn a_post_page_has_the_author_canonical_twitter_and_article_tags() {
    let f = fixture().await;
    let r = f.get(&format!("/posts/{PUBLIC_SLUG}")).await;
    let url = format!("{ORIGIN}/posts/{PUBLIC_SLUG}");
    for tag in [
        "<meta name=\"author\" content=\"Eitan Seri-Levi\">".to_string(),
        format!("<link rel=\"canonical\" href=\"{url}\">"),
        "<meta name=\"twitter:card\" content=\"summary\">".into(),
        "<meta name=\"twitter:site\" content=\"@0xUncleBill\">".into(),
        "<meta name=\"twitter:creator\" content=\"@0xUncleBill\">".into(),
        "<meta property=\"article:published_time\" content=\"2026-02-01T00:00:00Z\">".into(),
        "<meta property=\"article:author\" content=\"Eitan Seri-Levi\">".into(),
        "<meta property=\"og:type\" content=\"article\">".into(),
    ] {
        assert!(r.body.contains(&tag), "no {tag}");
    }
    assert!(
        r.body
            .contains("<meta property=\"article:modified_time\" content=\"20")
    );
    assert!(r.body.contains(
        "<title>A public post with &quot;quotes&quot; &amp; &lt;script&gt; · Eitan&#39;s Logbook</title>"
    ));

    let ld = &json_ld(&r.body)[0];
    assert_eq!(ld["@context"], "https://schema.org");
    let person = node(ld, "Person").unwrap();
    assert_eq!(person["@id"], format!("{ORIGIN}/#person"));
    assert_eq!(person["name"], "Eitan Seri-Levi");
    assert_eq!(
        person["alternateName"],
        json!(["Uncle Bill", "@0xUncleBill"])
    );
    assert_eq!(person["url"], format!("{ORIGIN}/"));
    assert_eq!(
        person["sameAs"],
        json!(["https://github.com/eserilev", "https://x.com/0xUncleBill"])
    );
    let site = node(ld, "WebSite").unwrap();
    assert_eq!(site["name"], "Eitan's Logbook");
    assert_eq!(site["url"], format!("{ORIGIN}/"));
    assert_eq!(site["author"]["@id"], format!("{ORIGIN}/#person"));
    let post = node(ld, "BlogPosting").unwrap();
    assert_eq!(post["headline"], "A public post with \"quotes\" & <script>");
    assert_eq!(post["description"], "summary");
    assert_eq!(post["datePublished"], "2026-02-01T00:00:00Z");
    assert!(post["dateModified"].as_str().unwrap().starts_with("20"));
    assert_eq!(post["url"], url);
    assert_eq!(post["mainEntityOfPage"], url);
    assert_eq!(post["author"]["@id"], format!("{ORIGIN}/#person"));
    assert_eq!(post["keywords"], json!(["Rust", "t1", "t2"]));
    assert_eq!(post["inLanguage"], "en");
}

#[tokio::test]
async fn home_and_topic_pages_have_canonical_links_and_json_ld() {
    let f = fixture().await;
    let home = f.get("/").await;
    assert!(
        home.body
            .contains(&format!("<link rel=\"canonical\" href=\"{ORIGIN}/\">"))
    );
    assert!(home.body.contains("<title>Eitan&#39;s Logbook</title>"));
    let ld = &json_ld(&home.body)[0];
    assert!(node(ld, "Person").is_some() && node(ld, "WebSite").is_some());
    assert!(node(ld, "BlogPosting").is_none());
    assert!(!home.body.contains("article:published_time"));

    let topic = f.get("/topics/rust").await;
    assert!(topic.body.contains(&format!(
        "<link rel=\"canonical\" href=\"{ORIGIN}/topics/rust\">"
    )));
    let ld = &json_ld(&topic.body)[0];
    assert_eq!(node(ld, "CollectionPage").unwrap()["name"], "Rust");
}

#[tokio::test]
async fn not_found_pages_have_no_canonical_link() {
    let f = fixture().await;
    for path in [
        format!("/posts/{DRAFT_SLUG}"),
        format!("/posts/{PRIVATE_SLUG}"),
        "/posts/missing".into(),
        "/topics/nope".into(),
    ] {
        let r = f.get(&path).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{path}");
        assert!(r.body.contains("content=\"noindex\""), "{path}");
        assert!(!r.body.contains("rel=\"canonical\""), "{path}");
        let ld = &json_ld(&r.body)[0];
        assert!(node(ld, "BlogPosting").is_none(), "{path}");
        assert!(node(ld, "CollectionPage").is_none(), "{path}");
        assert!(!r.body.contains(SECRET), "{path}");
    }
}

#[tokio::test]
async fn json_ld_cannot_close_its_script_element() {
    let f = fixture().await;
    sqlx::query("UPDATE posts SET title = '</script><script>alert(1)</script><!--' WHERE slug = ?")
        .bind(PUBLIC_SLUG)
        .execute(&f.pool)
        .await
        .unwrap();
    let r = f.get(&format!("/posts/{PUBLIC_SLUG}")).await;
    assert!(!r.body.contains("<script>alert"), "{}", r.body);
    let ld = json_ld(&r.body);
    assert_eq!(ld.len(), 1);
    assert_eq!(
        node(&ld[0], "BlogPosting").unwrap()["headline"],
        "</script><script>alert(1)</script><!--"
    );
}

#[tokio::test]
async fn the_page_shows_the_author_and_the_public_posts() {
    let f = fixture().await;
    let r = f.get("/").await;
    let year = time::OffsetDateTime::now_utc().year();
    assert!(
        r.body.contains(&format!(
            "© {year} <span id=\"site-author\">Eitan Seri-Levi</span>."
        )),
        "no footer name"
    );
    // The Win98 table and the phone links, from the server.
    for slug in [PUBLIC_SLUG, OLDER_SLUG] {
        assert!(
            r.body
                .contains(&format!("<a class=\"ttl\" href=\"/posts/{slug}\">")),
            "no row for {slug}"
        );
        assert!(
            r.body.contains(&format!("<li><a href=\"/posts/{slug}\">")),
            "no link for {slug}"
        );
    }
    assert!(r.body.contains(
        "<a class=\"ttl\" href=\"/posts/a-public-post-with-quotes-script\">A public post with &quot;quotes&quot; &amp; &lt;script&gt;</a><span class=\"ex\">summary</span>"
    ));
    for slug in [DRAFT_SLUG, PRIVATE_SLUG] {
        assert!(
            !r.body.contains(&format!("/posts/{slug}\"")),
            "{slug} is listed"
        );
    }
    assert!(!r.body.contains(SECRET));

    // A topic page lists only its posts in the table.
    let surf = f.get("/topics/surf").await.body;
    assert!(surf.contains(&format!("<a class=\"ttl\" href=\"/posts/{OLDER_SLUG}\">")));
    assert!(!surf.contains(&format!("<a class=\"ttl\" href=\"/posts/{PUBLIC_SLUG}\">")));
}

#[tokio::test]
async fn the_sitemap_lists_exactly_the_public_posts_and_their_topics() {
    let f = fixture().await;
    let r = f.get("/sitemap.xml").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.headers[header::CONTENT_TYPE],
        "application/xml; charset=utf-8"
    );
    assert_eq!(r.headers[header::CACHE_CONTROL], "public, max-age=300");
    assert!(!r.body.contains(SECRET));
    let locs: Vec<&str> = r
        .body
        .split("<loc>")
        .skip(1)
        .map(|s| &s[..s.find("</loc>").unwrap()])
        .collect();
    assert_eq!(
        locs,
        [
            format!("{ORIGIN}/"),
            format!("{ORIGIN}/posts/{PUBLIC_SLUG}"),
            format!("{ORIGIN}/posts/{OLDER_SLUG}"),
            // Sidebar order. Topics with no public post (jiu-jitsu, and the others) are left out.
            format!("{ORIGIN}/topics/rust"),
            format!("{ORIGIN}/topics/surf"),
        ]
    );
    // Each post has the date of its last change.
    assert_eq!(r.body.matches("<lastmod>").count(), 2);
    let today = time::OffsetDateTime::now_utc().date().to_string();
    assert!(
        r.body.contains(&format!("<lastmod>{today}</lastmod>")),
        "{}",
        r.body
    );
    assert!(r.body.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"));
}

#[tokio::test]
async fn a_hidden_post_never_reaches_the_sitemap() {
    let f = fixture().await;
    // Hide the newest public post. It leaves the sitemap, and so does its topic.
    sqlx::query("UPDATE posts SET state = 'private' WHERE slug = ?")
        .bind(PUBLIC_SLUG)
        .execute(&f.pool)
        .await
        .unwrap();
    let body = f.get("/sitemap.xml").await.body;
    assert!(!body.contains(PUBLIC_SLUG));
    assert!(!body.contains(DRAFT_SLUG) && !body.contains(PRIVATE_SLUG));
    assert!(
        !body.contains("/topics/rust"),
        "rust has no public post now"
    );
    assert!(body.contains(OLDER_SLUG) && body.contains("/topics/surf"));
}

#[tokio::test]
async fn robots_txt_allows_the_site_and_names_the_sitemap() {
    let f = fixture().await;
    let r = f.get("/robots.txt").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.headers[header::CONTENT_TYPE], "text/plain; charset=utf-8");
    assert_eq!(
        r.body,
        format!(
            "User-agent: *\nAllow: /\nDisallow: /write\nDisallow: /setup\nDisallow: /api/owner/\n\nSitemap: {ORIGIN}/sitemap.xml\n"
        )
    );
}

#[tokio::test]
async fn feed_items_name_the_author() {
    let f = fixture().await;
    let body = f.get("/feed.xml").await.body;
    let channel = rss::Channel::read_from(body.as_bytes()).expect("valid RSS");
    for item in channel.items() {
        let dc = item.dublin_core_ext().expect("dc extension");
        assert_eq!(dc.creators(), ["Eitan Seri-Levi"]);
    }
    assert!(body.contains("xmlns:dc=\"http://purl.org/dc/elements/1.1/\""));
}
