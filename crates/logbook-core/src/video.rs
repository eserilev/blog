//! YouTube addresses in a video block (spec 4.10, theorem T18).
//!
//! The first line of a video block is a YouTube address. [`youtube_id`] finds
//! the 11-byte video ID in it. The renderer builds the embed from the ID only,
//! never from the rest of the address.

/// The length of a YouTube video ID.
pub const YOUTUBE_ID_LEN: usize = 11;

/// `A-Z`, `a-z`, `0-9`, `_`, or `-`: the bytes of a YouTube video ID.
fn is_id_byte(c: u8) -> bool {
    (c >= b'A' && c <= b'Z')
        || (c >= b'a' && c <= b'z')
        || (c >= b'0' && c <= b'9')
        || c == b'_'
        || c == b'-'
}

/// `s` starts with `p`.
fn starts(s: &[u8], p: &[u8]) -> bool {
    if s.len() < p.len() {
        return false;
    }
    let mut j = 0;
    while j < p.len() {
        if s[j] != p[j] {
            return false;
        }
        j += 1;
    }
    true
}

/// The length of the accepted address part before the ID, or 0 if `url` starts
/// with no accepted part. No accepted part starts another, so at most one matches.
fn youtube_prefix(url: &[u8]) -> usize {
    if starts(url, b"https://www.youtube.com/watch?v=") {
        32
    } else if starts(url, b"https://youtube.com/watch?v=") {
        28
    } else if starts(url, b"https://m.youtube.com/watch?v=") {
        30
    } else if starts(url, b"https://youtu.be/") {
        17
    } else if starts(url, b"https://www.youtube.com/shorts/") {
        31
    } else if starts(url, b"https://youtube.com/shorts/") {
        27
    } else {
        0
    }
}

/// `url[at..at + 11]` are ID bytes, and `url` ends after them or goes on with
/// `?`, `&`, or `#`.
fn id_at(url: &[u8], at: usize) -> bool {
    if at > url.len() || url.len() - at < YOUTUBE_ID_LEN {
        return false;
    }
    let mut j = 0;
    while j < YOUTUBE_ID_LEN {
        if !is_id_byte(url[at + j]) {
            return false;
        }
        j += 1;
    }
    let end = at + YOUTUBE_ID_LEN;
    end == url.len() || url[end] == b'?' || url[end] == b'&' || url[end] == b'#'
}

/// The start of the video ID in a YouTube address, or `None`.
///
/// Accepted forms, with `<id>` 11 bytes of `A-Z a-z 0-9 _ -`:
/// `https://www.youtube.com/watch?v=<id>`, `https://youtube.com/watch?v=<id>`,
/// `https://m.youtube.com/watch?v=<id>`, `https://youtu.be/<id>`,
/// `https://www.youtube.com/shorts/<id>`, `https://youtube.com/shorts/<id>`.
/// After the ID: the end, or `?`, `&`, or `#` and any bytes.
#[must_use]
pub fn youtube_id(url: &[u8]) -> Option<usize> {
    let at = youtube_prefix(url);
    if at > 0 && id_at(url, at) {
        Some(at)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const PREFIXES: [&str; 6] = [
        "https://www.youtube.com/watch?v=",
        "https://youtube.com/watch?v=",
        "https://m.youtube.com/watch?v=",
        "https://youtu.be/",
        "https://www.youtube.com/shorts/",
        "https://youtube.com/shorts/",
    ];

    /// The rule, written plainly.
    fn reference(url: &[u8]) -> Option<usize> {
        let p = PREFIXES.iter().find(|p| url.starts_with(p.as_bytes()))?;
        let rest = &url[p.len()..];
        let ok = rest.len() >= YOUTUBE_ID_LEN
            && rest[..YOUTUBE_ID_LEN]
                .iter()
                .all(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-')
            && matches!(rest.get(YOUTUBE_ID_LEN), None | Some(b'?' | b'&' | b'#'));
        ok.then_some(p.len())
    }

    fn id(url: &str) -> Option<&str> {
        youtube_id(url.as_bytes()).map(|i| &url[i..i + YOUTUBE_ID_LEN])
    }

    #[test]
    fn prefix_lengths_match() {
        for p in PREFIXES {
            assert_eq!(youtube_prefix(p.as_bytes()), p.len(), "{p}");
        }
    }

    #[test]
    fn accepts_the_known_forms() {
        for url in [
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://youtube.com/watch?v=dQw4w9WgXcQ&t=42s",
            "https://m.youtube.com/watch?v=dQw4w9WgXcQ#t=1",
            "https://youtu.be/dQw4w9WgXcQ?si=abc",
            "https://www.youtube.com/shorts/dQw4w9WgXcQ",
            "https://youtube.com/shorts/dQw4w9WgXcQ",
        ] {
            assert_eq!(id(url), Some("dQw4w9WgXcQ"), "{url}");
        }
        assert_eq!(id("https://youtu.be/a_b-c_d-e_f"), Some("a_b-c_d-e_f"));
    }

    #[test]
    fn rejects_everything_else() {
        for url in [
            "",
            "dQw4w9WgXcQ",
            "javascript:alert(1)//https://youtu.be/dQw4w9WgXcQ",
            "http://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://www.youtube.com.evil.test/watch?v=dQw4w9WgXcQ",
            "https://evil.test/https://youtu.be/dQw4w9WgXcQ",
            "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ",
            "https://www.youtube.com/watch?v=dQw4w9WgXc",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQQ",
            "https://www.youtube.com/watch?v=dQw4w9WgX\"Q",
            "https://www.youtube.com/watch?v=dQw4w9WgX<Q",
            "https://youtu.be/dQw4w9WgXcQ\"><script>",
            "https://youtu.be/dQw4w9WgXcQ/",
            "https://youtu.be/dQw4w9WgXc%51",
            "https://YOUTU.BE/dQw4w9WgXcQ",
            " https://youtu.be/dQw4w9WgXcQ",
            "https://youtu.be/",
        ] {
            assert_eq!(id(url), None, "{url}");
        }
    }

    proptest! {
        /// T18, the YouTube half: the same answer as the plain rule.
        #[test]
        fn agrees_with_the_rule(
            p in prop::sample::select(PREFIXES.to_vec()),
            rest in prop::collection::vec(prop_oneof![
                Just(b'a'), Just(b'Z'), Just(b'0'), Just(b'_'), Just(b'-'), Just(b'?'),
                Just(b'&'), Just(b'#'), Just(b'"'), Just(b'<'), Just(b'/'), any::<u8>(),
            ], 0..16),
        ) {
            let mut url = p.as_bytes().to_vec();
            url.extend_from_slice(&rest);
            prop_assert_eq!(youtube_id(&url), reference(&url));
        }

        #[test]
        fn any_bytes_agree(url in prop::collection::vec(any::<u8>(), 0..64)) {
            prop_assert_eq!(youtube_id(&url), reference(&url));
        }

        /// An accepted ID never holds a byte that HTML or a URL treats as special.
        #[test]
        fn accepted_ids_are_plain(url in "https://youtu\\.be/[ -~]{0,14}") {
            if let Some(i) = youtube_id(url.as_bytes()) {
                prop_assert!(url.as_bytes()[i..i + YOUTUBE_ID_LEN]
                    .iter()
                    .all(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-'));
            }
        }
    }
}
