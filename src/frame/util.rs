use std::fmt;

use super::Error;
use bytes::{Buf, Bytes};

/// RFC 3986 §3.1 scheme: `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`.
///
/// `http::uri::Scheme` is more permissive (empty and digit-leading tokens).
/// nghttp2 uses the same grammar in `check_scheme`.
pub(crate) fn is_valid_scheme(s: &str) -> bool {
    let mut bytes = s.as_bytes().iter().copied();
    match bytes.next() {
        Some(b) if b.is_ascii_alphabetic() => {}
        _ => return false,
    }
    bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

/// RFC 9113 §8.2.1: field values MUST NOT have leading or trailing SP/HTAB.
///
/// Recipients MUST discard or reject such values. `http::HeaderValue` accepts
/// them; nghttp2 rejects via `nghttp2_check_header_value_rfc9113`.
pub(crate) fn header_value_has_leading_trailing_ws(value: &[u8]) -> bool {
    match value {
        [] => false,
        [first, ..] if *first == b' ' || *first == b'\t' => true,
        [.., last] if *last == b' ' || *last == b'\t' => true,
        _ => false,
    }
}

/// RFC 9113 §8.3.1 / nghttp2: for http(s), `:path` is path-absolute (`/`…) or
/// OPTIONS asterisk-form (`*`). Query-only forms like `?q=1` (accepted by
/// `http::uri::PathAndQuery`) are not valid `:path` values.
pub(crate) fn is_valid_path(path: &str, is_options: bool) -> bool {
    if path.is_empty() {
        return false;
    }
    if path == "*" {
        return is_options;
    }
    path.as_bytes()[0] == b'/'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_grammar() {
        assert!(is_valid_scheme("http"));
        assert!(is_valid_scheme("https"));
        assert!(is_valid_scheme("HTTP"));
        assert!(is_valid_scheme("a"));
        assert!(is_valid_scheme("a+b-c.1"));
        assert!(!is_valid_scheme(""));
        assert!(!is_valid_scheme("1http"));
        assert!(!is_valid_scheme("+http"));
        assert!(!is_valid_scheme("ht!tp"));
    }

    #[test]
    fn header_value_leading_trailing_ws() {
        assert!(!header_value_has_leading_trailing_ws(b""));
        assert!(!header_value_has_leading_trailing_ws(b"ok"));
        assert!(!header_value_has_leading_trailing_ws(b"a b"));
        assert!(header_value_has_leading_trailing_ws(b" leading"));
        assert!(header_value_has_leading_trailing_ws(b"trailing "));
        assert!(header_value_has_leading_trailing_ws(b" both "));
        assert!(header_value_has_leading_trailing_ws(b"\t"));
        assert!(header_value_has_leading_trailing_ws(b"\tx"));
        assert!(header_value_has_leading_trailing_ws(b"x\t"));
    }

    #[test]
    fn path_form() {
        assert!(is_valid_path("/", false));
        assert!(is_valid_path("/a", false));
        assert!(is_valid_path("/?q=1", false));
        assert!(is_valid_path("//x", false));
        assert!(is_valid_path("*", true));
        assert!(!is_valid_path("*", false));
        assert!(!is_valid_path("", false));
        assert!(!is_valid_path("?q=1", false));
        assert!(!is_valid_path("?", false));
        assert!(!is_valid_path("foo", false));
    }
}

/// Strip padding from the given payload.
///
/// It is assumed that the frame had the padded flag set. This means that the
/// first byte is the length of the padding with that many
/// 0 bytes expected to follow the actual payload.
///
/// # Returns
///
/// A slice of the given payload where the actual one is found and the length
/// of the padding.
///
/// If the padded payload is invalid (e.g. the length of the padding is equal
/// to the total length), returns `None`.
pub fn strip_padding(payload: &mut Bytes) -> Result<u8, Error> {
    let payload_len = payload.len();
    if payload_len == 0 {
        // If this is the case, the frame is invalid as no padding length can be
        // extracted, even though the frame should be padded.
        return Err(Error::TooMuchPadding);
    }

    let pad_len = payload[0] as usize;

    if pad_len >= payload_len {
        // This is invalid: the padding length MUST be less than the
        // total frame size.
        return Err(Error::TooMuchPadding);
    }

    payload.advance(1);
    payload.truncate(payload_len - pad_len - 1);

    Ok(pad_len as u8)
}

pub(super) fn debug_flags<'a, 'f: 'a>(
    fmt: &'a mut fmt::Formatter<'f>,
    bits: u8,
) -> DebugFlags<'a, 'f> {
    let result = write!(fmt, "({:#x}", bits);
    DebugFlags {
        fmt,
        result,
        started: false,
    }
}

pub(super) struct DebugFlags<'a, 'f: 'a> {
    fmt: &'a mut fmt::Formatter<'f>,
    result: fmt::Result,
    started: bool,
}

impl<'a, 'f: 'a> DebugFlags<'a, 'f> {
    pub(super) fn flag_if(&mut self, enabled: bool, name: &str) -> &mut Self {
        if enabled {
            self.result = self.result.and_then(|()| {
                let prefix = if self.started {
                    " | "
                } else {
                    self.started = true;
                    ": "
                };

                write!(self.fmt, "{}{}", prefix, name)
            });
        }
        self
    }

    pub(super) fn finish(&mut self) -> fmt::Result {
        self.result.and_then(|()| write!(self.fmt, ")"))
    }
}
