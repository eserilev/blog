//! Spec 7.3 `render`: any bytes as markdown. No panic, and the output passes the
//! sanitizer check of spec 7.2.
#![no_main]

use libfuzzer_sys::fuzz_target;
use scraper::{Html, Node};

fuzz_target!(|data: &[u8]| {
    let md = String::from_utf8_lossy(data);
    let html = logbook_render::render(&md);
    let doc = Html::parse_fragment(&html);
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
                "forbidden <{name}>"
            );
            for (attr, value) in el.attrs() {
                assert!(
                    !attr.starts_with("on") && attr != "style",
                    "attribute {attr}"
                );
                let v = value.trim().to_ascii_lowercase();
                assert!(
                    !v.starts_with("javascript:")
                        && !v.starts_with("data:")
                        && !v.starts_with("vbscript:"),
                    "URL {value}"
                );
            }
        }
    }
});
