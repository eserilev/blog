//! Sign-in tests (spec 6.6, 7.4): real passkey ceremonies with a software
//! authenticator, setup tokens, sessions, CSRF, rate limit, revocation.

mod common;

use axum::http::{StatusCode, header};
use common::*;
use logbook_server::auth;
use url::Url;
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};

/// The `logbook_session` value from a `Set-Cookie` header.
fn session_from(r: &Reply) -> Option<String> {
    let v = r.headers.get(header::SET_COOKIE)?.to_str().ok()?;
    let value = v
        .split(';')
        .next()?
        .strip_prefix(&format!("{}=", auth::SESSION_COOKIE))?;
    (!value.is_empty()).then(|| value.to_string())
}

/// Registers a passkey through the HTTP API. Returns the session cookie.
async fn register(
    f: &Fixture,
    key: &mut WebauthnAuthenticator<SoftPasskey>,
    setup_token: Option<&str>,
    session: Option<&str>,
) -> Reply {
    let body = match setup_token {
        Some(t) => serde_json::json!({ "setup_token": t }),
        None => serde_json::json!({}),
    };
    let start = f
        .send(Req::post("/auth/register/start", body).cookie(session))
        .await;
    if start.status != StatusCode::OK {
        return start;
    }
    let start = start.json();
    let options = serde_json::from_value(start["options"].clone()).unwrap();
    let credential = key
        .do_registration(Url::parse(ORIGIN).unwrap(), options)
        .expect("soft passkey registers");
    f.send(
        Req::post(
            "/auth/register/finish",
            serde_json::json!({ "ceremony": start["ceremony"], "credential": credential, "label": "Laptop" }),
        )
        .cookie(session),
    )
    .await
}

async fn login(f: &Fixture, key: &mut WebauthnAuthenticator<SoftPasskey>) -> Reply {
    let start = f
        .send(Req::post("/auth/login/start", serde_json::json!({})))
        .await;
    if start.status != StatusCode::OK {
        return start;
    }
    let start = start.json();
    let options = serde_json::from_value(start["options"].clone()).unwrap();
    let credential = key
        .do_authentication(Url::parse(ORIGIN).unwrap(), options)
        .expect("soft passkey signs");
    f.send(Req::post(
        "/auth/login/finish",
        serde_json::json!({ "ceremony": start["ceremony"], "credential": credential }),
    ))
    .await
}

fn soft_key() -> WebauthnAuthenticator<SoftPasskey> {
    WebauthnAuthenticator::new(SoftPasskey::new(true))
}

async fn me(f: &Fixture, session: Option<&str>) -> bool {
    f.send(Req::get("/api/me").cookie(session)).await.json()["owner"]
        .as_bool()
        .unwrap()
}

#[tokio::test]
async fn setup_link_register_sign_out_sign_in() {
    let f = fixture().await;
    let mut key = soft_key();
    assert!(!me(&f, None).await);

    // No passkey yet: sign-in is refused with a clear message.
    let r = f
        .send(Req::post("/auth/login/start", serde_json::json!({})))
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert!(r.body.contains("setup-link"));

    // Register with a setup token. It signs the owner in.
    let token = auth::create_setup_token(&f.pool).await.unwrap();
    let r = register(&f, &mut key, Some(&token), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let session = session_from(&r).expect("registration sets a session");
    assert!(me(&f, Some(&session)).await);
    let cookie = r.headers[header::SET_COOKIE].to_str().unwrap();
    for attr in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/"] {
        assert!(cookie.contains(attr), "cookie lacks {attr}: {cookie}");
    }

    // The database holds only hashes, never the tokens.
    let raw: Vec<Vec<u8>> = sqlx::query_scalar("SELECT token_hash FROM sessions")
        .fetch_all(&f.pool)
        .await
        .unwrap();
    assert!(raw.iter().all(|h| h.len() == 32 && h != session.as_bytes()));
    let used: Option<String> = sqlx::query_scalar("SELECT used_at FROM setup_tokens")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert!(used.is_some(), "the setup token is marked used");

    // Sign out: the session stops working, even if the cookie is kept.
    let r = f
        .send(Req::post("/auth/logout", serde_json::json!({})).cookie(Some(&session)))
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        r.headers[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    assert!(!me(&f, Some(&session)).await);
    assert_eq!(
        f.send(Req::get("/api/owner/passkeys").cookie(Some(&session)))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );

    // Sign in again with the passkey.
    let r = login(&f, &mut key).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let session = session_from(&r).unwrap();
    let list = f
        .send(Req::get("/api/owner/passkeys").cookie(Some(&session)))
        .await
        .json();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["label"], "Laptop");
    assert!(list[0]["last_used_at"].is_string());
}

#[tokio::test]
async fn a_setup_token_works_once() {
    let f = fixture().await;
    let token = auth::create_setup_token(&f.pool).await.unwrap();
    assert_eq!(
        register(&f, &mut soft_key(), Some(&token), None)
            .await
            .status,
        StatusCode::OK
    );
    let r = register(&f, &mut soft_key(), Some(&token), None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED, "{}", r.body);
}

#[tokio::test]
async fn an_expired_or_unknown_setup_token_is_refused() {
    let f = fixture().await;
    let token = auth::create_setup_token(&f.pool).await.unwrap();
    sqlx::query("UPDATE setup_tokens SET expires_at = '2000-01-01T00:00:00Z'")
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        register(&f, &mut soft_key(), Some(&token), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        register(&f, &mut soft_key(), Some("nope"), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        register(&f, &mut soft_key(), None, None).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn a_token_used_between_start_and_finish_is_refused() {
    let f = fixture().await;
    let token = auth::create_setup_token(&f.pool).await.unwrap();
    let start = f
        .send(Req::post(
            "/auth/register/start",
            serde_json::json!({ "setup_token": token }),
        ))
        .await
        .json();
    // Another tab uses the token first.
    sqlx::query("UPDATE setup_tokens SET used_at = '2026-01-01T00:00:00Z'")
        .execute(&f.pool)
        .await
        .unwrap();
    let options = serde_json::from_value(start["options"].clone()).unwrap();
    let credential = soft_key()
        .do_registration(Url::parse(ORIGIN).unwrap(), options)
        .unwrap();
    let r = f
        .send(Req::post(
            "/auth/register/finish",
            serde_json::json!({ "ceremony": start["ceremony"], "credential": credential }),
        ))
        .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passkeys")
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "no passkey is stored");
}

#[tokio::test]
async fn a_ceremony_works_once() {
    let f = fixture().await;
    let token = auth::create_setup_token(&f.pool).await.unwrap();
    let start = f
        .send(Req::post(
            "/auth/register/start",
            serde_json::json!({ "setup_token": token }),
        ))
        .await
        .json();
    let options = serde_json::from_value(start["options"].clone()).unwrap();
    let credential = soft_key()
        .do_registration(Url::parse(ORIGIN).unwrap(), options)
        .unwrap();
    let body = serde_json::json!({ "ceremony": start["ceremony"], "credential": credential });
    assert_eq!(
        f.send(Req::post("/auth/register/finish", body.clone()))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        f.send(Req::post("/auth/register/finish", body))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn a_second_passkey_needs_a_session_and_the_last_cannot_be_revoked() {
    let f = fixture().await;
    let token = auth::create_setup_token(&f.pool).await.unwrap();
    let session = session_from(&register(&f, &mut soft_key(), Some(&token), None).await).unwrap();

    let list = |s: String| {
        let f = &f;
        async move {
            f.send(Req::get("/api/owner/passkeys").cookie(Some(&s)))
                .await
                .json()
        }
    };
    let only = list(session.clone()).await[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = f
        .send(Req::delete(&format!("/api/owner/passkeys/{only}")).cookie(Some(&session)))
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);

    // Add a second passkey from the session, then revoke the first.
    assert_eq!(
        register(&f, &mut soft_key(), None, Some(&session))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(list(session.clone()).await.as_array().unwrap().len(), 2);
    let r = f
        .send(Req::delete(&format!("/api/owner/passkeys/{only}")).cookie(Some(&session)))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(list(session.clone()).await.as_array().unwrap().len(), 1);

    // Revoke as a guest: refused.
    let other = list(session).await[0]["id"].as_str().unwrap().to_string();
    assert_eq!(
        f.send(Req::delete(&format!("/api/owner/passkeys/{other}")))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn expired_sessions_are_refused() {
    let f = fixture().await;
    let expired = f.session(-1).await;
    assert!(!me(&f, Some(&expired)).await);
    assert_eq!(
        f.send(Req::get("/api/owner/passkeys").cookie(Some(&expired)))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let fresh = f.session(30).await;
    assert!(me(&f, Some(&fresh)).await);
}

#[tokio::test]
async fn writes_need_the_same_origin_and_json() {
    let f = fixture().await;
    let body = serde_json::json!({});
    let mut r = Req::post("/auth/login/start", body.clone());
    r.origin = None;
    assert_eq!(
        f.send(r).await.status,
        StatusCode::FORBIDDEN,
        "missing Origin"
    );
    let mut r = Req::post("/auth/login/start", body.clone());
    r.origin = Some("https://evil.test");
    assert_eq!(
        f.send(r).await.status,
        StatusCode::FORBIDDEN,
        "wrong Origin"
    );
    let mut r = Req::post("/auth/login/start", body.clone());
    r.content_type = Some("text/plain");
    assert_eq!(f.send(r).await.status, StatusCode::FORBIDDEN, "not JSON");
    let mut r = Req::delete("/api/owner/passkeys/x");
    r.origin = Some("https://logbook.test.evil.test");
    assert_eq!(
        f.send(r).await.status,
        StatusCode::FORBIDDEN,
        "lookalike Origin"
    );
    // Same origin and JSON: allowed through to the handler (409: no passkey yet).
    assert_eq!(
        f.send(Req::post("/auth/login/start", body)).await.status,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn sign_in_routes_are_rate_limited() {
    let f = fixture_with(false, 5).await;
    for _ in 0..5 {
        assert_eq!(
            f.send(Req::post("/auth/login/start", serde_json::json!({})))
                .await
                .status,
            StatusCode::CONFLICT
        );
    }
    let r = f
        .send(Req::post("/auth/login/start", serde_json::json!({})))
        .await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    // Public reads have no limit.
    assert_eq!(f.get("/api/posts").await.status, StatusCode::OK);
}

#[tokio::test]
async fn a_signature_from_an_unknown_key_is_refused() {
    let f = fixture().await;
    let token = auth::create_setup_token(&f.pool).await.unwrap();
    assert_eq!(
        register(&f, &mut soft_key(), Some(&token), None)
            .await
            .status,
        StatusCode::OK
    );
    // A different authenticator, never registered, cannot answer the challenge.
    let start = f
        .send(Req::post("/auth/login/start", serde_json::json!({})))
        .await
        .json();
    let options = serde_json::from_value(start["options"].clone()).unwrap();
    let result = soft_key().do_authentication(Url::parse(ORIGIN).unwrap(), options);
    if let Ok(credential) = result {
        let r = f
            .send(Req::post(
                "/auth/login/finish",
                serde_json::json!({ "ceremony": start["ceremony"], "credential": credential }),
            ))
            .await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
        assert!(session_from(&r).is_none());
    }
}
