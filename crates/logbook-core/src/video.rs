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

    const PREFIXES: [&str; 6] = [
        "https://www.youtube.com/watch?v=",
        "https://youtube.com/watch?v=",
        "https://m.youtube.com/watch?v=",
        "https://youtu.be/",
        "https://www.youtube.com/shorts/",
        "https://youtube.com/shorts/",
    ];

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
}
