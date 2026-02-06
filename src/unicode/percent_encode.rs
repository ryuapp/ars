use crate::compat::{String, ToString};
use crate::error::{ParseError, Result};
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

// Define encode sets following WHATWG URL spec
// Based on https://url.spec.whatwg.org/#percent-encoded-bytes

/// C0 control percent-encode set
pub const C0_CONTROL_SET: &AsciiSet = CONTROLS;

/// Fragment percent-encode set
/// C0 control + space, ", <, >, \`
pub const FRAGMENT_SET: &AsciiSet = &C0_CONTROL_SET
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`');

/// Path percent-encode set
/// Fragment + #, ?, ^, \`, {, }
/// Per WHATWG spec and WPT test #736: ^ must be encoded in paths
pub const PATH_SET: &AsciiSet = &FRAGMENT_SET
    .add(b'#')
    .add(b'?')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// Opaque path percent-encode set (for non-special schemes with authority)
/// `PATH_SET` + ^ (per WPT test expectations)
pub const OPAQUE_PATH_SET: &AsciiSet = &PATH_SET.add(b'^');

/// Userinfo percent-encode set
/// Path + /, :, ;, =, @, [, \, ], ^, |
pub const USERINFO_SET: &AsciiSet = &PATH_SET
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'|');

/// Percent-encode a string using the provided encode set
pub fn percent_encode_with_set(input: &str, encode_set: &'static AsciiSet) -> String {
    utf8_percent_encode(input, encode_set).to_string()
}

/// Write percent-encoded string directly to buffer
/// Manually iterates to avoid write! macro overhead
pub fn percent_encode_into(buffer: &mut String, input: &str, encode_set: &'static AsciiSet) {
    // Reserve space to reduce reallocations
    buffer.reserve(input.len());

    // Encode as needed
    for chunk in utf8_percent_encode(input, encode_set) {
        buffer.push_str(chunk);
    }
}

/// Percent-encode path directly into buffer (zero-copy if no encoding needed)
pub fn percent_encode_path_into(buffer: &mut String, input: &str) {
    percent_encode_into(buffer, input, PATH_SET);
}

/// Percent-encode for userinfo
pub fn percent_encode_userinfo(input: &str) -> String {
    percent_encode_with_set(input, USERINFO_SET)
}

/// Special query percent-encode set (for special URLs like http, https, etc.)
/// C0 control + space, ", #, <, >, '
/// Note: Does NOT encode backtick \`, {, }, etc. (different from `FRAGMENT_SET`)
pub const SPECIAL_QUERY_SET: &AsciiSet = &C0_CONTROL_SET
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'\'');

/// Query percent-encode set (for non-special URLs)
/// C0 control + space, ", #, <, >
/// Note: Does NOT encode single quote ' (different from `SPECIAL_QUERY_SET`)
pub const QUERY_SET: &AsciiSet = &C0_CONTROL_SET
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>');

/// Percent-encode fragment directly into buffer
pub fn percent_encode_fragment_into(buffer: &mut String, input: &str) {
    percent_encode_into(buffer, input, FRAGMENT_SET);
}

/// Percent-encode userinfo directly into buffer (zero-copy if no encoding needed)
pub fn percent_encode_userinfo_into(buffer: &mut String, input: &str) {
    percent_encode_into(buffer, input, USERINFO_SET);
}

/// Decode percent-encoded string
pub fn percent_decode(input: &str) -> Result<String> {
    percent_encoding::percent_decode_str(input)
        .decode_utf8()
        .map(Into::into)
        .map_err(|_| ParseError::InvalidPercentEncoding)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("hello%20world").unwrap(), "hello world");
        assert_eq!(percent_decode("test").unwrap(), "test");
        assert_eq!(percent_decode("%2F").unwrap(), "/");
        assert_eq!(percent_decode("%C3%A9").unwrap(), "é");
    }
}
