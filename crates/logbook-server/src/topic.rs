//! Topic names and URL slugs (spec 4.2).

use logbook_core::Topic;

/// Every topic, in sidebar order.
pub const ALL: [Topic; 6] = [
    Topic::Ethereum,
    Topic::Rust,
    Topic::Surf,
    Topic::Snowboarding,
    Topic::JiuJitsu,
    Topic::ClassicWow,
];

/// URL and database slug, for example `jiu-jitsu`.
#[must_use]
pub fn slug(t: Topic) -> &'static str {
    match t {
        Topic::Ethereum => "ethereum",
        Topic::Rust => "rust",
        Topic::Surf => "surf",
        Topic::Snowboarding => "snowboarding",
        Topic::JiuJitsu => "jiu-jitsu",
        Topic::ClassicWow => "classic-wow",
    }
}

/// Display name, for example `Jiu jitsu`.
#[must_use]
pub fn name(t: Topic) -> &'static str {
    match t {
        Topic::Ethereum => "Ethereum",
        Topic::Rust => "Rust",
        Topic::Surf => "Surf",
        Topic::Snowboarding => "Snowboarding",
        Topic::JiuJitsu => "Jiu jitsu",
        Topic::ClassicWow => "Classic WoW",
    }
}

/// Parses a slug. Unknown slugs give `None`.
#[must_use]
pub fn parse(s: &str) -> Option<Topic> {
    ALL.into_iter().find(|t| slug(*t) == s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_round_trip() {
        for t in ALL {
            assert_eq!(parse(slug(t)), Some(t));
        }
        assert_eq!(parse("nope"), None);
        assert_eq!(parse("Rust"), None);
    }
}
