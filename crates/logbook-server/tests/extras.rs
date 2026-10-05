//! Now box, RSS, visitor counter, and export tests (spec 4.4, 4.7, 4.8, 6.10).

mod common;

use std::process::Command;

use axum::http::{StatusCode, header};
use common::*;
use logbook_server::export::{self, GitExport};
use serde_json::json;

const BROWSER: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/150 Safari/537.36";

#[tokio::test]
async fn now_box_round_trip() {
    let f = fixture().await;
    assert_eq!(
        f.json("/api/now").await,
        json!({ "body_html": "", "updated_at": null })
    );
    let s = f.session(30).await;
    let r = f
        .send(
            Req::put(
                "/api/owner/now",
                json!({ "body_md": "- Working on **ePBS**" }),
            )
            .cookie(Some(&s)),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let now = f.json("/api/now").await;
    assert!(
        now["body_html"]
            .as_str()
            .unwrap()
            .contains("<strong>ePBS</strong>")
    );
    assert!(now["updated_at"].is_string());
    assert!(now.get("body_md").is_none(), "guests get HTML only");
    let long = "x".repeat(5_001);
    let r = f
        .send(Req::put("/api/owner/now", json!({ "body_md": long })).cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn feed_has_public_posts_only_with_absolute_links() {
    let f = fixture().await;
    let r = f.get("/feed.xml").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        r.headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("application/rss+xml")
    );
    let channel = rss::Channel::read_from(r.body.as_bytes()).expect("valid RSS");
    let titles: Vec<&str> = channel.items().iter().filter_map(|i| i.title()).collect();
    assert_eq!(
        titles,
        [
            "A public post with \"quotes\" & <script>",
            "An older public post"
        ]
    );
    let item = &channel.items()[0];
    assert_eq!(
        item.link(),
        Some(format!("{ORIGIN}/posts/{PUBLIC_SLUG}").as_str())
    );
    assert!(item.guid().unwrap().value().starts_with("logbook-post-"));
    assert!(!item.guid().unwrap().is_permalink());
    assert_eq!(item.pub_date(), Some("Sun, 01 Feb 2026 00:00:00 +0000"));
    assert!(!r.body.contains(SECRET));
}

#[tokio::test]
async fn visitors_count_people_not_bots() {
    let f = fixture().await;
    for _ in 0..3 {
        f.send(Req::get("/").ua(BROWSER)).await;
    }
    f.send(Req::get(&format!("/posts/{PUBLIC_SLUG}")).ua(BROWSER))
        .await;
    for bot in ["Googlebot/2.1", "curl/8.0", "Feedly/1.0", ""] {
        f.send(Req::get("/").ua(bot)).await;
    }
    f.get("/api/posts").await; // not a page
    assert_eq!(f.json("/api/visitors").await["total"], 4);
    // A flush moves the count into the database; the total stays the same.
    f.state.counter.flush(&f.pool).await.unwrap();
    assert_eq!(f.state.counter.pending(), 0);
    assert_eq!(f.json("/api/visitors").await["total"], 4);
    let stored: i64 = sqlx::query_scalar("SELECT SUM(count) FROM visits")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(stored, 4);
}

#[tokio::test]
async fn owner_zip_has_every_post_as_markdown() {
    let f = fixture().await;
    let s = f.session(30).await;
    f.send(Req::put("/api/owner/now", json!({ "body_md": "now text" })).cookie(Some(&s)))
        .await;
    let r = f
        .send(Req::get("/api/owner/export.zip").cookie(Some(&s)))
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.headers[header::CONTENT_TYPE], "application/zip");
    // The body was read as UTF-8 lossy; fetch the raw bytes again.
    let bytes = raw(&f, &s).await;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "now.md",
            "posts/a-draft.md",
            "posts/a-private-post.md",
            &format!("posts/{PUBLIC_SLUG}.md"),
            "posts/an-older-public-post.md"
        ]
    );
    let mut text = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("posts/a-draft.md").unwrap(), &mut text)
        .unwrap();
    let p = export::parse(&text).expect("the file parses");
    assert_eq!(p.state, "draft");
    assert!(p.body_md.contains("SECRETMARKER draft body"));
}

async fn raw(f: &Fixture, session: &str) -> Vec<u8> {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let req = axum::http::Request::get("/api/owner/export.zip")
        .header(header::COOKIE, format!("logbook_session={session}"))
        .body(Body::empty())
        .unwrap();
    let res = f.app.clone().oneshot(req).await.unwrap();
    res.into_body().collect().await.unwrap().to_bytes().to_vec()
}

fn git(dir: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[tokio::test]
async fn git_export_pushes_public_posts_only() {
    let f = fixture().await;
    let tmp = tempfile::tempdir().unwrap();
    let remote = tmp.path().join("remote.git");
    std::fs::create_dir(&remote).unwrap();
    git(&remote, &["init", "-q", "--bare", "-b", "main"]);
    let cfg = GitExport {
        repo: remote.to_string_lossy().into_owned(),
        branch: "main".into(),
        dir: tmp.path().join("work/export"),
        deploy_key: None,
    };

    // First export: the public posts are pushed.
    assert!(export::export_once(&f.pool, &cfg).await.unwrap());
    let check = tmp.path().join("check");
    git(
        tmp.path(),
        &["clone", "-q", &cfg.repo, check.to_str().unwrap()],
    );
    let mut files: Vec<String> = std::fs::read_dir(check.join("posts"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    assert_eq!(
        files,
        [
            format!("{PUBLIC_SLUG}.md"),
            "an-older-public-post.md".to_string()
        ]
    );
    for name in &files {
        let text = std::fs::read_to_string(check.join("posts").join(name)).unwrap();
        assert!(!text.contains(SECRET), "{name} has hidden content");
    }

    // Nothing changed: no new commit.
    assert!(!export::export_once(&f.pool, &cfg).await.unwrap());

    // A post leaves public: it leaves the export.
    sqlx::query("UPDATE posts SET state = 'private' WHERE slug = 'an-older-public-post'")
        .execute(&f.pool)
        .await
        .unwrap();
    assert!(export::export_once(&f.pool, &cfg).await.unwrap());
    git(&check, &["pull", "-q"]);
    assert!(!check.join("posts/an-older-public-post.md").exists());
    assert_eq!(git(&check, &["rev-list", "--count", "HEAD"]).trim(), "2");
}
