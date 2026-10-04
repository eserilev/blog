//! Spec 7.3 `media_key`: any path bytes. Agrees with theorem T10.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_core::media_key_ok;

fuzz_target!(|key: &[u8]| {
    let hex = |c: &u8| c.is_ascii_digit() || (b'a'..=b'f').contains(c);
    let want = key.len() >= 68
        && key[..64].iter().all(hex)
        && key[64] == b'.'
        && matches!(&key[65..], b"png" | b"jpg" | b"gif" | b"webp");
    assert_eq!(media_key_ok(key), want);
    if want {
        assert!(!key.contains(&b'/') && !key.contains(&b'\\') && !key.contains(&0));
    }
});
