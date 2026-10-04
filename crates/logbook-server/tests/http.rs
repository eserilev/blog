//! HTTP tests against the real router, in process, with a temporary SQLite file.

use std::path::PathBuf;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use logbook_core::{State, Topic};
use logbook_server::{
    AppState, Config, app, db,
    headers::{CSP, SECURITY_HEADERS},
    posts::{self, NewPost},
    routes::{Access, Kind, ROUTES},
};
use tower::ServiceExt;

fn static_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../static")
}

/// A secret marker in every non-public post. No guest response may contain one.
const SECRET: &str = "SECRETMARKER";

struct Fixture {
    app: Router,
    _dir: tempfile::TempDir,
}

/// Slugs of the fixture posts, in the order they are created.
const PUBLIC_SLUG: &str = "a-public-post-with-quotes-script";
const OLDER_SLUG: &str = "an-older-public-post";
const DRAFT_SLUG: &str = "a-draft";
const PRIVATE_SLUG: &str = "a-private-post";

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let config = Config {
        addr: "127.0.0.1:0".parse().unwrap(),
        static_dir: static_dir(),
        db_path: dir.path().join("test.db"),
        origin: "https://logbook.test".into(),
    };
    let pool = db::connect(&config.db_path).await.unwrap();
    let new = |title, topic, state, body, at| NewPost {
        title,
        summary: "summary",
        topic,
        tags: &["t1", "t2"],
        body_md: body,
        state,
        published_at: at,
    };
    for p in [
        new(
            "An older public post",
            Topic::Surf,
            State::Public,
            "Older.",
            Some("2026-01-01T00:00:00Z"),
        ),
        new(
            "A public post with \"quotes\" & <script>",
            Topic::Rust,
            State::Public,
            "# Hi\n\n```rust\nfn x() {}\n```\n",
            Some("2026-02-01T00:00:00Z"),
        ),
        new(
            "A draft",
            Topic::Rust,
            State::Draft,
            "SECRETMARKER draft body",
            None,
        ),
        new(
            "A private post",
            Topic::Rust,
            State::Private,
            "SECRETMARKER private body",
            None,
        ),
    ] {
        posts::create(&pool, &p).await.unwrap();
    }
    // Put the marker in the titles and summaries of the hidden posts too.
    sqlx::query("UPDATE posts SET title = title || ' SECRETMARKER', summary = 'SECRETMARKER' WHERE state != 'public'")
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(&config, pool).unwrap();
    Fixture {
        app: app(&config, state),
        _dir: dir,
    }
}

struct Reply {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: String,
}

impl Fixture {
    async fn get(&self, path: &str) -> Reply {
        let res = self
            .app
            .clone()
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

    async fn json(&self, path: &str) -> serde_json::Value {
        let r = self.get(path).await;
        assert_eq!(r.status, StatusCode::OK, "{path}: {}", r.body);
        serde_json::from_str(&r.body).unwrap()
    }
}

/// Every concrete path to try for a route pattern.
fn paths(pattern: &str) -> Vec<String> {
    let fills: &[&str] = if pattern.contains("{slug}") {
        &[PUBLIC_SLUG, OLDER_SLUG, DRAFT_SLUG, PRIVATE_SLUG, "missing"]
    } else if pattern.contains("{topic}") {
        &["rust", "surf", "nope"]
    } else if pattern.contains('{') {
        &["1", "x"]
    } else {
        &[""]
    };
    fills
        .iter()
        .map(|f| {
            pattern
                .split('/')
                .map(|seg| if seg.starts_with('{') { *f } else { seg })
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect()
}

/// The first access test (spec 7.4). Step 3 extends it to the owner and sessions.
/// Every public route, with every fixture slug: no secret marker in the response.
#[tokio::test]
async fn no_public_route_leaks_a_hidden_post() {
    let f = fixture().await;
    let mut checked = 0;
    for route in ROUTES {
        assert_eq!(
            route.access,
            Access::Public,
            "step 2 has public routes only: {}",
            route.path
        );
        for path in paths(route.path) {
            let r = f.get(&path).await;
            let all_headers = format!("{:?}", r.headers);
            assert!(
                !r.body.contains(SECRET),
                "{path} leaks a hidden post: {}",
                r.body
            );
            assert!(!all_headers.contains(SECRET), "{path} leaks in headers");
            checked += 1;
        }
    }
    assert!(checked > 20);
}

#[tokio::test]
async fn list_has_only_public_posts_newest_first() {
    let f = fixture().await;
    let list = f.json("/api/posts").await;
    let slugs: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["slug"].as_str().unwrap())
        .collect();
    assert_eq!(slugs, [PUBLIC_SLUG, OLDER_SLUG]);
}

#[tokio::test]
async fn guest_json_has_no_owner_fields() {
    let f = fixture().await;
    let post = f.json(&format!("/api/posts/{PUBLIC_SLUG}")).await;
    for field in ["body_md", "state", "version", "id", "updated_at"] {
        assert!(post.get(field).is_none(), "guest JSON has {field}");
    }
    assert_eq!(post["topic"], "rust");
    assert_eq!(post["topic_name"], "Rust");
    assert_eq!(post["tags"], serde_json::json!(["t1", "t2"]));
    assert_eq!(post["reading_minutes"], 1);
    let html = post["body_html"].as_str().unwrap();
    assert!(html.contains("<h1>Hi</h1>"));
    assert!(html.contains("class=\"hl-"));
}

#[tokio::test]
async fn hidden_and_missing_posts_are_404_in_the_api() {
    let f = fixture().await;
    for slug in [DRAFT_SLUG, PRIVATE_SLUG, "missing"] {
        let r = f.get(&format!("/api/posts/{slug}")).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{slug}");
        assert!(
            r.headers[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("application/json")
        );
    }
}

#[tokio::test]
async fn topics_filter_public_posts() {
    let f = fixture().await;
    let rust = f.json("/api/topics/rust").await;
    assert_eq!(rust.as_array().unwrap().len(), 1);
    assert_eq!(
        f.json("/api/topics/snowboarding").await,
        serde_json::json!([])
    );
    assert_eq!(
        f.get("/api/topics/nope").await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_public_post_page_has_its_escaped_title_in_the_head() {
    let f = fixture().await;
    let r = f.get(&format!("/posts/{PUBLIC_SLUG}")).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.body.contains("<title>A public post with &quot;quotes&quot; &amp; &lt;script&gt; · Eitan&#39;s Logbook</title>"), "{}", r.body);
    assert!(r.body.contains(&format!(
        "<meta property=\"og:url\" content=\"https://logbook.test/posts/{PUBLIC_SLUG}\">"
    )));
    assert!(r.body.contains("content=\"article\""));
    assert!(!r.body.contains("noindex"));
    assert!(
        !r.body.contains("<script>"),
        "the title must not become a tag"
    );
}

#[tokio::test]
async fn hidden_and_missing_post_pages_are_404_noindex() {
    let f = fixture().await;
    for slug in [DRAFT_SLUG, PRIVATE_SLUG, "missing"] {
        let r = f.get(&format!("/posts/{slug}")).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{slug}");
        assert!(
            r.body
                .contains("<meta name=\"robots\" content=\"noindex\">"),
            "{slug}"
        );
        assert!(
            r.body
                .contains("<title>Not found · Eitan&#39;s Logbook</title>"),
            "{slug}"
        );
        assert!(
            !r.body.contains(slug) || slug == "missing",
            "the page must not echo a hidden slug"
        );
    }
}

#[tokio::test]
async fn topic_pages_are_404_for_unknown_topics() {
    let f = fixture().await;
    assert_eq!(f.get("/topics/rust").await.status, StatusCode::OK);
    let r = f.get("/topics/nope").await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert!(r.body.contains("noindex"));
}

#[tokio::test]
async fn plain_page_routes_serve_index_html() {
    let f = fixture().await;
    for route in ROUTES.iter().filter(|r| r.kind == Kind::Page) {
        for path in paths(route.path) {
            let r = f.get(&path).await;
            assert_eq!(r.status, StatusCode::OK, "{path}");
            assert!(
                r.headers[header::CONTENT_TYPE]
                    .to_str()
                    .unwrap()
                    .starts_with("text/html"),
                "{path}"
            );
            assert!(
                r.body.contains("<title>Eitan&#39;s Logbook</title>"),
                "{path}"
            );
        }
    }
}

#[tokio::test]
async fn healthz_answers() {
    let f = fixture().await;
    let r = f.get("/healthz").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body, "ok");
}

#[tokio::test]
async fn unknown_paths_are_404() {
    let f = fixture().await;
    for path in [
        "/nope",
        "/posts",
        "/posts/a/b",
        "/api/whatever",
        "/api/posts/a/b",
    ] {
        assert_eq!(f.get(path).await.status, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn security_headers_are_on_every_kind_of_response() {
    let f = fixture().await;
    for path in [
        "/",
        "/posts/missing",
        "/api/posts",
        "/api/posts/missing",
        "/healthz",
        "/nope",
        "/static/css/logbook.css",
        "/static/missing.css",
    ] {
        let r = f.get(path).await;
        for (name, value) in SECURITY_HEADERS {
            assert_eq!(
                r.headers.get(*name).map(|v| v.to_str().unwrap()),
                Some(*value),
                "{name} on {path}"
            );
        }
    }
}

#[test]
fn csp_allows_no_inline_code_and_no_third_parties() {
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
    let f = fixture().await;
    for (path, ty) in [
        ("/static/css/logbook.css", "text/css"),
        ("/static/css/code.css", "text/css"),
        ("/static/js/components.js", "text/javascript"),
        ("/static/fonts/tinos-400.woff2", "font/woff2"),
    ] {
        let r = f.get(path).await;
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
    let f = fixture().await;
    for path in [
        "/static/../Cargo.toml",
        "/static/%2e%2e/Cargo.toml",
        "/static/..%2fCargo.toml",
    ] {
        let r = f.get(path).await;
        assert_ne!(r.status, StatusCode::OK, "{path}");
        assert!(!r.body.contains("[workspace]"), "{path}");
    }
}

#[tokio::test]
async fn slugs_are_unique_and_fall_back_to_the_id() {
    let dir = tempfile::tempdir().unwrap();
    let pool = db::connect(&dir.path().join("t.db")).await.unwrap();
    let p = NewPost {
        title: "Same title",
        summary: "",
        topic: Topic::Rust,
        tags: &[],
        body_md: "x",
        state: State::Draft,
        published_at: None,
    };
    let a = posts::create(&pool, &p).await.unwrap();
    let b = posts::create(&pool, &p).await.unwrap();
    let hebrew = posts::create(
        &pool,
        &NewPost {
            title: "שלום",
            ..p.clone()
        },
    )
    .await
    .unwrap();
    let slugs: Vec<(i64, String)> = sqlx::query_as("SELECT id, slug FROM posts ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(slugs[0], (a.cast_signed(), "same-title".into()));
    assert_eq!(slugs[1], (b.cast_signed(), format!("post-{b}")));
    assert_eq!(slugs[2], (hebrew.cast_signed(), format!("post-{hebrew}")));
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
    let has_handler = lower.split(char::is_whitespace).any(|w| {
        w.starts_with("on")
            && w.contains("=\"")
            && w[2..]
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
                    !text.contains("googleapis") && !text.contains("gstatic"),
                    "{} loads Google Fonts",
                    path.display()
                );
            }
        }
    }
}
