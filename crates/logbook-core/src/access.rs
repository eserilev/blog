//! Route access levels and the owner check (spec 6.4, 6.6, theorem T16).

/// Who can call a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Anyone. The response never depends on the session.
    Public,
    /// Anyone. The response depends on the session, so it is `no-store`. Never 401.
    Session,
    /// Sign-in routes. Anyone, with a rate limit. `no-store`.
    Auth,
    /// The owner only: 401 without a valid session. `no-store`.
    Owner,
}

/// The session of a request, as the server finds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// The request has no session cookie.
    NoCookie,
    /// No session row has the hash of the cookie. Sign-out deletes the row, so a
    /// revoked session is in this state.
    Unknown,
    /// The session row exists, but its expiry time is not in the future.
    Expired,
    /// The session row exists and its expiry time is in the future.
    Valid,
}

/// The access decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The handler runs.
    Allow,
    /// 401. The handler does not run.
    Unauthorized,
}

/// The access decision for a route (T16). Owner routes need a valid session. The
/// other routes never give 401 here.
#[must_use]
pub fn authorize(level: Access, state: SessionState) -> Decision {
    // The parameter names differ from the module names `access` and `session`: the
    // Lean model puts both in one namespace.
    match level {
        Access::Owner => match state {
            SessionState::Valid => Decision::Allow,
            SessionState::NoCookie | SessionState::Unknown | SessionState::Expired => {
                Decision::Unauthorized
            }
        },
        Access::Public | Access::Session | Access::Auth => Decision::Allow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_ACCESS: [Access; 4] = [Access::Public, Access::Session, Access::Auth, Access::Owner];
    const ALL_SESSIONS: [SessionState; 4] = [
        SessionState::NoCookie,
        SessionState::Unknown,
        SessionState::Expired,
        SessionState::Valid,
    ];

    /// T16, for all 16 inputs.
    #[test]
    fn only_owner_routes_need_a_valid_session() {
        for a in ALL_ACCESS {
            for s in ALL_SESSIONS {
                let want = if a == Access::Owner && s != SessionState::Valid {
                    Decision::Unauthorized
                } else {
                    Decision::Allow
                };
                assert_eq!(authorize(a, s), want, "{a:?} {s:?}");
            }
        }
    }
}
