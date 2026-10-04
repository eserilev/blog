//! `logbook seed-sample`: sample posts for local development and screenshots.
//! It refuses to run on a database that already has posts.

use logbook_core::{State, Topic};
use sqlx::SqlitePool;

use crate::posts::{self, NewPost};

const BAL: &str = include_str!("../sample/block-level-access-lists.md");
const EPBS: &str = include_str!("../sample/epbs-client-view.md");

/// Sample posts: six public, one private, one draft.
fn sample() -> Vec<NewPost<'static>> {
    let public = |title, summary, topic, tags: &'static [&'static str], body, at| NewPost {
        title,
        summary,
        topic,
        tags,
        body_md: body,
        state: State::Public,
        published_at: Some(at),
    };
    vec![
        public(
            "Splitboarding: end of season notes",
            "Route, gear, and conditions from the last backcountry tour of the season.",
            Topic::Snowboarding,
            &["snow", "backcountry"],
            "Sample post. Route, gear, and conditions from the last tour of the season.\n",
            "2026-04-11T16:00:00Z",
        ),
        public(
            "Surf log, summer 2026",
            "Conditions, a new board, and why I keep my mornings free of calls.",
            Topic::Surf,
            &["surf"],
            "Sample post. A short entry about conditions and a new board.\n",
            "2026-07-30T14:00:00Z",
        ),
        public(
            "Four years of jiu jitsu",
            "Notes on training consistently while working on a protocol.",
            Topic::JiuJitsu,
            &["bjj"],
            "Sample post. Notes on training while working on a protocol.\n",
            "2026-08-19T18:00:00Z",
        ),
        public(
            "What classic WoW addons got right about interfaces",
            "Raid frames and threat meters solved information density problems that many dashboards still get wrong.",
            Topic::ClassicWow,
            &["design", "wow"],
            "Sample post. Raid frames and threat meters, and what dashboards can learn.\n",
            "2026-09-02T17:00:00Z",
        ),
        public(
            "Zero-copy SSZ decoding in Rust",
            "Decoding beacon state without allocations: borrowing from the input buffer, and where it paid off.",
            Topic::Rust,
            &["rust", "ssz", "performance"],
            "Sample post.\n\n```rust\npub fn decode<'a>(bytes: &'a [u8]) -> Result<View<'a>, Error> {\n    View::new(bytes)\n}\n```\n",
            "2026-09-14T15:00:00Z",
        ),
        public(
            "Block-level access lists and parallel execution",
            "EIP-7928 makes each block declare the accounts and storage slots it touches. What this gives clients, and what it costs builders.",
            Topic::Ethereum,
            &["glamsterdam", "eip-7928", "clients"],
            BAL,
            "2026-09-28T14:02:00Z",
        ),
        NewPost {
            title: "ePBS from a client's perspective",
            summary: "Fork choice now has to track the payload, not only the block.",
            topic: Topic::Ethereum,
            tags: &["glamsterdam", "epbs"],
            body_md: EPBS,
            state: State::Draft,
            published_at: None,
        },
        NewPost {
            title: "Molten Core as a scheduling problem",
            summary: "Private notes.",
            topic: Topic::ClassicWow,
            tags: &["wow"],
            body_md: "Private sample post.\n",
            state: State::Private,
            published_at: None,
        },
    ]
}

/// Inserts the sample posts. Returns how many.
///
/// # Errors
///
/// Fails if the database already has posts, or on database errors.
pub async fn seed_sample(pool: &SqlitePool) -> Result<usize, String> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    if count > 0 {
        return Err(format!(
            "the database already has {count} posts; seed-sample only runs on an empty database"
        ));
    }
    let posts = sample();
    for p in &posts {
        posts::create(pool, p).await.map_err(|e| e.to_string())?;
    }
    Ok(posts.len())
}
