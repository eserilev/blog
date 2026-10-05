//! Spec 7.3 `post_input`: any bytes as the JSON body of `PUT /api/owner/posts/{id}`.
//! No panic. Accepted input always meets the limits.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_server::posts::{BODY_MAX, PostInput, SUMMARY_MAX, TAG_MAX, TAGS_MAX, TITLE_MAX};

fuzz_target!(|data: &[u8]| {
    let Ok(input) = serde_json::from_slice::<PostInput>(data) else {
        return;
    };
    let Ok(v) = input.validate() else { return };
    assert!(!v.title.is_empty() && v.title.chars().count() <= TITLE_MAX);
    assert!(!v.title.chars().any(char::is_control));
    assert!(v.summary.chars().count() <= SUMMARY_MAX);
    assert!(v.body_md.len() <= BODY_MAX);
    let tags: Vec<String> = serde_json::from_str(&v.tags_json).expect("tags_json is JSON");
    assert!(tags.len() <= TAGS_MAX);
    for t in tags {
        assert!(!t.is_empty() && t.len() <= TAG_MAX);
        assert!(
            t.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        );
    }
});
