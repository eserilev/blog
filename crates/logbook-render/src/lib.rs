//! The markdown pipeline (spec 6.7): comrak, then syntect, then ammonia.
//!
//! The server calls [`render`] at save time. The editor preview runs the same
//! code, compiled to WebAssembly (build step 5), so both outputs match.

use std::{borrow::Cow, collections::HashMap, fmt, sync::LazyLock};

use comrak::{
    Options, adapters::SyntaxHighlighterAdapter, markdown_to_html_with_plugins, options::Plugins,
};
use syntect::{
    highlighting::ThemeSet,
    html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

/// Prefix for syntax highlight classes. The sanitizer allows only these classes.
const CLASS_PREFIX: &str = "hl-";
const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed {
    prefix: CLASS_PREFIX,
};
/// The syntect theme for `static/css/code.css`.
pub const CODE_THEME: &str = "base16-eighties.dark";

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// Renders markdown to safe HTML.
///
/// - CommonMark, plus GitHub tables, strikethrough, autolinks, task lists, footnotes.
/// - Raw HTML in the markdown is escaped, not passed through.
/// - Code blocks get syntax classes (`hl-*`), not inline styles, so the CSP holds.
/// - The result passes the ammonia allow-list in [`sanitize`].
#[must_use]
pub fn render(md: &str) -> String {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.render.r#unsafe = false;
    // Show raw HTML as text, so the author sees what they typed. (The default drops it.)
    options.render.escape = true;

    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&Highlighter);

    sanitize(&markdown_to_html_with_plugins(md, &options, &plugins))
}

/// Number of words in a markdown source, for the reading time.
#[must_use]
pub fn word_count(md: &str) -> u32 {
    u32::try_from(
        md.split_whitespace()
            .filter(|w| w.chars().any(char::is_alphanumeric))
            .count(),
    )
    .unwrap_or(u32::MAX)
}

/// CSS for the highlight classes. `static/css/code.css` must equal this (see tests).
///
/// # Panics
///
/// Panics if the built-in theme is missing, which a test catches.
#[must_use]
pub fn code_css() -> String {
    let themes = ThemeSet::load_defaults();
    css_for_theme_with_class_style(&themes.themes[CODE_THEME], CLASS_STYLE)
        .expect("theme renders to CSS")
}

struct Highlighter;

/// HTML-escapes an attribute value.
fn escape_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// The language tag for `data-lang`: the first word of the info string, if it is plain.
fn lang_tag(lang: Option<&str>) -> Option<&str> {
    let lang = lang?.split_whitespace().next()?;
    let ok = !lang.is_empty()
        && lang.len() <= 20
        && lang
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+#-_".contains(&b));
    ok.then_some(lang)
}

impl SyntaxHighlighterAdapter for Highlighter {
    fn write_highlighted(
        &self,
        output: &mut dyn fmt::Write,
        lang: Option<&str>,
        code: &str,
    ) -> fmt::Result {
        let syntax = lang_tag(lang).and_then(|l| SYNTAXES.find_syntax_by_token(l));
        let Some(syntax) = syntax else {
            return output.write_str(&escape_attr(code));
        };
        let mut generator =
            ClassedHTMLGenerator::new_with_class_style(syntax, &SYNTAXES, CLASS_STYLE);
        for line in LinesWithEndings::from(code) {
            if generator
                .parse_html_for_line_which_includes_newline(line)
                .is_err()
            {
                return output.write_str(&escape_attr(code));
            }
        }
        output.write_str(&generator.finalize())
    }

    /// Writes nothing. comrak gives the language to the `<code>` tag only, so
    /// [`Self::write_code_tag`] writes both opening tags. comrak closes both.
    fn write_pre_tag(
        &self,
        _output: &mut dyn fmt::Write,
        _attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        Ok(())
    }

    fn write_code_tag(
        &self,
        output: &mut dyn fmt::Write,
        attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        let lang = attributes
            .get("class")
            .and_then(|c| c.strip_prefix("language-"))
            .map(ToString::to_string);
        match lang_tag(lang.as_deref()) {
            Some(l) => {
                let l = escape_attr(l);
                write!(
                    output,
                    "<pre data-lang=\"{l}\"><code class=\"language-{l}\">"
                )
            }
            None => output.write_str("<pre><code>"),
        }
    }
}

fn plain_token(v: &str, max: usize) -> bool {
    !v.is_empty()
        && v.len() <= max
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+#-_".contains(&b))
}

static SANITIZER: LazyLock<ammonia::Builder<'static>> = LazyLock::new(|| {
    let mut b = ammonia::Builder::empty();
    b.add_tags([
        "p",
        "br",
        "hr",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "strong",
        "em",
        "del",
        "code",
        "pre",
        "span",
        "blockquote",
        "ul",
        "ol",
        "li",
        "a",
        "img",
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
        "sup",
        "section",
        "input",
    ])
    .add_tag_attributes("a", ["href", "title", "id"])
    .add_tag_attributes("img", ["src", "alt", "title"])
    .add_tag_attributes("th", ["align"])
    .add_tag_attributes("td", ["align"])
    .add_tag_attributes("pre", ["data-lang"])
    .add_tag_attributes("code", ["class"])
    .add_tag_attributes("span", ["class"])
    .add_tag_attributes("li", ["id"])
    .add_tag_attributes("sup", ["class"])
    .add_tag_attributes("section", ["class"])
    .add_tag_attributes("ol", ["start"])
    .add_tag_attributes("input", ["type", "checked", "disabled"])
    .url_schemes(["http", "https", "mailto"].into())
    .link_rel(Some("noopener noreferrer"))
    .attribute_filter(|element, attribute, value| {
        let keep = match (element, attribute) {
            ("code", "class") => value
                .strip_prefix("language-")
                .is_some_and(|l| plain_token(l, 20)),
            ("span", "class") => value
                .split(' ')
                .all(|c| c.starts_with(CLASS_PREFIX) && plain_token(c, 60)),
            ("pre", "data-lang") => plain_token(value, 20),
            ("sup", "class") => value == "footnote-ref",
            ("section", "class") => value == "footnotes",
            ("a" | "li", "id") => {
                (value.starts_with("fn-") || value.starts_with("fnref-")) && plain_token(value, 40)
            }
            ("input", "type") => value == "checkbox",
            ("th" | "td", "align") => matches!(value, "left" | "right" | "center"),
            ("ol", "start") => value.len() <= 9 && value.bytes().all(|b| b.is_ascii_digit()),
            _ => true,
        };
        keep.then(|| value.into())
    });
    b
});

/// Removes every tag, attribute, and URL scheme outside the allow-list.
#[must_use]
pub fn sanitize(html: &str) -> String {
    SANITIZER.clean(html).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use scraper::{Html, Node};

    /// Spec 7.2: the output contains nothing that runs code or loads from elsewhere.
    pub(crate) fn assert_safe(html: &str) {
        let doc = Html::parse_fragment(html);
        for node in doc.tree.nodes() {
            if let Node::Element(el) = node.value() {
                let name = el.name();
                assert!(
                    !matches!(
                        name,
                        "script"
                            | "iframe"
                            | "object"
                            | "embed"
                            | "style"
                            | "svg"
                            | "math"
                            | "form"
                            | "link"
                            | "meta"
                            | "base"
                    ),
                    "forbidden <{name}> in {html}"
                );
                for (attr, value) in el.attrs() {
                    assert!(!attr.starts_with("on"), "event handler {attr} in {html}");
                    assert!(attr != "style", "style attribute in {html}");
                    let v = value.trim().to_ascii_lowercase();
                    assert!(
                        !v.starts_with("javascript:")
                            && !v.starts_with("data:")
                            && !v.starts_with("vbscript:"),
                        "bad URL {value} in {html}"
                    );
                }
            }
        }
    }

    #[test]
    fn renders_basic_markdown() {
        let html = render("# Title\n\nSome **bold** and *em* and `code`.\n\n- a\n- b\n");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<code>code</code>"));
        assert!(html.contains("<li>a</li>"));
    }

    #[test]
    fn highlights_rust_with_classes_only() {
        let html = render("```rust\nfn main() { let x = 1; }\n```\n");
        assert!(html.contains("<pre data-lang=\"rust\">"), "{html}");
        assert!(html.contains("<code class=\"language-rust\">"), "{html}");
        assert!(html.contains("class=\"hl-"), "{html}");
        assert!(!html.contains("style="), "{html}");
    }

    #[test]
    fn unknown_languages_stay_plain_and_escaped() {
        let html = render("```nolang\n<b>x</b>\n```\n");
        assert!(html.contains("&lt;b&gt;x&lt;/b&gt;"), "{html}");
    }

    #[test]
    fn raw_html_shows_as_text() {
        let html = render("<b>bold?</b> and <script>x</script>\n");
        assert!(html.contains("&lt;b&gt;bold?&lt;/b&gt;"), "{html}");
        assert!(html.contains("&lt;script&gt;"), "{html}");
    }

    #[test]
    fn raw_html_is_escaped() {
        let html = render("<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>\n");
        assert!(!html.contains("<script"), "{html}");
        assert!(!html.contains("onerror=\""), "{html}");
        assert_safe(&html);
    }

    #[test]
    fn dangerous_links_lose_their_href() {
        for md in [
            "[x](javascript:alert(1))",
            "[x](JaVaScRiPt:alert(1))",
            "[x](data:text/html,hi)",
            "![x](data:image/png;base64,AA)",
        ] {
            let html = render(md);
            assert_safe(&html);
        }
        let html = render("[x](https://example.com)");
        assert!(html.contains("rel=\"noopener noreferrer\""), "{html}");
    }

    #[test]
    fn tables_tasks_and_footnotes_survive() {
        let html =
            render("| a | b |\n|:-|-:|\n| 1 | 2 |\n\n- [x] done\n\nText[^1].\n\n[^1]: Note.\n");
        assert!(html.contains("<table>"), "{html}");
        assert!(html.contains("type=\"checkbox\""), "{html}");
        assert!(html.contains("class=\"footnotes\""), "{html}");
        assert_safe(&html);
    }

    #[test]
    fn counts_words() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("# Hello world\n\n- one\n- two --- three"), 5);
    }

    /// `static/css/code.css` is generated. Regenerate with `UPDATE_CODE_CSS=1 cargo test`.
    #[test]
    fn code_css_is_current() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../static/css/code.css");
        let css = format!(
            "/* Generated by logbook-render from the {CODE_THEME} theme. Do not edit. */\n{}",
            code_css()
        );
        if std::env::var_os("UPDATE_CODE_CSS").is_some() {
            std::fs::write(path, &css).unwrap();
        }
        assert_eq!(
            std::fs::read_to_string(path).unwrap_or_default(),
            css,
            "run UPDATE_CODE_CSS=1 cargo test -p logbook-render"
        );
    }

    fn markdownish() -> impl Strategy<Value = String> {
        let piece = prop_oneof![
            Just("<script>x</script>".to_string()),
            Just("<img src=x onerror=y>".to_string()),
            Just("<svg onload=x>".to_string()),
            Just("<a href=\"javascript:x\">".to_string()),
            Just("[l](javascript:x)".to_string()),
            Just("![i](data:x)".to_string()),
            Just("```rust\n".to_string()),
            Just("```\n".to_string()),
            Just("<style>*{}</style>".to_string()),
            Just(" style=\"x\" ".to_string()),
            Just("**".to_string()),
            Just("\n\n".to_string()),
            Just("| a | b |\n|-|-|\n".to_string()),
            Just("[^1]: x\n".to_string()),
            "[ -~]{0,20}",
        ];
        prop::collection::vec(piece, 0..30).prop_map(|v| v.concat())
    }

    proptest! {
        /// Spec 7.2, sanitizer property: any input gives safe output.
        #[test]
        fn output_is_always_safe(md in markdownish()) {
            assert_safe(&render(&md));
        }

        #[test]
        fn arbitrary_text_is_always_safe(md in "\\PC{0,400}") {
            assert_safe(&render(&md));
        }
    }
}
