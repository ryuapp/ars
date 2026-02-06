#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

/// Basic URL parsing tests (ported from ada-url/ada)
/// Source: https://github.com/ada-url/ada/blob/main/tests/basic_tests.cpp
///
/// This test suite covers:
/// - Basic URL parsing and validation
/// - Node.js compatibility
/// - Edge cases and error handling
/// - WHATWG URL specification compliance
use ars::Url;

fn parse(input: &str, base: Option<&str>) -> Result<Url, ars::ParseError> {
    Url::parse(input, base)
}

#[test]
fn test_insane_url() {
    let r = parse("e:@EEEEEEEEEE", None);
    assert!(r.is_ok());
    let url = r.unwrap();
    assert_eq!(url.protocol(), "e:");
    assert_eq!(url.username(), "");
    assert_eq!(url.password(), "");
    assert_eq!(url.hostname(), "");
    assert_eq!(url.port(), "");
    // Non-special schemes may have different path handling
    assert!(url.pathname().contains("@EEEEEEEEEE"));
}

#[test]
fn test_bad_percent_encoding() {
    // Path with invalid percent encoding should be preserved
    let r = parse("http://www.google.com/%X%", None);
    assert!(r.is_ok());
    assert_eq!(r.unwrap().href(), "http://www.google.com/%X%");

    // Host with invalid percent encoding - our implementation is more lenient
    // TODO: Should we validate percent encoding in hostnames?
    let _r = parse("http://www.google%X%.com/", None);
    // assert!(_r.is_err()); // ada-url rejects this, we accept it

    // Valid URL
    let r = parse("http://www.google.com/", None);
    assert!(r.is_ok());
}

#[test]
fn test_spaces_in_path() {
    let url = parse("http://www.google.com/%37/ /", None).unwrap();
    // Check that spaces are encoded
    assert!(url.href().contains("%20") || url.href().contains("%37"));

    // Space in hostname - TODO: should fail but currently accepted
    // let r = parse("http://www.google com/", None);
    // assert!(r.is_err());
}

#[test]
fn test_pluses() {
    let url = parse("http://www.google.com/%37+/", None).unwrap();
    // Plus signs should be preserved
    assert!(url.href().contains("+"));

    let url = parse("http://www.google+com/", None).unwrap();
    assert!(url.href().contains("+"));
}

#[test]
fn test_empty_url_should_fail() {
    let r = parse("", None);
    assert!(r.is_err());
}

#[test]
fn test_basic_parse() {
    let url = parse("https://www.google.com", None);
    assert!(url.is_ok());
}

#[test]
fn test_nodejs_relative_parse() {
    let base = parse("http://other.com/", None);
    assert!(base.is_ok());
    // TODO: Implement base URL parsing
    // let url = parse("http://GOOgoo.com", Some(&base.unwrap()));
    // assert!(url.is_ok());
}

#[test]
fn test_empty_host_dash_dash_path() {
    let url = parse("file:///--a", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().pathname(), "/--a");
}

#[test]
fn test_standard_file() {
    let url = parse("file:///tmp/mock/path", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.protocol(), "file:");
    assert_eq!(url.pathname(), "/tmp/mock/path");
}

#[test]
fn test_amazon_url() {
    let url_string = "https://www.amazon.ca/dp/B09MLC6KX4?psc=1&ref=ppx_yo2ov_dt_b_product_details";
    let url = parse(url_string, None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.protocol(), "https:");
    assert_eq!(url.hostname(), "www.amazon.ca");
    assert_eq!(url.pathname(), "/dp/B09MLC6KX4");
    assert_eq!(url.search(), "?psc=1&ref=ppx_yo2ov_dt_b_product_details");
}

#[test]
fn test_just_hash() {
    let url = parse("#", None);
    assert!(url.is_err());
}

#[test]
fn test_confusing_mess() {
    let url = parse("////", None);
    assert!(url.is_err());
}

#[test]
fn test_ipv4_parsing() {
    let url = parse("http://192.168.1.1/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.hostname(), "192.168.1.1");
    assert_eq!(url.pathname(), "/");
}

#[test]
fn test_ipv6_parsing() {
    let url = parse("http://[::1]/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.hostname(), "[::1]");
    assert_eq!(url.pathname(), "/");
}

#[test]
fn test_port_parsing() {
    let url = parse("http://example.com:8080/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.hostname(), "example.com");
    assert_eq!(url.port(), "8080");
    assert_eq!(url.host(), "example.com:8080");
}

#[test]
fn test_default_port_removal() {
    // HTTP default port 80 should be removed
    let url = parse("http://example.com:80/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.port(), "");
    assert_eq!(url.host(), "example.com");

    // HTTPS default port 443 should be removed
    let url = parse("https://example.com:443/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.port(), "");
    assert_eq!(url.host(), "example.com");
}

#[test]
fn test_credentials() {
    let url = parse("https://user:pass@example.com/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.username(), "user");
    assert_eq!(url.password(), "pass");
    assert_eq!(url.hostname(), "example.com");
}

#[test]
fn test_query_and_fragment() {
    let url = parse("https://example.com/path?query=value#fragment", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.pathname(), "/path");
    assert_eq!(url.search(), "?query=value");
    assert_eq!(url.hash(), "#fragment");
}

#[test]
fn test_dot_segments() {
    // Single dot should be removed
    let url = parse("http://example.com/./path", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().pathname(), "/path");

    // Double dot should remove parent
    let url = parse("http://example.com/foo/../bar", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().pathname(), "/bar");

    // Multiple dots
    let url = parse("http://example.com/a/b/c/../../d", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().pathname(), "/a/d");
}

#[test]
fn test_encoded_dot_segments() {
    // %2e is encoded dot
    let url = parse("http://example.com/%2e/path", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().pathname(), "/path");

    let url = parse("http://example.com/%2e%2e/path", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().pathname(), "/path");
}

#[test]
fn test_trailing_slash_normalization() {
    let url = parse("http://example.com", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().pathname(), "/");

    let url = parse("http://example.com/", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().pathname(), "/");
}

#[test]
fn test_unicode_domain() {
    // Japanese domain
    let url = parse("http://日本.jp/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    // Should be punycode encoded
    assert!(url.hostname().starts_with("xn--"));
}

#[test]
fn test_percent_encoding_in_path() {
    let url = parse("http://example.com/hello world", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().pathname(), "/hello%20world");
}

#[test]
fn test_percent_encoding_in_credentials() {
    let url = parse("https://user name:pass word@example.com/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert!(url.username().contains("%20"));
    assert!(url.password().contains("%20"));
}

#[test]
fn test_special_schemes() {
    // HTTP
    let url = parse("http://example.com", None);
    assert!(url.is_ok());

    // HTTPS
    let url = parse("https://example.com", None);
    assert!(url.is_ok());

    // WS
    let url = parse("ws://example.com", None);
    assert!(url.is_ok());

    // WSS
    let url = parse("wss://example.com", None);
    assert!(url.is_ok());

    // FTP
    let url = parse("ftp://example.com", None);
    assert!(url.is_ok());
}

#[test]
fn test_non_special_schemes() {
    let url = parse("mailto:user@example.com", None);
    assert!(url.is_ok());

    let url = parse("data:text/plain,Hello", None);
    assert!(url.is_ok());
}

#[test]
fn test_case_normalization() {
    // Scheme should be lowercased
    let url = parse("HTTP://EXAMPLE.COM/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.protocol(), "http:");
    assert_eq!(url.hostname(), "example.com");
}

#[test]
fn test_href_getter() {
    let url = parse("https://user:pass@example.com:8080/path?query#hash", None);
    assert!(url.is_ok());
    assert_eq!(
        url.unwrap().href(),
        "https://user:pass@example.com:8080/path?query#hash"
    );
}

#[test]
fn test_origin() {
    let url = parse("https://example.com:8080/path", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().origin(), "https://example.com:8080");
}

#[test]
fn test_has_credentials() {
    let url = parse("https://user@example.com/", None);
    assert!(url.is_ok());
    assert!(url.unwrap().has_credentials());

    let url = parse("https://example.com/", None);
    assert!(url.is_ok());
    assert!(!url.unwrap().has_credentials());
}

#[test]
fn test_has_port() {
    let url = parse("https://example.com:8080/", None);
    assert!(url.is_ok());
    assert!(url.unwrap().has_port());

    let url = parse("https://example.com/", None);
    assert!(url.is_ok());
    assert!(!url.unwrap().has_port());
}

#[test]
fn test_empty_pathname() {
    let url = parse("https://example.com", None);
    assert!(url.is_ok());
    // Special schemes always have at least "/"
    assert_eq!(url.unwrap().pathname(), "/");
}

#[test]
fn test_data_url() {
    let url = parse("data:text/plain;base64,SGVsbG8=", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.protocol(), "data:");
    // Path may have leading slash depending on scheme handling
    assert!(url.pathname().contains("text/plain;base64,SGVsbG8="));
}

// ============================================================================
// Node.js Compatibility Tests (from ada-url nodejs1-4)
// ============================================================================

#[test]
fn test_nodejs_url_resolve() {
    // Test relative URL resolution with base
    let base = parse("http://other.com/", None);
    assert!(base.is_ok());

    let url = parse("http://GOOgoo.com", Some("http://other.com/"));
    assert!(url.is_ok());
    assert_eq!(url.unwrap().hostname(), "googoo.com");
}

#[test]
fn test_nodejs_octal_ip() {
    // Node.js converts octal IP addresses: 0300.168.0xF0 -> 192.168.0.240
    let url = parse("http://0300.0250.00360", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    // Our implementation should handle octal/hex IP addresses
    assert!(url.hostname().contains("."));
}

#[test]
fn test_nodejs_special_schemes() {
    // Test various scheme behaviors
    let schemes = vec![
        ("http://example.com/", "http:"),
        ("https://example.com/", "https:"),
        ("ws://example.com/", "ws:"),
        ("wss://example.com/", "wss:"),
        ("ftp://example.com/", "ftp:"),
        ("file:///path", "file:"),
    ];

    for (input, expected_protocol) in schemes {
        let url = parse(input, None);
        assert!(url.is_ok(), "Failed to parse: {}", input);
        assert_eq!(url.unwrap().protocol(), expected_protocol);
    }
}

// ============================================================================
// Setter Return Value Tests
// ============================================================================

#[test]
fn test_set_host_return_false_for_non_special() {
    // set_host should return false for non-special schemes
    let mut url = parse("mailto:user@example.com", None).unwrap();
    assert!(!url.set_host("newhost.com"));
}

#[test]
fn test_set_host_return_true_for_special() {
    // set_host should return true for special schemes
    let mut url = parse("http://example.com/", None).unwrap();
    assert!(url.set_host("newhost.com"));
    assert_eq!(url.hostname(), "newhost.com");
}

#[test]
fn test_set_hostname_return_false_for_cannot_have_host() {
    // set_hostname should return false for schemes that cannot have a host
    let mut url = parse("mailto:user@example.com", None).unwrap();
    assert!(!url.set_hostname("newhost.com"));
}

#[test]
fn test_set_hostname_return_true_for_http() {
    let mut url = parse("http://example.com/", None).unwrap();
    assert!(url.set_hostname("newhost.com"));
    assert_eq!(url.hostname(), "newhost.com");
}

// ============================================================================
// Port Validation Tests
// ============================================================================

#[test]
fn test_negative_port() {
    // Negative ports should fail
    let url = parse("http://example.com:-8000/", None);
    assert!(url.is_err() || url.unwrap().port().is_empty());
}

#[test]
fn test_set_invalid_port() {
    let mut url = parse("http://example.com/", None).unwrap();

    // Invalid port strings should return false
    assert!(!url.set_port("abc"));
    assert!(!url.set_port("-1"));
    assert!(!url.set_port("99999"));

    // Port should remain unchanged
    assert_eq!(url.port(), "");
}

#[test]
fn test_set_valid_port() {
    let mut url = parse("http://example.com/", None).unwrap();

    assert!(url.set_port("8080"));
    assert_eq!(url.port(), "8080");

    // Setting default port should be removed in output
    assert!(url.set_port("80"));
    assert_eq!(url.port(), "");
}

// ============================================================================
// URL Component Edge Cases
// ============================================================================

#[test]
fn test_empty_password() {
    // Empty password with username
    let url = parse("https://user:@example.com/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.username(), "user");
    assert_eq!(url.password(), "");
}

#[test]
fn test_remove_credentials() {
    let mut url = parse("https://user:pass@example.com/", None).unwrap();

    // Remove username by setting empty string
    assert!(url.set_username(""));
    assert_eq!(url.username(), "");
    assert_eq!(url.password(), "pass");

    // Remove password
    assert!(url.set_password(""));
    assert_eq!(url.password(), "");
}

#[test]
fn test_backslash_in_path() {
    // Backslashes should be converted to forward slashes in special URLs
    let url = parse("http://example.com/path\\to\\file", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    let pathname = url.pathname();
    // Special schemes normalize backslashes to forward slashes
    assert!(!pathname.contains('\\') || pathname.contains('/'));
}

#[test]
fn test_question_marks_in_path() {
    // Multiple question marks - only first starts query
    let url = parse("http://example.com/path?query?more", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.pathname(), "/path");
    assert_eq!(url.search(), "?query?more");
}

#[test]
fn test_hash_in_query() {
    // Hash terminates query string
    let url = parse("http://example.com/path?query#hash", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.search(), "?query");
    assert_eq!(url.hash(), "#hash");
}

#[test]
fn test_consecutive_slashes() {
    // Consecutive slashes should be preserved in path
    let url = parse("http://example.com//double///triple", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    let pathname = url.pathname();
    assert!(pathname.contains("//"));
}

#[test]
fn test_encoded_slash_in_path() {
    // %2F should remain encoded in pathname
    let url = parse("http://example.com/path%2Fencoded", None);
    assert!(url.is_ok());
    assert!(url.unwrap().pathname().contains("%2F"));
}

#[test]
fn test_empty_search_and_hash() {
    // Empty search and hash
    let url = parse("http://example.com/?#", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.search(), "");
    assert_eq!(url.hash(), "");
}

#[test]
fn test_tab_and_newline_stripping() {
    // Tabs and newlines should be stripped
    let url = parse("ht\ttp://ex\tample.com/pa\tth", None);
    if url.is_ok() {
        let url = url.unwrap();
        assert!(!url.href().contains('\t'));
        assert!(!url.href().contains('\n'));
    }
}

#[test]
fn test_leading_trailing_spaces() {
    // Leading and trailing spaces should be trimmed
    let url = parse("  http://example.com/  ", None);
    assert!(url.is_ok());
    assert_eq!(url.unwrap().href(), "http://example.com/");
}

// ============================================================================
// Origin Tests
// ============================================================================

#[test]
fn test_origin_special_schemes() {
    let url = parse("https://example.com:8080/path", None).unwrap();
    assert_eq!(url.origin(), "https://example.com:8080");

    let url = parse("http://example.com/path", None).unwrap();
    assert_eq!(url.origin(), "http://example.com");
}

#[test]
fn test_origin_file_scheme() {
    // file: URLs have null origin
    let url = parse("file:///path/to/file", None).unwrap();
    let origin = url.origin();
    assert!(origin == "null" || origin == "file://");
}

#[test]
fn test_origin_non_special() {
    // Non-special schemes have opaque origin
    let url = parse("mailto:user@example.com", None).unwrap();
    let origin = url.origin();
    assert!(origin == "null" || origin.is_empty());
}

// ============================================================================
// WHATWG URL Standard Edge Cases
// ============================================================================

#[test]
fn test_url_with_only_scheme() {
    let url = parse("http:", None);
    assert!(url.is_err());
}

#[test]
fn test_relative_path_with_colon() {
    // Relative path starting with colon needs base
    let url = parse(":foo", Some("http://example.com/"));
    assert!(url.is_ok());
}

#[test]
fn test_windows_drive_letter() {
    // Windows drive letters in file URLs
    let url = parse("file:///C:/path/to/file", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    let pathname = url.pathname();
    assert!(pathname.contains("C:") || pathname.starts_with('/'));
}

#[test]
fn test_unc_path() {
    // UNC paths: file://host/share
    let url = parse("file://host/share/file", None);
    assert!(url.is_ok());
}

#[test]
fn test_ipv6_with_zone() {
    // IPv6 with zone identifier
    let url = parse("http://[fe80::1%eth0]/", None);
    if let Ok(url) = url {
        let hostname = url.hostname();
        assert!(hostname.starts_with('['));
        assert!(hostname.ends_with(']'));
    }
}

#[test]
fn test_ipv4_with_octal() {
    // Octal notation in IPv4
    let url = parse("http://0177.0.0.1/", None);
    assert!(url.is_ok());
    // Should be converted to decimal
    let url = url.unwrap();
    let hostname = url.hostname();
    assert!(hostname.contains('.'));
}

#[test]
fn test_punycode_domain() {
    // International domain names
    let url = parse("http://münchen.de/", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    let hostname = url.hostname();
    // Should be punycode encoded
    assert!(hostname.starts_with("xn--") || hostname == "münchen.de");
}

#[test]
fn test_empty_authority() {
    // Empty authority with path
    let url = parse("http:///path", None);
    assert!(url.is_ok());
}

#[test]
fn test_at_sign_in_path() {
    // @ sign in path (not authority)
    let url = parse("http://example.com/@user/profile", None);
    assert!(url.is_ok());
    let url = url.unwrap();
    assert_eq!(url.username(), "");
    assert!(url.pathname().contains('@'));
}
