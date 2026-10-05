//! The CSRF decision for a request (spec 6.6, theorem T13).
//!
//! The server reads the method, the `Origin` header, the body headers, the
//! `Content-Type` header, and the path. This module makes the decision from them.

/// ASCII `A-Z` to `a-z`. Other bytes stay the same.
fn lower(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' { c + 32 } else { c }
}

/// A space or a tab: the only white space in a valid header value.
fn is_space(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

/// The first index at or after `i` that is not a space or a tab, or `s.len()`.
fn skip_spaces(s: &[u8], i: usize) -> usize {
    let mut j = i;
    while j < s.len() {
        if !is_space(s[j]) {
            return j;
        }
        j += 1;
    }
    j
}

/// `s[at..]` starts with `want`, with ASCII letters of `s` made lowercase.
/// `want` is lowercase.
fn starts_with_lower(s: &[u8], at: usize, want: &[u8]) -> bool {
    if at > s.len() || s.len() - at < want.len() {
        return false;
    }
    let mut j = 0;
    while j < want.len() {
        if lower(s[at + j]) != want[j] {
            return false;
        }
        j += 1;
    }
    true
}

/// True if and only if the media type of `content_type` is `application/json`.
///
/// The media type is the part before the first `;`, without spaces and tabs at
/// either end. Case does not matter. So the value is: spaces and tabs,
/// `application/json` in any case, spaces and tabs, then the end or a `;`.
#[must_use]
pub fn is_json(content_type: &[u8]) -> bool {
    let start = skip_spaces(content_type, 0);
    if !starts_with_lower(content_type, start, b"application/json") {
        return false;
    }
    let end = skip_spaces(content_type, start + 16);
    end == content_type.len() || content_type[end] == b';'
}

/// True if and only if `content_type` starts with `multipart/form-data`, in any case.
#[must_use]
pub fn is_multipart_form(content_type: &[u8]) -> bool {
    starts_with_lower(content_type, 0, b"multipart/form-data")
}

/// The CSRF decision (T13). True if the request can go to its handler.
///
/// - `is_read`: the method is `GET` or `HEAD`. A read is always allowed.
/// - `origin_matches`: the `Origin` header equals the site origin. A write without
///   it is always refused.
/// - `has_body`: the request has a body. A write with a body is allowed only as
///   `application/json`, or as `multipart/form-data` on the upload path.
/// - `content_type`: the `Content-Type` value, or empty if there is none.
/// - `is_upload_path`: the path is `/api/owner/uploads`.
// Plain parameters, not a struct: the theorem names each fact of the request.
#[allow(clippy::fn_params_excessive_bools)]
#[must_use]
pub fn write_allowed(
    is_read: bool,
    origin_matches: bool,
    has_body: bool,
    content_type: &[u8],
    is_upload_path: bool,
) -> bool {
    if is_read {
        return true;
    }
    if !origin_matches {
        return false;
    }
    if !has_body {
        return true;
    }
    is_json(content_type) || (is_upload_path && is_multipart_form(content_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// The rule as the server wrote it before T13, on `str`.
    fn json_reference(v: &str) -> bool {
        v.split(';')
            .next()
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("application/json"))
    }

    fn multipart_reference(v: &str) -> bool {
        v.to_ascii_lowercase().starts_with("multipart/form-data")
    }

    #[test]
    fn json_examples() {
        for v in [
            "application/json",
            "Application/JSON",
            "application/json; charset=utf-8",
            " \tapplication/json \t;x",
            "application/json;",
        ] {
            assert!(is_json(v.as_bytes()), "{v:?}");
        }
        for v in [
            "",
            "application/jso",
            "application/jsonx",
            "application/json x",
            "text/plain",
            "application/x-www-form-urlencoded",
            ";application/json",
            "application/ json",
        ] {
            assert!(!is_json(v.as_bytes()), "{v:?}");
        }
    }

    #[test]
    fn multipart_examples() {
        assert!(is_multipart_form(b"multipart/form-data; boundary=x"));
        assert!(is_multipart_form(b"Multipart/Form-Data"));
        assert!(!is_multipart_form(b" multipart/form-data"));
        assert!(!is_multipart_form(b"multipart/form-dat"));
    }

    #[test]
    fn decision_examples() {
        let json = b"application/json";
        let form = b"multipart/form-data; boundary=x";
        assert!(write_allowed(true, false, true, b"text/plain", false));
        assert!(!write_allowed(false, false, false, json, false));
        assert!(write_allowed(false, true, false, b"", false));
        assert!(write_allowed(false, true, true, json, false));
        assert!(!write_allowed(false, true, true, b"text/plain", false));
        assert!(!write_allowed(false, true, true, form, false));
        assert!(write_allowed(false, true, true, form, true));
    }

    /// Bytes that `HeaderValue::to_str` accepts: visible ASCII, space, tab.
    fn header_value() -> impl Strategy<Value = String> {
        prop_oneof![
            "[ \t]{0,3}(?i:application/json)[ \t]{0,3}(;[ -~]{0,10})?",
            "(?i:multipart/form-data)[ -~]{0,10}",
            "[ \t!-~]{0,40}",
        ]
    }

    proptest! {
        #[test]
        fn json_matches_the_old_rule(v in header_value()) {
            prop_assert_eq!(is_json(v.as_bytes()), json_reference(&v));
        }

        #[test]
        fn multipart_matches_the_old_rule(v in header_value()) {
            prop_assert_eq!(is_multipart_form(v.as_bytes()), multipart_reference(&v));
        }

        #[test]
        fn never_panics(v in prop::collection::vec(any::<u8>(), 0..60)) {
            let _ = is_json(&v);
            let _ = is_multipart_form(&v);
        }

        /// T13.
        #[test]
        fn decision_matches_the_rule(
            is_read: bool, origin: bool, body: bool, upload: bool, v in header_value()
        ) {
            let want = is_read
                || (origin && (!body || json_reference(&v) || (upload && multipart_reference(&v))));
            prop_assert_eq!(write_allowed(is_read, origin, body, v.as_bytes(), upload), want);
        }
    }
}
