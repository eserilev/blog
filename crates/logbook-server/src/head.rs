//! `<head>` tags for page responses (spec 6.3, 6.16). All values are HTML-escaped.
//! `serde_json` builds the JSON-LD block, and [`script_json`] escapes it for a
//! `<script>` element.

use serde_json::{Value, json};

/// The author of the site and of every post (spec 6.16). The page, the head, the
/// JSON-LD, and the feed get the name from here.
pub const AUTHOR_NAME: &str = "Eitan Seri-Levi";
/// Other names of the author, for the JSON-LD `Person`.
pub const AUTHOR_ALIASES: [&str; 2] = ["Uncle Bill", "@0xUncleBill"];
/// The X handle of the author, for `twitter:site` and `twitter:creator`.
pub const AUTHOR_X: &str = "@0xUncleBill";
/// The profiles of the author, for the JSON-LD `sameAs`.
pub const AUTHOR_PROFILES: [&str; 2] = ["https://github.com/eserilev", "https://x.com/0xUncleBill"];

/// The post data for a post page.
#[derive(Debug, Clone, Copy)]
pub struct Article<'a> {
    /// `published_at`, RFC 3339.
    pub published: &'a str,
    /// `updated_at`, RFC 3339.
    pub modified: &'a str,
    /// The topic name, then the tags.
    pub keywords: &'a [String],
}

/// The kind of page.
#[derive(Debug, Clone, Copy)]
pub enum PageKind<'a> {
    /// The home page, the other plain pages, and 404 pages.
    Website,
    /// A topic page. The title is the topic name.
    Topic,
    /// A public post.
    Post(Article<'a>),
}

/// What the `<head>` describes.
#[derive(Debug, Clone, Copy)]
pub struct Head<'a> {
    /// Page title, without the site name. `None` for the home page.
    pub title: Option<&'a str>,
    pub description: &'a str,
    /// Absolute URL of the page.
    pub url: &'a str,
    /// The site origin, with no trailing slash.
    pub origin: &'a str,
    /// `og:type` is `article` for a post, else `website`.
    pub kind: PageKind<'a>,
    /// Adds `<meta name="robots" content="noindex">`. Removes the canonical link.
    pub noindex: bool,
    /// The site title (spec 4.9), in titles and `og:site_name`.
    pub site: &'a str,
}

/// Escapes text for use in element content and quoted attributes.
///
/// [`logbook_core::escape_html`] does the work (theorem T14). It changes only ASCII
/// bytes, so the result is valid UTF-8.
#[must_use]
pub fn escape(s: &str) -> String {
    String::from_utf8(logbook_core::escape_html(s.as_bytes()))
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// The tags that go between `<!--head-->` and `<!--/head-->` in `index.html`.
#[must_use]
pub fn head_tags(h: &Head<'_>) -> String {
    use std::fmt::Write as _;
    let full_title = match h.title {
        Some(t) => format!("{t} · {}", h.site),
        None => h.site.to_string(),
    };
    let og_title = h.title.unwrap_or(h.site);
    let (title, og_title, desc, url) = (
        escape(&full_title),
        escape(og_title),
        escape(h.description),
        escape(h.url),
    );
    let og_type = if matches!(h.kind, PageKind::Post(_)) {
        "article"
    } else {
        "website"
    };
    let mut out = format!(
        "<title>{title}</title>\n\
<meta name=\"description\" content=\"{desc}\">\n\
<meta name=\"author\" content=\"{author}\">\n\
<meta property=\"og:site_name\" content=\"{site}\">\n\
<meta property=\"og:title\" content=\"{og_title}\">\n\
<meta property=\"og:description\" content=\"{desc}\">\n\
<meta property=\"og:type\" content=\"{og_type}\">\n\
<meta property=\"og:url\" content=\"{url}\">\n\
<meta name=\"twitter:card\" content=\"summary\">\n\
<meta name=\"twitter:site\" content=\"{x}\">\n\
<meta name=\"twitter:creator\" content=\"{x}\">\n",
        site = escape(h.site),
        author = escape(AUTHOR_NAME),
        x = escape(AUTHOR_X),
    );
    if let PageKind::Post(a) = h.kind {
        let _ = write!(
            out,
            "<meta property=\"article:published_time\" content=\"{}\">\n\
<meta property=\"article:modified_time\" content=\"{}\">\n\
<meta property=\"article:author\" content=\"{}\">\n",
            escape(a.published),
            escape(a.modified),
            escape(AUTHOR_NAME),
        );
    }
    if h.noindex {
        out.push_str("<meta name=\"robots\" content=\"noindex\">\n");
    } else {
        let _ = writeln!(out, "<link rel=\"canonical\" href=\"{url}\">");
    }
    let _ = writeln!(
        out,
        "<script type=\"application/ld+json\">{}</script>",
        script_json(&json_ld(h))
    );
    out
}

/// The schema.org data of a page: the author, the site, and the post or the topic.
#[must_use]
pub fn json_ld(h: &Head<'_>) -> Value {
    let home = format!("{}/", h.origin);
    let person = format!("{}/#person", h.origin);
    let website = format!("{}/#website", h.origin);
    let mut graph = vec![
        json!({
            "@type": "Person",
            "@id": person,
            "name": AUTHOR_NAME,
            "alternateName": AUTHOR_ALIASES,
            "url": home,
            "sameAs": AUTHOR_PROFILES,
        }),
        json!({
            "@type": "WebSite",
            "@id": website,
            "name": h.site,
            "url": home,
            "inLanguage": "en",
            "author": { "@id": person },
        }),
    ];
    match h.kind {
        PageKind::Post(a) => graph.push(json!({
            "@type": "BlogPosting",
            "@id": format!("{}#post", h.url),
            "headline": h.title.unwrap_or(h.site),
            "description": h.description,
            "datePublished": a.published,
            "dateModified": a.modified,
            "url": h.url,
            "mainEntityOfPage": h.url,
            "author": { "@id": person },
            "isPartOf": { "@id": website },
            "keywords": a.keywords,
            "inLanguage": "en",
        })),
        PageKind::Topic if !h.noindex => graph.push(json!({
            "@type": "CollectionPage",
            "@id": h.url,
            "name": h.title.unwrap_or(h.site),
            "url": h.url,
            "isPartOf": { "@id": website },
            "inLanguage": "en",
        })),
        PageKind::Topic | PageKind::Website => {}
    }
    json!({ "@context": "https://schema.org", "@graph": graph })
}

/// JSON text for a `<script>` element. `<`, `>`, and `&` become `<`, `>`,
/// and `&`. Then no value can close the element or open a comment, and the
/// JSON has the same meaning. These characters occur only in JSON strings, so the
/// escapes are valid.
#[must_use]
pub fn script_json(v: &Value) -> String {
    v.to_string()
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

/// Replaces the head block of `index.html` with `tags`.
///
/// # Panics
///
/// Panics if `index.html` has no `<!--head-->...<!--/head-->` block. A test checks
/// the real file, and [`crate::AppState::new`] checks it at startup.
#[must_use]
pub fn inject(index_html: &str, tags: &str) -> String {
    let start = index_html.find(START).expect("head start marker") + START.len();
    let end = index_html.find(END).expect("head end marker");
    format!("{}\n{tags}{}", &index_html[..start], &index_html[end..])
}

/// Start marker of the head block.
pub const START: &str = "<!--head-->";
/// End marker of the head block.
pub const END: &str = "<!--/head-->";

#[cfg(test)]
mod tests {
    use super::*;

    fn head<'a>(title: Option<&'a str>, kind: PageKind<'a>) -> Head<'a> {
        Head {
            title,
            description: "d",
            url: "https://e.com/posts/x",
            origin: "https://e.com",
            kind,
            noindex: false,
            site: "S",
        }
    }

    /// The JSON in the `<script type="application/ld+json">` element.
    fn ld(tags: &str) -> Value {
        let start = tags.find("<script type=\"application/ld+json\">").unwrap()
            + "<script type=\"application/ld+json\">".len();
        let end = start + tags[start..].find("</script>").unwrap();
        serde_json::from_str(&tags[start..end]).unwrap()
    }

    #[test]
    fn values_cannot_break_out_of_attributes() {
        let h = Head {
            title: Some("\"><script>x</script>"),
            description: "a\" onload=\"x",
            url: "https://e.com/\"",
            origin: "https://e.com",
            kind: PageKind::Post(Article {
                published: "\"><b>",
                modified: "2026-01-01T00:00:00Z",
                keywords: &[],
            }),
            noindex: false,
            site: "S",
        };
        let all = head_tags(&h);
        assert!(!all.contains("<script>") && !all.contains("x</script>"));
        // The tags before the JSON-LD. The JSON has its own escape test.
        let tags = &all[..all.find("<script type=").unwrap()];
        assert!(!tags.contains("\" onload"));
        assert!(!tags.contains("<b>"));
        assert!(tags.contains("&quot;&gt;&lt;script&gt;"));
    }

    #[test]
    fn json_ld_cannot_close_its_script_element() {
        let title = "</script><script>alert(1)</script><!-- & -->";
        let tags = head_tags(&head(
            Some(title),
            PageKind::Post(Article {
                published: "2026-01-01T00:00:00Z",
                modified: "2026-01-02T00:00:00Z",
                keywords: &["</script>".to_string()],
            }),
        ));
        // One script element, and it ends at the end of the tags.
        assert_eq!(tags.matches("<script").count(), 1, "{tags}");
        assert_eq!(tags.matches("</script>").count(), 1, "{tags}");
        assert!(tags.trim_end().ends_with("</script>"), "{tags}");
        assert!(!tags.contains("<!--"), "{tags}");
        // The JSON keeps the real title.
        let v = ld(&tags);
        assert_eq!(v["@graph"][2]["headline"], title);
        assert_eq!(v["@graph"][2]["keywords"][0], "</script>");
    }

    #[test]
    fn noindex_is_added_on_request_and_removes_the_canonical_link() {
        let mut h = head(None, PageKind::Website);
        let tags = head_tags(&h);
        assert!(!tags.contains("noindex"));
        assert!(tags.contains("<link rel=\"canonical\" href=\"https://e.com/posts/x\">"));
        h.noindex = true;
        let tags = head_tags(&h);
        assert!(tags.contains("content=\"noindex\""));
        assert!(!tags.contains("canonical"));
    }

    #[test]
    fn every_page_names_the_author() {
        let tags = head_tags(&head(None, PageKind::Website));
        assert!(tags.contains("<meta name=\"author\" content=\"Eitan Seri-Levi\">"));
        assert!(tags.contains("<meta name=\"twitter:card\" content=\"summary\">"));
        assert!(tags.contains("<meta name=\"twitter:site\" content=\"@0xUncleBill\">"));
        assert!(tags.contains("<meta name=\"twitter:creator\" content=\"@0xUncleBill\">"));
        assert!(!tags.contains("article:"));
        let v = ld(&tags);
        let graph = v["@graph"].as_array().unwrap();
        assert_eq!(graph.len(), 2);
        assert_eq!(v["@context"], "https://schema.org");
        assert_eq!(graph[0]["@type"], "Person");
        assert_eq!(graph[0]["@id"], "https://e.com/#person");
        assert_eq!(graph[0]["name"], "Eitan Seri-Levi");
        assert_eq!(
            graph[0]["alternateName"],
            json!(["Uncle Bill", "@0xUncleBill"])
        );
        assert_eq!(graph[0]["url"], "https://e.com/");
        assert_eq!(
            graph[0]["sameAs"],
            json!(["https://github.com/eserilev", "https://x.com/0xUncleBill"])
        );
        assert_eq!(graph[1]["@type"], "WebSite");
        assert_eq!(graph[1]["name"], "S");
        assert_eq!(graph[1]["author"]["@id"], "https://e.com/#person");
    }

    #[test]
    fn a_post_has_article_times_and_a_blog_posting() {
        let kw = ["Rust".to_string(), "ssz".to_string()];
        let tags = head_tags(&head(
            Some("T"),
            PageKind::Post(Article {
                published: "2026-01-01T00:00:00Z",
                modified: "2026-01-02T00:00:00Z",
                keywords: &kw,
            }),
        ));
        assert!(tags.contains(
            "<meta property=\"article:published_time\" content=\"2026-01-01T00:00:00Z\">"
        ));
        assert!(tags.contains(
            "<meta property=\"article:modified_time\" content=\"2026-01-02T00:00:00Z\">"
        ));
        assert!(tags.contains("<meta property=\"article:author\" content=\"Eitan Seri-Levi\">"));
        let v = ld(&tags);
        let p = &v["@graph"][2];
        assert_eq!(p["@type"], "BlogPosting");
        assert_eq!(p["headline"], "T");
        assert_eq!(p["description"], "d");
        assert_eq!(p["datePublished"], "2026-01-01T00:00:00Z");
        assert_eq!(p["dateModified"], "2026-01-02T00:00:00Z");
        assert_eq!(p["url"], "https://e.com/posts/x");
        assert_eq!(p["mainEntityOfPage"], "https://e.com/posts/x");
        assert_eq!(p["author"]["@id"], "https://e.com/#person");
        assert_eq!(p["keywords"], json!(["Rust", "ssz"]));
        assert_eq!(p["inLanguage"], "en");
    }

    #[test]
    fn a_topic_is_a_collection_page() {
        let v = ld(&head_tags(&head(Some("Rust"), PageKind::Topic)));
        assert_eq!(v["@graph"][2]["@type"], "CollectionPage");
        assert_eq!(v["@graph"][2]["name"], "Rust");
    }

    #[test]
    fn inject_replaces_only_the_block() {
        let html = "<head><!--head--><title>old</title><!--/head--><link></head>";
        let out = inject(html, "<title>new</title>\n");
        assert_eq!(
            out,
            "<head><!--head-->\n<title>new</title>\n<!--/head--><link></head>"
        );
    }
}
