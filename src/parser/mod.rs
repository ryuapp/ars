mod parse_aggregator;
mod state;

pub use parse_aggregator::{parse_url_aggregator, validate_url};
pub use state::State;

use crate::error::Result;

/// Trait for types that can be parsed from URL strings
pub trait Parseable: Sized {
    /// Parse from input string with optional base URL
    fn parse(input: &str, base: Option<&str>) -> Result<Self>;
}

/// Parse a URL string into a URL type
pub fn parse<T: Parseable>(input: &str, base: Option<&str>) -> Result<T> {
    T::parse(input, base)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::Url;

    #[test]
    fn test_parse_basic() {
        let result = parse::<Url>("http://example.com", None);
        match result {
            Ok(url) => {
                assert_eq!(url.protocol(), "http:");
                assert_eq!(url.hostname(), "example.com");
                assert_eq!(url.pathname(), "/");
            }
            Err(e) => {
                panic!("Failed to parse: {e:?}");
            }
        }
    }

    #[test]
    fn test_parse_with_path() {
        let url = parse::<Url>("http://example.com/path/to/resource", None).unwrap();
        assert_eq!(url.protocol(), "http:");
        assert_eq!(url.hostname(), "example.com");
        assert_eq!(url.pathname(), "/path/to/resource");
    }

    #[test]
    fn test_parse_with_query() {
        let url = parse::<Url>("http://example.com/path?query=value&foo=bar", None).unwrap();
        assert_eq!(url.protocol(), "http:");
        assert_eq!(url.hostname(), "example.com");
        assert_eq!(url.pathname(), "/path");
        assert_eq!(url.search(), "?query=value&foo=bar");
    }

    #[test]
    fn test_parse_with_hash() {
        let url = parse::<Url>("http://example.com/path#fragment", None).unwrap();
        assert_eq!(url.protocol(), "http:");
        assert_eq!(url.hostname(), "example.com");
        assert_eq!(url.pathname(), "/path");
        assert_eq!(url.hash(), "#fragment");
    }

    #[test]
    fn test_parse_with_port() {
        let url = parse::<Url>("http://example.com:8080/path", None).unwrap();
        assert_eq!(url.protocol(), "http:");
        assert_eq!(url.host(), "example.com:8080");
        assert_eq!(url.hostname(), "example.com");
        assert_eq!(url.port(), "8080");
        assert_eq!(url.pathname(), "/path");
    }

    #[test]
    fn test_parse_with_credentials() {
        let url = parse::<Url>("http://user:pass@example.com/path", None).unwrap();
        assert_eq!(url.protocol(), "http:");
        assert_eq!(url.username(), "user");
        assert_eq!(url.password(), "pass");
        assert_eq!(url.hostname(), "example.com");
        assert_eq!(url.pathname(), "/path");
    }

    #[test]
    fn test_parse_https() {
        let url = parse::<Url>("https://secure.example.com", None).unwrap();
        assert_eq!(url.protocol(), "https:");
        assert_eq!(url.hostname(), "secure.example.com");
    }

    #[test]
    fn test_parse_ipv4() {
        let url = parse::<Url>("http://192.168.1.1/path", None).unwrap();
        assert_eq!(url.protocol(), "http:");
        assert_eq!(url.hostname(), "192.168.1.1");
        assert_eq!(url.pathname(), "/path");
    }

    #[test]
    fn test_parse_ipv6() {
        let url = parse::<Url>("http://[2001:db8::1]/path", None).unwrap();
        assert_eq!(url.protocol(), "http:");
        assert!(url.hostname().contains("2001"));
        assert_eq!(url.pathname(), "/path");
    }

    #[test]
    fn test_parse_relative_with_base() {
        let url = parse::<Url>("/relative/path", Some("http://example.com/base")).unwrap();
        assert_eq!(url.protocol(), "http:");
        assert_eq!(url.hostname(), "example.com");
        assert_eq!(url.pathname(), "/relative/path");
    }

    #[test]
    fn test_parse_complete_url() {
        let url =
            parse::<Url>("https://user:pass@example.com:8080/path?query=1#hash", None).unwrap();
        assert_eq!(url.protocol(), "https:");
        assert_eq!(url.username(), "user");
        assert_eq!(url.password(), "pass");
        assert_eq!(url.host(), "example.com:8080");
        assert_eq!(url.hostname(), "example.com");
        assert_eq!(url.port(), "8080");
        assert_eq!(url.pathname(), "/path");
        assert_eq!(url.search(), "?query=1");
        assert_eq!(url.hash(), "#hash");
        assert_eq!(
            url.href(),
            "https://user:pass@example.com:8080/path?query=1#hash"
        );
    }
}
