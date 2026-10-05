//! Which address is the client (spec 6.6, theorem T15).
//!
//! The server parses the `X-Forwarded-For` hops and checks each address against
//! the trusted proxy networks. This module picks the address from those results.

/// The address to use as the client IP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientAddr {
    /// The direct peer of the connection.
    Peer,
    /// The `X-Forwarded-For` hop at this index, counted from the left.
    Hop(usize),
}

/// Picks the client address (T15).
///
/// - `peer_trusted`: the direct peer is a trusted proxy.
/// - `hop_trusted[i]`: hop `i` of `X-Forwarded-For`, from the left, is a trusted proxy.
///
/// If the peer is not trusted, the result is the peer, and the header has no effect.
/// If the peer is trusted, the result is the rightmost hop that is not trusted, or
/// the peer if all hops are trusted.
#[must_use]
pub fn client_addr(peer_trusted: bool, hop_trusted: &[bool]) -> ClientAddr {
    if !peer_trusted {
        return ClientAddr::Peer;
    }
    let mut i = hop_trusted.len();
    while i > 0 {
        i -= 1;
        if !hop_trusted[i] {
            return ClientAddr::Hop(i);
        }
    }
    ClientAddr::Peer
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn examples() {
        assert_eq!(client_addr(false, &[false, false]), ClientAddr::Peer);
        assert_eq!(client_addr(true, &[]), ClientAddr::Peer);
        assert_eq!(client_addr(true, &[false, true]), ClientAddr::Hop(0));
        assert_eq!(client_addr(true, &[false, false, true]), ClientAddr::Hop(1));
        assert_eq!(client_addr(true, &[true, true]), ClientAddr::Peer);
    }

    proptest! {
        /// T15.
        #[test]
        fn picks_the_rightmost_untrusted_hop(peer: bool, hops in prop::collection::vec(any::<bool>(), 0..20)) {
            let want = if peer {
                hops.iter().rposition(|t| !t).map_or(ClientAddr::Peer, ClientAddr::Hop)
            } else {
                ClientAddr::Peer
            };
            prop_assert_eq!(client_addr(peer, &hops), want);
        }
    }
}
