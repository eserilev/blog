//! Posts, states, reading time (spec 4.1, 4.2).

/// Words per minute for the reading time (spec 4.2).
pub const WORDS_PER_MINUTE: u32 = 220;

/// Who can see a post (spec 4.1). One field. Nothing else controls visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Work in progress. Owner only.
    Draft,
    /// Finished, kept to self. Owner only.
    Private,
    /// Published. Everyone.
    Public,
}

/// A post as the core logic sees it. Text fields are UTF-8 bytes.
///
/// `Clone` is written out by hand: Aeneas models `Vec::clone` but not
/// `Option::clone`, and `filter_public` clones posts.
#[derive(Debug, PartialEq, Eq)]
pub struct Post {
    pub id: u64,
    pub state: State,
    /// Topic slug, for example `rust`. The owner manages the topic list (spec 4.2).
    pub topic: Vec<u8>,
    pub word_count: u32,
    pub slug: Vec<u8>,
    pub title: Vec<u8>,
    pub summary: Vec<u8>,
    /// JSON array of strings.
    pub tags: Vec<u8>,
    pub body_html: Vec<u8>,
    /// RFC 3339 timestamp. `None` until the first change to `Public`.
    pub published_at: Option<Vec<u8>>,
    /// RFC 3339 timestamp.
    pub updated_at: Vec<u8>,
}

fn clone_opt(v: &Option<Vec<u8>>) -> Option<Vec<u8>> {
    match v {
        Some(b) => Some(b.clone()),
        None => None,
    }
}

impl Clone for Post {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            state: self.state,
            topic: self.topic.clone(),
            word_count: self.word_count,
            slug: self.slug.clone(),
            title: self.title.clone(),
            summary: self.summary.clone(),
            tags: self.tags.clone(),
            body_html: self.body_html.clone(),
            published_at: clone_opt(&self.published_at),
            updated_at: self.updated_at.clone(),
        }
    }
}

/// Reading time in whole minutes: `max(1, ceil(words / 220))` (theorem T11).
/// Cannot overflow, for any input.
#[must_use]
pub fn reading_minutes(words: u32) -> u32 {
    let full = words / WORDS_PER_MINUTE;
    let rest = words % WORDS_PER_MINUTE;
    let minutes = if rest == 0 { full } else { full + 1 };
    if minutes == 0 { 1 } else { minutes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn short_posts_show_one_minute() {
        assert_eq!(reading_minutes(0), 1);
        assert_eq!(reading_minutes(1), 1);
        assert_eq!(reading_minutes(220), 1);
    }

    #[test]
    fn a_partial_minute_rounds_up() {
        assert_eq!(reading_minutes(221), 2);
        assert_eq!(reading_minutes(440), 2);
        assert_eq!(reading_minutes(441), 3);
    }

    #[test]
    fn the_largest_input_does_not_overflow() {
        assert_eq!(reading_minutes(u32::MAX), u32::MAX / 220 + 1);
    }

    proptest! {
        /// T11.
        #[test]
        fn matches_the_spec_formula(words: u32) {
            let expected = u64::from(words).div_ceil(220).max(1);
            prop_assert_eq!(u64::from(reading_minutes(words)), expected);
        }

        #[test]
        fn more_words_never_read_faster(a: u32, b: u32) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            prop_assert!(reading_minutes(lo) <= reading_minutes(hi));
        }
    }
}
