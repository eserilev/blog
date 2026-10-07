//! Spec 7.3 `video_source`: any first line of a video block. Agrees with theorem
//! T18: a YouTube address gives an 11-byte ID of `A-Z a-z 0-9 _ -`, and the
//! renderer takes the ID only.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logbook_core::{YOUTUBE_ID_LEN, youtube_id};
use logbook_render::video::id;

const PREFIXES: [&[u8]; 6] = [
    b"https://www.youtube.com/watch?v=",
    b"https://youtube.com/watch?v=",
    b"https://m.youtube.com/watch?v=",
    b"https://youtu.be/",
    b"https://www.youtube.com/shorts/",
    b"https://youtube.com/shorts/",
];

fn id_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
}

fuzz_target!(|url: &[u8]| {
    let want = PREFIXES.iter().find(|p| url.starts_with(p)).and_then(|p| {
        let rest = &url[p.len()..];
        let ok = rest.len() >= YOUTUBE_ID_LEN
            && rest[..YOUTUBE_ID_LEN].iter().all(|c| id_byte(*c))
            && matches!(rest.get(YOUTUBE_ID_LEN), None | Some(b'?' | b'&' | b'#'));
        ok.then_some(p.len())
    });
    assert_eq!(youtube_id(url), want);
    if let Ok(line) = std::str::from_utf8(url)
        && let Some(id) = id(line)
    {
        assert!(id.len() == YOUTUBE_ID_LEN && id.bytes().all(id_byte), "{id:?}");
    }
});
