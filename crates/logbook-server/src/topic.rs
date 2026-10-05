//! Topics (spec 4.2). The owner adds, renames, orders, and deletes them.
//!
//! - The slug is the URL (`/topics/<slug>`). It comes from the name when the topic
//!   is made and never changes, so old links keep working.
//! - A topic with posts cannot be deleted. The last topic cannot be deleted.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{AppState, auth::Owner, posts::OwnerError};

/// Most characters in a topic name.
pub const NAME_MAX: usize = 40;
/// Most topics.
pub const TOPICS_MAX: usize = 30;

/// A topic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Topic {
    pub slug: String,
    pub name: String,
}

/// Every topic, in sidebar order.
///
/// # Errors
///
/// Database errors.
pub async fn all(pool: &SqlitePool) -> Result<Vec<Topic>, sqlx::Error> {
    sqlx::query_as("SELECT slug, name FROM topics ORDER BY position, slug")
        .fetch_all(pool)
        .await
}

/// Slug → name, for post lists.
///
/// # Errors
///
/// Database errors.
pub async fn names(pool: &SqlitePool) -> Result<HashMap<String, String>, sqlx::Error> {
    Ok(all(pool)
        .await?
        .into_iter()
        .map(|t| (t.slug, t.name))
        .collect())
}

/// The topic with this slug.
///
/// # Errors
///
/// Database errors.
pub async fn get(pool: &SqlitePool, slug: &str) -> Result<Option<Topic>, sqlx::Error> {
    sqlx::query_as("SELECT slug, name FROM topics WHERE slug = ?")
        .bind(slug)
        .fetch_optional(pool)
        .await
}

/// The first topic, for a new post.
///
/// # Errors
///
/// Database errors.
pub async fn first(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT slug FROM topics ORDER BY position, slug LIMIT 1")
        .fetch_optional(pool)
        .await
}

/// A trimmed name: not empty, at most [`NAME_MAX`] characters, no control characters.
///
/// # Errors
///
/// A message for the rule that fails.
pub fn clean_name(name: &str) -> Result<String, &'static str> {
    let name = name.trim();
    if name.is_empty() {
        return Err("the topic name is empty");
    }
    if name.chars().count() > NAME_MAX || name.chars().any(char::is_control) {
        return Err("the topic name is too long or has control characters");
    }
    Ok(name.to_string())
}

/// The slug for a new topic name. A name with no ASCII letter or digit has none.
#[must_use]
pub fn slug_for(name: &str) -> Option<String> {
    if !name.bytes().any(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    String::from_utf8(logbook_core::make_slug(name.as_bytes(), 0)).ok()
}

/// `GET /api/topics`.
///
/// # Errors
///
/// 500 on database errors.
pub async fn api_topics(State(s): State<AppState>) -> Result<Json<Vec<Topic>>, OwnerError> {
    Ok(Json(all(&s.pool).await?))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewTopic {
    pub name: String,
}

/// `POST /api/owner/topics`: a new topic at the end of the list.
///
/// # Errors
///
/// 401, 400 (bad name, too many topics), 409 (the slug exists).
pub async fn create(
    _: Owner,
    State(s): State<AppState>,
    Json(input): Json<NewTopic>,
) -> Result<Response, OwnerError> {
    let name = clean_name(&input.name).map_err(OwnerError::BadRequest)?;
    let slug = slug_for(&name).ok_or(OwnerError::BadRequest(
        "the topic name needs a letter or a digit (a-z, 0-9)",
    ))?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM topics")
        .fetch_one(&s.pool)
        .await?;
    if usize::try_from(count).unwrap_or(usize::MAX) >= TOPICS_MAX {
        return Err(OwnerError::BadRequest("too many topics"));
    }
    let done = sqlx::query(
        "INSERT INTO topics (slug, name, position)
         VALUES (?, ?, (SELECT COALESCE(MAX(position), 0) + 1 FROM topics))
         ON CONFLICT (slug) DO NOTHING",
    )
    .bind(&slug)
    .bind(&name)
    .execute(&s.pool)
    .await?;
    if done.rows_affected() == 0 {
        return Err(OwnerError::Taken("a topic with this address exists"));
    }
    s.content_changed();
    Ok((StatusCode::CREATED, Json(Topic { slug, name })).into_response())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopicList {
    pub topics: Vec<Topic>,
}

/// `PUT /api/owner/topics`: new names and a new order. The list must have every
/// topic once, and no other topic. Slugs never change here.
///
/// # Errors
///
/// 401, 400 (a bad name, or not the same set of topics).
pub async fn save(
    _: Owner,
    State(s): State<AppState>,
    Json(input): Json<TopicList>,
) -> Result<Json<Vec<Topic>>, OwnerError> {
    let mut tx = s.pool.begin().await?;
    let mut have: Vec<String> = sqlx::query_scalar("SELECT slug FROM topics")
        .fetch_all(&mut *tx)
        .await?;
    let mut sent: Vec<String> = input.topics.iter().map(|t| t.slug.clone()).collect();
    have.sort();
    sent.sort();
    if have != sent {
        return Err(OwnerError::BadRequest(
            "send every topic once; add and delete topics one at a time",
        ));
    }
    for (i, t) in input.topics.iter().enumerate() {
        let name = clean_name(&t.name).map_err(OwnerError::BadRequest)?;
        sqlx::query("UPDATE topics SET name = ?, position = ? WHERE slug = ?")
            .bind(name)
            .bind(i64::try_from(i).unwrap_or(i64::MAX))
            .bind(&t.slug)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    s.content_changed();
    Ok(Json(all(&s.pool).await?))
}

/// `DELETE /api/owner/topics/{topic}`.
///
/// # Errors
///
/// 401, 404, 409 (the topic has posts, or it is the last topic).
pub async fn delete(
    _: Owner,
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<Topic>>, OwnerError> {
    let mut tx = s.pool.begin().await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM topics WHERE slug = ?)")
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await?;
    if !exists {
        return Err(OwnerError::NotFound);
    }
    let posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE topic = ?")
        .bind(&slug)
        .fetch_one(&mut *tx)
        .await?;
    if posts > 0 {
        return Err(OwnerError::Taken(
            "this topic has posts; move them to another topic first",
        ));
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM topics")
        .fetch_one(&mut *tx)
        .await?;
    if count <= 1 {
        return Err(OwnerError::Taken("the last topic cannot be deleted"));
    }
    sqlx::query("DELETE FROM topics WHERE slug = ?")
        .bind(&slug)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    s.content_changed();
    Ok(Json(all(&s.pool).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_come_from_names() {
        assert_eq!(slug_for("Free Diving").as_deref(), Some("free-diving"));
        assert_eq!(slug_for("  Jiu   jitsu ").as_deref(), Some("jiu-jitsu"));
        assert_eq!(slug_for("Classic WoW!").as_deref(), Some("classic-wow"));
        assert_eq!(slug_for("שלום"), None);
        assert_eq!(slug_for("---"), None);
    }

    #[test]
    fn names_are_checked() {
        assert_eq!(clean_name("  Surf ").unwrap(), "Surf");
        assert!(clean_name("   ").is_err());
        assert!(clean_name("a\nb").is_err());
        assert!(clean_name(&"x".repeat(NAME_MAX + 1)).is_err());
        assert!(clean_name(&"x".repeat(NAME_MAX)).is_ok());
    }
}
