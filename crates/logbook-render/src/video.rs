//! Video blocks (spec 4.10).
//!
//! A fenced block with the info string `video` holds one or two lines:
//!
//! ```text
//! /media/<key>.mp4            or a YouTube address
//! Optional title
//! ```
//!
//! A valid block becomes one neutral `<video-embed>` element. The `<video-embed>`
//! component (`static/js/video.js`) gives it the look of the current mode. Without
//! scripts, a file is a plain `<video controls>`, and a YouTube video is a link.
//! The source comes from `logbook_core` only (theorem T18): a video key, or the
//! 11-byte YouTube ID. No other part of the line reaches the output.
//! Any other block stays a plain code block.

use std::fmt::Write as _;

use logbook_core::{YOUTUBE_ID_LEN, video_key_ok, youtube_id};

use crate::escape_attr;

/// Most characters in a video title.
pub const TITLE_MAX: usize = 200;

/// Where a video comes from.
#[derive(Debug, PartialEq, Eq)]
pub enum Source<'a> {
    /// An uploaded file: the media key, for example `<sha256>.mp4`.
    File(&'a str),
    /// A YouTube video: the 11-byte ID.
    YouTube(&'a str),
}

/// A valid video block: the source and the title (empty if none).
#[derive(Debug, PartialEq, Eq)]
pub struct Block<'a> {
    pub source: Source<'a>,
    pub title: &'a str,
}

/// The source of a first line: `/media/<video key>`, or an accepted YouTube address.
#[must_use]
pub fn source(line: &str) -> Option<Source<'_>> {
    if let Some(key) = line.strip_prefix("/media/") {
        return video_key_ok(key.as_bytes()).then_some(Source::File(key));
    }
    let at = youtube_id(line.as_bytes())?;
    // The address part before the ID and the ID are ASCII, so both ends are
    // character boundaries.
    line.get(at..at + YOUTUBE_ID_LEN).map(Source::YouTube)
}

/// Parses the text of a `video` block. `None` if it is not a valid block.
#[must_use]
pub fn parse(literal: &str) -> Option<Block<'_>> {
    let mut lines = literal.lines().map(str::trim).filter(|l| !l.is_empty());
    let source = source(lines.next()?)?;
    let title = lines.next().unwrap_or("");
    let title_ok = title.chars().count() <= TITLE_MAX && !title.chars().any(char::is_control);
    (lines.next().is_none() && title_ok).then_some(Block { source, title })
}

/// The `<video-embed>` element for a block. Every value is escaped.
#[must_use]
pub fn html(block: &Block<'_>) -> String {
    let title = escape_attr(block.title);
    let mut out = String::new();
    match block.source {
        Source::File(key) => {
            let src = format!("/media/{}", escape_attr(key));
            let _ = write!(
                out,
                "<video-embed data-kind=\"file\" data-src=\"{src}\" data-title=\"{title}\">\
                 <video src=\"{src}\" controls preload=\"metadata\"></video>"
            );
            if !title.is_empty() {
                let _ = write!(out, "<p>{title}</p>");
            }
        }
        Source::YouTube(id) => {
            let id = escape_attr(id);
            let text = if title.is_empty() {
                "Watch on YouTube".to_string()
            } else {
                title.clone()
            };
            let _ = write!(
                out,
                "<video-embed data-kind=\"youtube\" data-id=\"{id}\" data-title=\"{title}\">\
                 <p><a href=\"https://www.youtube.com/watch?v={id}\">{text}</a></p>"
            );
        }
    }
    out.push_str("</video-embed>");
    out
}

/// True if `value` is a YouTube ID: 11 bytes of `A-Z a-z 0-9 _ -`.
#[must_use]
pub fn id_ok(value: &str) -> bool {
    value.len() == YOUTUBE_ID_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// True if `value` is `/media/<video key>`.
#[must_use]
pub fn src_ok(value: &str) -> bool {
    value
        .strip_prefix("/media/")
        .is_some_and(|k| video_key_ok(k.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.mp4";

    #[test]
    fn parses_files_and_youtube() {
        let block = format!("/media/{KEY}\nBend on the GPU demo\n");
        assert_eq!(
            parse(&block),
            Some(Block {
                source: Source::File(KEY),
                title: "Bend on the GPU demo"
            })
        );
        assert_eq!(
            parse("  https://youtu.be/dQw4w9WgXcQ?t=3  \n"),
            Some(Block {
                source: Source::YouTube("dQw4w9WgXcQ"),
                title: ""
            })
        );
    }

    #[test]
    fn refuses_bad_blocks() {
        for block in [
            String::new(),
            "\n\n".to_string(),
            "javascript:alert(1)".to_string(),
            "https://evil.test/x.mp4".to_string(),
            "/media/x.mp4".to_string(),
            format!("/media/{}", KEY.replace(".mp4", ".png")),
            format!("/media/../{KEY}"),
            format!("/media/{KEY}\"><script>"),
            format!("/media/{KEY}\ntitle\nthird line"),
            format!("/media/{KEY}\n{}", "x".repeat(TITLE_MAX + 1)),
            format!("/media/{KEY}\nbad\u{7}title"),
            "https://youtu.be/dQw4w9WgXc".to_string(),
            "https://youtu.be/dQw4w9WgX\"Q".to_string(),
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
