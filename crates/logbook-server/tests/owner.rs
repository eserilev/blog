//! Owner post API tests (spec 6.4, 7.4): create, save with versions, states, delete.
#![allow(clippy::many_single_char_names)] // f (fixture), o (owner), p (post), v (version) read best short here

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{Value, json};

struct Owner<'a> {
    f: &'a Fixture,
    session: String,
}

impl Owner<'_> {
    async fn create(&self) -> Value {
        let r = self
            .f
            .send(Req::post("/api/owner/posts", json!({})).cookie(Some(&self.session)))
            .await;
        assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
        r.json()
    }
    async fn save(&self, id: i64, version: i64, body: Value) -> Reply {
        self.f
            .send(
                Req::put(&format!("/api/owner/posts/{id}"), body)
                    .if_match(version)
                    .cookie(Some(&self.session)),
            )
            .await
    }
    async fn set_state(&self, id: i64, version: i64, state: &str) -> Reply {
        self.f
            .send(
                Req::post(
                    &format!("/api/owner/posts/{id}/state"),
                    json!({ "state": state }),
                )
                .if_match(version)
                .cookie(Some(&self.session)),
            )
            .await
    }
    async fn get(&self, id: i64) -> Reply {
        self.f
            .send(Req::get(&format!("/api/owner/posts/{id}")).cookie(Some(&self.session)))
            .await
    }
}

fn input(title: &str, body: &str) -> Value {
    json!({ "title": title, "summary": "A summary.", "topic": "rust", "tags": ["rust", "ssz"], "body_md": body })
}

async fn owner(f: &Fixture) -> Owner<'_> {
    Owner {
        f,
        session: f.session(30).await,
    }
}

#[tokio::test]
async fn create_save_publish_edit() {
    let f = fixture_with(false, 1000).await;
    let o = owner(&f).await;

    // A new post is an empty draft.
    let p = o.create().await;
    assert_eq!(p["state"], "draft");
    assert_eq!(p["title"], "Untitled");
    let (id, v1) = (p["id"].as_i64().unwrap(), p["version"].as_i64().unwrap());

    // Save: the version goes up, the slug follows the title, the body is rendered.
    let r = o
        .save(
            id,
            v1,
            input("Zero-copy SSZ decoding", "# Hi\n\nSome words here."),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let p = r.json();
    assert_eq!(p["version"], v1 + 1);
    assert_eq!(p["slug"], "zero-copy-ssz-decoding");
    assert_eq!(p["tags"], json!(["rust", "ssz"]));
    assert_eq!(p["word_count"], 4, "the # is not a word");
    // Still a draft: guests cannot see it.
    assert_eq!(
        f.get("/api/posts/zero-copy-ssz-decoding").await.status,
        StatusCode::NOT_FOUND
    );

    // Publish.
    let r = o.set_state(id, v1 + 1, "public").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let p = r.json();
    let published_at = p["published_at"].as_str().unwrap().to_string();
    let guest = f.json("/api/posts/zero-copy-ssz-decoding").await;
    assert!(guest["body_html"].as_str().unwrap().contains("<h1>Hi</h1>"));

    // After the first publish, a new title does not change the slug.
    let r = o.save(id, v1 + 2, input("A new title", "Body.")).await;
    assert_eq!(r.json()["slug"], "zero-copy-ssz-decoding");
    assert_eq!(
        f.json("/api/posts/zero-copy-ssz-decoding").await["title"],
        "A new title"
    );

    // Private, then public again: published_at does not change.
    assert_eq!(
        o.set_state(id, v1 + 3, "private").await.status,
        StatusCode::OK
    );
    assert_eq!(
        f.get("/api/posts/zero-copy-ssz-decoding").await.status,
        StatusCode::NOT_FOUND
    );
    let p = o.set_state(id, v1 + 4, "public").await.json();
    assert_eq!(p["published_at"], published_at.as_str());
}

#[tokio::test]
async fn a_stale_version_is_a_conflict() {
    let f = fixture_with(false, 1000).await;
    let o = owner(&f).await;
    let p = o.create().await;
    let (id, v) = (p["id"].as_i64().unwrap(), p["version"].as_i64().unwrap());
    assert_eq!(
        o.save(id, v, input("First tab", "a")).await.status,
        StatusCode::OK
    );
    // A second tab still has the old version.
    let r = o.save(id, v, input("Second tab", "b")).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert!(r.body.contains("another device"));
    assert_eq!(
        o.set_state(id, v, "public").await.status,
        StatusCode::CONFLICT
    );
    // The first save is kept.
    assert_eq!(o.get(id).await.json()["title"], "First tab");
}

#[tokio::test]
async fn writes_need_if_match() {
    let f = fixture_with(false, 1000).await;
    let o = owner(&f).await;
    let id = o.create().await["id"].as_i64().unwrap();
    let r = f
        .send(Req::put(&format!("/api/owner/posts/{id}"), input("T", "b")).cookie(Some(&o.session)))
        .await;
    assert_eq!(r.status, StatusCode::PRECONDITION_REQUIRED);
}

#[tokio::test]
async fn bad_input_is_refused() {
    let f = fixture_with(false, 1000).await;
    let o = owner(&f).await;
    let p = o.create().await;
    let (id, v) = (p["id"].as_i64().unwrap(), p["version"].as_i64().unwrap());
    let long = "x".repeat(201);
    for (body, why) in [
        (input("  ", "b"), "empty title"),
        (input(&long, "b"), "long title"),
        (
            json!({ "title": "T", "summary": "", "topic": "nope", "tags": [], "body_md": "" }),
            "unknown topic",
        ),
        (
            json!({ "title": "T", "summary": "", "topic": "rust", "tags": ["Bad Tag"], "body_md": "" }),
            "bad tag",
        ),
        (
            json!({ "title": "T", "summary": "", "topic": "rust", "tags": [], "body_md": "x".repeat(200_001) }),
            "long body",
        ),
    ] {
        assert_eq!(
            o.save(id, v, body).await.status,
            StatusCode::BAD_REQUEST,
            "{why}"
        );
    }
    // Unknown fields are refused by the JSON extractor.
    let r = o.save(id, v, json!({ "title": "T", "summary": "", "topic": "rust", "tags": [], "body_md": "", "state": "public" })).await;
    assert!(r.status.is_client_error(), "unknown field: {}", r.status);
    assert_eq!(o.get(id).await.json()["version"], v, "nothing was saved");
}

#[tokio::test]
async fn a_taken_slug_falls_back_to_the_id() {
    let f = fixture_with(false, 1000).await;
    let o = owner(&f).await;
    let a = o.create().await;
    let b = o.create().await;
    assert_eq!(
        o.save(
            a["id"].as_i64().unwrap(),
            a["version"].as_i64().unwrap(),
            input("Same title", "x")
        )
        .await
        .json()["slug"],
        "same-title"
    );
    let bid = b["id"].as_i64().unwrap();
    assert_eq!(
        o.save(
            bid,
            b["version"].as_i64().unwrap(),
            input("Same title", "x")
        )
        .await
        .json()["slug"],
        format!("post-{bid}")
    );
}

#[tokio::test]
async fn list_and_delete() {
    let f = fixture().await;
    let o = owner(&f).await;
    let list = f
        .send(Req::get("/api/owner/posts").cookie(Some(&o.session)))
        .await
        .json();
    let states: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["state"].as_str().unwrap())
        .collect();
    assert!(states.contains(&"draft") && states.contains(&"private") && states.contains(&"public"));
    let id = list[0]["id"].as_i64().unwrap();
    let r = f
        .send(Req::delete(&format!("/api/owner/posts/{id}")).cookie(Some(&o.session)))
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(o.get(id).await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        f.send(Req::delete(&format!("/api/owner/posts/{id}")).cookie(Some(&o.session)))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}
