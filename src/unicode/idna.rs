use crate::compat::String;
use crate::error::{ParseError, Result};

/// Check if 4 bytes match "xn--" (case insensitive)
fn is_punycode_prefix(slice: &[u8]) -> bool {
    slice.len() >= 4
        && matches!(slice[0], b'x' | b'X')
        && matches!(slice[1], b'n' | b'N')
        && slice[2] == b'-'
        && slice[3] == b'-'
}

/// Check if domain contains Punycode (xn-- prefix, case insensitive)
pub fn has_punycode(domain: &str) -> bool {
    let bytes = domain.as_bytes();
    if bytes.len() < 4 {
        return false;
    }

    // Check if starts with xn--
    if is_punycode_prefix(bytes) {
        return true;
    }

    // Check for .xn-- patterns using memchr for faster scanning
    memchr::memchr_iter(b'.', bytes).any(|pos| is_punycode_prefix(&bytes[pos + 1..]))
}

/// Process a domain using IDNA `ToASCII` algorithm
pub fn domain_to_ascii(domain: &str) -> Result<String> {
    // Fast path: Pure ASCII without percent-encoding or Punycode
    // Most common case - avoid expensive IDNA processing
    // Skip fast path for Punycode (xn--) as it needs validation
    if domain.is_ascii() && !domain.contains('%') && !has_punycode(domain) {
        // Quick validation and lowercase conversion
        let mut result = String::with_capacity(domain.len());

        for b in domain.bytes() {
            match b {
                // Valid hostname chars: a-z, A-Z, 0-9, ., -
                b'A'..=b'Z' => result.push((b + 32) as char), // Lowercase
                b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' => result.push(b as char),
                _ => return Err(ParseError::InvalidHost),
            }
        }

        return Ok(result);
    }

    // Slow path: Unicode, percent-encoded, or Punycode - use full IDNA processing
    idna::domain_to_ascii(domain).map_err(|_| ParseError::IdnaError)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_to_ascii() {
        // ASCII domain should pass through
        assert_eq!(domain_to_ascii("example.com").unwrap(), "example.com");

        // Unicode domain should be converted
        let result = domain_to_ascii("日本.jp");
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with("xn--"));
    }
}
