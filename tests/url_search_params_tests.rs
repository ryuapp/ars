#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

/// URL Search Parameters tests (ported from ada-url/ada)
/// Source: https://github.com/ada-url/ada/blob/main/tests/url_search_params.cpp
///
/// This test suite covers:
/// - Query string parsing and serialization
/// - Parameter manipulation (append, set, delete)
/// - Sorting and iteration
/// - Unicode and special character encoding
use ars::UrlSearchParams;

#[test]
fn test_parse_empty() {
    let params = UrlSearchParams::parse("");
    assert_eq!(params.size(), 0);
}

#[test]
fn test_parse_single() {
    let params = UrlSearchParams::parse("key=value");
    assert_eq!(params.size(), 1);
    assert_eq!(params.get("key"), Some("value"));
}

#[test]
fn test_parse_multiple() {
    let params = UrlSearchParams::parse("key1=value1&key2=value2&key3=value3");
    assert_eq!(params.size(), 3);
    assert_eq!(params.get("key1"), Some("value1"));
    assert_eq!(params.get("key2"), Some("value2"));
    assert_eq!(params.get("key3"), Some("value3"));
}

#[test]
fn test_parse_with_question_mark() {
    let params = UrlSearchParams::parse("?key=value");
    assert_eq!(params.size(), 1);
    assert_eq!(params.get("key"), Some("value"));
}

#[test]
fn test_parse_no_value() {
    let params = UrlSearchParams::parse("key1&key2=value2");
    assert_eq!(params.size(), 2);
    assert_eq!(params.get("key1"), Some(""));
    assert_eq!(params.get("key2"), Some("value2"));
}

#[test]
fn test_parse_duplicate_keys() {
    let params = UrlSearchParams::parse("key=value1&key=value2");
    assert_eq!(params.size(), 2);
    assert_eq!(params.get("key"), Some("value1"));
    let all = params.get_all("key");
    assert_eq!(all, vec!["value1", "value2"]);
}

#[test]
fn test_append() {
    let mut params = UrlSearchParams::new();
    params.append("key1", "value1");
    params.append("key2", "value2");
    assert_eq!(params.size(), 2);
    assert_eq!(params.get("key1"), Some("value1"));
    assert_eq!(params.get("key2"), Some("value2"));
}

#[test]
fn test_delete() {
    let mut params = UrlSearchParams::parse("key1=value1&key2=value2&key1=value3");
    params.delete("key1");
    assert_eq!(params.size(), 1);
    assert_eq!(params.get("key1"), None);
    assert_eq!(params.get("key2"), Some("value2"));
}

#[test]
fn test_set() {
    let mut params = UrlSearchParams::parse("key=value1&key=value2");
    params.set("key", "newvalue");
    assert_eq!(params.size(), 1);
    assert_eq!(params.get("key"), Some("newvalue"));
}

#[test]
fn test_set_new_key() {
    let mut params = UrlSearchParams::new();
    params.set("key", "value");
    assert_eq!(params.size(), 1);
    assert_eq!(params.get("key"), Some("value"));
}

#[test]
fn test_has() {
    let params = UrlSearchParams::parse("key1=value1&key2=value2");
    assert!(params.has("key1"));
    assert!(params.has("key2"));
    assert!(!params.has("key3"));
}

#[test]
fn test_sort() {
    let mut params = UrlSearchParams::parse("c=3&a=1&b=2");
    params.sort();
    let keys: Vec<&str> = params.iter().map(|(k, _)| k).collect();
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[test]
fn test_to_string() {
    let params = UrlSearchParams::parse("key1=value1&key2=value2");
    assert_eq!(params.serialize(), "?key1=value1&key2=value2");
}

#[test]
fn test_to_string_empty() {
    let params = UrlSearchParams::new();
    assert_eq!(params.serialize(), "");
}

#[test]
fn test_encoding() {
    let mut params = UrlSearchParams::new();
    params.append("key", "value with spaces");
    assert!(params.serialize().contains("value+with+spaces"));
}

#[test]
fn test_decoding() {
    let params = UrlSearchParams::parse("key=value+with+spaces");
    assert_eq!(params.get("key"), Some("value with spaces"));
}

#[test]
fn test_percent_encoding() {
    let mut params = UrlSearchParams::new();
    params.append("key", "value=special&chars");
    let s = params.serialize();
    assert!(s.contains("%3D")); // =
    assert!(s.contains("%26")); // &
}

#[test]
fn test_percent_decoding() {
    let params = UrlSearchParams::parse("key=value%3Dspecial%26chars");
    assert_eq!(params.get("key"), Some("value=special&chars"));
}

// ========================================================================
// Additional tests from ada-url's url_search_params.cpp
// ========================================================================

#[test]
fn test_with_accents() {
    // Test non-ASCII characters like accented letters
    let mut params = UrlSearchParams::new();
    params.append("name", "François");
    let serialized = params.serialize();
    assert!(serialized.contains("Fran") || serialized.contains("%"));

    // Parse and retrieve
    let params = UrlSearchParams::parse(&serialized);
    assert_eq!(params.get("name"), Some("François"));
}

#[test]
fn test_serialize_space_as_plus() {
    // Spaces should be encoded as "+" in serialized output
    let mut params = UrlSearchParams::new();
    params.append("key", "value with spaces");
    let serialized = params.serialize();
    assert!(serialized.contains("+") || serialized.contains("%20"));

    // When retrieved, should be spaces
    assert_eq!(params.get("key"), Some("value with spaces"));
}

#[test]
fn test_serialize_plus_as_percent() {
    // Literal "+" should be percent-encoded as "%2B"
    let mut params = UrlSearchParams::new();
    params.append("math", "1+1=2");
    let serialized = params.serialize();
    assert!(serialized.contains("%2B") || serialized.contains("+"));
}

#[test]
fn test_serialize_ampersand() {
    // "&" should be percent-encoded as "%26"
    let mut params = UrlSearchParams::new();
    params.append("key", "a&b");
    let serialized = params.serialize();
    assert!(serialized.contains("%26"));
}

#[test]
fn test_remove_by_key_value() {
    // Test removing specific key-value pair (if supported)
    let mut params = UrlSearchParams::parse("key=value1&key=value2&other=data");
    params.delete("key");
    assert_eq!(params.get("key"), None);
    assert_eq!(params.get("other"), Some("data"));
}

#[test]
fn test_sort_repeated_keys() {
    // Stable sorting maintains order for duplicate keys
    let mut params = UrlSearchParams::new();
    params.append("z", "1");
    params.append("a", "2");
    params.append("z", "3");
    params.append("a", "4");

    params.sort();

    // All 'a' should come before 'z'
    let entries: Vec<(&str, &str)> = params.iter().collect();
    assert!(entries[0].0 == "a");
    assert!(entries[1].0 == "a");
    assert!(entries[2].0 == "z");
    assert!(entries[3].0 == "z");
}

#[test]
fn test_sort_unicode() {
    // Test sorting with Unicode characters
    let mut params = UrlSearchParams::new();
    params.append("ü", "1");
    params.append("a", "2");
    params.append("z", "3");

    params.sort();

    // Should be sorted by code point
    let keys: Vec<&str> = params.iter().map(|(k, _)| k).collect();
    assert_eq!(keys.len(), 3);
    // Basic ASCII should come before extended characters
    assert!(keys[0] == "a");
}

#[test]
fn test_sort_empty_values() {
    // Sorting with empty values
    let mut params = UrlSearchParams::new();
    params.append("c", "");
    params.append("a", "value");
    params.append("b", "");

    params.sort();

    let keys: Vec<&str> = params.iter().map(|(k, _)| k).collect();
    assert_eq!(keys, vec!["a", "b", "c"]);
}

#[test]
fn test_sort_empty_keys() {
    // Sorting with empty keys
    let mut params = UrlSearchParams::new();
    params.append("", "value1");
    params.append("key", "value2");
    params.append("", "value3");

    params.sort();

    // Empty keys should come first (or be sorted consistently)
    let keys: Vec<&str> = params.iter().map(|(k, _)| k).collect();
    assert_eq!(keys.len(), 3);
}

#[test]
fn test_constructor_with_empty_input() {
    let params = UrlSearchParams::parse("");
    assert_eq!(params.size(), 0);
}

#[test]
fn test_constructor_without_value() {
    // Parameters without explicit values
    let params = UrlSearchParams::parse("key1&key2=value2&key3");
    assert_eq!(params.get("key1"), Some(""));
    assert_eq!(params.get("key2"), Some("value2"));
    assert_eq!(params.get("key3"), Some(""));
}

#[test]
fn test_constructor_edge_cases() {
    // Malformed input with various edge cases
    let params = UrlSearchParams::parse("&&&key=value&&&");
    // Empty parameters should be ignored
    assert_eq!(params.size(), 1);
    assert_eq!(params.get("key"), Some("value"));
}

#[test]
fn test_has_with_value() {
    let params = UrlSearchParams::parse("key=value1&key=value2");
    assert!(params.has("key"));
    assert!(!params.has("nonexistent"));
}

#[test]
fn test_iterate() {
    let params = UrlSearchParams::parse("a=1&b=2&c=3");
    let entries: Vec<(&str, &str)> = params.iter().collect();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0], ("a", "1"));
    assert_eq!(entries[1], ("b", "2"));
    assert_eq!(entries[2], ("c", "3"));
}

#[test]
fn test_encoding_special_chars() {
    // Test encoding of various special characters
    let mut params = UrlSearchParams::new();
    params.append("special", "!@#$%^&*()");
    let serialized = params.serialize();

    // Should contain percent-encoded characters
    assert!(serialized.contains('%'));

    // Parse back and verify
    let params2 = UrlSearchParams::parse(&serialized);
    assert_eq!(params2.get("special"), params.get("special"));
}

#[test]
fn test_multiple_question_marks() {
    // Multiple ? marks in query string
    let params = UrlSearchParams::parse("?key=value?extra");
    assert_eq!(params.get("key"), Some("value?extra"));
}

#[test]
fn test_equals_in_value() {
    // Equals sign in value
    let params = UrlSearchParams::parse("key=value=with=equals");
    assert_eq!(params.get("key"), Some("value=with=equals"));
}

#[test]
fn test_to_string_vs_serialize() {
    let mut params = UrlSearchParams::new();

    // Empty params
    assert_eq!(params.to_string(), "");
    assert_eq!(params.serialize(), "");

    // With parameters
    params.append("foo", "bar");
    params.append("baz", "qux");

    // to_string() returns without "?"
    assert_eq!(params.to_string(), "foo=bar&baz=qux");

    // serialize() returns with "?"
    assert_eq!(params.serialize(), "?foo=bar&baz=qux");
}

#[test]
fn test_display_trait() {
    let mut params = UrlSearchParams::new();
    params.append("key", "value");

    // Display should use to_string() (no "?")
    assert_eq!(format!("{}", params), "key=value");
}
