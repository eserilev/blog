//! Export (spec 6.10): posts as markdown files with a front-matter header.
//!
//! - `GET /api/owner/export.zip`: every post, every state, for the owner.
//! - The git export: public posts only, pushed to `EXPORT_REPO` (`logbook-posts`)
//!   after changes and once a day.
//!
//! String values in the header are JSON strings, so any title round-trips. JSON
//! strings are also valid YAML, so other tools read the files too.

use std::{
    io::Write,
    path::{Path as FsPath, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};

use axum::{
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::{AppState, auth::Owner};

/// One exported post.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExportPost {
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub topic: String,
    /// JSON array, as stored.
    pub tags: String,
    pub state: String,
    pub published_at: Option<String>,
    pub updated_at: String,
    pub body_md: String,
}

fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

/// The file text: a `---` header, a blank line, the markdown.
#[must_use]
pub fn write(p: &ExportPost) -> String {
    use std::fmt::Write as _;
    let tags: Vec<String> = serde_json::from_str(&p.tags).unwrap_or_default();
    let tags = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into());
    let published = p
        .published_at
        .as_deref()
        .map_or_else(|| "null".to_string(), json_str);
    let mut out = String::from("---\n");
    // Writing to a String cannot fail.
    let _ = writeln!(out, "title: {}", json_str(&p.title));
    let _ = writeln!(out, "slug: {}", json_str(&p.slug));
    let _ = writeln!(out, "summary: {}", json_str(&p.summary));
    let _ = writeln!(out, "topic: {}", json_str(&p.topic));
    let _ = writeln!(out, "tags: {tags}");
    let _ = writeln!(out, "state: {}", json_str(&p.state));
    let _ = writeln!(out, "published: {published}");
    let _ = writeln!(out, "updated: {}", json_str(&p.updated_at));
    out.push_str("---\n\n");
    out.push_str(&p.body_md);
    out
}

/// Parses a file from [`write`]. `None` for anything else. Never panics
/// (fuzz target `frontmatter`).
#[must_use]
pub fn parse(text: &str) -> Option<ExportPost> {
    let rest = text.strip_prefix("---\n")?;
    let (header, body) = rest.split_once("\n---\n\n")?;
    let mut get = std::collections::HashMap::new();
    for line in header.lines() {
        let (k, v) = line.split_once(": ")?;
        get.insert(k, v);
    }
    let s = |k: &str| -> Option<String> { serde_json::from_str::<String>(get.get(k)?).ok() };
    let tags: Vec<String> = serde_json::from_str(get.get("tags")?).ok()?;
    let published_at = match *get.get("published")? {
        "null" => None,
        v => Some(serde_json::from_str::<String>(v).ok()?),
    };
    Some(ExportPost {
        title: s("title")?,
        slug: s("slug")?,
        summary: s("summary")?,
        topic: s("topic")?,
        tags: serde_json::to_string(&tags).ok()?,
        state: s("state")?,
        published_at,
        updated_at: s("updated")?,
        body_md: body.to_string(),
    })
}

const SELECT: &str = "SELECT slug, title, summary, topic, tags, state, published_at, updated_at, body_md FROM posts ORDER BY id";

async fn now_md(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT body_md FROM now_box WHERE id = 1")
        .fetch_optional(pool)
        .await
}

/// `GET /api/owner/export.zip`: every post in every state, and the Now box.
pub async fn owner_zip(_: Owner, State(s): State<AppState>) -> Response {
    let built = async {
        let posts: Vec<ExportPost> = sqlx::query_as(SELECT).fetch_all(&s.pool).await?;
        let now = now_md(&s.pool).await?;
        Ok::<_, sqlx::Error>((posts, now))
    }
    .await;
    let (posts, now) = match built {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("export failed: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response();
        }
    };
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut add = |name: String, text: &str| -> zip::result::ZipResult<()> {
        zip.start_file(name, opts)?;
        zip.write_all(text.as_bytes())?;
        Ok(())
    };
    let mut result = posts
        .iter()
        .try_for_each(|p| add(format!("posts/{}.md", p.slug), &write(p)));
    if let (Ok(()), Some(n)) = (&result, now) {
        result = add("now.md".into(), &n);
    }
    let bytes = result
        .and_then(|()| zip.finish())
        .map(std::io::Cursor::into_inner);
    match bytes {
        Ok(b) => (
            [
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/zip"),
                ),
                (
                    header::CONTENT_DISPOSITION,
                    HeaderValue::from_static("attachment; filename=\"logbook-export.zip\""),
                ),
            ],
            b,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("zip failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
        }
    }
}

/// Settings for the git export.
#[derive(Debug, Clone)]
pub struct GitExport {
    /// `EXPORT_REPO`, for example `git@github.com:eserilev/logbook-posts.git`.
    pub repo: String,
    /// `EXPORT_BRANCH`, default `main`.
    pub branch: String,
    /// Local clone. Rebuildable, so it is not state.
    pub dir: PathBuf,
    /// The private deploy key (from `EXPORT_DEPLOY_KEY`, base64), if the repo needs one.
    pub deploy_key: Option<Vec<u8>>,
}

fn git(dir: &FsPath, env: &[(String, String)], args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// Writes the public posts into the clone, commits, and pushes. Returns true if
/// something changed.
///
/// # Errors
///
/// Database, file, or git errors.
pub async fn export_once(pool: &SqlitePool, cfg: &GitExport) -> Result<bool, String> {
    let posts: Vec<ExportPost> = sqlx::query_as(SELECT)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let public: Vec<&ExportPost> = posts.iter().filter(|p| p.state == "public").collect();
    let now = now_md(pool).await.map_err(|e| e.to_string())?;
    let cfg = cfg.clone();
    let files: Vec<(String, String)> = public
        .iter()
        .map(|p| (format!("posts/{}.md", p.slug), write(p)))
        .collect();
    tokio::task::spawn_blocking(move || sync_repo(&cfg, &files, now.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

fn sync_repo(
    cfg: &GitExport,
    files: &[(String, String)],
    now: Option<&str>,
) -> Result<bool, String> {
    let mut env = Vec::new();
    let key_path = cfg.dir.with_extension("key");
    if let Some(key) = &cfg.deploy_key {
        std::fs::write(&key_path, key).map_err(|e| format!("cannot write the deploy key: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| e.to_string())?;
        }
        let known = cfg.dir.with_extension("known_hosts");
        env.push((
            "GIT_SSH_COMMAND".to_string(),
            format!(
                "ssh -i {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new -o UserKnownHostsFile={}",
                key_path.display(),
                known.display()
            ),
        ));
    }
    let parent = cfg.dir.parent().ok_or("export dir has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    if cfg.dir.join(".git").exists() {
        // An empty remote has no branch yet; ignore that case.
        if git(&cfg.dir, &env, &["fetch", "origin", &cfg.branch]).is_ok() {
            git(
                &cfg.dir,
                &env,
                &["reset", "--hard", &format!("origin/{}", cfg.branch)],
            )?;
        }
    } else {
        let dir = cfg.dir.to_string_lossy().into_owned();
        git(parent, &env, &["clone", &cfg.repo, &dir])?;
        git(&cfg.dir, &env, &["checkout", "-B", &cfg.branch])?;
    }
    let posts_dir = cfg.dir.join("posts");
    if posts_dir.exists() {
        std::fs::remove_dir_all(&posts_dir).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&posts_dir).map_err(|e| e.to_string())?;
    for (name, text) in files {
        std::fs::write(cfg.dir.join(name), text).map_err(|e| e.to_string())?;
    }
    let now_path = cfg.dir.join("now.md");
    match now {
        Some(n) => std::fs::write(&now_path, n).map_err(|e| e.to_string())?,
        None if now_path.exists() => std::fs::remove_file(&now_path).map_err(|e| e.to_string())?,
        None => {}
    }
    git(&cfg.dir, &env, &["add", "-A"])?;
    if git(&cfg.dir, &env, &["status", "--porcelain"])?
        .trim()
        .is_empty()
    {
        return Ok(false);
    }
    let id = [
        "-c",
        "user.name=logbook",
        "-c",
        "user.email=logbook@localhost",
    ];
    git(
        &cfg.dir,
        &env,
        &[
            id[0],
            id[1],
            id[2],
            id[3],
            "commit",
            "-q",
            "-m",
            "Export public posts",
        ],
    )?;
    git(
        &cfg.dir,
        &env,
        &["push", "-q", "origin", &format!("HEAD:{}", cfg.branch)],
    )?;
    Ok(true)
}

/// Runs the git export after each change signal (10 s debounce) and once a day.
pub fn spawn_git_export(pool: SqlitePool, cfg: GitExport, changed: Arc<Notify>) {
    tokio::spawn(async move {
        loop {
            let _ = tokio::time::timeout(Duration::from_hours(24), changed.notified()).await;
            tokio::time::sleep(Duration::from_secs(10)).await;
            match export_once(&pool, &cfg).await {
                Ok(true) => tracing::info!("exported public posts to {}", cfg.repo),
                Ok(false) => {}
                Err(e) => tracing::error!("git export failed: {e}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn post() -> ExportPost {
        ExportPost {
            slug: "a-post".into(),
            title: "A \"post\": with: colons\nand a newline".into(),
            summary: "s".into(),
            topic: "rust".into(),
            tags: r#"["a","b"]"#.into(),
            state: "public".into(),
            published_at: Some("2026-09-28T14:02:00Z".into()),
            updated_at: "2026-09-29T00:00:00Z".into(),
            body_md: "# Hi\n\n---\n\nbody".into(),
        }
    }

    #[test]
    fn round_trips() {
        let p = post();
        assert_eq!(parse(&write(&p)), Some(p.clone()));
        let draft = ExportPost {
            published_at: None,
            state: "draft".into(),
            ..p
        };
        assert_eq!(parse(&write(&draft)), Some(draft));
    }

    #[test]
    fn rejects_other_text() {
        for t in [
            "",
            "no header",
            "---\ntitle: x\n",
            "---\ntitle: not json\n---\n\nbody",
        ] {
            assert_eq!(parse(t), None, "{t:?}");
        }
    }

    proptest! {
        /// Spec 7.2: `parse(write(post)) == post`.
        #[test]
        fn any_post_round_trips(title in "\\PC*", summary in "\\PC*", body in "\\PC*", tags in prop::collection::vec("[a-z0-9-]{1,10}", 0..5), published in proptest::option::of("[0-9TZ:-]{1,25}")) {
            let p = ExportPost {
                slug: "s".into(), title, summary, topic: "rust".into(),
                tags: serde_json::to_string(&tags).unwrap(), state: "draft".into(),
                published_at: published, updated_at: "u".into(), body_md: body,
            };
            prop_assert_eq!(parse(&write(&p)), Some(p));
        }
    }
}
