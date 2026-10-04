//! Who can see which post (spec 7.6, theorems T1–T3).
//!
//! Guest code paths take [`PublicPost`] only. [`reveal`] is the only way to make one,
//! so a guest route cannot send a draft or a private post: it does not compile.

use crate::post::{Post, State};

/// A post that a guest is allowed to see.
///
/// The field is private. Only [`reveal`] builds this type. Do not add `Default`,
/// `Deserialize`, `From<Post>`, or any other constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicPost(Post);

impl PublicPost {
    /// The post. Read access only.
    #[must_use]
    pub fn post(&self) -> &Post {
        &self.0
    }
}

/// Returns the post as a [`PublicPost`] if and only if its state is `Public` (T1).
/// The post itself does not change (T2).
#[must_use]
pub fn reveal(p: Post) -> Option<PublicPost> {
    match p.state {
        State::Public => Some(PublicPost(p)),
        State::Draft | State::Private => None,
    }
}

/// Exactly the public posts of `ps`, in the same order (T3).
#[must_use]
pub fn filter_public(ps: &[Post]) -> Vec<PublicPost> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < ps.len() {
        if let Some(q) = reveal(ps[i].clone()) {
            out.push(q);
        }
        i += 1;
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::post::Topic;
    use proptest::prelude::*;

    pub(crate) fn post(id: u64, state: State) -> Post {
        Post {
            id,
            state,
            topic: Topic::Rust,
            word_count: 10,
            slug: format!("p-{id}").into_bytes(),
            title: format!("Post {id}").into_bytes(),
            summary: Vec::new(),
            tags: b"[]".to_vec(),
            body_html: b"<p>x</p>".to_vec(),
            published_at: None,
            updated_at: b"2026-10-04T00:00:00Z".to_vec(),
        }
    }

    pub(crate) fn any_state() -> impl Strategy<Value = State> {
        prop_oneof![
            Just(State::Draft),
            Just(State::Private),
            Just(State::Public)
        ]
    }

    #[test]
    fn guests_never_see_drafts_or_private_posts() {
        assert!(reveal(post(1, State::Draft)).is_none());
        assert!(reveal(post(2, State::Private)).is_none());
        assert!(reveal(post(3, State::Public)).is_some());
    }

    proptest! {
        /// T1 and T2.
        #[test]
        fn reveal_passes_exactly_the_public_posts_unchanged(id: u64, state in any_state()) {
            let p = post(id, state);
            if let Some(q) = reveal(p.clone()) {
                prop_assert_eq!(state, State::Public);
                prop_assert_eq!(q.post(), &p);
            } else {
                prop_assert_ne!(state, State::Public);
            }
        }

        /// T3.
        #[test]
        fn filter_public_keeps_exactly_the_public_posts_in_order(states in prop::collection::vec(any_state(), 0..40)) {
            let ps: Vec<Post> = states.iter().enumerate().map(|(i, s)| post(i as u64, *s)).collect();
            let got: Vec<Post> = filter_public(&ps).iter().map(|q| q.post().clone()).collect();
            let want: Vec<Post> = ps.iter().filter(|p| p.state == State::Public).cloned().collect();
            prop_assert_eq!(got, want);
        }
    }
}
