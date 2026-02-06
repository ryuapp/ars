//! Advanced URL parsing tests
//!
//! This module contains tests for advanced parsing scenarios including:
//! - Internationalized Domain Names (IDN/Punycode)
//! - Percent-encoded characters and normalization
//! - Edge cases (invalid characters, special patterns)
//! - Batch copy optimizations
//! - Complex URL structures (credentials, ports, subdomains)
//! - Fast path vs slow path consistency

use ars::Url;

fn parse(input: &str, base: Option<&str>) -> Result<Url, ars::ParseError> {
    Url::parse(input, base)
}

#[test]
fn test_percent_encoded_dots_normalization() {
    // %2e should be decoded to . and normalized
    let url = parse("http://example.com/%2e/path", None).unwrap();
    assert_eq!(url.pathname(), "/path");

    // %2E (uppercase) should also work
    let url = parse("http://example.com/%2E/path", None).unwrap();
    assert_eq!(url.pathname(), "/path");

    // %2e%2e should be decoded to .. and normalized
    let url = parse("http://example.com/%2e%2e/path", None).unwrap();
    assert_eq!(url.pathname(), "/path");

    // Mixed case
    let url = parse("http://example.com/%2E%2e/path", None).unwrap();
    assert_eq!(url.pathname(), "/path");

    // Multiple levels
    let url = parse("http://example.com/a/b/%2e%2e/c", None).unwrap();
    assert_eq!(url.pathname(), "/a/c");
}

#[test]
fn test_invalid_hostname_characters() {
    // Space should fail
    assert!(parse("http://a b/", None).is_err());
    assert!(parse("http://foo bar.com/", None).is_err());

    // Angle brackets should fail
    assert!(parse("http://a<b", None).is_err());
    assert!(parse("http://a>b", None).is_err());

    // Square brackets (not IPv6) should fail
    assert!(parse("http://a[b/", None).is_err());
    assert!(parse("http://a]b/", None).is_err());

    // Caret should fail
    assert!(parse("http://a^b", None).is_err());

    // Pipe should fail
    assert!(parse("http://a|b/", None).is_err());

    // Note: Tabs and newlines are stripped by WHATWG spec before parsing
    // So we test other control characters instead

    // Backspace (control character)
    assert!(parse("http://a\x08b/", None).is_err());
}

#[test]
fn test_valid_hostname_characters() {
    // Lowercase letters, digits, dots, dashes
    let url = parse("http://example-123.com/", None).unwrap();
    assert_eq!(url.host(), "example-123.com");

    // Uppercase should be lowercased
    let url = parse("http://EXAMPLE.COM/", None).unwrap();
    assert_eq!(url.host(), "example.com");

    // Mixed case
    let url = parse("http://ExAmPlE.CoM/", None).unwrap();
    assert_eq!(url.host(), "example.com");

    // Subdomain with dashes
    let url = parse("http://sub-domain.example.com/", None).unwrap();
    assert_eq!(url.host(), "sub-domain.example.com");
}

#[test]
fn test_long_wikipedia_style_paths() {
    // Wikipedia-style URLs with underscores
    let url = parse("https://en.wikipedia.org/wiki/Article_Name", None).unwrap();
    assert_eq!(url.pathname(), "/wiki/Article_Name");

    // With percent-encoded characters
    let url = parse("https://en.wikipedia.org/wiki/Law_%26_Order", None).unwrap();
    assert_eq!(url.pathname(), "/wiki/Law_%26_Order");

    // With UTF-8 percent encoding
    let url = parse("https://en.wikipedia.org/wiki/Caf%C3%A9", None).unwrap();
    assert_eq!(url.pathname(), "/wiki/Caf%C3%A9");

    // Long path
    let url = parse(
        "https://en.wikipedia.org/wiki/This_Is_A_Very_Long_Article_Name_With_Many_Words",
        None,
    )
    .unwrap();
    assert_eq!(
        url.pathname(),
        "/wiki/This_Is_A_Very_Long_Article_Name_With_Many_Words"
    );
}

#[test]
fn test_ipv4_detection() {
    // Valid IPv4 should fail in fast path
    let url = parse("http://192.168.1.1/", None).unwrap();
    assert_eq!(url.host(), "192.168.1.1");

    // Invalid IPv4-like patterns should fail
    assert!(parse("http://999.999.999.999/", None).is_err());
    assert!(parse("http://foo.1.2.3/", None).is_err());
    assert!(parse("http://1.2.3.4.5/", None).is_err());

    // Hostname ending with digit should work
    let url = parse("http://example1.com/", None).unwrap();
    assert_eq!(url.host(), "example1.com");
}

#[test]
fn test_punycode_detection() {
    // ASCII hostname without xn-- prefix
    let url = parse("http://example.com/", None).unwrap();
    assert_eq!(url.host(), "example.com");

    // Hostname with xn-- should be processed by slow path
    let url = parse("http://xn--e1afmkfd.xn--p1ai/", None).unwrap();
    assert_eq!(url.host(), "xn--e1afmkfd.xn--p1ai");

    // Case insensitive xn-- detection
    let url = parse("http://XN--e1afmkfd.com/", None).unwrap();
    assert_eq!(url.host(), "xn--e1afmkfd.com");

    // Subdomain with punycode
    let url = parse("http://test.xn--e1afmkfd.com/", None).unwrap();
    assert_eq!(url.host(), "test.xn--e1afmkfd.com");
}

#[test]
fn test_batch_copy_optimization() {
    // Long hostname that benefits from batch copying
    let url = parse("http://very-long-subdomain-name.example-domain.com/", None).unwrap();
    assert_eq!(url.host(), "very-long-subdomain-name.example-domain.com");

    // Long path that benefits from batch copying
    let url = parse(
        "http://example.com/this/is/a/very/long/path/with/many/segments",
        None,
    )
    .unwrap();
    assert_eq!(
        url.pathname(),
        "/this/is/a/very/long/path/with/many/segments"
    );

    // Mixed case hostname should still batch copy lowercase parts
    let url = parse("http://Example.Com/", None).unwrap();
    assert_eq!(url.host(), "example.com");
}

#[test]
fn test_path_with_special_characters() {
    // Tilde is valid
    let url = parse("http://example.com/~user/", None).unwrap();
    assert_eq!(url.pathname(), "/~user/");

    // Dash and underscore
    let url = parse("http://example.com/path-name_here/", None).unwrap();
    assert_eq!(url.pathname(), "/path-name_here/");

    // Percent signs in path (already encoded)
    let url = parse("http://example.com/path%20with%20spaces/", None).unwrap();
    assert_eq!(url.pathname(), "/path%20with%20spaces/");

    // Dots in path (not dot segments)
    let url = parse("http://example.com/file.txt", None).unwrap();
    assert_eq!(url.pathname(), "/file.txt");
}

#[test]
fn test_fast_path_vs_slow_path_consistency() {
    // Simple URLs should use fast path
    let fast = parse("http://example.com/path", None).unwrap();
    assert_eq!(fast.href(), "http://example.com/path");

    // URLs with credentials use slow path
    let slow = parse("http://user:pass@example.com/path", None).unwrap();
    assert_eq!(slow.href(), "http://user:pass@example.com/path");

    // Both should produce same result for common components
    assert_eq!(fast.host(), "example.com");
    assert_eq!(slow.host(), "example.com");
    assert_eq!(fast.pathname(), "/path");
    assert_eq!(slow.pathname(), "/path");
}

#[test]
fn test_edge_cases() {
    // Single character hostname
    let url = parse("http://x/", None).unwrap();
    assert_eq!(url.host(), "x");

    // Single character path
    let url = parse("http://example.com/a", None).unwrap();
    assert_eq!(url.pathname(), "/a");

    // Empty path (default /)
    let url = parse("http://example.com", None).unwrap();
    assert_eq!(url.pathname(), "/");

    // Path with trailing slash
    let url = parse("http://example.com/path/", None).unwrap();
    assert_eq!(url.pathname(), "/path/");

    // Multiple consecutive slashes in path
    let url = parse("http://example.com//path//", None).unwrap();
    assert_eq!(url.pathname(), "//path//");
}

#[test]
fn test_japanese_domain_names() {
    // Japanese government domain: 総務省.jp → xn--lhr645fjve.jp
    let url = parse("https://総務省.jp/", None).unwrap();
    assert_eq!(url.host(), "xn--lhr645fjve.jp");
    assert_eq!(url.href(), "https://xn--lhr645fjve.jp/");

    // Japanese domain with subdomain
    let url = parse("https://www.総務省.jp/", None).unwrap();
    assert_eq!(url.host(), "www.xn--lhr645fjve.jp");

    // Japanese domain with port
    let url = parse("http://総務省.jp:8080/", None).unwrap();
    assert_eq!(url.hostname(), "xn--lhr645fjve.jp");
    assert_eq!(url.port(), "8080");

    // Japanese domain with credentials
    let url = parse("http://user:pass@総務省.jp/", None).unwrap();
    assert_eq!(url.host(), "xn--lhr645fjve.jp");
    assert_eq!(url.username(), "user");
    assert_eq!(url.password(), "pass");
}

#[test]
fn test_unicode_domain_idna_processing() {
    // German umlaut
    let url = parse("http://münchen.de/", None).unwrap();
    assert_eq!(url.host(), "xn--mnchen-3ya.de");

    // French accent
    let url = parse("http://café.fr/", None).unwrap();
    assert_eq!(url.host(), "xn--caf-dma.fr");

    // Already encoded Punycode should pass through
    let url = parse("http://xn--wgv71a.jp/", None).unwrap();
    assert_eq!(url.host(), "xn--wgv71a.jp");

    // Mixed case Punycode
    let url = parse("http://XN--wgv71a.jp/", None).unwrap();
    assert_eq!(url.host(), "xn--wgv71a.jp");
}
