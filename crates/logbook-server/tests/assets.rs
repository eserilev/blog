//! Versioned static files and their cache headers (spec 6.3, 6.9).

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use common::*;
use logbook_server::assets::{Assets, HASH_LEN, IMMUTABLE};
use tower::ServiceExt;

/// Each `"/static/..."` URL in `text`, with its query.
fn static_urls(text: &str) -> Vec<String> {
    text.match_indices("\"/static/")
        .map(|(i, _)| {
            let rest = &text[i + 1..];
            rest[..rest.find('"').unwrap()].to_owned()
        })
        .collect()
}

fn cache(r: &Reply) -> &str {
    r.headers[header::CACHE_CONTROL].to_str().unwrap()
}

/// The versioned URL of `path` in the served home page.
async fn versioned(f: &Fixture, path: &str) -> String {
    let html = f.get("/").await.body;
    static_urls(&html)
        .into_iter()
        .find(|u| u.starts_with(&format!("{path}?v=")))
        .unwrap_or_else(|| panic!("no versioned {path} in index.html"))
}

#[tokio::test]
async fn every_static_url_in_the_page_has_a_version() {
    let f = fixture().await;
    for page in ["/", "/about", "/posts/missing"] {
        let html = f.get(page).await.body;
        let urls = static_urls(&html);
        assert!(urls.len() >= 4, "{page}: {urls:?}");
        for url in urls {
            let (path, v) = url.split_once("?v=").unwrap_or((&url, ""));
            let file = static_dir().join(path.trim_start_matches("/static/"));
            if file.is_file() {
                assert_eq!(v.len(), HASH_LEN, "{page}: {url}");
            } else {
                // The WASM file exists only after editor-wasm/build.sh.
                assert!(
                    path.ends_with("/logbook_render.wasm"),
                    "{page}: {url} names no file"
                );
            }
        }
    }
}

#[tokio::test]
async fn a_versioned_request_gets_the_long_cache() {
    let f = fixture().await;
    for path in [
        "/static/css/logbook.css",
        "/static/css/code.css",
        "/static/js/components.js",
        "/static/js/app.js",
    ] {
        let url = versioned(&f, path).await;
        let r = f.get(&url).await;
        assert_eq!(r.status, StatusCode::OK, "{url}");
        assert_eq!(cache(&r), IMMUTABLE, "{url}");
    }
}

#[tokio::test]
async fn an_unversioned_or_stale_request_gets_no_cache() {
    let f = fixture().await;
    for url in [
        "/static/css/logbook.css",
        "/static/css/logbook.css?v=0000000000",
        "/static/js/app.js",
        "/static/js/app.js?v=0000000000",
        "/static/fonts/tinos-400.woff2?v=",
    ] {
        let r = f.get(url).await;
        assert_eq!(r.status, StatusCode::OK, "{url}");
        assert_eq!(cache(&r), "no-cache", "{url}");
    }
    let r = f.get("/static/missing.css?v=0000000000").await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(cache(&r), "no-cache");
}

#[tokio::test]
async fn fonts_in_the_served_css_have_a_version() {
    let f = fixture().await;
    let css = f.get("/static/css/logbook.css").await.body;
    let fonts: Vec<&str> = css
        .match_indices("url(\"../fonts/")
        .map(|(i, _)| {
            let rest = &css[i + 5..];
            &rest[..rest.find('"').unwrap()]
        })
        .collect();
    assert!(!fonts.is_empty());
    for font in fonts {
        let (path, v) = font.split_once("?v=").expect(font);
        assert_eq!(v.len(), HASH_LEN, "{font}");
        let url = format!("/static/{}", path.trim_start_matches("../"));
        let r = f.get(&format!("{url}?v={v}")).await;
        assert_eq!(r.status, StatusCode::OK, "{url}");
        assert_eq!(cache(&r), IMMUTABLE, "{url}");
    }
}

#[tokio::test]
async fn unversioned_files_keep_their_validators() {
    let f = fixture().await;
    let r = f.get("/static/fonts/tinos-400.woff2").await;
    assert!(r.headers.contains_key(header::ETAG));
    assert!(r.headers.contains_key(header::LAST_MODIFIED));

    // The CSS comes from memory, with its hash as the ETag.
    let r = f.get("/static/css/logbook.css").await;
    let etag = r.headers[header::ETAG].clone();
    let res = f
        .app
        .clone()
        .oneshot(
            Request::get("/static/css/logbook.css")
                .header(header::IF_NONE_MATCH, etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_MODIFIED);
}

#[test]
fn the_hash_follows_the_content() {
    let dir = tempfile::tempdir().unwrap();
    let write = |path: &str, text: &str| {
        let file = dir.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    };
    write(
        "css/a.css",
        r#"@font-face { src: url("../fonts/f.woff2"); }"#,
    );
    write("fonts/f.woff2", "font one");
    write("js/a.js", "one");
    let before = Assets::load(dir.path()).unwrap();

    write("js/a.js", "two");
    let after_js = Assets::load(dir.path()).unwrap();
    assert_ne!(before.version("js/a.js"), after_js.version("js/a.js"));
    assert_eq!(before.version("css/a.css"), after_js.version("css/a.css"));

    // A new font changes the font URL in the CSS, so the CSS hash changes too.
    write("fonts/f.woff2", "font two");
    let after_font = Assets::load(dir.path()).unwrap();
    assert_ne!(
        before.version("fonts/f.woff2"),
        after_font.version("fonts/f.woff2")
    );
    assert_ne!(before.version("css/a.css"), after_font.version("css/a.css"));
    assert_eq!(after_font.version("missing.js"), None);
}
