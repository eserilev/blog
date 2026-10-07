//! Video uploads and video responses (spec 4.10, 6.8).
//!
//! - Upload: MP4 or WebM, checked by magic bytes, at most [`VIDEO_MAX`] bytes. The
//!   file streams to a temporary file, never into memory. In an MP4, the server
//!   blanks the `udta` and `meta` boxes of `moov` and each `trak` (they hold the GPS
//!   position that phones write). The key is `<sha256 hex>.<ext>` of the result.
//! - `/media/{key}` for a video key: one byte range per request (HTTP 206), read
//!   from the store as a stream. Seeking in a `<video>` needs this.

use std::{
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
};

use axum::{
    body::Body,
    extract::multipart::Field,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use object_store::{GetOptions, GetRange, ObjectStore, ObjectStoreExt, WriteMultipart};
use sha2::{Digest, Sha256};

use crate::media::{Media, MediaError, hex, store_path};

/// Most bytes in one uploaded video.
pub const VIDEO_MAX: usize = 100 * 1024 * 1024;
/// Most bytes of a `moov` box that the server reads to blank its metadata.
const MOOV_MAX: u64 = 64 * 1024 * 1024;
/// Upload parts in flight to the store at one time.
const PARTS_IN_FLIGHT: usize = 2;

/// A video container that the server accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Mp4,
    WebM,
}

impl Container {
    /// The extension of the key.
    #[must_use]
    pub fn ext(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::WebM => "webm",
        }
    }
}

/// The MP4 brands that the server accepts. Not the Apple `qt  ` brand.
const MP4_BRANDS: &[&[u8; 4]] = &[
    b"isom", b"iso2", b"iso3", b"iso4", b"iso5", b"iso6", b"mp41", b"mp42", b"avc1", b"M4V ",
    b"dash", b"mmp4",
];

/// The container of a file, from its first bytes. `None` for anything else.
///
/// - MP4: an `ftyp` box first, with an accepted major brand.
/// - WebM: the EBML magic, and the `DocType` element `webm` in the first 64 bytes
///   (not `matroska`).
#[must_use]
pub fn sniff(head: &[u8]) -> Option<Container> {
    if head.len() >= 12 && &head[4..8] == b"ftyp" && MP4_BRANDS.iter().any(|b| head[8..12] == b[..])
    {
        return Some(Container::Mp4);
    }
    if head.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        let head = &head[..head.len().min(64)];
        // DocType (0x4282), size 4 as a one- or two-byte number, then "webm".
        let doc_type = |w: &[u8]| {
            w.windows(7).any(|w| w == b"\x42\x82\x84webm")
                || w.windows(8).any(|w| w == b"\x42\x82\x40\x04webm")
        };
        if doc_type(head) {
            return Some(Container::WebM);
        }
    }
    None
}

/// The box types whose payload the server blanks: user data and metadata.
fn private_box(kind: &[u8]) -> bool {
    kind == b"udta" || kind == b"meta"
}

/// One ISO box in `buf`: header length, total length, and the type.
fn box_at(buf: &[u8], at: usize) -> Option<(usize, usize, [u8; 4])> {
    let head = buf.get(at..at + 8)?;
    let size = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
    let kind = [head[4], head[5], head[6], head[7]];
    let rest = buf.len() - at;
    let (header, len) = match size {
        0 => (8, rest),
        1 => {
            let big = buf.get(at + 8..at + 16)?;
            let len = usize::try_from(u64::from_be_bytes(big.try_into().ok()?)).ok()?;
            (16, len)
        }
        n => (8, usize::try_from(n).ok()?),
    };
    (len >= header && len <= rest).then_some((header, len, kind))
}

/// Blanks the private boxes among the children of `buf[range]`: the type becomes
/// `free`, and the payload becomes zeros. The sizes do not change, so no offset in
/// the file changes. Goes into each `trak`. `None` if a box does not fit.
fn blank_children(buf: &mut [u8], range: Range<usize>) -> Option<()> {
    let mut at = range.start;
    while at < range.end {
        if range.end - at < 8 {
            // A few bytes of padding after the last box.
            return buf[at..range.end].iter().all(|b| *b == 0).then_some(());
        }
        let (header, len, kind) = box_at(&buf[..range.end], at)?;
        if private_box(&kind) {
            buf[at + 4..at + 8].copy_from_slice(b"free");
            buf[at + header..at + len].fill(0);
        } else if &kind == b"trak" {
            blank_children(buf, at + header..at + len)?;
        }
        at += len;
    }
    Some(())
}

/// Blanks the private boxes of an MP4 file in place (see [`blank_children`]):
/// the top-level `udta` and `meta`, and those in `moov` and each `trak`.
///
/// # Errors
///
/// A safe message if the boxes do not parse.
pub fn strip_mp4(file: &mut std::fs::File) -> Result<(), &'static str> {
    const BAD: &str = "the video does not parse";
    let size = file.metadata().map_err(|_| BAD)?.len();
    let mut at = 0u64;
    while at < size {
        let mut head = [0u8; 16];
        let n = usize::try_from((size - at).min(16)).map_err(|_| BAD)?;
        file.seek(SeekFrom::Start(at)).map_err(|_| BAD)?;
        file.read_exact(&mut head[..n]).map_err(|_| BAD)?;
        if n < 8 {
            return Err(BAD);
        }
        let small = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
        let kind = [head[4], head[5], head[6], head[7]];
        let (header, len) = match small {
            0 => (8, size - at),
            1 if n == 16 => (
                16,
                u64::from_be_bytes(head[8..16].try_into().map_err(|_| BAD)?),
            ),
            1 => return Err(BAD),
            s => (8, u64::from(s)),
        };
        if len < header || len > size - at {
            return Err(BAD);
        }
        if private_box(&kind) {
            file.seek(SeekFrom::Start(at + 4)).map_err(|_| BAD)?;
            file.write_all(b"free").map_err(|_| BAD)?;
            zero(file, at + header, len - header).map_err(|_| BAD)?;
        } else if &kind == b"moov" {
            if len > MOOV_MAX {
                return Err(BAD);
            }
            let mut moov = vec![0u8; usize::try_from(len).map_err(|_| BAD)?];
            file.seek(SeekFrom::Start(at)).map_err(|_| BAD)?;
            file.read_exact(&mut moov).map_err(|_| BAD)?;
            let header = usize::try_from(header).map_err(|_| BAD)?;
            let end = moov.len();
            blank_children(&mut moov, header..end).ok_or(BAD)?;
            file.seek(SeekFrom::Start(at)).map_err(|_| BAD)?;
            file.write_all(&moov).map_err(|_| BAD)?;
        }
        at += len;
    }
    file.flush().map_err(|_| BAD)
}

/// Writes `len` zero bytes at `at`.
fn zero(file: &mut std::fs::File, at: u64, mut len: u64) -> std::io::Result<()> {
    let block = [0u8; 8192];
    file.seek(SeekFrom::Start(at))?;
    while len > 0 {
        let n = usize::try_from(len.min(block.len() as u64)).unwrap_or(block.len());
        file.write_all(&block[..n])?;
        len -= n as u64;
    }
    Ok(())
}

/// The SHA-256 of a file, as hex.
fn hash_file(file: &mut std::fs::File) -> std::io::Result<String> {
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

/// Receives a video upload: `head` holds the first bytes of `field`, already read.
/// Streams the rest to a temporary file, blanks the MP4 metadata, and puts the file
/// in the store. Returns the key.
///
/// # Errors
///
/// 413 above [`VIDEO_MAX`], 400 for a broken file, 500 for a store error.
pub async fn receive(
    media: &Media,
    container: Container,
    head: Vec<u8>,
    mut field: Field<'_>,
) -> Result<String, MediaError> {
    let internal = |e: &dyn std::fmt::Display| MediaError::Internal(e.to_string());
    let std_file = tempfile::tempfile().map_err(|e| internal(&e))?;
    let mut file = tokio::fs::File::from_std(std_file.try_clone().map_err(|e| internal(&e))?);
    let mut total = 0usize;
    let mut chunk = bytes::Bytes::from(head);
    loop {
        total += chunk.len();
        if total > VIDEO_MAX {
            return Err(MediaError::TooLarge("the video is larger than 100 MB"));
        }
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| internal(&e))?;
        match field
            .chunk()
            .await
            .map_err(|_| MediaError::BadRequest("the upload is too large or broken"))?
        {
            Some(c) => chunk = c,
            None => break,
        }
    }
    tokio::io::AsyncWriteExt::flush(&mut file)
        .await
        .map_err(|e| internal(&e))?;
    drop(file);

    let (std_file, key) = tokio::task::spawn_blocking(move || {
        let mut f = std_file;
        if container == Container::Mp4 {
            strip_mp4(&mut f).map_err(MediaError::BadRequest)?;
        }
        let hash = hash_file(&mut f).map_err(|e| MediaError::Internal(e.to_string()))?;
        Ok::<_, MediaError>((f, format!("{hash}.{}", container.ext())))
    })
    .await
    .map_err(|e| internal(&e))??;

    let path = store_path(&key);
    // The same video again: the key is a content hash, so the file is already there.
    if media.store.head(&path).await.is_ok() {
        return Ok(key);
    }
    put_file(media.store.as_ref(), &path, std_file).await?;
    Ok(key)
}

/// Copies a file to the store in parts, so at most a few parts are in memory.
async fn put_file(
    store: &dyn ObjectStore,
    path: &object_store::path::Path,
    mut file: std::fs::File,
) -> Result<(), MediaError> {
    let internal = |e: &dyn std::fmt::Display| MediaError::Internal(e.to_string());
    file.seek(SeekFrom::Start(0)).map_err(|e| internal(&e))?;
    let mut file = tokio::fs::File::from_std(file);
    let upload = store.put_multipart(path).await.map_err(|e| internal(&e))?;
    let mut writer = WriteMultipart::new(upload);
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = match tokio::io::AsyncReadExt::read(&mut file, &mut buf).await {
            Ok(n) => n,
            Err(e) => {
                let _ = writer.abort().await;
                return Err(internal(&e));
            }
        };
        if n == 0 {
            break;
        }
        if let Err(e) = writer.wait_for_capacity(PARTS_IN_FLIGHT).await {
            let _ = writer.abort().await;
            return Err(internal(&e));
        }
        writer.write(&buf[..n]);
    }
    writer.finish().await.map_err(|e| internal(&e))?;
    Ok(())
}

/// The bytes that a response sends.
#[derive(Debug, PartialEq, Eq)]
pub enum ByteRange {
    /// The whole file (no `Range` header, or one that the server ignores).
    Full,
    /// Bytes `first..=last`.
    Part(u64, u64),
    /// The range starts after the end of the file.
    Unsatisfiable,
}

/// The range of a `Range` header for a file of `size` bytes (RFC 9110, 14.2).
/// One `bytes` range only. A header with several ranges or a bad value is
/// ignored, and the response sends the whole file.
#[must_use]
pub fn byte_range(header: Option<&str>, size: u64) -> ByteRange {
    let Some(spec) = header.and_then(|h| h.trim().strip_prefix("bytes=")) else {
        return ByteRange::Full;
    };
    if spec.contains(',') {
        return ByteRange::Full;
    }
    let Some((a, b)) = spec.split_once('-') else {
        return ByteRange::Full;
    };
    let (a, b) = (a.trim(), b.trim());
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit());
    if a.is_empty() {
        // The last `n` bytes.
        let Some(n) = digits(b).then(|| b.parse::<u64>().ok()).flatten() else {
            return ByteRange::Full;
        };
        if n == 0 || size == 0 {
            return ByteRange::Unsatisfiable;
        }
        return ByteRange::Part(size.saturating_sub(n), size - 1);
    }
    let Some(first) = digits(a).then(|| a.parse::<u64>().ok()).flatten() else {
        return ByteRange::Full;
    };
    if first >= size {
        return ByteRange::Unsatisfiable;
    }
    let last = if b.is_empty() {
        size - 1
    } else {
        match digits(b).then(|| b.parse::<u64>().ok()).flatten() {
            Some(l) if l >= first => l.min(size - 1),
            _ => return ByteRange::Full,
        }
    };
    ByteRange::Part(first, last)
}

/// `GET /media/{key}` for a video key: the whole file or one byte range, as a stream.
pub async fn serve(
    media: &Media,
    key: &str,
    content_type: &'static str,
    req: &HeaderMap,
) -> Response {
    let path = store_path(key);
    let size = match media.store.head(&path).await {
        Ok(meta) => meta.size,
        Err(object_store::Error::NotFound { .. }) => {
            return (StatusCode::NOT_FOUND, "not found").into_response();
        }
        Err(e) => {
            tracing::error!("media head failed: {e}");
            return (StatusCode::BAD_GATEWAY, "video unavailable").into_response();
        }
    };
    let range = byte_range(req.get(header::RANGE).and_then(|v| v.to_str().ok()), size);
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    let (status, bytes) = match range {
        ByteRange::Unsatisfiable => {
            if let Ok(v) = HeaderValue::from_str(&format!("bytes */{size}")) {
                headers.insert(header::CONTENT_RANGE, v);
            }
            return (StatusCode::RANGE_NOT_SATISFIABLE, headers).into_response();
        }
        ByteRange::Full => (StatusCode::OK, 0..size),
        ByteRange::Part(first, last) => {
            if let Ok(v) = HeaderValue::from_str(&format!("bytes {first}-{last}/{size}")) {
                headers.insert(header::CONTENT_RANGE, v);
            }
            (StatusCode::PARTIAL_CONTENT, first..last + 1)
        }
    };
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from(bytes.end - bytes.start),
    );
    if bytes.is_empty() {
        return (status, headers, Body::empty()).into_response();
    }
    let options = GetOptions {
        range: Some(GetRange::Bounded(bytes)),
        ..GetOptions::default()
    };
    match media.store.get_opts(&path, options).await {
        Ok(result) => (status, headers, Body::from_stream(result.into_stream())).into_response(),
        Err(object_store::Error::NotFound { .. }) => {
            (StatusCode::NOT_FOUND, "not found").into_response()
        }
        Err(e) => {
            tracing::error!("media read failed: {e}");
            (StatusCode::BAD_GATEWAY, "video unavailable").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One ISO box.
    pub(crate) fn mp4_box(kind: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut b = u32::try_from(payload.len() + 8)
            .unwrap()
            .to_be_bytes()
            .to_vec();
        b.extend_from_slice(kind);
        b.extend_from_slice(payload);
        b
    }

    /// A small MP4-shaped file with a GPS position in `moov/udta` and `trak/meta`.
    pub(crate) fn mp4_with_gps() -> Vec<u8> {
        let gps = mp4_box(b"\xa9xyz", b"+33.8467-118.3980/");
        let trak = mp4_box(
            b"trak",
            &[mp4_box(b"tkhd", &[0; 12]), mp4_box(b"meta", b"GPSPOS")].concat(),
        );
        let moov = mp4_box(
            b"moov",
            &[mp4_box(b"mvhd", &[0; 12]), mp4_box(b"udta", &gps), trak].concat(),
        );
        [
            mp4_box(b"ftyp", b"isom\0\0\x02\0isomiso2mp41"),
            moov,
            mp4_box(b"mdat", &[7; 32]),
        ]
        .concat()
    }

    #[test]
    fn sniffs_mp4_and_webm_only() {
        assert_eq!(sniff(&mp4_with_gps()), Some(Container::Mp4));
        let webm = b"\x1a\x45\xdf\xa3\x9f\x42\x86\x81\x01\x42\xf7\x81\x01\x42\xf2\x81\x04\x42\xf3\x81\x08\x42\x82\x84webm\x42\x87\x81\x04";
        assert_eq!(sniff(webm), Some(Container::WebM));
        let mkv = b"\x1a\x45\xdf\xa3\x9f\x42\x86\x81\x01\x42\x82\x88matroska";
        for (bytes, why) in [
            (&mkv[..], "matroska"),
            (b"\0\0\0\x14ftypqt  \0\0\0\0qt  ", "quicktime"),
            (b"\x89PNG\r\n\x1a\n\0\0\0\0", "png"),
            (b"<svg onload=x>", "svg"),
            (b"\0\0\0\x14ftyp", "short"),
            (b"", "empty"),
        ] {
            assert_eq!(sniff(bytes), None, "{why}");
        }
    }

    #[test]
    fn blanks_gps_and_keeps_offsets() {
        let original = mp4_with_gps();
        let mut f = tempfile::tempfile().unwrap();
        f.write_all(&original).unwrap();
        strip_mp4(&mut f).unwrap();
        let mut out = Vec::new();
        f.seek(SeekFrom::Start(0)).unwrap();
        f.read_to_end(&mut out).unwrap();
        assert_eq!(out.len(), original.len(), "same size, same offsets");
        let has = |w: &[u8]| out.windows(w.len()).any(|x| x == w);
        assert!(!has(b"+33.8467"), "GPS survived");
        assert!(!has(b"GPSPOS"), "trak metadata survived");
        assert!(!has(b"udta") && !has(b"meta"));
        assert!(has(b"mvhd") && has(b"tkhd") && has(&[7; 32]), "media kept");
        assert_eq!(&out[..8], &original[..8]);
    }

    #[test]
    fn refuses_broken_boxes() {
        for bytes in [
            [
                mp4_box(b"ftyp", b"isom\0\0\0\0"),
                vec![0, 0, 0xff, 0xff, b'm', b'o'],
            ]
            .concat(),
            [
                mp4_box(b"ftyp", b"isom\0\0\0\0"),
                vec![0, 0, 0, 4, b'f', b'r', b'e', b'e'],
            ]
            .concat(),
            [mp4_box(b"ftyp", b"isom\0\0\0\0"), vec![0, 0, 0, 1]].concat(),
        ] {
            let mut f = tempfile::tempfile().unwrap();
            f.write_all(&bytes).unwrap();
            assert!(strip_mp4(&mut f).is_err(), "{bytes:?}");
        }
    }

    #[test]
    fn parses_ranges() {
        use ByteRange::{Full, Part, Unsatisfiable};
        for (h, want) in [
            (None, Full),
            (Some("bytes=0-"), Part(0, 999)),
            (Some("bytes=0-0"), Part(0, 0)),
            (Some("bytes=100-199"), Part(100, 199)),
            (Some("bytes=900-5000"), Part(900, 999)),
            (Some("bytes=-100"), Part(900, 999)),
            (Some("bytes=-5000"), Part(0, 999)),
            (Some("bytes=1000-"), Unsatisfiable),
            (Some("bytes=-0"), Unsatisfiable),
            (Some("bytes=5-3"), Full),
            (Some("bytes=0-1,5-6"), Full),
            (Some("bytes=a-b"), Full),
            (Some("bytes=+1-2"), Full),
            (Some("items=0-1"), Full),
            (Some("bytes=-"), Full),
        ] {
            assert_eq!(byte_range(h, 1000), want, "{h:?}");
        }
        assert_eq!(byte_range(Some("bytes=0-"), 0), Unsatisfiable);
    }
}
