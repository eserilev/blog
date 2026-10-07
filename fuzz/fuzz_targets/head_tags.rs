//! Spec 7.3 `head_tags`: any title, summary, and site title. The output parses to exactly the
//! expected elements, so no value breaks out of its attribute or element. The JSON-LD
//! parses and keeps the title.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_server::head::{Article, Head, PageKind, head_tags};
use scraper::{Html, Node};

fuzz_target!(|input: (&str, &str, &str, bool)| {
    let (title, summary, site, noindex) = input;
    let keywords = [title.to_string()];
    let tags = head_tags(&Head {
        title: Some(title),
        description: summary,
        url: "https://e.test/x",
        origin: "https://e.test",
        kind: PageKind::Post(Article {
            published: summary,
            modified: site,
            keywords: &keywords,
        }),
        noindex,
        site,
    });
    let doc = Html::parse_document(&format!(
        "<!doctype html><html><head>{tags}</head><body></body></html>"
    ));
    let mut names = Vec::new();
    let mut script = None;
    for node in doc.tree.nodes() {
        if let Node::Element(el) = node.value() {
            names.push(el.name().to_string());
            if el.name() == "script" {
                script = node
                    .first_child()
                    .and_then(|c| c.value().as_text().map(|t| t.to_string()));
            }
        }
    }
    let count = |name: &str| names.iter().filter(|n| *n == name).count();
    let metas = count("meta");
    assert_eq!(count("title"), 1, "{tags}");
    assert_eq!(metas, if noindex { 14 } else { 13 }, "{tags}");
    assert_eq!(count("link"), usize::from(!noindex), "{tags}");
    assert_eq!(count("script"), 1, "{tags}");
    // html, head, body, title, the metas, the canonical link, and the script.
    assert_eq!(
        names.len(),
        3 + 1 + metas + usize::from(!noindex) + 1,
        "unexpected elements: {names:?}\n{tags}"
    );
    let ld: serde_json::Value =
        serde_json::from_str(&script.expect("script text")).expect("JSON-LD");
    assert_eq!(ld["@graph"][2]["headline"], title, "{tags}");
    assert_eq!(ld["@graph"][2]["keywords"][0], title, "{tags}");
});
