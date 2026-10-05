//! Request guards (spec 6.6): CSRF checks on writes, rate limit on `/auth/*`,
//! and the client IP behind a trusted proxy.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use ipnet::IpNet;

use crate::AppState;

fn forbidden(msg: &'static str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({ "error": msg })),
    )
        .into_response()
}

/// CSRF check for every request that is not `GET` or `HEAD`:
/// - the `Origin` header must equal the site origin;
/// - a request with a body must be `application/json`.
///
/// With the `SameSite=Strict` session cookie, this needs no token (spec 6.6).
pub async fn csrf(State(s): State<AppState>, req: Request, next: Next) -> Response {
    if matches!(*req.method(), Method::GET | Method::HEAD) {
        return next.run(req).await;
    }
    let origin_ok = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        == Some(&*s.origin);
    if !origin_ok {
        return forbidden("cross-origin write refused");
    }
    let has_body = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v != "0")
        || req.headers().contains_key(header::TRANSFER_ENCODING);
    let json = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|t| t.trim().eq_ignore_ascii_case("application/json"))
        });
    let multipart_upload = req.uri().path() == "/api/owner/uploads"
        && req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.to_ascii_lowercase().starts_with("multipart/form-data"));
    if has_body && !json && !multipart_upload {
        return forbidden("writes must be application/json");
    }
    next.run(req).await
}

/// The client IP. `X-Forwarded-For` counts only when the direct peer is a trusted
/// proxy; then the result is the rightmost address that is not a trusted proxy.
#[must_use]
pub fn client_ip(headers: &HeaderMap, peer: Option<IpAddr>, trusted: &[IpNet]) -> Option<IpAddr> {
    let peer = peer?;
    if !trusted.iter().any(|n| n.contains(&peer)) {
        return Some(peer);
    }
    let forwarded: Vec<IpAddr> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    forwarded
        .into_iter()
        .rev()
        .find(|ip| !trusted.iter().any(|n| n.contains(ip)))
        .or(Some(peer))
}

/// A fixed-window rate limiter keyed by client IP.
pub struct RateLimiter {
    limit: u32,
    window: Duration,
    hits: Mutex<HashMap<Option<IpAddr>, (Instant, u32)>>,
}

impl RateLimiter {
    /// `limit` requests per `window` for each client IP.
    #[must_use]
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            limit,
            window,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Counts one request. False if the client is over the limit.
    pub fn allow(&self, ip: Option<IpAddr>) -> bool {
        let mut hits = self
            .hits
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Instant::now();
        if hits.len() > 10_000 {
            hits.retain(|_, (start, _)| now.duration_since(*start) < self.window);
        }
        let entry = hits.entry(ip).or_insert((now, 0));
        if now.duration_since(entry.0) >= self.window {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= self.limit
    }
}

/// Rate limit middleware for the `/auth/*` routes.
pub async fn rate_limit(State(s): State<AppState>, req: Request, next: Next) -> Response {
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip());
    let ip = client_ip(req.headers(), peer, &s.trusted_proxies);
    if !s.auth_limiter.allow(ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": "too many sign-in attempts; wait a minute" })),
        )
            .into_response();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(xff: &str) -> HeaderMap {
        let mut m = HeaderMap::new();
        m.insert("x-forwarded-for", xff.parse().unwrap());
        m
    }

    #[test]
    fn untrusted_peers_cannot_spoof_the_ip() {
        let trusted: Vec<IpNet> = vec!["172.18.0.0/16".parse().unwrap()];
        let peer = "203.0.113.9".parse().ok();
        assert_eq!(client_ip(&h("1.2.3.4"), peer, &trusted), peer);
    }

    #[test]
    fn trusted_proxies_give_the_rightmost_untrusted_address() {
        let trusted: Vec<IpNet> = vec!["172.18.0.0/16".parse().unwrap()];
        let peer = "172.18.0.2".parse().ok();
        assert_eq!(
            client_ip(&h("6.6.6.6, 198.51.100.7"), peer, &trusted),
            "198.51.100.7".parse().ok()
        );
        assert_eq!(
            client_ip(&h("198.51.100.7, 172.18.0.5"), peer, &trusted),
            "198.51.100.7".parse().ok()
        );
        assert_eq!(client_ip(&HeaderMap::new(), peer, &trusted), peer);
    }

    #[test]
    fn the_limiter_blocks_after_the_limit_per_ip() {
        let l = RateLimiter::new(3, Duration::from_mins(1));
        let a = "198.51.100.1".parse().ok();
        let b = "198.51.100.2".parse().ok();
        assert!(l.allow(a) && l.allow(a) && l.allow(a));
        assert!(!l.allow(a));
        assert!(l.allow(b));
    }
}
