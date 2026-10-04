//! Health check and heartbeat (spec 6.12).

use std::time::Duration;

use axum::{extract::State, http::StatusCode};
use sqlx::SqlitePool;

use crate::AppState;

/// How often the heartbeat row is written.
pub const HEARTBEAT_EVERY: Duration = Duration::from_mins(10);

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

/// Writes the heartbeat every [`HEARTBEAT_EVERY`], so the replica age always means
/// something, even when nobody writes posts.
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

/// `GET /healthz`: 200 if the database answers. Step 7 adds the replica age check.
pub async fn healthz(State(s): State<AppState>) -> (StatusCode, &'static str) {
    match sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&s.pool)
        .await
    {
        Ok(_) => (StatusCode::OK, "ok"),
        Err(e) => {
            tracing::error!("health check failed: {e}");
            (StatusCode::SERVICE_UNAVAILABLE, "database unavailable")
        }
    }
}
