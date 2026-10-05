//! The visitor counter (spec 4.8): page loads counted in memory, no cookies, no IPs.
//! Flushed to the `visits` table once per minute.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use axum::{Json, extract::State, http::HeaderMap};
use sqlx::SqlitePool;

use crate::{AppState, posts::OwnerError};

/// Pending page loads, not yet in the database.
#[derive(Debug, Default)]
pub struct Counter {
    pending: AtomicU64,
}

/// User agents of crawlers and tools. Not counted.
const BOTS: &[&str] = &[
    "bot",
    "crawl",
    "spider",
    "slurp",
    "curl",
    "wget",
    "python",
    "feed",
    "preview",
    "headless",
    "facebookexternalhit",
    "monitor",
];

/// True for a user agent that looks like a person's browser.
#[must_use]
pub fn is_person(headers: &HeaderMap) -> bool {
    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    !ua.is_empty() && !BOTS.iter().any(|b| ua.contains(b))
}

impl Counter {
    /// Counts one page load, if it looks like a person.
    pub fn hit(&self, headers: &HeaderMap) {
        if is_person(headers) {
            self.pending.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Page loads not yet flushed.
    #[must_use]
    pub fn pending(&self) -> u64 {
        self.pending.load(Ordering::Relaxed)
    }

    /// Adds the pending count to today's row.
    ///
    /// # Errors
    ///
    /// Database errors. The count is kept for the next flush.
    pub async fn flush(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let n = self.pending.swap(0, Ordering::Relaxed);
        if n == 0 {
            return Ok(());
        }
        let n_i = i64::try_from(n).unwrap_or(i64::MAX);
        let done = sqlx::query(
            "INSERT INTO visits (day, count) VALUES (date('now'), ?)
             ON CONFLICT (day) DO UPDATE SET count = count + excluded.count",
        )
        .bind(n_i)
        .execute(pool)
        .await;
        if let Err(e) = done {
            self.pending.fetch_add(n, Ordering::Relaxed);
            return Err(e);
        }
        Ok(())
    }
}

/// Flushes the counter every minute.
pub fn spawn_flush(state: AppState) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_mins(1));
        loop {
            tick.tick().await;
            if let Err(e) = state.counter.flush(&state.pool).await {
                tracing::error!("visitor count flush failed: {e}");
            }
        }
    });
}

/// `GET /api/visitors`: `{ "total": n }`.
///
/// # Errors
///
/// 500 on database errors.
pub async fn api_visitors(
    State(s): State<AppState>,
) -> Result<Json<serde_json::Value>, OwnerError> {
    let stored: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(count), 0) FROM visits")
        .fetch_one(&s.pool)
        .await?;
    let total = u64::try_from(stored).unwrap_or(0) + s.counter.pending();
    Ok(Json(serde_json::json!({ "total": total })))
}
