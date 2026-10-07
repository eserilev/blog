//! Video blocks (spec 4.10).
//!
//! A fenced block with the info string `video` holds one or two lines:
//!
//! ```text
//! https://www.youtube.com/watch?v=<id>
//! Optional title
//! ```
//!
//! A valid block becomes one neutral `<video-embed>` element. The `<video-embed>`
//! component (`static/js/video.js`) gives it the look of the current mode. Without
//! scripts, it is a link to YouTube. The ID comes from `logbook_core::youtube_id`
//! (theorem T18). No other part of the address reaches the output. Any other
//! block stays a plain code block.

use std::fmt::Write as _;

use logbook_core::{YOUTUBE_ID_LEN, youtube_id};

use crate::escape_attr;

/// Most characters in a video title.
pub const TITLE_MAX: usize = 200;

/// A valid video block: the YouTube ID and the title (empty if none).
#[derive(Debug, PartialEq, Eq)]
pub struct Block<'a> {
    pub id: &'a str,
    pub title: &'a str,
}

/// The YouTube ID of a first line, if the line is an accepted address.
#[must_use]
pub fn id(line: &str) -> Option<&str> {
    let at = youtube_id(line.as_bytes())?;
    // The address part before the ID and the ID are ASCII, so both ends are
    // character boundaries.
    line.get(at..at + YOUTUBE_ID_LEN)
}

/// Parses the text of a `video` block. `None` if it is not a valid block.
#[must_use]
pub fn parse(literal: &str) -> Option<Block<'_>> {
    let mut lines = literal.lines().map(str::trim).filter(|l| !l.is_empty());
    let id = id(lines.next()?)?;
    let title = lines.next().unwrap_or("");
    let title_ok = title.chars().count() <= TITLE_MAX && !title.chars().any(char::is_control);
    (lines.next().is_none() && title_ok).then_some(Block { id, title })
}

/// The `<video-embed>` element for a block. Every value is escaped.
#[must_use]
pub fn html(block: &Block<'_>) -> String {
    let title = escape_attr(block.title);
    let id = escape_attr(block.id);
    let text = if title.is_empty() {
        "Watch on YouTube"
    } else {
        &title
    };
    let mut out = String::new();
    let _ = write!(
        out,
        "<video-embed data-id=\"{id}\" data-title=\"{title}\">\
         <p><a href=\"https://www.youtube.com/watch?v={id}\">{text}</a></p></video-embed>"
    );
    out
}

/// True if `value` is a YouTube ID: 11 bytes of `A-Z a-z 0-9 _ -`. The check is
/// `youtube_id` (T18) on the short form of the address.
#[must_use]
pub fn id_ok(value: &str) -> bool {
    value.len() == YOUTUBE_ID_LEN && id(&format!("https://youtu.be/{value}")) == Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_youtube_blocks() {
        assert_eq!(
            parse("  https://youtu.be/dQw4w9WgXcQ?t=3  \nBend on the GPU demo\n"),
            Some(Block {
                id: "dQw4w9WgXcQ",
                title: "Bend on the GPU demo"
            })
        );
        assert_eq!(
            parse("https://www.youtube.com/shorts/dQw4w9WgXcQ"),
            Some(Block {
                id: "dQw4w9WgXcQ",
                title: ""
            })
        );
    }

    #[test]
    fn refuses_bad_blocks() {
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        for block in [
            String::new(),
            "\n\n".to_string(),
            "javascript:alert(1)".to_string(),
            "https://evil.test/x.mp4".to_string(),
            format!("/media/{key}.mp4"),
            "https://youtu.be/dQw4w9WgXcQ\ntitle\nthird line".to_string(),
            format!(
                "https://youtu.be/dQw4w9WgXcQ\n{}",
                "x".repeat(TITLE_MAX + 1)
            ),
            "https://youtu.be/dQw4w9WgXcQ\nbad\u{7}title".to_string(),
            "https://youtu.be/dQw4w9WgXc".to_string(),
            "https://youtu.be/dQw4w9WgX\"Q".to_string(),
            "http://youtu.be/dQw4w9WgXcQ".to_string(),
            "https://vimeo.com/123".to_string(),
        ] {
            assert_eq!(parse(&block), None, "{block:?}");
        }
    }

    #[test]
    fn titles_are_escaped() {
        let b = parse("https://youtu.be/dQw4w9WgXcQ\n\"><script>alert(1)</script>").unwrap();
        let h = html(&b);
        assert!(!h.contains("<script"), "{h}");
        assert!(h.contains("&quot;&gt;&lt;script&gt;"), "{h}");
    }
}
