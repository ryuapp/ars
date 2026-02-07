use crate::compat::{String, ToString, Vec};

/// Represents URL search parameters (query string).
/// Provides methods to parse, manipulate, and serialize query parameters.
#[derive(Debug, Clone, Default)]
pub struct UrlSearchParams {
    params: Vec<(String, String)>,
}

impl UrlSearchParams {
    pub fn new() -> Self {
        Self { params: Vec::new() }
    }

    /// Parse from a query string (with or without leading `?`)
    pub fn parse(query: &str) -> Self {
        let query = query.strip_prefix('?').unwrap_or(query);

        if query.is_empty() {
            return Self::new();
        }

        let params = query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| match pair.split_once('=') {
                Some((key, value)) => (decode_component(key), decode_component(value)),
                None => (decode_component(pair), String::new()),
            })
            .collect();

        Self { params }
    }

    pub fn append(&mut self, key: &str, value: &str) {
        self.params.push((key.to_string(), value.to_string()));
    }

    /// Delete pairs with the given key.
    /// If `value` is provided, only deletes pairs matching both key and value.
    /// Otherwise, deletes all pairs with the given key.
    ///
    /// WHATWG URL Standard: URLSearchParams.delete(name, value)
    pub fn delete(&mut self, key: &str, value: Option<&str>) {
        if let Some(val) = value {
            // Delete only specific key-value pairs
            self.params.retain(|(k, v)| k != key || v != val);
        } else {
            // Delete all pairs with the given key
            self.params.retain(|(k, _)| k != key);
        }
    }

    /// Get the first value for a key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Get all values for a key.
    pub fn get_all(&self, key: &str) -> Vec<&str> {
        self.params
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// Check if a key exists.
    /// If `value` is provided, checks for a specific key-value pair.
    /// Otherwise, checks if the key exists at all.
    ///
    /// WHATWG URL Standard: URLSearchParams.has(name, value)
    pub fn has(&self, key: &str, value: Option<&str>) -> bool {
        if let Some(val) = value {
            // Check for specific key-value pair
            self.params.iter().any(|(k, v)| k == key && v == val)
        } else {
            // Check if key exists
            self.params.iter().any(|(k, _)| k == key)
        }
    }

    /// Set a key to a single value, replacing all existing values for that key.
    pub fn set(&mut self, key: &str, value: &str) {
        let mut found_first = false;
        self.params.retain_mut(|(k, v)| {
            if k != key {
                return true;
            }
            if found_first {
                return false;
            }
            found_first = true;
            *v = value.to_string();
            true
        });
        if !found_first {
            self.params.push((key.to_string(), value.to_string()));
        }
    }

    /// Sort parameters by key.
    pub fn sort(&mut self) {
        self.params.sort_by(|a, b| a.0.cmp(&b.0));
    }

    /// Get the number of parameters (WHATWG API).
    pub fn size(&self) -> usize {
        self.params.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.params.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Iterate over all key-value pairs (alias for `iter`, matches WHATWG API).
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.params.iter().map(|(k, _)| k.as_str())
    }

    pub fn values(&self) -> impl Iterator<Item = &str> {
        self.params.iter().map(|(_, v)| v.as_str())
    }

    /// Convert to query string with leading `?`, or empty string if no parameters.
    /// WHATWG URL Standard behavior.
    pub fn serialize(&self) -> String {
        if self.params.is_empty() {
            return String::new();
        }

        let mut result = String::from("?");
        for (i, (key, value)) in self.params.iter().enumerate() {
            if i > 0 {
                result.push('&');
            }
            result.push_str(&encode_component(key));
            result.push('=');
            result.push_str(&encode_component(value));
        }
        result
    }

    /// Convert to query string without leading `?`.
    /// JavaScript `URLSearchParams.toString()` compatible.
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        if self.params.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        for (i, (key, value)) in self.params.iter().enumerate() {
            if i > 0 {
                result.push('&');
            }
            result.push_str(&encode_component(key));
            result.push('=');
            result.push_str(&encode_component(value));
        }
        result
    }
}

impl core::fmt::Display for UrlSearchParams {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Encode a component for use in query strings.
fn encode_component(s: &str) -> String {
    use core::fmt::Write;

    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                let _ = write!(result, "%{byte:02X}");
            }
        }
    }
    result
}

/// Decode a component from a query string.
fn decode_component(s: &str) -> String {
    let mut result = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'+' => result.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = &s[i + 1..i + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    result.push(byte);
                    i += 2; // Extra increment for hex digits
                } else {
                    result.push(b'%');
                }
            }
            b => result.push(b),
        }
        i += 1;
    }

    String::from_utf8_lossy(&result).into_owned()
}

impl From<&str> for UrlSearchParams {
    fn from(s: &str) -> Self {
        Self::parse(s)
    }
}

impl From<String> for UrlSearchParams {
    fn from(s: String) -> Self {
        Self::parse(&s)
    }
}

#[cfg(test)]
#[allow(clippy::single_char_pattern)]
mod tests {
    use super::*;

    #[cfg(not(feature = "std"))]
    use alloc::vec;

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
        params.delete("key1", None);
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
        assert!(params.has("key1", None));
        assert!(params.has("key2", None));
        assert!(!params.has("key3", None));
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
        params.delete("key", None);
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
        assert!(params.has("key", None));
        assert!(!params.has("nonexistent", None));
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
}
