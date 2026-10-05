//! The visitor counter (spec 4.8): unique visitors per day, no cookies, no stored IPs.
//!
//! A visitor is a hash of a daily random salt, the client IP, and the user agent.
//! The hashes live in memory for the current UTC day only. At the next day the
//! server drops them and makes a new salt, so no day links to another.
//! Daily totals are flushed to the `visits` table once per minute.

use std::{
    collections::HashSet,
    net::IpAddr,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{Json, extract::State, http::HeaderMap};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::{AppState, posts::OwnerError};

/// Most visitor hashes kept for one day. Above this, every new hash counts, so a
/// flood of fake visitors cannot use unbounded memory.
pub const SEEN_MAX: usize = 100_000;

/// The visitors of the current UTC day.
#[derive(Debug, Default)]
struct Day {
    /// Days since 1970-01-01 (UTC).
    number: u64,
    salt: [u8; 16],
    seen: HashSet<[u8; 16]>,
}

/// Pending visitors, not yet in the database.
#[derive(Debug, Default)]
pub struct Counter {
    pending: AtomicU64,
    day: Mutex<Day>,
}

fn today() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400)
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
    /// Counts a page load as a visitor, if it looks like a person and is the first
    /// load of this IP and user agent today.
    pub fn hit(&self, headers: &HeaderMap, ip: Option<IpAddr>) {
        if !is_person(headers) {
            return;
        }
        let ua = headers
            .get(axum::http::header::USER_AGENT)
            .map_or(&[][..], axum::http::HeaderValue::as_bytes);
        if self.first_visit(today(), ip, ua) {
            self.pending.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// True the first time this IP and user agent appear on day `day`.
    fn first_visit(&self, day: u64, ip: Option<IpAddr>, ua: &[u8]) -> bool {
        let Ok(mut d) = self.day.lock() else {
            return true;
        };
        if d.number != day || d.salt == [0; 16] {
            d.number = day;
            d.seen.clear();
            if getrandom::fill(&mut d.salt).is_err() {
                d.salt = [1; 16];
            }
        }
        let mut h = Sha256::new();
        h.update(d.salt);
        match ip {
            Some(IpAddr::V4(a)) => h.update(a.octets()),
            Some(IpAddr::V6(a)) => h.update(a.octets()),
            None => h.update([0u8]),
        }
        h.update(ua);
        let mut key = [0u8; 16];
        key.copy_from_slice(&h.finalize()[..16]);
        if d.seen.len() >= SEEN_MAX {
            return !d.seen.contains(&key);
        }
        d.seen.insert(key)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    const A: Option<IpAddr> = Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)));
    const B: Option<IpAddr> = Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 9)));

    #[test]
    fn one_visit_per_ip_and_browser_per_day() {
        let c = Counter::default();
        assert!(c.first_visit(100, A, b"firefox"));
        assert!(!c.first_visit(100, A, b"firefox"), "a refresh");
        assert!(
            c.first_visit(100, A, b"safari"),
            "another browser on the same IP"
        );
        assert!(c.first_visit(100, B, b"firefox"), "another IP");
        assert!(
            c.first_visit(101, A, b"firefox"),
            "the next day counts again"
        );
        assert!(!c.first_visit(101, A, b"firefox"));
    }

    #[test]
    fn a_new_day_drops_the_hashes_and_the_salt() {
        let c = Counter::default();
        c.first_visit(100, A, b"x");
        let salt = c.day.lock().unwrap().salt;
        c.first_visit(101, B, b"y");
        let d = c.day.lock().unwrap();
        assert_eq!(d.seen.len(), 1);
        assert_ne!(d.salt, salt);
    }
}
