/// WPT tests for URL.canParse()
///
/// Based on: https://github.com/web-platform-tests/wpt/blob/master/url/url-statics-canparse.any.js
use ars::Url;

#[derive(Debug)]
struct CanParseTest {
    url: Option<&'static str>,
    base: Option<&'static str>,
    expected: bool,
}

const CAN_PARSE_TESTS: &[CanParseTest] = &[
    // undefined, undefined -> false
    CanParseTest {
        url: None,
        base: None,
        expected: false,
    },
    // "aaa:b", undefined -> true
    CanParseTest {
        url: Some("aaa:b"),
        base: None,
        expected: true,
    },
    // undefined, "aaa:b" -> false
    CanParseTest {
        url: None,
        base: Some("aaa:b"),
        expected: false,
    },
    // undefined, "https://test:test/" -> false
    CanParseTest {
        url: None,
        base: Some("https://test:test/"),
        expected: false,
    },
    // "aaa:/b", undefined -> true
    CanParseTest {
        url: Some("aaa:/b"),
        base: None,
        expected: true,
    },
    // undefined, "aaa:/b" -> true
    CanParseTest {
        url: None,
        base: Some("aaa:/b"),
        expected: true,
    },
    // "https://test:test", undefined -> false (invalid port)
    CanParseTest {
        url: Some("https://test:test"),
        base: None,
        expected: false,
    },
    // "a", "https://b/" -> true
    CanParseTest {
        url: Some("a"),
        base: Some("https://b/"),
        expected: true,
    },
];

#[test]
#[allow(clippy::unwrap_used)]
fn test_wpt_canparse_suite() {
    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    for (idx, test) in CAN_PARSE_TESTS.iter().enumerate() {
        // Convert None to empty string to match JavaScript undefined behavior
        let url_str = test.url.unwrap_or("");
        let base_str = test.base;

        let result = Url::can_parse(url_str, base_str);

        if result == test.expected {
            passed += 1;
        } else {
            failed += 1;
            failures.push(format!(
                "Test {}: URL.canParse({:?}, {:?}) = {} (expected {})",
                idx, test.url, test.base, result, test.expected
            ));
        }
    }

    if !failures.is_empty() {
        eprintln!("\nFailed can_parse tests:");
        for failure in &failures {
            eprintln!("  {}", failure);
        }
    }

    println!("\ncan_parse WPT results:");
    println!(
        "Passed: {}, Failed: {}, Total: {}",
        passed,
        failed,
        CAN_PARSE_TESTS.len()
    );
    println!(
        "Pass rate: {:.2}%",
        (passed as f64 / CAN_PARSE_TESTS.len() as f64) * 100.0
    );

    assert_eq!(
        failed, 0,
        "Failed {} can_parse WPT tests. See output above for details.",
        failed
    );
}
