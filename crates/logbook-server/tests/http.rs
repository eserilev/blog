//! HTTP tests against the real router, in process.

use std::path::PathBuf;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use logbook_server::{
    Config, app,
    headers::{CSP, SECURITY_HEADERS},
    routes::{Kind, ROUTES},
};
use tower::ServiceExt;

fn static_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../static")
}

fn test_app() -> Router {
    let config = Config {
        addr: "127.0.0.1:0".parse().unwrap(),
        static_dir: static_dir(),
    };
    app(&config).expect("static/index.html exists")
}

struct Reply {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: String,
}

async fn get(path: &str) -> Reply {
    let res = test_app()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        headers,
        body: String::from_utf8_lossy(&bytes).into_owned(),
    }
}

/// Turns a route pattern into a concrete path, for example `/posts/{slug}` → `/posts/x`.
fn concrete(pattern: &str) -> String {
    pattern
        .split('/')
        .map(|seg| if seg.starts_with('{') { "x" } else { seg })
        .collect::<Vec<_>>()
        .join("/")
}

#[tokio::test]
async fn every_page_route_serves_index_html() {
    for route in ROUTES.iter().filter(|r| r.kind == Kind::Page) {
        let path = concrete(route.path);
        let r = get(&path).await;
        assert_eq!(r.status, StatusCode::OK, "{path}");
        assert!(
            r.headers[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/html"),
            "{path}"
        );
        assert!(r.body.contains("<title>Eitan's Logbook</title>"), "{path}");
    }
}

#[tokio::test]
async fn healthz_answers() {
    let r = get("/healthz").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body, "ok");
}

#[tokio::test]
async fn unknown_paths_are_404() {
    for path in ["/nope", "/posts", "/posts/a/b", "/api/whatever"] {
        assert_eq!(get(path).await.status, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn security_headers_are_on_every_kind_of_response() {
    for path in [
        "/",
        "/posts/x",
        "/healthz",
        "/nope",
        "/static/css/logbook.css",
        "/static/missing.css",
    ] {
        let r = get(path).await;
        for (name, value) in SECURITY_HEADERS {
            assert_eq!(
                r.headers.get(*name).map(|v| v.to_str().unwrap()),
                Some(*value),
                "{name} on {path}"
            );
        }
    }
}

#[tokio::test]
async fn csp_allows_no_inline_code_and_no_third_parties() {
    assert!(!CSP.contains("unsafe-inline"));
    assert!(
        !CSP.contains(" 'unsafe-eval'"),
        "only 'wasm-unsafe-eval' is allowed"
    );
    assert!(!CSP.contains("http"), "no third-party origins");
    assert!(CSP.contains("object-src 'none'"));
    assert!(CSP.contains("frame-ancestors 'none'"));
}

#[tokio::test]
async fn static_files_have_the_right_types() {
    for (path, ty) in [
        ("/static/css/logbook.css", "text/css"),
        ("/static/js/components.js", "text/javascript"),
        ("/static/fonts/tinos-400.woff2", "font/woff2"),
    ] {
        let r = get(path).await;
        assert_eq!(r.status, StatusCode::OK, "{path}");
        assert!(
            r.headers[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with(ty),
            "{path}"
        );
    }
}

#[tokio::test]
async fn static_files_cannot_escape_the_folder() {
    for path in [
        "/static/../Cargo.toml",
        "/static/%2e%2e/Cargo.toml",
        "/static/..%2fCargo.toml",
    ] {
        let r = get(path).await;
        assert_ne!(r.status, StatusCode::OK, "{path}");
        assert!(!r.body.contains("[workspace]"), "{path}");
    }
}

/// The CSP forbids inline code. This test fails before the browser would.
#[test]
fn static_html_has_no_inline_code_and_no_third_party_requests() {
    let html = std::fs::read_to_string(static_dir().join("index.html")).unwrap();
    let lower = html.to_ascii_lowercase();

    for (i, _) in lower.match_indices("<script") {
        let tag_end = i + lower[i..].find('>').unwrap();
        assert!(
            lower[i..tag_end].contains(" src="),
            "inline <script> at byte {i}"
        );
    }
    assert!(!lower.contains("<style"), "inline <style>");
    assert!(!lower.contains(" style=\""), "inline style attribute");
    let has_handler = lower.split(|c: char| c.is_whitespace()).any(|word| {
        word.starts_with("on")
            && word.contains("=\"")
            && word[2..]
                .split('=')
                .next()
                .unwrap()
                .chars()
                .all(|c| c.is_ascii_lowercase())
    });
    assert!(!has_handler, "inline event handler attribute");

    for dir in ["", "css", "js"] {
        for entry in std::fs::read_dir(static_dir().join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                assert!(
                    !text.contains("googleapis"),
                    "{} loads Google Fonts",
                    path.display()
                );
                assert!(
                    !text.contains("gstatic"),
                    "{} loads from gstatic",
                    path.display()
                );
            }
        }
    }
}
