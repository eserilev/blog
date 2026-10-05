//! Versioned static files (spec 6.3, 6.9).
//!
//! At startup the server hashes each file in the static folder. It adds `?v=<hash>`
//! to each `/static/` URL in `index.html` and to each `url(...)` in the CSS files.
//! A request with the current hash in `v` gets a one-year cache. All other requests
//! get `no-cache`, so old pages and direct links still work.

use std::{collections::HashMap, path::Path, sync::Arc};

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use sha2::{Digest, Sha256};

/// `Cache-Control` for a request with the current hash.
pub const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// Hex characters in a version hash.
pub const HASH_LEN: usize = 10;

/// The hashes of the static files, and the CSS files with versioned URLs.
#[derive(Debug, Default)]
pub struct Assets {
    /// Path in the static folder (for example `css/logbook.css`) → version hash.
    hashes: HashMap<String, String>,
    /// CSS files with versioned `url(...)` references. The server sends these from memory.
    css: HashMap<String, Bytes>,
}

/// The version hash of `bytes`: the first [`HASH_LEN`] hex characters of SHA-256.
#[must_use]
pub fn hash(bytes: &[u8]) -> String {
    let mut h = crate::media::hex(&Sha256::digest(bytes));
    h.truncate(HASH_LEN);
    h
}

impl Assets {
    /// Reads and hashes each file in `dir` and its subfolders.
    ///
    /// The hash of a CSS file is the hash of its versioned text. A change to a font
    /// thus changes the URL of the CSS file too.
    ///
    /// # Errors
    ///
    /// Fails if a folder or a file cannot be read.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut files = Vec::new();
        walk(dir, "", &mut files)?;
        let mut hashes: HashMap<String, String> =
            files.iter().map(|(p, b)| (p.clone(), hash(b))).collect();
        let mut css = HashMap::new();
        for (path, bytes) in &files {
            if !Path::new(path)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("css"))
            {
                continue;
            }
            let Ok(text) = std::str::from_utf8(bytes) else {
                continue;
            };
            // A url() to another CSS file gets the raw hash of that file. That hash
            // does not match, so the file gets no-cache. It is never stale.
            let versioned = version_css(&hashes, path, text);
            css.insert(path.clone(), Bytes::from(versioned));
        }
        for (path, text) in &css {
            hashes.insert(path.clone(), hash(text));
        }
        Ok(Self { hashes, css })
    }

    /// The version hash of `path` (relative to the static folder), if the file exists.
    #[must_use]
    pub fn version(&self, path: &str) -> Option<&str> {
        self.hashes.get(path).map(String::as_str)
    }

    /// Adds `?v=<hash>` to each double-quoted `"/static/..."` URL in `html` that
    /// names a known file and has no query.
    #[must_use]
    pub fn version_html(&self, html: &str) -> String {
        const PREFIX: &str = "\"/static/";
        let mut out = String::with_capacity(html.len() + 256);
        let mut rest = html;
        while let Some(i) = rest.find(PREFIX) {
            let (head, tail) = rest.split_at(i + PREFIX.len());
            out.push_str(head);
            let end = tail.find('"').unwrap_or(tail.len());
            let path = &tail[..end];
            out.push_str(path);
            if let Some(h) = self.version(path) {
                out.push_str("?v=");
                out.push_str(h);
            }
            rest = &tail[end..];
        }
        out.push_str(rest);
        out
    }
}

/// Collects `(path, bytes)` for each file under `dir`. Paths use `/`.
fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let rel = format!("{prefix}{name}");
        if path.is_dir() {
            walk(&path, &format!("{rel}/"), out)?;
        } else if path.is_file() {
            let bytes =
                std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            out.push((rel, bytes));
        }
    }
    Ok(())
}

/// Adds `?v=<hash>` to each `url(...)` in `css` that names a known file.
/// `css_path` is the path of the CSS file, for relative URLs.
fn version_css(hashes: &HashMap<String, String>, css_path: &str, css: &str) -> String {
    let base = css_path.rsplit_once('/').map_or("", |(d, _)| d);
    let mut out = String::with_capacity(css.len() + 256);
    let mut rest = css;
    while let Some(i) = rest.find("url(") {
        let (head, tail) = rest.split_at(i + "url(".len());
        out.push_str(head);
        let end = tail.find(')').unwrap_or(tail.len());
        let inner = &tail[..end];
        let url = inner.trim().trim_matches(['"', '\'']);
        match resolve(base, url).and_then(|p| hashes.get(&p)) {
            Some(h) if !url.is_empty() => {
                out.push_str(&inner.replacen(url, &format!("{url}?v={h}"), 1));
            }
            _ => out.push_str(inner),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// The path in the static folder that `url` names, relative to the folder `base`.
/// `None` for other origins, `data:` URLs, URLs with a query, and paths outside the folder.
fn resolve(base: &str, url: &str) -> Option<String> {
    if url.contains([':', '?', '#']) {
        return None;
    }
    let joined = if let Some(rest) = url.strip_prefix("/static/") {
        rest.to_owned()
    } else if url.starts_with('/') {
        return None;
    } else {
        format!("{base}/{url}")
    };
    let mut parts: Vec<&str> = Vec::new();
    for part in joined.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

/// Middleware for `/static`. It sends the versioned CSS from memory, and it sets the
/// one-year cache when `v` is the current hash of the file.
pub async fn serve(State(assets): State<Arc<Assets>>, req: Request, next: Next) -> Response {
    let path = req.uri().path().trim_start_matches('/').to_owned();
    let fresh = match (query_v(req.uri().query()), assets.version(&path)) {
        (Some(v), Some(h)) => v == h,
        _ => false,
    };
    let css = assets
        .css
        .get(&path)
        .filter(|_| matches!(*req.method(), Method::GET | Method::HEAD));
    let mut res = match css {
        Some(text) => css_response(&req, text, assets.version(&path).unwrap_or_default()),
        None => next.run(req).await,
    };
    if fresh && (res.status() == StatusCode::OK || res.status() == StatusCode::NOT_MODIFIED) {
        res.headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(IMMUTABLE));
    }
    res
}

/// The value of the `v` query parameter.
fn query_v(query: Option<&str>) -> Option<&str> {
    query?.split('&').find_map(|kv| kv.strip_prefix("v="))
}

/// A versioned CSS file, with an `ETag` of its hash.
fn css_response(req: &Request, text: &Bytes, hash: &str) -> Response {
    let etag = format!("\"{hash}\"");
    let matches = req
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == etag || t.trim() == "*"));
    let status = if matches {
        StatusCode::NOT_MODIFIED
    } else {
        StatusCode::OK
    };
    let body = if matches {
        Body::empty()
    } else {
        Body::from(text.clone())
    };
    (
        status,
        [
            (header::CONTENT_TYPE, "text/css".to_owned()),
            (header::ETAG, etag),
        ],
        body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_handles_relative_and_absolute_paths() {
        assert_eq!(
            resolve("css", "../fonts/a.woff2").as_deref(),
            Some("fonts/a.woff2")
        );
        assert_eq!(resolve("css", "./b.css").as_deref(), Some("css/b.css"));
        assert_eq!(
            resolve("css", "/static/fonts/a.woff2").as_deref(),
            Some("fonts/a.woff2")
        );
        assert_eq!(resolve("", "../../x"), None);
        assert_eq!(resolve("css", "/media/x.png"), None);
        assert_eq!(resolve("css", "data:image/png;base64,AA"), None);
        assert_eq!(resolve("css", "a.woff2?x=1"), None);
    }

    #[test]
    fn css_urls_get_the_hash_and_keep_their_quotes() {
        let hashes = HashMap::from([("fonts/a.woff2".to_owned(), "abc".to_owned())]);
        let css = r#"a{src:url("../fonts/a.woff2")} b{src:url( '../fonts/a.woff2' )} c{src:url(../fonts/a.woff2)} d{src:url(../fonts/missing.woff2)}"#;
        assert_eq!(
            version_css(&hashes, "css/x.css", css),
            r#"a{src:url("../fonts/a.woff2?v=abc")} b{src:url( '../fonts/a.woff2?v=abc' )} c{src:url(../fonts/a.woff2?v=abc)} d{src:url(../fonts/missing.woff2)}"#
        );
    }

    #[test]
    fn html_urls_get_the_hash() {
        let assets = Assets {
            hashes: HashMap::from([("js/a.js".to_owned(), "abc".to_owned())]),
            css: HashMap::new(),
        };
        assert_eq!(
            assets.version_html(r#"<script src="/static/js/a.js"></script><a href="/static/x">"#),
            r#"<script src="/static/js/a.js?v=abc"></script><a href="/static/x">"#
        );
    }

    #[test]
    fn query_v_finds_the_parameter() {
        assert_eq!(query_v(Some("v=abc")), Some("abc"));
        assert_eq!(query_v(Some("x=1&v=abc")), Some("abc"));
        assert_eq!(query_v(Some("vv=abc")), None);
        assert_eq!(query_v(None), None);
    }
}
