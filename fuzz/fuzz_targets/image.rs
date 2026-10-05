//! Spec 7.3 `image`: any bytes as an upload. No panic in decode and re-encode.
//! Accepted output has a valid key and no EXIF segment.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_server::media::reencode;

fuzz_target!(|data: &[u8]| {
    if let Ok((out, key)) = reencode(data) {
        assert!(logbook_core::media_key_ok(key.as_bytes()), "{key}");
        assert!(!out.windows(6).any(|w| w == b"Exif\0\0"), "EXIF survived");
    }
});
