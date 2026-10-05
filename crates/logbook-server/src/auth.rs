//! Owner sign-in (spec 6.6): passkeys, sessions, setup tokens.
//!
//! - One owner. No sign-up. The first passkey needs a setup token from the CLI.
//! - The database stores only SHA-256 hashes of session and setup tokens, so a
//!   leaked backup gives no way to sign in.
//! - Passkey ceremonies (start → finish) are kept in memory for 5 minutes.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{FromRequestParts, Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use logbook_core::{Access, Decision, SessionState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use time::{
    OffsetDateTime,
    format_description::{BorrowedFormatItem, well_known::Rfc3339},
    macros::format_description,
};
use webauthn_rs::prelude::{
    CredentialID, Passkey, PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential,
    RegisterPublicKeyCredential, Url, Uuid, Webauthn, WebauthnBuilder,
};

use crate::AppState;

/// Session cookie name.
pub const SESSION_COOKIE: &str = "logbook_session";
/// Session lifetime.
pub const SESSION_DAYS: i64 = 30;
/// Setup token lifetime, for messages. [`logbook_core::SETUP_TOKEN_SECONDS`] sets it.
pub const SETUP_MINUTES: i64 = logbook_core::SETUP_TOKEN_SECONDS / 60;
/// The time format of the database: `strftime('%Y-%m-%dT%H:%M:%SZ')`.
const DB_TIME: &[BorrowedFormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");
/// How long a passkey ceremony stays open.
const CEREMONY_TTL: Duration = Duration::from_mins(5);
/// The most open ceremonies at one time.
const CEREMONY_MAX: usize = 64;
/// The owner's fixed WebAuthn user handle.
const OWNER_ID: Uuid = Uuid::from_u128(0x6c6f_6762_6f6f_6b00_0000_0000_0000_0001);

enum Ceremony {
    Register {
        state: PasskeyRegistration,
        setup_token_hash: Option<[u8; 32]>,
    },
    Login(PasskeyAuthentication),
}

/// Sign-in state shared by all requests.
pub struct Auth {
    webauthn: Webauthn,
    ceremonies: Mutex<HashMap<String, (Instant, Ceremony)>>,
}

impl Auth {
    /// Builds the WebAuthn relying party from the public origin. The RP ID is the
    /// origin's host (spec 6.6).
    ///
    /// # Errors
    ///
    /// Fails if the origin is not a valid URL or the host is not a valid RP ID.
    pub fn new(origin: &str) -> Result<Arc<Self>, String> {
        let url = Url::parse(origin).map_err(|e| format!("bad origin {origin:?}: {e}"))?;
        let rp_id = url
            .host_str()
            .ok_or_else(|| format!("origin {origin:?} has no host"))?
            .to_string();
        let webauthn = WebauthnBuilder::new(&rp_id, &url)
            .and_then(|b| b.rp_name("Eitan's Logbook").allow_subdomains(true).build())
            .map_err(|e| format!("cannot set up WebAuthn for {rp_id}: {e}"))?;
        Ok(Arc::new(Self {
            webauthn,
            ceremonies: Mutex::new(HashMap::new()),
        }))
    }

    fn put(&self, c: Ceremony) -> String {
        let id = random_token();
        let mut map = self
            .ceremonies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        map.retain(|_, (at, _)| at.elapsed() < CEREMONY_TTL);
        if map.len() >= CEREMONY_MAX
            && let Some(oldest) = map
                .iter()
                .min_by_key(|(_, (at, _))| *at)
                .map(|(k, _)| k.clone())
        {
            map.remove(&oldest);
        }
        map.insert(id.clone(), (Instant::now(), c));
        id
    }

    fn take(&self, id: &str) -> Option<Ceremony> {
        let mut map = self
            .ceremonies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        map.remove(id)
            .filter(|(at, _)| at.elapsed() < CEREMONY_TTL)
            .map(|(_, c)| c)
    }
}

/// 32 random bytes, base64url.
///
/// # Panics
///
/// Panics if the OS random source fails, which leaves no safe way to continue.
#[must_use]
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS random source");
    B64.encode(bytes)
}

/// SHA-256 of a token.
#[must_use]
pub fn hash(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// Reads one cookie from the request headers.
#[must_use]
pub fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v)
}

fn session_cookie(token: &str) -> HeaderValue {
    let max_age = SESSION_DAYS * 86_400;
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/; Max-Age={max_age}; HttpOnly; Secure; SameSite=Strict"
    ))
    .expect("token is base64url")
}

fn clear_cookie() -> HeaderValue {
    HeaderValue::from_static(
        "logbook_session=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Strict",
    )
}

/// An error from an auth or owner route. Details go to the log only.
#[derive(Debug)]
pub enum AuthError {
    Unauthorized,
    BadRequest(&'static str),
    Conflict(&'static str),
    Internal(String),
}

impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "sign in first"),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            Self::Conflict(m) => (StatusCode::CONFLICT, m),
            Self::Internal(e) => {
                tracing::error!("auth error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error")
            }
        };
        (
            status,
            [(header::CACHE_CONTROL, "no-store")],
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response()
    }
}

/// Proof that the request has a valid owner session. Owner handlers take this as an
/// argument, so a request without a session gets 401 before the handler runs.
#[derive(Debug, Clone, Copy)]
pub struct Owner;

/// The current time in Unix seconds.
fn now_unix() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

/// A time from the database (RFC 3339) in Unix seconds. `None` if it does not parse.
fn unix_seconds(t: &str) -> Option<i64> {
    OffsetDateTime::parse(t, &Rfc3339)
        .ok()
        .map(OffsetDateTime::unix_timestamp)
}

/// Finds the session of the request in the `sessions` table.
async fn session_state(
    pool: &SqlitePool,
    headers: &HeaderMap,
) -> Result<SessionState, sqlx::Error> {
    let Some(token) = cookie(headers, SESSION_COOKIE) else {
        return Ok(SessionState::NoCookie);
    };
    let expires_at: Option<String> =
        sqlx::query_scalar("SELECT expires_at FROM sessions WHERE token_hash = ?")
            .bind(hash(token).to_vec())
            .fetch_optional(pool)
            .await?;
    Ok(match expires_at {
        None => SessionState::Unknown,
        // A time that does not parse counts as expired.
        Some(t) => match unix_seconds(&t) {
            Some(exp) if logbook_core::session_valid(now_unix(), exp) => SessionState::Valid,
            _ => SessionState::Expired,
        },
    })
}

/// True if the request has a valid owner session. [`logbook_core::authorize`] makes
/// the decision (theorem T16).
async fn is_owner(pool: &SqlitePool, headers: &HeaderMap) -> Result<bool, sqlx::Error> {
    let state = session_state(pool, headers).await?;
    Ok(logbook_core::authorize(Access::Owner, state) == Decision::Allow)
}

impl FromRequestParts<AppState> for Owner {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, s: &AppState) -> Result<Self, Self::Rejection> {
        let state = session_state(&s.pool, &parts.headers).await?;
        match logbook_core::authorize(Access::Owner, state) {
            Decision::Allow => Ok(Self),
            Decision::Unauthorized => Err(AuthError::Unauthorized),
        }
    }
}

async fn create_session(pool: &SqlitePool) -> Result<HeaderValue, sqlx::Error> {
    let token = random_token();
    sqlx::query(
        "INSERT INTO sessions (token_hash, created_at, expires_at)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now', ?))",
    )
    .bind(hash(&token).to_vec())
    .bind(format!("+{SESSION_DAYS} days"))
    .execute(pool)
    .await?;
    Ok(session_cookie(&token))
}

/// Creates a setup token and returns it. Only the hash is stored (spec 6.6).
/// [`logbook_core::setup_token_expiry`] sets the expiry time: 15 minutes (theorem T17).
///
/// # Errors
///
/// Database errors.
pub async fn create_setup_token(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let token = random_token();
    let expires_at =
        OffsetDateTime::from_unix_timestamp(logbook_core::setup_token_expiry(now_unix()))
            .ok()
            .and_then(|t| t.format(DB_TIME).ok())
            .ok_or_else(|| sqlx::Error::Protocol("setup token expiry out of range".into()))?;
    sqlx::query("INSERT INTO setup_tokens (token_hash, expires_at) VALUES (?, ?)")
        .bind(hash(&token).to_vec())
        .bind(expires_at)
        .execute(pool)
        .await?;
    Ok(token)
}

/// Checks a setup token. [`logbook_core::setup_token_usable`] makes the decision
/// (theorem T17). Single use relies on the atomic `UPDATE` in [`register_finish`].
async fn setup_token_usable(pool: &SqlitePool, token_hash: &[u8; 32]) -> Result<bool, sqlx::Error> {
    let row: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT expires_at, used_at FROM setup_tokens WHERE token_hash = ?")
            .bind(token_hash.to_vec())
            .fetch_optional(pool)
            .await?;
    Ok(row.is_some_and(|(expires_at, used_at)| {
        unix_seconds(&expires_at)
            .is_some_and(|exp| logbook_core::setup_token_usable(now_unix(), exp, used_at.is_some()))
    }))
}

async fn load_passkeys(pool: &SqlitePool) -> Result<Vec<Passkey>, AuthError> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT passkey FROM passkeys ORDER BY created_at")
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(|r| {
            serde_json::from_str(r)
                .map_err(|e| AuthError::Internal(format!("stored passkey does not parse: {e}")))
        })
        .collect()
}

/// `GET /api/me`. Never 401. `Cache-Control: no-store`, because it depends on the session.
pub async fn me(State(s): State<AppState>, headers: HeaderMap) -> Response {
    let owner = is_owner(&s.pool, &headers).await.unwrap_or(false);
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(serde_json::json!({ "owner": owner })),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct RegisterStart {
    #[serde(default)]
    pub setup_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CeremonyStart<T> {
    pub ceremony: String,
    pub options: T,
}

/// `POST /auth/register/start`. Needs a usable setup token or an owner session.
///
/// # Errors
///
/// 401 without a token or session. 500 on internal errors.
pub async fn register_start(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegisterStart>,
) -> Result<Response, AuthError> {
    let setup_token_hash = match body.setup_token.as_deref() {
        Some(t) => {
            let h = hash(t);
            if !setup_token_usable(&s.pool, &h).await? {
                return Err(AuthError::Unauthorized);
            }
            Some(h)
        }
        None if is_owner(&s.pool, &headers).await? => None,
        None => return Err(AuthError::Unauthorized),
    };
    let exclude: Vec<CredentialID> = load_passkeys(&s.pool)
        .await?
        .iter()
        .map(|p| p.cred_id().clone())
        .collect();
    let (options, state) = s
        .auth
        .webauthn
        .start_passkey_registration(OWNER_ID, "eitan", "Eitan", Some(exclude))
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    let ceremony = s.auth.put(Ceremony::Register {
        state,
        setup_token_hash,
    });
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(CeremonyStart { ceremony, options }),
    )
        .into_response())
}

#[derive(Debug, Deserialize)]
pub struct RegisterFinish {
    pub ceremony: String,
    pub credential: RegisterPublicKeyCredential,
    #[serde(default)]
    pub label: Option<String>,
}

/// `POST /auth/register/finish`. Stores the passkey, marks the setup token used in the
/// same transaction, and signs the owner in.
///
/// # Errors
///
/// 401 for an unknown ceremony, a used or expired token, or a failed check.
pub async fn register_finish(
    State(s): State<AppState>,
    Json(body): Json<RegisterFinish>,
) -> Result<Response, AuthError> {
    let Some(Ceremony::Register {
        state,
        setup_token_hash,
    }) = s.auth.take(&body.ceremony)
    else {
        return Err(AuthError::Unauthorized);
    };
    let passkey = s
        .auth
        .webauthn
        .finish_passkey_registration(&body.credential, &state)
        .map_err(|e| {
            tracing::warn!("passkey registration failed: {e}");
            AuthError::Unauthorized
        })?;
    let label: String = body
        .label
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.is_control())
        .take(60)
        .collect();
    let label = if label.trim().is_empty() {
        "Passkey".to_string()
    } else {
        label.trim().to_string()
    };
    let json = serde_json::to_string(&passkey).map_err(|e| AuthError::Internal(e.to_string()))?;

    let mut tx = s.pool.begin().await?;
    if let Some(h) = setup_token_hash {
        let used = sqlx::query(
            "UPDATE setup_tokens SET used_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE token_hash = ? AND used_at IS NULL AND expires_at > strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
        )
        .bind(h.to_vec())
        .execute(&mut *tx)
        .await?;
        if used.rows_affected() != 1 {
            return Err(AuthError::Unauthorized);
        }
    }
    sqlx::query("INSERT INTO passkeys (id, passkey, label, created_at) VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))")
        .bind(passkey.cred_id().as_ref().to_vec())
        .bind(json)
        .bind(label)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let cookie = create_session(&s.pool).await?;
    Ok((
        [
            (header::SET_COOKIE, cookie),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        Json(serde_json::json!({ "ok": true })),
    )
        .into_response())
}

/// `POST /auth/login/start`.
///
/// # Errors
///
/// 409 if no passkey exists yet.
pub async fn login_start(State(s): State<AppState>) -> Result<Response, AuthError> {
    let passkeys = load_passkeys(&s.pool).await?;
    if passkeys.is_empty() {
        return Err(AuthError::Conflict(
            "no passkey is registered yet; run `logbook setup-link` on the server",
        ));
    }
    let (options, state) = s
        .auth
        .webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(|e| AuthError::Internal(e.to_string()))?;
    let ceremony = s.auth.put(Ceremony::Login(state));
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(CeremonyStart { ceremony, options }),
    )
        .into_response())
}

#[derive(Debug, Deserialize)]
pub struct LoginFinish {
    pub ceremony: String,
    pub credential: PublicKeyCredential,
}

/// `POST /auth/login/finish`. Updates the passkey counter and signs the owner in.
///
/// # Errors
///
/// 401 for an unknown ceremony or a failed check.
pub async fn login_finish(
    State(s): State<AppState>,
    Json(body): Json<LoginFinish>,
) -> Result<Response, AuthError> {
    let Some(Ceremony::Login(state)) = s.auth.take(&body.ceremony) else {
        return Err(AuthError::Unauthorized);
    };
    let result = s
        .auth
        .webauthn
        .finish_passkey_authentication(&body.credential, &state)
        .map_err(|e| {
            tracing::warn!("passkey sign-in failed: {e}");
            AuthError::Unauthorized
        })?;
    for mut pk in load_passkeys(&s.pool).await? {
        if pk.cred_id() == result.cred_id() {
            pk.update_credential(&result);
            let json =
                serde_json::to_string(&pk).map_err(|e| AuthError::Internal(e.to_string()))?;
            sqlx::query("UPDATE passkeys SET passkey = ?, last_used_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?")
                .bind(json)
                .bind(pk.cred_id().as_ref().to_vec())
                .execute(&s.pool)
                .await?;
        }
    }
    let cookie = create_session(&s.pool).await?;
    Ok((
        [
            (header::SET_COOKIE, cookie),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        Json(serde_json::json!({ "ok": true })),
    )
        .into_response())
}

/// `POST /auth/logout`. Deletes the session row and clears the cookie.
///
/// # Errors
///
/// 500 on database errors.
pub async fn logout(State(s): State<AppState>, headers: HeaderMap) -> Result<Response, AuthError> {
    if let Some(token) = cookie(&headers, SESSION_COOKIE) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(hash(token).to_vec())
            .execute(&s.pool)
            .await?;
    }
    Ok((
        [
            (header::SET_COOKIE, clear_cookie()),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        Json(serde_json::json!({ "ok": true })),
    )
        .into_response())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PasskeyInfo {
    #[serde(skip)]
    raw_id: Vec<u8>,
    #[sqlx(skip)]
    pub id: String,
    pub label: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// `GET /api/owner/passkeys`.
///
/// # Errors
///
/// 401 without a session. 500 on database errors.
pub async fn list_passkeys(
    _: Owner,
    State(s): State<AppState>,
) -> Result<Json<Vec<PasskeyInfo>>, AuthError> {
    let mut rows: Vec<PasskeyInfo> = sqlx::query_as(
        "SELECT id AS raw_id, label, created_at, last_used_at FROM passkeys ORDER BY created_at",
    )
    .fetch_all(&s.pool)
    .await?;
    for r in &mut rows {
        r.id = B64.encode(&r.raw_id);
    }
    Ok(Json(rows))
}

/// `DELETE /api/owner/passkeys/{id}`. Refused for the last passkey (spec 6.6).
///
/// # Errors
///
/// 401 without a session, 400 for a bad id, 409 for the last passkey.
pub async fn delete_passkey(
    _: Owner,
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AuthError> {
    let raw = B64
        .decode(id.as_bytes())
        .map_err(|_| AuthError::BadRequest("bad passkey id"))?;
    let mut tx = s.pool.begin().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passkeys")
        .fetch_one(&mut *tx)
        .await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM passkeys WHERE id = ?)")
        .bind(&raw)
        .fetch_one(&mut *tx)
        .await?;
    if !exists {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    if count <= 1 {
        return Err(AuthError::Conflict(
            "cannot revoke the last passkey; use `logbook setup-link` to add another first",
        ));
    }
    sqlx::query("DELETE FROM passkeys WHERE id = ?")
        .bind(&raw)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(serde_json::json!({ "ok": true })),
    )
        .into_response())
}
