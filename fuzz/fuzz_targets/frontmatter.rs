//! Spec 7.3 `frontmatter`: any bytes as an export file. No panic. If it parses,
//! writing it and parsing it again gives the same post.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_server::export::{parse, write};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Some(p) = parse(text) {
        assert_eq!(parse(&write(&p)), Some(p));
    }
});
