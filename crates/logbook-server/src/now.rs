//! The Now box (spec 4.4): a short markdown list on the home page.

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{AppState, auth::Owner, posts::OwnerError};

/// Most characters in the Now box.
pub const NOW_MAX: usize = 5_000;

/// The Now box for guests.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PublicNow {
    pub body_html: String,
    pub updated_at: Option<String>,
}

/// The Now box for the editor.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OwnerNow {
    pub body_md: String,
    pub body_html: String,
    pub updated_at: Option<String>,
}

/// `GET /api/now`. Empty until the owner writes it.
///
/// # Errors
///
/// 500 on database errors.
pub async fn api_now(State(s): State<AppState>) -> Result<Json<PublicNow>, OwnerError> {
    let row = sqlx::query_as("SELECT body_html, updated_at FROM now_box WHERE id = 1")
        .fetch_optional(&s.pool)
        .await?;
    Ok(Json(row.unwrap_or(PublicNow {
        body_html: String::new(),
        updated_at: None,
    })))
}

/// `GET /api/owner/now`.
///
/// # Errors
///
/// 401 without a session. 500 on database errors.
pub async fn owner_now(_: Owner, State(s): State<AppState>) -> Result<Json<OwnerNow>, OwnerError> {
    let row = sqlx::query_as("SELECT body_md, body_html, updated_at FROM now_box WHERE id = 1")
        .fetch_optional(&s.pool)
        .await?;
    Ok(Json(row.unwrap_or(OwnerNow {
        body_md: String::new(),
        body_html: String::new(),
        updated_at: None,
    })))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NowInput {
    pub body_md: String,
}

/// `PUT /api/owner/now`. One owner, so the last save wins.
///
/// # Errors
///
/// 401 without a session, 400 if too long.
pub async fn save_now(
    _: Owner,
    State(s): State<AppState>,
    Json(input): Json<NowInput>,
) -> Result<Json<OwnerNow>, OwnerError> {
    if input.body_md.chars().count() > NOW_MAX {
        return Err(OwnerError::BadRequest("the Now box is too long"));
    }
    let html = logbook_render::render(&input.body_md);
    sqlx::query(
        "INSERT INTO now_box (id, body_md, body_html, updated_at) VALUES (1, ?, ?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
         ON CONFLICT (id) DO UPDATE SET body_md = excluded.body_md, body_html = excluded.body_html, updated_at = excluded.updated_at",
    )
    .bind(&input.body_md)
    .bind(&html)
    .execute(&s.pool)
    .await?;
    s.content_changed();
    owner_now(Owner, State(s)).await
}
