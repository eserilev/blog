//! Spec 7.3 `slug`: any title bytes and id. The slug follows the rules of spec 4.3
//! and making a slug from it changes nothing.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_core::{SLUG_MAX, make_slug};

fuzz_target!(|input: (u64, &[u8])| {
    let (id, title) = input;
    let s = make_slug(title, id);
    assert!(!s.is_empty() && s.len() <= SLUG_MAX);
    assert!(
        s.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
    );
    assert!(s[0] != b'-' && s[s.len() - 1] != b'-');
    assert!(!s.windows(2).any(|w| w == b"--"));
    assert_eq!(make_slug(&s, id), s);
});
