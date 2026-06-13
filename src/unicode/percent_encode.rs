use crate::compat::{String, Vec};
use crate::error::{ParseError, Result};

#[derive(Clone, Copy)]
pub struct AsciiSet {
    mask: [u8; 16],
}

impl AsciiSet {
    pub const fn new() -> Self {
        Self { mask: [0; 16] }
    }

    pub const fn add(mut self, byte: u8) -> Self {
        if byte < 128 {
            let index = (byte / 8) as usize;
            let bit = byte % 8;
            self.mask[index] |= 1 << bit;
        }
        self
    }

    const fn contains(self, byte: u8) -> bool {
        if byte >= 128 {
            false
        } else {
            let index = (byte / 8) as usize;
            let bit = byte % 8;
            (self.mask[index] & (1 << bit)) != 0
        }
    }
}

const fn controls() -> AsciiSet {
    let mut set = AsciiSet::new();
    let mut byte = 0;
    while byte < 0x20 {
        set = set.add(byte);
        byte += 1;
    }
    set.add(0x7f)
}

// Define encode sets following WHATWG URL spec
// Based on https://url.spec.whatwg.org/#percent-encoded-bytes

/// C0 control percent-encode set
pub const C0_CONTROL_SET: AsciiSet = controls();

/// Fragment percent-encode set
/// C0 control + space, ", <, >, \`
pub const FRAGMENT_SET: AsciiSet = C0_CONTROL_SET
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`');

/// Path percent-encode set
/// Fragment + #, ?, ^, \`, {, }
/// Per WHATWG spec and WPT test #736: ^ must be encoded in paths
pub const PATH_SET: AsciiSet = FRAGMENT_SET
    .add(b'#')
    .add(b'?')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// Opaque path percent-encode set (for non-special schemes with authority)
/// `PATH_SET` + ^ (per WPT test expectations)
pub const OPAQUE_PATH_SET: AsciiSet = PATH_SET.add(b'^');

/// Userinfo percent-encode set
/// Path + /, :, ;, =, @, [, \, ], ^, |
pub const USERINFO_SET: AsciiSet = PATH_SET
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
pub fn percent_encode_with_set(input: &str, encode_set: AsciiSet) -> String {
    let mut buffer = String::new();
    percent_encode_into(&mut buffer, input, encode_set);
    buffer
}

/// Write percent-encoded string directly to buffer
/// Manually iterates to avoid write! macro overhead
pub fn percent_encode_into(buffer: &mut String, input: &str, encode_set: AsciiSet) {
    // Reserve space to reduce reallocations
    buffer.reserve(input.len());

    for &byte in input.as_bytes() {
        if byte >= 128 || encode_set.contains(byte) {
            buffer.push('%');
            buffer.push(hex_digit(byte >> 4));
            buffer.push(hex_digit(byte & 0x0f));
        } else {
            buffer.push(byte as char);
        }
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
pub const SPECIAL_QUERY_SET: AsciiSet = C0_CONTROL_SET
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'\'');

/// Query percent-encode set (for non-special URLs)
/// C0 control + space, ", #, <, >
/// Note: Does NOT encode single quote ' (different from `SPECIAL_QUERY_SET`)
pub const QUERY_SET: AsciiSet = C0_CONTROL_SET
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
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                output.push(bytes[index]);
                index += 1;
                continue;
            }

            let Some(high) = hex_value(bytes[index + 1]) else {
                output.push(bytes[index]);
                index += 1;
                continue;
            };
            let Some(low) = hex_value(bytes[index + 2]) else {
                output.push(bytes[index]);
                index += 1;
                continue;
            };

            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(output).map_err(|_| ParseError::InvalidPercentEncoding)
}

const fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'A' + (nibble - 10)) as char,
    }
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
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

    #[test]
    fn test_percent_encode_non_ascii_as_utf8_bytes() {
        assert_eq!(percent_encode_with_set("café", PATH_SET), "caf%C3%A9");
    }
}
