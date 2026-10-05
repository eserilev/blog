//! Image uploads and `/media/{key}` (spec 6.8).
//!
//! - Upload: max 10 MB. PNG, JPEG, WebP, GIF only, checked by magic bytes. No SVG.
//!   The server decodes and re-encodes each image, which drops EXIF (GPS) and all
//!   other metadata. The key is `<sha256 hex>.<ext>` of the re-encoded bytes.
//! - Storage: the bucket under `uploads/` (production), or a local folder (development).
//! - `/media/{key}`: the key must pass `logbook_core::media_key_ok` (theorem T10).
//!   Images are cached on local disk, at most 1 GB, oldest first out.

use std::{
    io::Cursor,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
};

use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use image::{ImageFormat, ImageReader, Limits};
use object_store::{ObjectStore, ObjectStoreExt, PutPayload, path::Path as StorePath};
use sha2::{Digest, Sha256};

use crate::{AppState, auth::Owner};

/// Most bytes in one upload.
pub const UPLOAD_MAX: usize = 10 * 1024 * 1024;
/// Most bytes in the local image cache.
pub const CACHE_MAX: u64 = 1024 * 1024 * 1024;

/// Where images live.
#[derive(Clone)]
pub struct Media {
    pub store: Arc<dyn ObjectStore>,
    /// Local cache for images from a remote store. `None` when the store is local.
    pub cache: Option<PathBuf>,
}

/// An upload error. The message is safe to show.
#[derive(Debug)]
pub enum MediaError {
    BadRequest(&'static str),
    Internal(String),
}

impl IntoResponse for MediaError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            Self::Internal(e) => {
                tracing::error!("media error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error")
            }
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

/// The extension for a stored format.
fn ext(f: ImageFormat) -> Option<&'static str> {
    match f {
        ImageFormat::Png => Some("png"),
        ImageFormat::Jpeg => Some("jpg"),
        ImageFormat::WebP => Some("webp"),
        ImageFormat::Gif => Some("gif"),
        _ => None,
    }
}

/// The content type for a key's extension. Only called after `media_key_ok`.
fn content_type(key: &str) -> &'static str {
    match key.rsplit('.').next() {
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    }
}

/// Checks, decodes, and re-encodes an image. Returns the new bytes and the key.
///
/// # Errors
///
/// A safe message for a file that is not an allowed image.
pub fn reencode(bytes: &[u8]) -> Result<(Vec<u8>, String), &'static str> {
    if bytes.len() > UPLOAD_MAX {
        return Err("the image is larger than 10 MB");
    }
    let format = image::guess_format(bytes).map_err(|_| "only PNG, JPEG, WebP, and GIF images")?;
    let ext = ext(format).ok_or("only PNG, JPEG, WebP, and GIF images")?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(8_000);
    limits.max_image_height = Some(8_000);
    limits.max_alloc = Some(256 * 1024 * 1024);
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(limits);
    let img = reader.decode().map_err(|_| "the image does not decode")?;
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, format)
        .map_err(|_| "the image does not encode")?;
    let out = out.into_inner();
    let key = format!("{}.{ext}", hex(&Sha256::digest(&out)));
    Ok((out, key))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn store_path(key: &str) -> StorePath {
    StorePath::from(format!("uploads/{key}"))
}

/// `POST /api/owner/uploads`: one multipart field named `file`.
///
/// # Errors
///
/// 401 without a session, 400 for a bad file, 413 above the body limit.
pub async fn upload(
    _: Owner,
    State(s): State<AppState>,
    mut form: Multipart,
) -> Result<Json<serde_json::Value>, MediaError> {
    let mut bytes = None;
    while let Some(field) = form
        .next_field()
        .await
        .map_err(|_| MediaError::BadRequest("bad upload form"))?
    {
        if field.name() == Some("file") {
            bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|_| MediaError::BadRequest("the upload is too large or broken"))?,
            );
        }
    }
    let bytes = bytes.ok_or(MediaError::BadRequest("no file in the upload"))?;
    let (out, key) = tokio::task::spawn_blocking(move || reencode(&bytes))
        .await
        .map_err(|e| MediaError::Internal(e.to_string()))?
        .map_err(MediaError::BadRequest)?;
    s.media
        .store
        .put(&store_path(&key), PutPayload::from(out))
        .await
        .map_err(|e| MediaError::Internal(e.to_string()))?;
    let url = format!("/media/{key}");
    Ok(Json(
        serde_json::json!({ "key": key, "url": url, "markdown": format!("![]({url})") }),
    ))
}

/// Reads an image: the local cache first, then the store.
///
/// # Errors
///
/// `Ok(None)` if the image does not exist.
pub async fn read(media: &Media, key: &str) -> Result<Option<Vec<u8>>, String> {
    if let Some(dir) = &media.cache
        && let Ok(b) = tokio::fs::read(dir.join(key)).await
    {
        return Ok(Some(b));
    }
    let bytes = match media.store.get(&store_path(key)).await {
        Ok(r) => r.bytes().await.map_err(|e| e.to_string())?.to_vec(),
        Err(object_store::Error::NotFound { .. }) => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    if let Some(dir) = &media.cache {
        let dir = dir.clone();
        let (key, b) = (key.to_string(), bytes.clone());
        tokio::task::spawn_blocking(move || {
            if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(dir.join(&key), b).is_ok() {
                trim_cache(&dir, CACHE_MAX);
            }
        });
    }
    Ok(Some(bytes))
}

/// Deletes the oldest cached files until the cache is under 90 % of `max`.
pub fn trim_cache(dir: &FsPath, max: u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            m.is_file().then(|| {
                (
                    m.modified().unwrap_or(std::time::UNIX_EPOCH),
                    m.len(),
                    e.path(),
                )
            })
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    if total <= max {
        return;
    }
    files.sort_by_key(|f| f.0);
    for (_, len, path) in files {
        if total <= max / 10 * 9 {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total -= len;
        }
    }
}

/// `GET /media/{key}`.
pub async fn serve(State(s): State<AppState>, Path(key): Path<String>) -> Response {
    if !logbook_core::media_key_ok(key.as_bytes()) {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    match read(&s.media, &key).await {
        Ok(Some(bytes)) => (
            [
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(content_type(&key)),
                ),
                (
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ),
            ],
            bytes,
        )
            .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => {
            tracing::error!("media read failed: {e}");
            (StatusCode::BAD_GATEWAY, "image unavailable").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png() -> Vec<u8> {
        let img = image::RgbImage::from_pixel(4, 3, image::Rgb([200, 30, 30]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn reencodes_and_names_by_hash() {
        let (out, key) = reencode(&png()).unwrap();
        assert!(logbook_core::media_key_ok(key.as_bytes()), "{key}");
        assert_eq!(
            std::path::Path::new(&key)
                .extension()
                .and_then(|e| e.to_str()),
            Some("png")
        );
        assert_eq!(&out[..8], b"\x89PNG\r\n\x1a\n");
        // The same image gives the same key.
        assert_eq!(reencode(&png()).unwrap().1, key);
    }

    #[test]
    fn refuses_other_files() {
        for (bytes, why) in [
            (
                b"<svg xmlns='http://www.w3.org/2000/svg' onload='x'/>".to_vec(),
                "svg",
            ),
            (b"hello".to_vec(), "text"),
            (b"\x89PNG\r\n\x1a\nbroken".to_vec(), "broken png"),
            (b"%PDF-1.7".to_vec(), "pdf"),
        ] {
            assert!(reencode(&bytes).is_err(), "{why}");
        }
    }

    #[test]
    fn trims_the_oldest_files() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..10u8 {
            std::fs::write(dir.path().join(format!("{i}")), vec![0u8; 100]).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        trim_cache(dir.path(), 500);
        let left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(left.len() <= 4, "{left:?}");
        assert!(left.contains(&"9".to_string()), "the newest file stays");
    }
}
