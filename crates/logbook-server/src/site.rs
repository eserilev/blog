//! The title section of the home page (spec 4.9): title, subtitle, tagline, intro.
//!
//! The server fills these values, and the topic list, into `index.html` on every page
//! load. So a guest without JavaScript and a search engine see the same text.

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{AppState, auth::Owner, head::escape, posts::OwnerError, topic::Topic};

/// Limits, in characters.
pub const TITLE_MAX: usize = 80;
pub const SUBTITLE_MAX: usize = 120;
pub const TAGLINE_MAX: usize = 300;
pub const INTRO_MAX: usize = 5_000;

/// The stored values.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Site {
    pub title: String,
    pub subtitle: String,
    pub tagline: String,
    pub intro_md: String,
    pub updated_at: Option<String>,
}

/// The values for guests: the intro as sanitized HTML.
#[derive(Debug, Serialize)]
pub struct PublicSite {
    pub title: String,
    pub subtitle: String,
    pub tagline: String,
    pub intro_html: String,
}

impl From<Site> for PublicSite {
    fn from(s: Site) -> Self {
        Self {
            intro_html: logbook_render::render(&s.intro_md),
            title: s.title,
            subtitle: s.subtitle,
            tagline: s.tagline,
        }
    }
}

/// The site row. Migration 0002 writes it, so it always exists.
///
/// # Errors
///
/// Database errors.
pub async fn get(pool: &SqlitePool) -> Result<Site, sqlx::Error> {
    sqlx::query_as("SELECT title, subtitle, tagline, intro_md, updated_at FROM site WHERE id = 1")
        .fetch_one(pool)
        .await
}

/// `GET /api/site`.
///
/// # Errors
///
/// 500 on database errors.
pub async fn api_site(State(s): State<AppState>) -> Result<Json<PublicSite>, OwnerError> {
    Ok(Json(get(&s.pool).await?.into()))
}

/// `GET /api/owner/site`: with the intro as markdown.
///
/// # Errors
///
/// 401 without a session. 500 on database errors.
pub async fn owner_site(_: Owner, State(s): State<AppState>) -> Result<Json<Site>, OwnerError> {
    Ok(Json(get(&s.pool).await?))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteInput {
    pub title: String,
    pub subtitle: String,
    pub tagline: String,
    pub intro_md: String,
}

/// One trimmed line: at most `max` characters, no control characters.
fn line(v: &str, max: usize, what: &'static str) -> Result<String, &'static str> {
    let v = v.trim();
    if v.chars().count() > max || v.chars().any(char::is_control) {
        return Err(what);
    }
    Ok(v.to_string())
}

impl SiteInput {
    /// Checks the limits. The title must not be empty.
    ///
    /// # Errors
    ///
    /// A message for the first rule that fails.
    pub fn validate(self) -> Result<Self, &'static str> {
        let title = line(
            &self.title,
            TITLE_MAX,
            "the title is too long or has control characters",
        )?;
        if title.is_empty() {
            return Err("the title is empty");
        }
        let subtitle = line(
            &self.subtitle,
            SUBTITLE_MAX,
            "the subtitle is too long or has control characters",
        )?;
        let tagline = line(
            &self.tagline,
            TAGLINE_MAX,
            "the tagline is too long or has control characters",
        )?;
        if self.intro_md.chars().count() > INTRO_MAX {
            return Err("the intro is too long");
        }
        Ok(Self {
            title,
            subtitle,
            tagline,
            intro_md: self.intro_md,
        })
    }
}

/// `PUT /api/owner/site`. One owner, so the last save wins.
///
/// # Errors
///
/// 401 without a session, 400 for bad input.
pub async fn save_site(
    _: Owner,
    State(s): State<AppState>,
    Json(input): Json<SiteInput>,
) -> Result<Json<Site>, OwnerError> {
    let v = input.validate().map_err(OwnerError::BadRequest)?;
    sqlx::query(
        "UPDATE site SET title = ?, subtitle = ?, tagline = ?, intro_md = ?,
            updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = 1",
    )
    .bind(&v.title)
    .bind(&v.subtitle)
    .bind(&v.tagline)
    .bind(&v.intro_md)
    .execute(&s.pool)
    .await?;
    Ok(Json(get(&s.pool).await?))
}

/// The markers in `index.html` that [`fill`] replaces.
pub const MARKERS: [&str; 6] = [
    "<!--site:title-->",
    "<!--site:subtitle-->",
    "<!--site:tagline-->",
    "<!--site:intro-->",
    "<!--site:topics-->",
    "<!--site:topic-options-->",
];

/// Puts the site values and the topics into `index.html`. Text is escaped. The
/// intro goes through the markdown pipeline, so it is sanitized HTML.
#[must_use]
pub fn fill(html: &str, site: &Site, topics: &[Topic]) -> String {
    use std::fmt::Write as _;
    let (mut links, mut options) = (String::new(), String::new());
    for t in topics {
        let (slug, name) = (escape(&t.slug), escape(&t.name));
        let _ = write!(links, "<li><a href=\"/topics/{slug}\">{name}</a></li>");
        let _ = write!(options, "<option value=\"{slug}\">{name}</option>");
    }
    html.replace(MARKERS[0], &escape(&site.title))
        .replace(MARKERS[1], &escape(&site.subtitle))
        .replace(MARKERS[2], &escape(&site.tagline))
        .replace(MARKERS[3], &logbook_render::render(&site.intro_md))
        .replace(MARKERS[4], &links)
        .replace(MARKERS[5], &options)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(title: &str) -> Site {
        Site {
            title: title.into(),
            subtitle: "sub".into(),
            tagline: "tag".into(),
            intro_md: "Hello <script>x</script> **bold**".into(),
            updated_at: None,
        }
    }

    #[test]
    fn fill_escapes_text_and_sanitizes_the_intro() {
        let html = "<h1><!--site:title--></h1><a title=\"<!--site:title-->\"></a><!--site:intro--><ul><!--site:topics--></ul>";
        let topics = [Topic {
            slug: "a".into(),
            name: "<b>A</b>".into(),
        }];
        let out = fill(html, &site("\"><script>"), &topics);
        assert!(!out.contains("<script"), "{out}");
        assert!(out.contains("<h1>&quot;&gt;&lt;script&gt;</h1>"), "{out}");
        assert!(out.contains("<strong>bold</strong>"), "{out}");
        assert!(
            out.contains("<li><a href=\"/topics/a\">&lt;b&gt;A&lt;/b&gt;</a></li>"),
            "{out}"
        );
        assert!(!out.contains("<!--site:"), "{out}");
    }

    #[test]
    fn input_is_checked() {
        let ok = |title: &str| SiteInput {
            title: title.into(),
            subtitle: String::new(),
            tagline: String::new(),
            intro_md: String::new(),
        };
        assert_eq!(ok("  My Logbook ").validate().unwrap().title, "My Logbook");
        assert!(ok("  ").validate().is_err());
        assert!(ok("a\tb").validate().is_err());
        assert!(ok(&"x".repeat(TITLE_MAX + 1)).validate().is_err());
        let mut long = ok("t");
        long.intro_md = "x".repeat(INTRO_MAX + 1);
        assert!(long.validate().is_err());
    }
}
