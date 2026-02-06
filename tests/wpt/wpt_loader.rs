/// WPT (Web Platform Tests) loader and runner
///
/// This module loads WHATWG URL test data and runs compliance tests.
/// Test data format is based on: https://github.com/web-platform-tests/wpt/tree/master/url
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum TestCase {
    /// A URL test case
    UrlTest {
        #[serde(default)]
        input: String,
        #[serde(default)]
        base: Option<String>,
        #[serde(default)]
        href: Option<String>,
        #[serde(default)]
        protocol: Option<String>,
        #[serde(default)]
        username: Option<String>,
        #[serde(default)]
        password: Option<String>,
        #[serde(default)]
        host: Option<String>,
        #[serde(default)]
        hostname: Box<Option<String>>,
        #[serde(default)]
        port: Box<Option<String>>,
        #[serde(default)]
        pathname: Box<Option<String>>,
        #[serde(default)]
        search: Box<Option<String>>,
        #[serde(default)]
        hash: Box<Option<String>>,
        #[serde(default)]
        origin: Box<Option<String>>,
        #[serde(default)]
        failure: Option<bool>,
    },
    /// A comment line (string)
    Comment(String),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WptTestResult {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub failures: Vec<WptFailure>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WptFailure {
    pub test_num: usize,
    pub input: String,
    pub base: Option<String>,
    pub field: String,
    pub expected: String,
    pub actual: String,
}

impl Default for WptTestResult {
    fn default() -> Self {
        Self::new()
    }
}

impl WptTestResult {
    pub fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
        }
    }

    pub fn pass_rate(&self) -> f64 {
        let total = self.passed + self.failed;
        if total == 0 {
            0.0
        } else {
            (self.passed as f64 / total as f64) * 100.0
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "Passed: {}, Failed: {}, Skipped: {}, Pass Rate: {:.2}%",
            self.passed,
            self.failed,
            self.skipped,
            self.pass_rate()
        )
    }
}

/// Simplified inline test data for initial testing
/// This is a subset of the full WPT tests for quick validation
pub fn get_inline_tests() -> Vec<TestCase> {
    vec![
        TestCase::UrlTest {
            input: "http://example.com/".to_string(),
            base: None,
            href: Some("http://example.com/".to_string()),
            protocol: Some("http:".to_string()),
            username: Some("".to_string()),
            password: Some("".to_string()),
            host: Some("example.com".to_string()),
            hostname: Box::new(Some("example.com".to_string())),
            port: Box::new(Some("".to_string())),
            pathname: Box::new(Some("/".to_string())),
            search: Box::new(Some("".to_string())),
            hash: Box::new(Some("".to_string())),
            origin: Box::new(Some("http://example.com".to_string())),
            failure: None,
        },
        TestCase::UrlTest {
            input: "https://user:pass@secure.example.com:8080/path?query#hash".to_string(),
            base: None,
            href: Some("https://user:pass@secure.example.com:8080/path?query#hash".to_string()),
            protocol: Some("https:".to_string()),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            host: Some("secure.example.com:8080".to_string()),
            hostname: Box::new(Some("secure.example.com".to_string())),
            port: Box::new(Some("8080".to_string())),
            pathname: Box::new(Some("/path".to_string())),
            search: Box::new(Some("?query".to_string())),
            hash: Box::new(Some("#hash".to_string())),
            origin: Box::new(Some("https://secure.example.com:8080".to_string())),
            failure: None,
        },
        TestCase::UrlTest {
            input: "/relative".to_string(),
            base: Some("http://example.com/base/".to_string()),
            href: Some("http://example.com/relative".to_string()),
            protocol: Some("http:".to_string()),
            username: Some("".to_string()),
            password: Some("".to_string()),
            host: Some("example.com".to_string()),
            hostname: Box::new(Some("example.com".to_string())),
            port: Box::new(Some("".to_string())),
            pathname: Box::new(Some("/relative".to_string())),
            search: Box::new(Some("".to_string())),
            hash: Box::new(Some("".to_string())),
            origin: Box::new(Some("http://example.com".to_string())),
            failure: None,
        },
        TestCase::UrlTest {
            input: "http://192.168.1.1/".to_string(),
            base: None,
            href: Some("http://192.168.1.1/".to_string()),
            protocol: Some("http:".to_string()),
            username: Some("".to_string()),
            password: Some("".to_string()),
            host: Some("192.168.1.1".to_string()),
            hostname: Box::new(Some("192.168.1.1".to_string())),
            port: Box::new(Some("".to_string())),
            pathname: Box::new(Some("/".to_string())),
            search: Box::new(Some("".to_string())),
            hash: Box::new(Some("".to_string())),
            origin: Box::new(Some("http://192.168.1.1".to_string())),
            failure: None,
        },
        TestCase::UrlTest {
            input: "not a valid url".to_string(),
            base: None,
            href: None,
            protocol: None,
            username: None,
            password: None,
            host: None,
            hostname: Box::new(None),
            port: Box::new(None),
            pathname: Box::new(None),
            search: Box::new(None),
            hash: Box::new(None),
            origin: Box::new(None),
            failure: Some(true),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inline_tests() {
        let tests = get_inline_tests();
        assert_eq!(tests.len(), 5);
    }

    #[test]
    fn test_wpt_result() {
        let mut result = WptTestResult::new();
        result.passed = 80;
        result.failed = 20;

        assert_eq!(result.pass_rate(), 80.0);
        assert!(result.summary().contains("80.00%"));
    }
}
