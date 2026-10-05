//! Shared test helpers: a fixture with posts in every state, request helpers,
//! and owner sessions created straight in the database.
#![allow(dead_code)]

use std::path::PathBuf;

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use logbook_core::{State, Topic};
use logbook_server::{
    AppState, Config, app, auth, db,
    posts::{self, NewPost},
};
use sqlx::SqlitePool;
use tower::ServiceExt;

pub const ORIGIN: &str = "https://logbook.test";
/// A secret marker in every non-public post. No guest response may contain one.
pub const SECRET: &str = "SECRETMARKER";

pub const PUBLIC_SLUG: &str = "a-public-post-with-quotes-script";
pub const OLDER_SLUG: &str = "an-older-public-post";
pub const DRAFT_SLUG: &str = "a-draft";
pub const PRIVATE_SLUG: &str = "a-private-post";

pub fn static_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../static")
}

pub struct Fixture {
    pub app: Router,
    pub pool: SqlitePool,
    pub state: AppState,
    _dir: tempfile::TempDir,
}

pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
}

impl Reply {
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).unwrap_or_else(|e| panic!("not JSON ({e}): {}", self.body))
    }
}

/// A fixture. `with_posts` adds posts in every state, with markers in the hidden ones.
pub async fn fixture_with(with_posts: bool, auth_rate_limit: u32) -> Fixture {
    fixture_full(with_posts, auth_rate_limit).await.0
}

/// A fixture plus its config, for tests that need the paths.
pub async fn fixture_full(with_posts: bool, auth_rate_limit: u32) -> (Fixture, Config) {
    let dir = tempfile::tempdir().unwrap();
    let config = Config {
        addr: "127.0.0.1:0".parse().unwrap(),
        static_dir: static_dir(),
        db_path: dir.path().join("test.db"),
        origin: ORIGIN.into(),
        trusted_proxies: Vec::new(),
        auth_rate_limit,
        git_export: None,
    };
    let pool = db::connect(&config.db_path).await.unwrap();
    if with_posts {
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
        sqlx::query("UPDATE posts SET title = title || ' SECRETMARKER', summary = 'SECRETMARKER' WHERE state != 'public'")
            .execute(&pool)
            .await
            .unwrap();
    }
    let state = AppState::new(&config, pool.clone()).unwrap();
    let fixture = Fixture {
        app: app(&config, state.clone()),
        pool,
        state,
        _dir: dir,
    };
    (fixture, config)
}

pub async fn fixture() -> Fixture {
    fixture_with(true, 1000).await
}

/// A request builder with sensible defaults: same-origin `Origin` on writes and
/// a JSON body when one is given.
pub struct Req<'a> {
    pub method: Method,
    pub path: &'a str,
    pub cookie: Option<String>,
    pub body: Option<String>,
    pub origin: Option<&'a str>,
    pub content_type: Option<&'a str>,
    pub if_match: Option<i64>,
    pub user_agent: Option<&'a str>,
}

impl<'a> Req<'a> {
    pub fn ua(mut self, ua: &'a str) -> Self {
        self.user_agent = Some(ua);
        self
    }
    pub fn get(path: &'a str) -> Self {
        Self {
            method: Method::GET,
            path,
            cookie: None,
            body: None,
            origin: None,
            content_type: None,
            if_match: None,
            user_agent: None,
        }
    }
    #[allow(clippy::needless_pass_by_value)] // call sites read better with json!(..) by value
    pub fn post(path: &'a str, body: serde_json::Value) -> Self {
        Self {
            method: Method::POST,
            path,
            cookie: None,
            body: Some(body.to_string()),
            origin: Some(ORIGIN),
            content_type: Some("application/json"),
            if_match: None,
            user_agent: None,
        }
    }
    #[allow(clippy::needless_pass_by_value)] // call sites read better with json!(..) by value
    pub fn put(path: &'a str, body: serde_json::Value) -> Self {
        Self {
            method: Method::PUT,
            ..Self::post(path, body)
        }
    }
    pub fn if_match(self, version: i64) -> Self {
        Self {
            if_match: Some(version),
            ..self
        }
    }
    pub fn delete(path: &'a str) -> Self {
        Self {
            method: Method::DELETE,
            path,
            cookie: None,
            body: None,
            origin: Some(ORIGIN),
            content_type: None,
            if_match: None,
            user_agent: None,
        }
    }
    pub fn cookie(mut self, c: Option<&str>) -> Self {
        self.cookie = c.map(|t| format!("{}={t}", auth::SESSION_COOKIE));
        self
    }
}

impl Fixture {
    pub async fn send(&self, r: Req<'_>) -> Reply {
        let mut b = Request::builder().method(r.method).uri(r.path);
        if let Some(c) = &r.cookie {
            b = b.header(header::COOKIE, c);
        }
        if let Some(o) = r.origin {
            b = b.header(header::ORIGIN, o);
        }
        if let Some(t) = r.content_type {
            b = b.header(header::CONTENT_TYPE, t);
        }
        if let Some(ua) = r.user_agent {
            b = b.header(header::USER_AGENT, ua);
        }
        if let Some(v) = r.if_match {
            b = b.header(header::IF_MATCH, v.to_string());
        }
        let body = r.body.unwrap_or_default();
        if !body.is_empty() {
            b = b.header(header::CONTENT_LENGTH, body.len());
        }
        let res = self
            .app
            .clone()
            .oneshot(b.body(Body::from(body)).unwrap())
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

    pub async fn get(&self, path: &str) -> Reply {
        self.send(Req::get(path)).await
    }

    pub async fn json(&self, path: &str) -> serde_json::Value {
        let r = self.get(path).await;
        assert_eq!(r.status, StatusCode::OK, "{path}: {}", r.body);
        r.json()
    }

    /// Inserts a session row and returns its cookie token. `days` < 0 makes it expired.
    pub async fn session(&self, days: i64) -> String {
        let token = auth::random_token();
        sqlx::query(
            "INSERT INTO sessions (token_hash, created_at, expires_at)
             VALUES (?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now', ?))",
        )
        .bind(auth::hash(&token).to_vec())
        .bind(format!("{days:+} days"))
        .execute(&self.pool)
        .await
        .unwrap();
        token
    }

    /// A token whose session row was deleted, as after sign-out.
    pub async fn revoked_session(&self) -> String {
        let token = self.session(30).await;
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(auth::hash(&token).to_vec())
            .execute(&self.pool)
            .await
            .unwrap();
        token
    }
}
