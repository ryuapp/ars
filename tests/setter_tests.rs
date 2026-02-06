#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

/// Tests for URL setter methods
use ars::Url;

fn parse(input: &str, base: Option<&str>) -> Result<Url, ars::ParseError> {
    Url::parse(input, base)
}

#[test]
fn test_set_protocol() {
    let mut url = parse("https://example.com/", None).unwrap();

    assert!(url.set_protocol("http"));
    assert_eq!(url.protocol(), "http:");
    assert_eq!(url.href(), "http://example.com/");

    // Should work with or without colon
    assert!(url.set_protocol("https:"));
    assert_eq!(url.protocol(), "https:");
}

#[test]
fn test_set_protocol_file_restriction() {
    let mut url = parse("file:///path", None).unwrap();

    // Can't change from file:
    assert!(!url.set_protocol("http"));
    assert_eq!(url.protocol(), "file:");

    let mut url2 = parse("http://example.com/", None).unwrap();

    // Can't change to file:
    assert!(!url2.set_protocol("file"));
    assert_eq!(url2.protocol(), "http:");
}

#[test]
fn test_set_username() {
    let mut url = parse("https://example.com/", None).unwrap();

    assert!(url.set_username("user"));
    assert_eq!(url.username(), "user");
    assert_eq!(url.href(), "https://user@example.com/");
}

#[test]
fn test_set_password() {
    let mut url = parse("https://user@example.com/", None).unwrap();

    assert!(url.set_password("pass"));
    assert_eq!(url.password(), "pass");
    assert_eq!(url.href(), "https://user:pass@example.com/");
}

#[test]
fn test_set_password_without_username() {
    let mut url = parse("https://example.com/", None).unwrap();

    // Can't set password without username
    assert!(!url.set_password("pass"));
}

#[test]
fn test_set_hostname() {
    let mut url = parse("https://example.com:8080/", None).unwrap();

    assert!(url.set_hostname("newhost.com"));
    assert_eq!(url.hostname(), "newhost.com");
    assert_eq!(url.port(), "8080"); // Port should be preserved
    assert_eq!(url.href(), "https://newhost.com:8080/");
}

#[test]
fn test_set_port() {
    let mut url = parse("https://example.com/", None).unwrap();

    assert!(url.set_port("8080"));
    assert_eq!(url.port(), "8080");
    assert_eq!(url.href(), "https://example.com:8080/");

    // Remove port
    assert!(url.set_port(""));
    assert_eq!(url.port(), "");
    assert_eq!(url.href(), "https://example.com/");
}

#[test]
fn test_set_pathname() {
    let mut url = parse("https://example.com/old", None).unwrap();

    assert!(url.set_pathname("/new/path"));
    assert_eq!(url.pathname(), "/new/path");
    assert_eq!(url.href(), "https://example.com/new/path");
}

#[test]
fn test_set_search() {
    let mut url = parse("https://example.com/", None).unwrap();

    url.set_search("query=value");
    assert_eq!(url.search(), "?query=value");
    assert_eq!(url.href(), "https://example.com/?query=value");

    // Remove search
    url.set_search("");
    assert_eq!(url.search(), "");
    assert_eq!(url.href(), "https://example.com/");
}

#[test]
fn test_set_hash() {
    let mut url = parse("https://example.com/", None).unwrap();

    url.set_hash("section");
    assert_eq!(url.hash(), "#section");
    assert_eq!(url.href(), "https://example.com/#section");

    // Remove hash
    url.set_hash("");
    assert_eq!(url.hash(), "");
    assert_eq!(url.href(), "https://example.com/");
}

#[test]
fn test_set_href() {
    let mut url = parse("https://example.com/", None).unwrap();

    assert!(url.set_href("http://newsite.com/path?query#hash").is_ok());
    assert_eq!(url.protocol(), "http:");
    assert_eq!(url.hostname(), "newsite.com");
    assert_eq!(url.pathname(), "/path");
    assert_eq!(url.search(), "?query");
    assert_eq!(url.hash(), "#hash");
}

#[test]
fn test_chained_setters() {
    let mut url = parse("https://example.com/", None).unwrap();

    url.set_username("user");
    url.set_password("pass");
    url.set_port("8080");
    url.set_pathname("/api/v1");
    url.set_search("key=value");
    url.set_hash("top");

    assert_eq!(
        url.href(),
        "https://user:pass@example.com:8080/api/v1?key=value#top"
    );
}

#[test]
fn test_set_search_with_existing_hash() {
    let mut url = parse("https://example.com/#hash", None).unwrap();

    url.set_search("query");
    assert_eq!(url.href(), "https://example.com/?query#hash");
}

#[test]
fn test_set_hash_with_existing_search() {
    let mut url = parse("https://example.com/?query", None).unwrap();

    url.set_hash("hash");
    assert_eq!(url.href(), "https://example.com/?query#hash");
}
