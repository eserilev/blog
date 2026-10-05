//! Title section and topic tests (spec 4.2, 4.9), and migration 0002 on an old database.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;

fn slugs(list: &serde_json::Value) -> Vec<&str> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|t| t["slug"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn the_home_page_has_the_default_title_section_and_topics() {
    let f = fixture().await;
    let r = f.get("/").await;
    assert!(
        r.body
            .contains("<span id=\"site-title\">Eitan&#39;s Logbook</span>"),
        "{}",
        r.body
    );
    assert!(
        r.body
            .contains("Software Engineering · Gaming · Random Fun")
    );
    assert!(
        r.body
            .contains("<li><a href=\"/topics/jiu-jitsu\">Jiu jitsu</a></li>")
    );
    assert!(
        r.body
            .contains("<option value=\"classic-wow\">Classic WoW</option>")
    );
    assert!(r.body.contains("<p>I work on Ethereum client software"));
    assert!(!r.body.contains("<!--site:"), "a marker was not filled");
    assert_eq!(slugs(&f.json("/api/topics").await).len(), 6);
}

#[tokio::test]
async fn the_owner_edits_the_title_section() {
    let f = fixture().await;
    let s = f.session(30).await;
    let input = json!({
        "title": "  Uncle Bill <script>x</script> ",
        "subtitle": "Software Engineering · Gaming · Random Fun",
        "tagline": "Notes \"and\" things",
        "intro_md": "Hello **world** <script>bad()</script>",
    });
    let r = f
        .send(Req::put("/api/owner/site", input).cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.json()["title"], "Uncle Bill <script>x</script>");

    let page = f.get("/").await.body;
    assert!(
        !page.contains("<script>x"),
        "the title must not become a tag"
    );
    assert!(!page.contains("bad()</script>"), "the intro is sanitized");
    assert!(
        page.contains("<title>Uncle Bill &lt;script&gt;x&lt;/script&gt;</title>"),
        "{page}"
    );
    assert!(page.contains("<strong>world</strong>"));
    assert!(
        page.contains("content=\"Notes &quot;and&quot; things\""),
        "the tagline is the description"
    );

    let post = f.get(&format!("/posts/{PUBLIC_SLUG}")).await.body;
    assert!(
        post.contains("· Uncle Bill &lt;script&gt;"),
        "post titles use the site title"
    );

    let feed = f.get("/feed.xml").await.body;
    assert!(
        feed.contains("<title>Uncle Bill &lt;script&gt;x&lt;/script&gt;</title>"),
        "{feed}"
    );

    let api = f.json("/api/site").await;
    assert!(api.get("intro_md").is_none(), "guests get HTML only");
    assert!(
        api["intro_html"]
            .as_str()
            .unwrap()
            .contains("<strong>world</strong>")
    );
}

#[tokio::test]
async fn bad_site_input_is_rejected() {
    let f = fixture().await;
    let s = f.session(30).await;
    let base = json!({ "title": "T", "subtitle": "", "tagline": "", "intro_md": "" });
    for (field, value) in [
        ("title", json!("   ")),
        ("title", json!("x".repeat(81))),
        ("subtitle", json!("a\nb")),
        ("intro_md", json!("x".repeat(5_001))),
    ] {
        let mut body = base.clone();
        body[field] = value;
        let r = f
            .send(Req::put("/api/owner/site", body).cookie(Some(&s)))
            .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{field}");
    }
    let r = f
        .send(Req::put("/api/owner/site", json!({ "title": "T" })).cookie(Some(&s)))
        .await;
    assert_ne!(r.status, StatusCode::OK, "missing fields");
}

#[tokio::test]
async fn the_owner_adds_renames_orders_and_deletes_topics() {
    let f = fixture().await;
    let s = f.session(30).await;
    let add = |name: &'static str| Req::post("/api/owner/topics", json!({ "name": name }));

    let r = f.send(add("  Free Diving ").cookie(Some(&s))).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(
        r.json(),
        json!({ "slug": "free-diving", "name": "Free Diving" })
    );
    let r = f.send(add("free diving!").cookie(Some(&s))).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "same address");
    let r = f.send(add("שלום").cookie(Some(&s))).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "no slug");

    // The new topic is last, it has a page, and the home page links it.
    let list = f.json("/api/topics").await;
    assert_eq!(slugs(&list).last(), Some(&"free-diving"));
    assert_eq!(f.get("/topics/free-diving").await.status, StatusCode::OK);
    assert!(
        f.get("/")
            .await
            .body
            .contains("<a href=\"/topics/free-diving\">Free Diving</a>")
    );

    // Rename and move to the top. The slug stays.
    let mut topics: Vec<serde_json::Value> = list.as_array().unwrap().clone();
    let moved = topics.pop().unwrap();
    topics.insert(0, json!({ "slug": moved["slug"], "name": "Freediving" }));
    let r = f
        .send(Req::put("/api/owner/topics", json!({ "topics": topics })).cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(
        r.json()[0],
        json!({ "slug": "free-diving", "name": "Freediving" })
    );

    // The list must be the same set of topics.
    let short = &topics[1..];
    let r = f
        .send(Req::put("/api/owner/topics", json!({ "topics": short })).cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let mut dup = topics.clone();
    dup[1] = dup[0].clone();
    let r = f
        .send(Req::put("/api/owner/topics", json!({ "topics": dup })).cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    // A topic with posts stays. An empty one goes, and its page is 404 after.
    let r = f
        .send(Req::delete("/api/owner/topics/rust").cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    let r = f
        .send(Req::delete("/api/owner/topics/free-diving").cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(
        f.get("/topics/free-diving").await.status,
        StatusCode::NOT_FOUND
    );
    let r = f
        .send(Req::delete("/api/owner/topics/free-diving").cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_last_topic_cannot_be_deleted() {
    let f = fixture_with(false, 1000).await;
    let s = f.session(30).await;
    let all = f.json("/api/topics").await;
    let all = slugs(&all);
    for (i, slug) in all.iter().enumerate() {
        let r = f
            .send(Req::delete(&format!("/api/owner/topics/{slug}")).cookie(Some(&s)))
            .await;
        let want = if i + 1 == all.len() {
            StatusCode::CONFLICT
        } else {
            StatusCode::OK
        };
        assert_eq!(r.status, want, "{slug}: {}", r.body);
    }
    // A new post takes the one topic that is left.
    let r = f
        .send(Req::post("/api/owner/posts", json!({})).cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.json()["topic"], *all.last().unwrap());
}

#[tokio::test]
async fn posts_use_new_topics_and_refuse_unknown_ones() {
    let f = fixture().await;
    let s = f.session(30).await;
    f.send(Req::post("/api/owner/topics", json!({ "name": "Gaming" })).cookie(Some(&s)))
        .await;
    let p = f
        .send(Req::post("/api/owner/posts", json!({})).cookie(Some(&s)))
        .await
        .json();
    let id = p["id"].as_i64().unwrap();
    let save = |topic: &str, version: i64| {
        Req::put(
            Box::leak(format!("/api/owner/posts/{id}").into_boxed_str()),
            json!({ "title": "Raid night", "summary": "", "topic": topic, "tags": [], "body_md": "x" }),
        )
        .if_match(version)
    };
    let r = f.send(save("nope", 1).cookie(Some(&s))).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    let r = f.send(save("gaming", 1).cookie(Some(&s))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    f.send(
        Req::post(
            Box::leak(format!("/api/owner/posts/{id}/state").into_boxed_str()),
            json!({ "state": "public" }),
        )
        .if_match(2)
        .cookie(Some(&s)),
    )
    .await;
    let list = f.json("/api/topics/gaming").await;
    assert_eq!(list[0]["topic_name"], "Gaming");
    // A topic with a post cannot be deleted now.
    let r = f
        .send(Req::delete("/api/owner/topics/gaming").cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
}

/// Migration 0002 builds `posts` again. Every row of an old database must survive.
#[tokio::test]
async fn migration_0002_keeps_every_post() {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.db");
    let opts = SqliteConnectOptions::from_str("sqlite:")
        .unwrap()
        .filename(&path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new().connect_with(opts).await.unwrap();
    let mut old = sqlx::migrate!("./migrations");
    old.migrations = old
        .migrations
        .iter()
        .take(1)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    old.run(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO posts (id, slug, title, topic, body_md, body_html, word_count, state, version, published_at, updated_at)
         VALUES (7, 'old-post', 'Old', 'classic-wow', 'x', '<p>x</p>', 1, 'public', 3, '2026-01-01T00:00:00Z', '2026-01-02T00:00:00Z'),
                (9, 'secret', 'Hidden', 'rust', 'y', '<p>y</p>', 1, 'private', 1, NULL, '2026-01-03T00:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    let pool = logbook_server::db::connect(&path).await.unwrap();
    let rows: Vec<(i64, String, String, String, i64)> =
        sqlx::query_as("SELECT id, slug, topic, state, version FROM posts ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        [
            (
                7,
                "old-post".into(),
                "classic-wow".into(),
                "public".into(),
                3
            ),
            (9, "secret".into(), "rust".into(), "private".into(), 1),
        ]
    );
    let fk: Vec<(String,)> = sqlx::query_as("SELECT \"table\" FROM pragma_foreign_key_check")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(fk.is_empty(), "{fk:?}");
    let err = sqlx::query("UPDATE posts SET topic = 'nope' WHERE id = 7")
        .execute(&pool)
        .await;
    assert!(err.is_err(), "the topic must exist");
}
