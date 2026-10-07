//! Spec 7.3 `media_key`: any path bytes. Agrees with theorems T10 and T18 (files).
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_core::{media_key_ok, video_key_ok};

fuzz_target!(|key: &[u8]| {
    let hex = |c: &u8| c.is_ascii_digit() || (b'a'..=b'f').contains(c);
    let shape = key.len() >= 68 && key[..64].iter().all(hex) && key[64] == b'.';
    let video = shape && matches!(&key[65..], b"mp4" | b"webm");
    let want = video || (shape && matches!(&key[65..], b"png" | b"jpg" | b"gif" | b"webp"));
    assert_eq!(media_key_ok(key), want);
    assert_eq!(video_key_ok(key), video);
    if want {
        assert!(!key.contains(&b'/') && !key.contains(&b'\\') && !key.contains(&0));
    }
});
