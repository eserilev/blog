//! Security headers on every response (spec 6.9).

use axum::{
    Router,
    http::{HeaderName, HeaderValue, header},
};
use tower_http::set_header::SetResponseHeaderLayer;

/// The Content Security Policy. No inline scripts or styles. WASM is allowed for the
/// editor preview. One third-party origin, for frames only: the YouTube player of a
/// video embed, after the reader clicks play (spec 4.10).
pub const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; \
style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; \
frame-src https://www.youtube-nocookie.com; \
object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'";

/// Every header that [`apply`] sets, as `(name, value)`.
pub const SECURITY_HEADERS: &[(&str, &str)] = &[
    ("content-security-policy", CSP),
    (
        "strict-transport-security",
        "max-age=63072000; includeSubDomains",
    ),
    ("x-content-type-options", "nosniff"),
    ("referrer-policy", "strict-origin-when-cross-origin"),
    (
        "permissions-policy",
        "camera=(), microphone=(), geolocation=()",
    ),
];

/// Adds the security headers to every response of `router`, 404s included.
/// A header that a handler already set is replaced.
pub fn apply<S: Clone + Send + Sync + 'static>(mut router: Router<S>) -> Router<S> {
    for (name, value) in SECURITY_HEADERS {
        router = router.layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        ));
    }
    router.layer(SetResponseHeaderLayer::if_not_present(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    ))
}
