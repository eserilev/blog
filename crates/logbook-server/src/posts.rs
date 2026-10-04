//! Post storage and the public post API (spec 6.4).
//!
//! Guest responses are built only from [`PublicPost`], which only
//! [`logbook_core::reveal`] can make. Queries load posts in every state and
//! leave the decision to `logbook-core`, so the verified code makes it.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use logbook_core::{
    Post, PublicPost, State as PostState, filter_public, make_slug, reading_minutes, reveal,
};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::{AppState, topic};

/// A row of `posts`.
#[derive(Debug, sqlx::FromRow)]
struct Row {
    id: i64,
    slug: String,
    title: String,
    summary: String,
    topic: String,
    tags: String,
    body_html: String,
    word_count: i64,
    state: String,
    published_at: Option<String>,
    updated_at: String,
}

fn parse_state(s: &str) -> Option<PostState> {
    match s {
        "draft" => Some(PostState::Draft),
        "private" => Some(PostState::Private),
        "public" => Some(PostState::Public),
        _ => None,
    }
}

/// The database name of a state.
#[must_use]
pub fn state_name(s: PostState) -> &'static str {
    match s {
        PostState::Draft => "draft",
        PostState::Private => "private",
        PostState::Public => "public",
    }
}

impl Row {
    /// Converts a row. The SQL CHECK constraints make a bad state or topic impossible;
    /// if one appears anyway, the post is treated as a draft, so it stays hidden.
    fn into_core(self) -> Post {
        let topic = topic::parse(&self.topic).unwrap_or(logbook_core::Topic::Ethereum);
        let state = parse_state(&self.state).unwrap_or(PostState::Draft);
        Post {
            id: u64::try_from(self.id).unwrap_or(0),
            state,
            topic,
            word_count: u32::try_from(self.word_count).unwrap_or(u32::MAX),
            slug: self.slug.into_bytes(),
            title: self.title.into_bytes(),
            summary: self.summary.into_bytes(),
            tags: self.tags.into_bytes(),
            body_html: self.body_html.into_bytes(),
            published_at: self.published_at.map(String::into_bytes),
            updated_at: self.updated_at.into_bytes(),
        }
    }
}

/// Every post, newest first.
///
/// # Errors
///
/// Database errors.
pub async fn all(pool: &SqlitePool) -> Result<Vec<Post>, sqlx::Error> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, slug, title, summary, topic, tags, body_html, word_count, state, published_at, updated_at
         FROM posts ORDER BY COALESCE(published_at, updated_at) DESC, id DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Row::into_core).collect())
}

/// The post with this slug, if it is public.
///
/// # Errors
///
/// Database errors.
pub async fn public_by_slug(
    pool: &SqlitePool,
    slug: &str,
) -> Result<Option<PublicPost>, sqlx::Error> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT id, slug, title, summary, topic, tags, body_html, word_count, state, published_at, updated_at
         FROM posts WHERE slug = ?",
    )
        .bind(slug)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Row::into_core).and_then(reveal))
}

/// Input for [`create`].
#[derive(Debug, Clone)]
pub struct NewPost<'a> {
    pub title: &'a str,
    pub summary: &'a str,
    pub topic: logbook_core::Topic,
    pub tags: &'a [&'a str],
    pub body_md: &'a str,
    pub state: PostState,
    /// Publish time for a public post. `None` means now.
    pub published_at: Option<&'a str>,
}

/// Creates a post. Renders the markdown and sets the slug (spec 4.3).
/// If the slug is taken, the post gets `post-<id>`.
///
/// # Errors
///
/// Database errors.
pub async fn create(pool: &SqlitePool, p: &NewPost<'_>) -> Result<u64, sqlx::Error> {
    let body_html = logbook_render::render(p.body_md);
    let words = logbook_render::word_count(p.body_md);
    let tags = serde_json::to_string(p.tags).unwrap_or_else(|_| "[]".into());
    let published_at = match (p.state, p.published_at) {
        (PostState::Public, Some(t)) => Some(t.to_string()),
        (PostState::Public, None) => Some(now(pool).await?),
        _ => None,
    };

    let mut tx = pool.begin().await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO posts (slug, title, summary, topic, tags, body_md, body_html, word_count, state, published_at, updated_at)
         VALUES (lower(hex(randomblob(16))), ?, ?, ?, ?, ?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
         RETURNING id",
    )
    .bind(p.title)
    .bind(p.summary)
    .bind(topic::slug(p.topic))
    .bind(tags)
    .bind(p.body_md)
    .bind(body_html)
    .bind(i64::from(words))
    .bind(state_name(p.state))
    .bind(published_at)
    .fetch_one(&mut *tx)
    .await?;

    let id_u = u64::try_from(id).unwrap_or(0);
    let wanted = String::from_utf8(make_slug(p.title.as_bytes(), id_u)).unwrap_or_default();
    let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM posts WHERE slug = ?)")
        .bind(&wanted)
        .fetch_one(&mut *tx)
        .await?;
    let slug = if taken {
        String::from_utf8(make_slug(b"", id_u)).unwrap_or_default()
    } else {
        wanted
    };
    sqlx::query("UPDATE posts SET slug = ? WHERE id = ?")
        .bind(slug)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(id_u)
}

async fn now(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT strftime('%Y-%m-%dT%H:%M:%SZ', 'now')")
        .fetch_one(pool)
        .await
}

/// A post in a guest list. Built only from a [`PublicPost`].
#[derive(Debug, Serialize)]
pub struct ListItem {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub topic: &'static str,
    pub topic_name: &'static str,
    pub tags: Vec<String>,
    pub published_at: Option<String>,
    pub word_count: u32,
    pub reading_minutes: u32,
}

/// A full post for guests. Built only from a [`PublicPost`].
#[derive(Debug, Serialize)]
pub struct FullPost {
    #[serde(flatten)]
    pub item: ListItem,
    pub body_html: String,
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

impl From<&PublicPost> for ListItem {
    fn from(pp: &PublicPost) -> Self {
        let p = pp.post();
        Self {
            slug: text(&p.slug),
            title: text(&p.title),
            summary: text(&p.summary),
            topic: topic::slug(p.topic),
            topic_name: topic::name(p.topic),
            tags: serde_json::from_slice(&p.tags).unwrap_or_default(),
            published_at: p.published_at.as_deref().map(text),
            word_count: p.word_count,
            reading_minutes: reading_minutes(p.word_count),
        }
    }
}

impl From<&PublicPost> for FullPost {
    fn from(pp: &PublicPost) -> Self {
        Self {
            item: ListItem::from(pp),
            body_html: text(&pp.post().body_html),
        }
    }
}

/// An API error. Internal details go to the log, never to the client.
#[derive(Debug)]
pub enum ApiError {
    NotFound,
    Internal(sqlx::Error),
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "not found"),
            Self::Internal(e) => {
                tracing::error!("database error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error")
            }
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

/// `GET /api/posts`: public posts, newest first.
///
/// # Errors
///
/// 500 on database errors.
pub async fn api_list(State(s): State<AppState>) -> Result<Json<Vec<ListItem>>, ApiError> {
    let posts = all(&s.pool).await?;
    Ok(Json(
        filter_public(&posts).iter().map(ListItem::from).collect(),
    ))
}

/// `GET /api/posts/{slug}`: one public post. 404 for any other post.
///
/// # Errors
///
/// 404 if the post is missing or not public. 500 on database errors.
pub async fn api_get(
    State(s): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<FullPost>, ApiError> {
    let post = public_by_slug(&s.pool, &slug)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(FullPost::from(&post)))
}

/// `GET /api/topics/{topic}`: public posts of one topic. 404 for an unknown topic.
///
/// # Errors
///
/// 404 for an unknown topic. 500 on database errors.
pub async fn api_topic(
    State(s): State<AppState>,
    Path(t): Path<String>,
) -> Result<Json<Vec<ListItem>>, ApiError> {
    let t = topic::parse(&t).ok_or(ApiError::NotFound)?;
    let posts = all(&s.pool).await?;
    Ok(Json(
        filter_public(&posts)
            .iter()
            .filter(|pp| pp.post().topic == t)
            .map(ListItem::from)
            .collect(),
    ))
}
