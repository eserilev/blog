//! Health and backup check tests (spec 6.12).

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use common::*;
use logbook_server::checks::{self, Replica};
use object_store::{ObjectStoreExt, PutPayload, path::Path};

#[tokio::test]
async fn healthz_fails_on_a_stale_or_unreadable_replica() {
    let f = fixture().await;
    assert_eq!(f.get("/healthz").await.status, StatusCode::OK);
    for (status, want) in [
        (Replica::Fresh(Duration::from_mins(3)), StatusCode::OK),
        (
            Replica::Stale(Duration::from_hours(2)),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            Replica::Error("no replica".into()),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    ] {
        *f.state.replica.write().await = status.clone();
        assert_eq!(f.get("/healthz").await.status, want, "{status:?}");
    }
}

#[tokio::test]
async fn replica_age_is_the_newest_object_under_db() {
    let f = fixture().await;
    let err = checks::replica_age(&f.state.media).await.unwrap_err();
    assert!(err.contains("no replica"), "{err}");
    // An image is not a replica.
    let store = &f.state.media.store;
    store
        .put(&Path::from("uploads/x.png"), PutPayload::from_static(b"x"))
        .await
        .unwrap();
    assert!(checks::replica_age(&f.state.media).await.is_err());
    store
        .put(
            &Path::from("db/generations/0001/ltx"),
            PutPayload::from_static(b"x"),
        )
        .await
        .unwrap();
    let age = checks::replica_age(&f.state.media).await.unwrap();
    assert!(age < Duration::from_mins(1), "{age:?}");
}

#[tokio::test]
async fn restored_copy_passes_with_a_fresh_heartbeat() {
    let (f, config) = fixture_full(true, 30).await;
    checks::beat(&f.pool).await.unwrap();
    f.pool.close().await;
    let msg = checks::check_restored(&config.db_path).await.unwrap();
    assert!(msg.starts_with("backup ok"), "{msg}");
}

#[tokio::test]
async fn restored_copy_fails_with_an_old_heartbeat() {
    let (f, config) = fixture_full(true, 30).await;
    sqlx::query("INSERT OR REPLACE INTO heartbeat (id, at) VALUES (1, '2026-01-01T00:00:00Z')")
        .execute(&f.pool)
        .await
        .unwrap();
    f.pool.close().await;
    let err = checks::check_restored(&config.db_path).await.unwrap_err();
    assert!(err.contains("min old"), "{err}");
}

#[tokio::test]
async fn restored_copy_fails_without_a_heartbeat_or_a_database() {
    let (f, config) = fixture_full(false, 30).await;
    f.pool.close().await;
    let err = checks::check_restored(&config.db_path).await.unwrap_err();
    assert!(err.contains("no heartbeat"), "{err}");
    let dir = tempfile::tempdir().unwrap();
    let junk = dir.path().join("junk.db");
    std::fs::write(&junk, b"this is not a database at all, not even close").unwrap();
    assert!(checks::check_restored(&junk).await.is_err());
}
