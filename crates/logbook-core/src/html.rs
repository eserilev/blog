//! HTML escaping (spec 6.3, theorem T14).
//!
//! The function works on bytes. In UTF-8, no byte of a multi-byte character is
//! ASCII, so the function changes only the five ASCII characters below, and valid
//! UTF-8 in gives valid UTF-8 out.

/// Escapes text for element content and for attributes in single or double quotes:
/// `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`, `"` → `&quot;`, `'` → `&#39;`.
/// Every other byte stays the same.
#[must_use]
pub fn escape_html(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == b'&' {
            out.push(b'&');
            out.push(b'a');
            out.push(b'm');
            out.push(b'p');
            out.push(b';');
        } else if c == b'<' {
            out.push(b'&');
            out.push(b'l');
            out.push(b't');
            out.push(b';');
        } else if c == b'>' {
            out.push(b'&');
            out.push(b'g');
            out.push(b't');
            out.push(b';');
        } else if c == b'"' {
            out.push(b'&');
            out.push(b'q');
            out.push(b'u');
            out.push(b'o');
            out.push(b't');
            out.push(b';');
        } else if c == b'\'' {
            out.push(b'&');
            out.push(b'#');
            out.push(b'3');
            out.push(b'9');
            out.push(b';');
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// The escape function as the server wrote it before T14, on `str`.
    fn reference(s: &str) -> String {
        let mut out = String::new();
        for c in s.chars() {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&#39;"),
                _ => out.push(c),
            }
        }
        out
    }

    fn unescape(s: &str) -> String {
        s.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&")
    }

    #[test]
    fn examples() {
        assert_eq!(escape_html(b""), b"");
        assert_eq!(
            escape_html(b"<a href=\"x\">Tom & Jerry's</a>"),
            b"&lt;a href=&quot;x&quot;&gt;Tom &amp; Jerry&#39;s&lt;/a&gt;"
        );
        assert_eq!(
            escape_html("שלום & 🌊".as_bytes()),
            "שלום &amp; 🌊".as_bytes()
        );
    }

    proptest! {
        #[test]
        fn matches_the_old_escape(s in any::<String>()) {
            prop_assert_eq!(escape_html(s.as_bytes()), reference(&s).into_bytes());
        }

        /// T14.
        #[test]
        fn output_is_safe_and_reversible(s in any::<String>()) {
            let out = String::from_utf8(escape_html(s.as_bytes())).unwrap();
            prop_assert!(!out.contains(['<', '>', '"', '\'']));
            for (i, _) in out.match_indices('&') {
                let rest = &out[i..];
                prop_assert!(["&amp;", "&lt;", "&gt;", "&quot;", "&#39;"].iter().any(|e| rest.starts_with(e)));
            }
            prop_assert_eq!(unescape(&out), s);
        }

        #[test]
        fn any_bytes(s in prop::collection::vec(any::<u8>(), 0..100)) {
            let out = escape_html(&s);
            prop_assert!(!out.iter().any(|c| matches!(c, b'<' | b'>' | b'"' | b'\'')));
        }
    }
}
