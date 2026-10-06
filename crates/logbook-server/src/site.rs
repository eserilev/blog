//! The title section of the home page (spec 4.9): title, subtitle, tagline, intro.
//!
//! The server fills these values, and the topic list, into `index.html` on every page
//! load. So a guest without JavaScript and a search engine see the same text.

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    AppState,
    auth::Owner,
    head::{AUTHOR_NAME, escape},
    posts::{ListItem, OwnerError},
    topic::Topic,
};

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
pub const MARKERS: [&str; 10] = [
    "<!--site:title-->",
    "<!--site:subtitle-->",
    "<!--site:tagline-->",
    "<!--site:intro-->",
    "<!--site:topics-->",
    "<!--site:topic-options-->",
    "<!--site:author-->",
    "<!--site:year-->",
    "<!--site:post-rows-->",
    "<!--site:post-links-->",
];

/// The values that [`fill`] puts into `index.html`.
#[derive(Debug, Clone, Copy)]
pub struct Fill<'a> {
    pub site: &'a Site,
    pub topics: &'a [Topic],
    /// Public posts, newest first. Only [`ListItem`] values, so only public posts.
    pub posts: &'a [ListItem],
    /// The topic of a topic page. The post table then lists only its posts.
    pub topic: Option<&'a str>,
    /// The year for the copyright line.
    pub year: i32,
}

/// `M/D/YY` from an RFC 3339 time, in UTC, as the post table shows it.
fn short_date(rfc3339: &str) -> String {
    time::OffsetDateTime::parse(rfc3339, &time::format_description::well_known::Rfc3339)
        .map(|d| {
            format!(
                "{}/{}/{:02}",
                u8::from(d.month()),
                d.day(),
                d.year().rem_euclid(100)
            )
        })
        .unwrap_or_default()
}

/// Puts the site values, the topics, the author, and the post lists into
/// `index.html`. Text is escaped. The intro goes through the markdown pipeline, so
/// it is sanitized HTML.
///
/// The post table rows and the post links are in the HTML, so a crawler and a
/// visitor without JavaScript see the posts (spec 6.16). `<post-list>` replaces the
/// rows when it loads.
#[must_use]
pub fn fill(html: &str, f: &Fill<'_>) -> String {
    use std::fmt::Write as _;
    let (mut links, mut options) = (String::new(), String::new());
    for t in f.topics {
        let (slug, name) = (escape(&t.slug), escape(&t.name));
        let _ = write!(links, "<li><a href=\"/topics/{slug}\">{name}</a></li>");
        let _ = write!(options, "<option value=\"{slug}\">{name}</option>");
    }
    let (mut rows, mut post_links) = (String::new(), String::new());
    for p in f.posts {
        let (slug, title, summary) = (escape(&p.slug), escape(&p.title), escape(&p.summary));
        let _ = write!(
            post_links,
            "<li><a href=\"/posts/{slug}\">{title}</a> <span>{summary}</span></li>"
        );
        if f.topic.is_some_and(|t| t != p.topic) {
            continue;
        }
        let _ = write!(
            rows,
            "<tr><td class=\"date\">{date}</td><td><a class=\"ttl\" href=\"/posts/{slug}\">{title}</a><span class=\"ex\">{summary}</span></td><td class=\"topic\">{topic}</td><td class=\"date\">{minutes} min</td></tr>",
            date = short_date(p.published_at.as_deref().unwrap_or_default()),
            topic = escape(&p.topic_name),
            minutes = p.reading_minutes,
        );
    }
    let site = f.site;
    html.replace(MARKERS[0], &escape(&site.title))
        .replace(MARKERS[1], &escape(&site.subtitle))
        .replace(MARKERS[2], &escape(&site.tagline))
        .replace(MARKERS[3], &logbook_render::render(&site.intro_md))
        .replace(MARKERS[4], &links)
        .replace(MARKERS[5], &options)
        .replace(MARKERS[6], &escape(AUTHOR_NAME))
        .replace(MARKERS[7], &f.year.to_string())
        .replace(MARKERS[8], &rows)
        .replace(MARKERS[9], &post_links)
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
        let site = site("\"><script>");
        let out = fill(
            html,
            &Fill {
                site: &site,
                topics: &topics,
                posts: &[],
                topic: None,
                year: 2026,
            },
        );
        assert!(!out.contains("<script"), "{out}");
        assert!(out.contains("<h1>&quot;&gt;&lt;script&gt;</h1>"), "{out}");
        assert!(out.contains("<strong>bold</strong>"), "{out}");
        assert!(
            out.contains("<li><a href=\"/topics/a\">&lt;b&gt;A&lt;/b&gt;</a></li>"),
            "{out}"
        );
        assert!(!out.contains("<!--site:"), "{out}");
    }

    fn item(slug: &str, topic: &str, title: &str) -> ListItem {
        ListItem {
            slug: slug.into(),
            title: title.into(),
            summary: "<i>s</i>".into(),
            topic: topic.into(),
            topic_name: "<T>".into(),
            tags: Vec::new(),
            published_at: Some("2026-02-03T23:00:00Z".into()),
            word_count: 10,
            reading_minutes: 1,
        }
    }

    #[test]
    fn fill_writes_the_author_the_year_and_the_posts() {
        let html = "<p>(c) <!--site:year--> <!--site:author--></p><tbody><!--site:post-rows--></tbody><ul><!--site:post-links--></ul>";
        let posts = [item("a", "rust", "A <b>"), item("b", "surf", "B")];
        let site = site("S");
        let mut f = Fill {
            site: &site,
            topics: &[],
            posts: &posts,
            topic: None,
            year: 2031,
        };
        let out = fill(html, &f);
        assert!(out.contains("<p>(c) 2031 Eitan Seri-Levi</p>"), "{out}");
        assert!(out.contains("<tr><td class=\"date\">2/3/26</td><td><a class=\"ttl\" href=\"/posts/a\">A &lt;b&gt;</a><span class=\"ex\">&lt;i&gt;s&lt;/i&gt;</span></td><td class=\"topic\">&lt;T&gt;</td><td class=\"date\">1 min</td></tr>"), "{out}");
        assert!(
            out.contains("<li><a href=\"/posts/b\">B</a> <span>&lt;i&gt;s&lt;/i&gt;</span></li>"),
            "{out}"
        );
        assert_eq!(out.matches("<tr>").count(), 2);
        f.topic = Some("surf");
        let out = fill(html, &f);
        assert_eq!(out.matches("<tr>").count(), 1, "{out}");
        assert!(
            out.contains("href=\"/posts/b\">B</a></td>")
                || out.contains("href=\"/posts/b\">B</a><span"),
            "{out}"
        );
        assert_eq!(out.matches("<li>").count(), 2, "the links list every post");
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
