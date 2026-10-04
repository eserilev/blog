//! Pure logic for the Logbook. No I/O, no async, no `unsafe`.
//!
//! Aeneas verifies the functions in this crate (spec 7.6), so the code keeps to
//! the Rust subset that Aeneas supports: plain integers, structs, enums, loops,
//! `Option`, `Result`, `Vec`, and bytes instead of `String`.

/// Words per minute for the reading time (spec 4.2).
pub const WORDS_PER_MINUTE: u32 = 220;

/// Reading time in whole minutes: `max(1, ceil(words / 220))`.
///
/// Theorem T11 states this formula. The function cannot overflow, for any input.
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
