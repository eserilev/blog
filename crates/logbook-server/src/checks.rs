//! Health and backup checks (spec 6.12).
//!
//! - Heartbeat: a write every 10 minutes, so the replica always has recent data.
//! - Replica age: every 10 minutes, the age of the newest object under `db/` in the
//!   bucket. Older than 1 hour → `/healthz` fails.
//! - Backup check (`logbook check-backup`, and every night): restore the replica to a
//!   temp file, run `PRAGMA integrity_check`, and make sure that the restored
//!   heartbeat is less than 1 hour old. Then ping `HEALTHCHECK_URL`.

use std::{
    path::Path,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{extract::State, http::StatusCode};
use futures::TryStreamExt;
use sqlx::{
    Connection, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteConnection},
};
use tokio::sync::RwLock;

use crate::{AppState, media::Media};

/// How often the heartbeat row is written.
pub const HEARTBEAT_EVERY: Duration = Duration::from_mins(10);
/// How often the replica age is checked.
pub const REPLICA_CHECK_EVERY: Duration = Duration::from_mins(10);
/// The oldest replica (and restored heartbeat) that is still healthy.
pub const MAX_AGE: Duration = Duration::from_hours(1);
/// How often the backup check runs.
pub const BACKUP_CHECK_EVERY: Duration = Duration::from_hours(24);

/// Writes the heartbeat row now.
///
/// # Errors
///
/// Database errors.
pub async fn beat(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO heartbeat (id, at) VALUES (1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
         ON CONFLICT (id) DO UPDATE SET at = excluded.at",
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Writes the heartbeat every [`HEARTBEAT_EVERY`].
pub fn spawn_heartbeat(pool: SqlitePool) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(HEARTBEAT_EVERY);
        loop {
            tick.tick().await;
            if let Err(e) = beat(&pool).await {
                tracing::error!("heartbeat write failed: {e}");
            }
        }
    });
}

/// What `/healthz` knows about the replica.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Replica {
    /// No bucket (local development), or not checked yet.
    #[default]
    Unchecked,
    /// The newest replica object is this old, and not older than [`MAX_AGE`].
    Fresh(Duration),
    /// Older than [`MAX_AGE`].
    Stale(Duration),
    /// The bucket cannot be read, or it has no replica.
    Error(String),
}

/// The replica status, shared with `/healthz`.
pub type ReplicaStatus = Arc<RwLock<Replica>>;

/// The status for a replica of this age.
#[must_use]
pub fn classify(age: Result<Duration, String>) -> Replica {
    match age {
        Ok(a) if a <= MAX_AGE => Replica::Fresh(a),
        Ok(a) => Replica::Stale(a),
        Err(e) => Replica::Error(e),
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// The age of the newest object under `db/`.
///
/// # Errors
///
/// Bucket errors, or no objects under `db/`.
pub async fn replica_age(media: &Media) -> Result<Duration, String> {
    let prefix = object_store::path::Path::from("db");
    let newest = media
        .store
        .list(Some(&prefix))
        .try_fold(None, |acc: Option<i64>, meta| async move {
            let t = meta.last_modified.timestamp();
            Ok(Some(acc.map_or(t, |a| a.max(t))))
        })
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no replica under db/")?;
    let secs = u64::try_from(unix_now() - newest).unwrap_or(0);
    Ok(Duration::from_secs(secs))
}

/// Checks the replica age every [`REPLICA_CHECK_EVERY`]. Only with a bucket.
pub fn spawn_replica_check(media: Media, status: ReplicaStatus) {
    tokio::spawn(async move {
        // The first check waits 1 minute, so litestream can write its first snapshot.
        let start = tokio::time::Instant::now() + Duration::from_mins(1);
        let mut tick = tokio::time::interval_at(start, REPLICA_CHECK_EVERY);
        loop {
            tick.tick().await;
            let now = classify(replica_age(&media).await);
            if !matches!(now, Replica::Fresh(_)) {
                tracing::error!("replica check: {now:?}");
            }
            *status.write().await = now;
        }
    });
}

/// `GET /healthz`: 200 if the database answers and the replica is not stale.
/// Else 503 with a short reason.
pub async fn healthz(State(s): State<AppState>) -> (StatusCode, &'static str) {
    if let Err(e) = sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&s.pool)
        .await
    {
        tracing::error!("health check failed: {e}");
        return (StatusCode::SERVICE_UNAVAILABLE, "database unavailable");
    }
    match &*s.replica.read().await {
        Replica::Stale(_) => (StatusCode::SERVICE_UNAVAILABLE, "replica stale"),
        Replica::Error(_) => (StatusCode::SERVICE_UNAVAILABLE, "replica unreadable"),
        Replica::Unchecked | Replica::Fresh(_) => (StatusCode::OK, "ok"),
    }
}

/// The backup check: restore the replica with litestream, then [`check_restored`].
///
/// # Errors
///
/// A message for the first step that fails.
pub async fn check_backup(db_path: &Path, litestream_config: &Path) -> Result<String, String> {
    let dir = tempfile::tempdir().map_err(|e| format!("cannot make a temp dir: {e}"))?;
    let out = dir.path().join("check.db");
    let env_or = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.into());
    let restore = tokio::process::Command::new("litestream")
        .arg("restore")
        .arg("-config")
        .arg(litestream_config)
        .arg("-o")
        .arg(&out)
        .arg(db_path)
        // The config reads these. The entrypoint sets the same defaults.
        .env("LOGBOOK_DB", db_path)
        .env("S3_REGION", env_or("S3_REGION", "auto"))
        .env(
            "S3_FORCE_PATH_STYLE",
            env_or("S3_FORCE_PATH_STYLE", "false"),
        )
        .output()
        .await
        .map_err(|e| format!("cannot run litestream: {e}"))?;
    if !restore.status.success() {
        return Err(format!(
            "restore failed: {}",
            String::from_utf8_lossy(&restore.stderr).trim()
        ));
    }
    check_restored(&out).await
}

/// Integrity check and heartbeat age of a restored copy.
///
/// # Errors
///
/// A message for the first check that fails.
pub async fn check_restored(path: &Path) -> Result<String, String> {
    let opts = SqliteConnectOptions::from_str("sqlite:")
        .map_err(|e| e.to_string())?
        .filename(path)
        .read_only(true);
    let mut conn = SqliteConnection::connect_with(&opts)
        .await
        .map_err(|e| format!("cannot open the restored copy: {e}"))?;
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&mut conn)
        .await
        .map_err(|e| format!("integrity check: {e}"))?;
    if integrity != "ok" {
        return Err(format!("integrity check: {integrity}"));
    }
    let at: Option<String> = sqlx::query_scalar("SELECT at FROM heartbeat WHERE id = 1")
        .fetch_optional(&mut conn)
        .await
        .map_err(|e| format!("heartbeat: {e}"))?;
    let at = at.ok_or("the restored copy has no heartbeat")?;
    let t = time::OffsetDateTime::parse(&at, &time::format_description::well_known::Rfc3339)
        .map_err(|e| format!("bad heartbeat {at:?}: {e}"))?;
    let age = Duration::from_secs(u64::try_from(unix_now() - t.unix_timestamp()).unwrap_or(0));
    if age > MAX_AGE {
        return Err(format!(
            "the restored heartbeat is {} min old",
            age.as_secs() / 60
        ));
    }
    let posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
        .fetch_one(&mut conn)
        .await
        .map_err(|e| format!("posts: {e}"))?;
    Ok(format!(
        "backup ok: integrity ok, {posts} posts, heartbeat {} min old",
        age.as_secs() / 60
    ))
}

/// Pings `url` on success, or `<url>/fail` on failure (healthchecks.io style).
pub async fn ping(url: &str, result: &Result<String, String>) {
    let (target, body) = match result {
        Ok(m) => (url.to_string(), m.clone()),
        Err(e) => (format!("{}/fail", url.trim_end_matches('/')), e.clone()),
    };
    let sent = reqwest::Client::new()
        .post(&target)
        .body(body)
        .timeout(Duration::from_secs(20))
        .send()
        .await;
    if let Err(e) = sent {
        tracing::error!("health ping failed: {e}");
    }
}

/// Runs the backup check every [`BACKUP_CHECK_EVERY`]. The first run is 1 hour after
/// start, so a new deploy has a fresh replica first.
pub fn spawn_backup_check(
    db_path: std::path::PathBuf,
    litestream_config: std::path::PathBuf,
    healthcheck_url: Option<String>,
) {
    tokio::spawn(async move {
        tokio::time::sleep(MAX_AGE).await;
        let mut tick = tokio::time::interval(BACKUP_CHECK_EVERY);
        loop {
            tick.tick().await;
            let result = check_backup(&db_path, &litestream_config).await;
            match &result {
                Ok(m) => tracing::info!("{m}"),
                Err(e) => tracing::error!("backup check failed: {e}"),
            }
            if let Some(url) = &healthcheck_url {
                ping(url, &result).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_ages() {
        assert_eq!(
            classify(Ok(Duration::from_mins(5))),
            Replica::Fresh(Duration::from_mins(5))
        );
        assert_eq!(classify(Ok(MAX_AGE)), Replica::Fresh(MAX_AGE));
        let old = MAX_AGE + Duration::from_secs(1);
        assert_eq!(classify(Ok(old)), Replica::Stale(old));
        assert_eq!(classify(Err("x".into())), Replica::Error("x".into()));
    }
}
