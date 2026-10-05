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

// ---------------------------------------------------------------------------
// Owner API (spec 6.4). Every handler takes `Owner`, so a request without a
// session gets 401 before the handler runs. Routes add `Cache-Control: no-store`.
// ---------------------------------------------------------------------------

use crate::auth::Owner;
use axum::http::HeaderMap;
use serde::Deserialize;

/// Limits on owner input.
pub const TITLE_MAX: usize = 200;
pub const SUMMARY_MAX: usize = 300;
pub const TAGS_MAX: usize = 10;
pub const TAG_MAX: usize = 30;
pub const BODY_MAX: usize = 200_000;

/// An owner API error.
#[derive(Debug)]
pub enum OwnerError {
    NotFound,
    BadRequest(&'static str),
    /// The post changed since the editor loaded it (spec 3.8).
    Conflict,
    /// `If-Match` is missing or not a version number.
    PreconditionRequired,
    Internal(sqlx::Error),
}

impl From<sqlx::Error> for OwnerError {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e)
    }
}

impl IntoResponse for OwnerError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "not found"),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            Self::Conflict => (
                StatusCode::CONFLICT,
                "this post changed on another device; reload to see the newer version",
            ),
            Self::PreconditionRequired => (
                StatusCode::PRECONDITION_REQUIRED,
                "send If-Match with the post version",
            ),
            Self::Internal(e) => {
                tracing::error!("database error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error")
            }
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

/// The version in `If-Match`, with or without quotes.
fn if_match(headers: &HeaderMap) -> Result<i64, OwnerError> {
    let v = headers
        .get(axum::http::header::IF_MATCH)
        .and_then(|v| v.to_str().ok())
        .ok_or(OwnerError::PreconditionRequired)?;
    v.trim()
        .trim_matches('"')
        .parse()
        .map_err(|_| OwnerError::PreconditionRequired)
}

/// A post in the owner's list.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OwnerListItem {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub topic: String,
    pub state: String,
    pub version: i64,
    pub published_at: Option<String>,
    pub updated_at: String,
}

/// A post for the editor, with `body_md` and `version`.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OwnerPost {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub topic: String,
    #[serde(serialize_with = "tags_json")]
    pub tags: String,
    pub body_md: String,
    pub state: String,
    pub version: i64,
    pub word_count: i64,
    pub published_at: Option<String>,
    pub updated_at: String,
}

fn tags_json<S: serde::Serializer>(tags: &str, s: S) -> Result<S::Ok, S::Error> {
    let v: Vec<String> = serde_json::from_str(tags).unwrap_or_default();
    v.serialize(s)
}

/// The editor's input for `PUT /api/owner/posts/{id}`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostInput {
    pub title: String,
    pub summary: String,
    pub topic: String,
    pub tags: Vec<String>,
    pub body_md: String,
}

/// Validated input.
#[derive(Debug)]
pub struct Valid {
    pub title: String,
    pub summary: String,
    pub topic: logbook_core::Topic,
    pub tags_json: String,
    pub body_md: String,
}

impl PostInput {
    /// Checks the limits. Never panics, for any input (fuzz target `post_input`).
    ///
    /// # Errors
    ///
    /// A message for the first rule that fails.
    pub fn validate(self) -> Result<Valid, &'static str> {
        let title = self.title.trim().to_string();
        if title.is_empty() {
            return Err("the title is empty");
        }
        if title.chars().count() > TITLE_MAX || title.chars().any(char::is_control) {
            return Err("the title is too long or has control characters");
        }
        let summary = self.summary.trim().to_string();
        if summary.chars().count() > SUMMARY_MAX || summary.chars().any(char::is_control) {
            return Err("the summary is too long or has control characters");
        }
        let topic = topic::parse(&self.topic).ok_or("unknown topic")?;
        if self.tags.len() > TAGS_MAX {
            return Err("too many tags");
        }
        for t in &self.tags {
            if t.is_empty()
                || t.len() > TAG_MAX
                || !t
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            {
                return Err("tags use a-z, 0-9, and -, at most 30 characters");
            }
        }
        if self.body_md.len() > BODY_MAX {
            return Err("the post is too long");
        }
        let tags_json = serde_json::to_string(&self.tags).map_err(|_| "bad tags")?;
        Ok(Valid {
            title,
            summary,
            topic,
            tags_json,
            body_md: self.body_md,
        })
    }
}

/// `GET /api/owner/posts`: every post, every state, newest change first.
///
/// # Errors
///
/// 401 without a session. 500 on database errors.
pub async fn owner_list(
    _: Owner,
    State(s): State<AppState>,
) -> Result<Json<Vec<OwnerListItem>>, OwnerError> {
    let rows = sqlx::query_as(
        "SELECT id, slug, title, topic, state, version, published_at, updated_at FROM posts ORDER BY updated_at DESC, id DESC",
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(rows))
}

async fn owner_post(pool: &SqlitePool, id: i64) -> Result<OwnerPost, OwnerError> {
    sqlx::query_as(
        "SELECT id, slug, title, summary, topic, tags, body_md, state, version, word_count, published_at, updated_at
         FROM posts WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(OwnerError::NotFound)
}

/// `GET /api/owner/posts/{id}`.
///
/// # Errors
///
/// 401 without a session. 404 for an unknown id.
pub async fn owner_get(
    _: Owner,
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<OwnerPost>, OwnerError> {
    Ok(Json(owner_post(&s.pool, id).await?))
}

/// `POST /api/owner/posts`: a new, empty draft.
///
/// # Errors
///
/// 401 without a session. 500 on database errors.
pub async fn owner_create(_: Owner, State(s): State<AppState>) -> Result<Response, OwnerError> {
    let id = create(
        &s.pool,
        &NewPost {
            title: "Untitled",
            summary: "",
            topic: logbook_core::Topic::Ethereum,
            tags: &[],
            body_md: "",
            state: PostState::Draft,
            published_at: None,
        },
    )
    .await?;
    let id = i64::try_from(id).unwrap_or(0);
    let post = owner_post(&s.pool, id).await?;
    Ok((StatusCode::CREATED, Json(post)).into_response())
}

/// The slug for `title`, or `post-<id>` if another post has it.
async fn free_slug(
    tx: &mut sqlx::SqliteConnection,
    title: &str,
    id: i64,
) -> Result<String, sqlx::Error> {
    let id_u = u64::try_from(id).unwrap_or(0);
    let wanted = String::from_utf8(make_slug(title.as_bytes(), id_u)).unwrap_or_default();
    let taken: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM posts WHERE slug = ? AND id != ?)")
            .bind(&wanted)
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    Ok(if taken {
        String::from_utf8(make_slug(b"", id_u)).unwrap_or_default()
    } else {
        wanted
    })
}

/// Fails with 409 if the post exists, else 404.
async fn missing_or_conflict(pool: &SqlitePool, id: i64) -> OwnerError {
    match sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM posts WHERE id = ?)")
        .bind(id)
        .fetch_one(pool)
        .await
    {
        Ok(true) => OwnerError::Conflict,
        Ok(false) => OwnerError::NotFound,
        Err(e) => OwnerError::Internal(e),
    }
}

/// `PUT /api/owner/posts/{id}`: save. Needs `If-Match: <version>`; a mismatch is 409.
/// The slug follows the title until the first publish, then it never changes (spec 4.3).
///
/// # Errors
///
/// 401, 400 (bad input), 404, 409, 428.
pub async fn owner_save(
    _: Owner,
    State(s): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<PostInput>,
) -> Result<Json<OwnerPost>, OwnerError> {
    let version = if_match(&headers)?;
    let v = input.validate().map_err(OwnerError::BadRequest)?;
    let body_html = logbook_render::render(&v.body_md);
    let words = i64::from(logbook_render::word_count(&v.body_md));

    let mut tx = s.pool.begin().await?;
    let slug = free_slug(&mut tx, &v.title, id).await?;
    let done = sqlx::query(
        "UPDATE posts SET title = ?, summary = ?, topic = ?, tags = ?, body_md = ?, body_html = ?, word_count = ?,
            slug = CASE WHEN published_at IS NULL THEN ? ELSE slug END,
            version = version + 1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ? AND version = ?",
    )
    .bind(&v.title)
    .bind(&v.summary)
    .bind(topic::slug(v.topic))
    .bind(&v.tags_json)
    .bind(&v.body_md)
    .bind(body_html)
    .bind(words)
    .bind(slug)
    .bind(id)
    .bind(version)
    .execute(&mut *tx)
    .await?;
    if done.rows_affected() != 1 {
        drop(tx);
        return Err(missing_or_conflict(&s.pool, id).await);
    }
    tx.commit().await?;
    Ok(Json(owner_post(&s.pool, id).await?))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateInput {
    pub state: String,
}

/// `POST /api/owner/posts/{id}/state`: draft, private, or public. Needs `If-Match`.
/// `published_at` is set on the first change to public and never changes after.
///
/// # Errors
///
/// 401, 400 (unknown state), 404, 409, 428.
pub async fn owner_set_state(
    _: Owner,
    State(s): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    Json(input): Json<StateInput>,
) -> Result<Json<OwnerPost>, OwnerError> {
    let version = if_match(&headers)?;
    let state = parse_state(&input.state)
        .ok_or(OwnerError::BadRequest("state is draft, private, or public"))?;
    let done = sqlx::query(
        "UPDATE posts SET state = ?,
            published_at = CASE WHEN ? = 'public' AND published_at IS NULL THEN strftime('%Y-%m-%dT%H:%M:%SZ', 'now') ELSE published_at END,
            version = version + 1, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ? AND version = ?",
    )
    .bind(state_name(state))
    .bind(state_name(state))
    .bind(id)
    .bind(version)
    .execute(&s.pool)
    .await?;
    if done.rows_affected() != 1 {
        return Err(missing_or_conflict(&s.pool, id).await);
    }
    Ok(Json(owner_post(&s.pool, id).await?))
}

/// `DELETE /api/owner/posts/{id}`.
///
/// # Errors
///
/// 401, 404.
pub async fn owner_delete(
    _: Owner,
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, OwnerError> {
    let done = sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(id)
        .execute(&s.pool)
        .await?;
    if done.rows_affected() == 0 {
        return Err(OwnerError::NotFound);
    }
    Ok(Json(serde_json::json!({ "ok": true })).into_response())
}
