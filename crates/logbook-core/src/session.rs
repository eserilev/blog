//! Session and setup token expiry (spec 6.6, theorem T17).
//!
//! Times are Unix seconds. The server reads the row and converts its RFC 3339 times.
//! A setup token works once because of the atomic SQL `UPDATE` in the server. These
//! functions make the time decision only.

/// Setup token lifetime in seconds: 15 minutes.
pub const SETUP_TOKEN_SECONDS: i64 = 900;

/// A session is valid if and only if `now` is before its expiry time.
#[must_use]
pub fn session_valid(now: i64, expires_at: i64) -> bool {
    now < expires_at
}

/// A setup token works if and only if it is not used and `now` is before its
/// expiry time.
#[must_use]
pub fn setup_token_usable(now: i64, expires_at: i64, used: bool) -> bool {
    !used && now < expires_at
}

/// The expiry time of a setup token made at `now`: 15 minutes later. At the end of
/// the `i64` range the result stops at `i64::MAX`, so it never overflows.
#[must_use]
pub fn setup_token_expiry(now: i64) -> i64 {
    if now > i64::MAX - SETUP_TOKEN_SECONDS {
        i64::MAX
    } else {
        now + SETUP_TOKEN_SECONDS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn examples() {
        assert!(session_valid(0, 1));
        assert!(!session_valid(1, 1));
        assert!(setup_token_usable(0, 1, false));
        assert!(!setup_token_usable(0, 1, true));
        assert!(!setup_token_usable(1, 1, false));
        assert_eq!(setup_token_expiry(1_000), 1_900);
        assert_eq!(setup_token_expiry(i64::MAX), i64::MAX);
        assert_eq!(setup_token_expiry(i64::MIN), i64::MIN + 900);
    }

    proptest! {
        /// T17.
        #[test]
        fn decisions(now: i64, exp: i64, used: bool) {
            prop_assert_eq!(session_valid(now, exp), now < exp);
            prop_assert_eq!(setup_token_usable(now, exp, used), !used && now < exp);
        }

        /// T17: a new setup token lives 15 minutes, never more.
        #[test]
        fn setup_tokens_live_15_minutes(now: i64) {
            let exp = setup_token_expiry(now);
            prop_assert!(exp >= now && i128::from(exp) - i128::from(now) <= 900);
            if now <= i64::MAX - 900 {
                prop_assert_eq!(exp, now + 900);
            }
        }
    }
}
