//! Media keys (spec 6.8, theorems T10 and T18).
//!
//! The server names each upload `<sha256 hex>.<ext>`. `/media/{key}` accepts exactly
//! that shape, so no key can leave its folder in the bucket. Images have the
//! extensions `png`, `jpg`, `gif`, and `webp`. Videos have `mp4` and `webm`.

// Plain comparisons, not `is_ascii_digit` or ranges: Aeneas has no model for those.
fn is_lower_hex(c: u8) -> bool {
    (c >= b'0' && c <= b'9') || (c >= b'a' && c <= b'f')
}

/// `key[at..]` equals `ext` exactly.
fn tail_is(key: &[u8], at: usize, ext: &[u8]) -> bool {
    if key.len() != at + ext.len() {
        return false;
    }
    let mut i = 0;
    while i < ext.len() {
        if key[at + i] != ext[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// `key` starts with 64 lowercase hex digits and a `.`.
fn hex_dot(key: &[u8]) -> bool {
    if key.len() < 65 {
        return false;
    }
    let mut i = 0;
    while i < 64 {
        if !is_lower_hex(key[i]) {
            return false;
        }
        i += 1;
    }
    key[64] == b'.'
}

/// The extension after the `.` at index 64 is `mp4` or `webm`.
fn video_ext(key: &[u8]) -> bool {
    tail_is(key, 65, b"mp4") || tail_is(key, 65, b"webm")
}

/// True if and only if `key` is 64 lowercase hex digits, a `.`, and one of
/// `png`, `jpg`, `gif`, `webp`, `mp4`, `webm`.
#[must_use]
pub fn media_key_ok(key: &[u8]) -> bool {
    hex_dot(key)
        && (tail_is(key, 65, b"png")
            || tail_is(key, 65, b"jpg")
            || tail_is(key, 65, b"gif")
            || tail_is(key, 65, b"webp")
            || video_ext(key))
}

/// True if and only if `key` is 64 lowercase hex digits, a `.`, and `mp4` or `webm`.
/// Only these keys become the source of a video (spec 4.10).
#[must_use]
pub fn video_key_ok(key: &[u8]) -> bool {
    hex_dot(key) && video_ext(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn accepts_server_keys() {
        for ext in ["png", "jpg", "gif", "webp", "mp4", "webm"] {
            assert!(media_key_ok(format!("{HEX}.{ext}").as_bytes()), "{ext}");
        }
        for ext in ["mp4", "webm"] {
            assert!(video_key_ok(format!("{HEX}.{ext}").as_bytes()), "{ext}");
        }
        for ext in ["png", "jpg", "gif", "webp"] {
            assert!(!video_key_ok(format!("{HEX}.{ext}").as_bytes()), "{ext}");
        }
    }

    #[test]
    fn rejects_everything_else() {
        for key in [
            String::new(),
            format!("{HEX}.svg"),
            format!("{HEX}.PNG"),
            format!("{HEX}.pngx"),
            format!("{HEX}png"),
            format!("{}.png", HEX.to_uppercase()),
            format!("../{HEX}.png"),
            format!("{}/.png", &HEX[..63]),
            format!("{}\0.png", &HEX[..63]),
            format!("{HEX}.png/.."),
            format!("{HEX}.mp4x"),
            format!("{HEX}.MP4"),
            format!("{HEX}.mov"),
            format!("{HEX}.webmp"),
        ] {
            assert!(!media_key_ok(key.as_bytes()), "{key:?}");
            assert!(!video_key_ok(key.as_bytes()), "{key:?}");
        }
    }

    proptest! {
        /// T10.
        #[test]
        fn accepts_exactly_the_spec_shape(key in prop::collection::vec(any::<u8>(), 0..80)) {
            let want = key.len() >= 68
                && key[..64].iter().all(|&c| is_lower_hex(c))
                && key[64] == b'.';
            let ext = key.get(65..).unwrap_or_default();
            let image = matches!(ext, b"png" | b"jpg" | b"gif" | b"webp");
            let video = matches!(ext, b"mp4" | b"webm");
            prop_assert_eq!(media_key_ok(&key), want && (image || video));
            prop_assert_eq!(video_key_ok(&key), want && video);
        }

        #[test]
        fn accepted_keys_cannot_escape(hex in "[0-9a-f]{64}", ext in "(png|jpg|gif|webp|mp4|webm)") {
            let key = format!("{hex}.{ext}");
            prop_assert!(media_key_ok(key.as_bytes()));
            prop_assert!(!key.contains('/') && !key.contains('\\') && !key.contains("..") && !key.contains('\0'));
        }
    }
}
