//! `<head>` tags for page responses (spec 6.3). All values are HTML-escaped.

/// What the `<head>` describes.
#[derive(Debug, Clone, Copy)]
pub struct Head<'a> {
    /// Page title, without the site name. `None` for the home page.
    pub title: Option<&'a str>,
    pub description: &'a str,
    /// Absolute URL of the page.
    pub url: &'a str,
    /// `og:type` is `article` if true, else `website`.
    pub article: bool,
    /// Adds `<meta name="robots" content="noindex">`.
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
    let og_type = if h.article { "article" } else { "website" };
    let mut out = format!(
        "<title>{title}</title>\n\
<meta name=\"description\" content=\"{desc}\">\n\
<meta property=\"og:site_name\" content=\"{site}\">\n\
<meta property=\"og:title\" content=\"{og_title}\">\n\
<meta property=\"og:description\" content=\"{desc}\">\n\
<meta property=\"og:type\" content=\"{og_type}\">\n\
<meta property=\"og:url\" content=\"{url}\">\n\
<meta name=\"twitter:card\" content=\"summary\">\n",
        site = escape(h.site),
    );
    if h.noindex {
        out.push_str("<meta name=\"robots\" content=\"noindex\">\n");
    }
    out
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

    #[test]
    fn values_cannot_break_out_of_attributes() {
        let h = Head {
            title: Some("\"><script>x</script>"),
            description: "a\" onload=\"x",
            url: "https://e.com/\"",
            article: true,
            noindex: false,
            site: "S",
        };
        let tags = head_tags(&h);
        assert!(!tags.contains("<script"));
        assert!(!tags.contains("\" onload"));
        assert!(tags.contains("&quot;&gt;&lt;script&gt;"));
    }

    #[test]
    fn noindex_is_added_on_request() {
        let mut h = Head {
            title: None,
            description: "",
            url: "/",
            article: false,
            noindex: false,
            site: "S",
        };
        assert!(!head_tags(&h).contains("noindex"));
        h.noindex = true;
        assert!(head_tags(&h).contains("content=\"noindex\""));
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
