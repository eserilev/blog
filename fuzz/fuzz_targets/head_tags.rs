//! Spec 7.3 `head_tags`: any title and summary. The output parses to exactly the
//! expected elements, so no value breaks out of its attribute or element.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_server::head::{Head, head_tags};
use scraper::{Html, Node};

fuzz_target!(|input: (&str, &str, bool)| {
    let (title, summary, noindex) = input;
    let tags = head_tags(&Head {
        title: Some(title),
        description: summary,
        url: "https://e.test/x",
        article: true,
        noindex,
    });
    let doc = Html::parse_document(&format!(
        "<!doctype html><html><head>{tags}</head><body></body></html>"
    ));
    let mut names = Vec::new();
    for node in doc.tree.nodes() {
        if let Node::Element(el) = node.value() {
            names.push(el.name().to_string());
        }
    }
    let metas = names.iter().filter(|n| *n == "meta").count();
    assert_eq!(names.iter().filter(|n| *n == "title").count(), 1, "{tags}");
    assert_eq!(metas, if noindex { 8 } else { 7 }, "{tags}");
    // html, head, body, title, and the metas.
    assert_eq!(
        names.len(),
        3 + 1 + metas,
        "unexpected elements: {names:?}\n{tags}"
    );
});
