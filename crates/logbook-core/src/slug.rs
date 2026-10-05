//! Slugs (spec 4.3, theorems T4–T9).
//!
//! Rules: `a-z`, `0-9`, `-`. 1–80 bytes. No `-` at either end. No `--`.
//! A title with no ASCII letters or digits gets `post-<id>`.

/// Maximum slug length in bytes.
pub const SLUG_MAX: usize = 80;

// Plain comparisons, not `u8::is_ascii_*`: Aeneas has no model for those.

fn lower(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' { c + 32 } else { c }
}

fn is_slug_char(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'0' && c <= b'9')
}

/// `post-<id>`.
fn fallback(id: u64) -> Vec<u8> {
    let mut digits = Vec::new();
    let mut n = id;
    loop {
        // `n % 10` is at most 9, so the cast cannot truncate.
        #[allow(clippy::cast_possible_truncation)]
        digits.push(b'0' + (n % 10) as u8);
        n /= 10;
        if n == 0 {
            break;
        }
    }
    let mut out = b"post-".to_vec();
    let mut i = digits.len();
    while i > 0 {
        i -= 1;
        out.push(digits[i]);
    }
    out
}

/// Makes the slug for a post title. Pure and total.
///
/// - ASCII letters become lowercase. Letters and digits are kept.
/// - An apostrophe is dropped. Every other run of bytes becomes one `-`.
/// - A `-` is written only before a kept character, so there is no `-` at the
///   start or end, and no `--`.
/// - The slug stops before it would pass 80 bytes.
/// - If nothing is kept, the slug is `post-<id>`.
#[must_use]
pub fn make_slug(title: &[u8], id: u64) -> Vec<u8> {
    let mut out = Vec::new();
    let mut gap = false;
    let mut i = 0;
    while i < title.len() {
        let c = lower(title[i]);
        if is_slug_char(c) {
            if gap && out.len() > 0 {
                if out.len() + 2 > SLUG_MAX {
                    break;
                }
                out.push(b'-');
            }
            if out.len() + 1 > SLUG_MAX {
                break;
            }
            out.push(c);
            gap = false;
        } else if c != b'\'' {
            gap = true;
        }
        i += 1;
    }
    if out.len() == 0 { fallback(id) } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn s(title: &str, id: u64) -> String {
        String::from_utf8(make_slug(title.as_bytes(), id)).unwrap()
    }

    #[test]
    fn examples() {
        assert_eq!(
            s("Block-level access lists and parallel execution", 1),
            "block-level-access-lists-and-parallel-execution"
        );
        assert_eq!(
            s("ePBS from a client's perspective", 1),
            "epbs-from-a-clients-perspective"
        );
        assert_eq!(s("  Rust:  zero-copy!! ", 1), "rust-zero-copy");
        assert_eq!(s("שלום עולם", 42), "post-42");
        assert_eq!(s("", 0), "post-0");
        assert_eq!(s("🌊", u64::MAX), "post-18446744073709551615");
    }

    #[test]
    fn long_titles_stop_at_80_bytes_on_a_word() {
        let title = "word ".repeat(40);
        let slug = s(&title, 1);
        assert!(slug.len() <= SLUG_MAX);
        assert!(!slug.ends_with('-'));
    }

    fn rules_hold(slug: &[u8]) -> bool {
        !slug.is_empty()
            && slug.len() <= SLUG_MAX
            && slug.iter().all(|&c| is_slug_char(c) || c == b'-')
            && slug[0] != b'-'
            && slug[slug.len() - 1] != b'-'
            && !slug.windows(2).any(|w| w == b"--")
    }

    proptest! {
        /// T4 and T5.
        #[test]
        fn every_slug_follows_the_rules(title in prop::collection::vec(any::<u8>(), 0..300), id: u64) {
            prop_assert!(rules_hold(&make_slug(&title, id)));
        }

        /// T4 and T5 on text that looks like real titles.
        #[test]
        fn title_like_input_follows_the_rules(title in "[A-Za-z0-9 '._:!-]{0,200}", id: u64) {
            prop_assert!(rules_hold(&make_slug(title.as_bytes(), id)));
        }

        /// T6.
        #[test]
        fn a_plain_title_is_its_own_slug(t in "[a-z0-9]{1,80}", id: u64) {
            prop_assert_eq!(make_slug(t.as_bytes(), id), t.into_bytes());
        }

        /// T7.
        #[test]
        fn no_ascii_alphanumerics_gives_the_fallback(t in "[^A-Za-z0-9]{0,40}", id: u64) {
            prop_assert_eq!(make_slug(t.as_bytes(), id), format!("post-{id}").into_bytes());
        }

        /// T8.
        #[test]
        fn case_does_not_matter(t in "[A-Za-z0-9 -]{0,120}", id: u64) {
            prop_assert_eq!(make_slug(t.to_ascii_uppercase().as_bytes(), id), make_slug(t.as_bytes(), id));
        }

        /// T9.
        #[test]
        fn a_slug_of_a_slug_is_the_same(title in prop::collection::vec(any::<u8>(), 0..300), id: u64) {
            let once = make_slug(&title, id);
            prop_assert_eq!(make_slug(&once, id), once);
        }
    }
}
