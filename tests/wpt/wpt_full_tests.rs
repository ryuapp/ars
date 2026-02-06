#![allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]

/// Full WPT test suite runner
use ars::Url;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum TestCase {
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
    #[allow(dead_code)]
    Comment(String),
}

#[derive(Debug, Clone)]
struct WptTestResult {
    passed: usize,
    failed: usize,
    skipped: usize,
    failures: Vec<WptFailure>,
}

#[derive(Debug, Clone)]
struct WptFailure {
    test_num: usize,
    input: String,
    base: Option<String>,
    field: String,
    expected: String,
    actual: String,
}

impl WptTestResult {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
            skipped: 0,
            failures: Vec::new(),
        }
    }

    fn pass_rate(&self) -> f64 {
        let total = self.passed + self.failed;
        if total == 0 {
            0.0
        } else {
            (self.passed as f64 / total as f64) * 100.0
        }
    }

    fn summary(&self) -> String {
        format!(
            "Passed: {}, Failed: {}, Skipped: {}, Pass Rate: {:.2}%",
            self.passed,
            self.failed,
            self.skipped,
            self.pass_rate()
        )
    }
}

fn run_wpt_tests(tests: Vec<TestCase>) -> WptTestResult {
    let mut result = WptTestResult::new();
    let mut test_num = 0;

    for test in tests {
        match test {
            TestCase::Comment(_) => {}
            TestCase::UrlTest {
                input,
                base,
                href,
                protocol,
                username,
                password,
                host,
                hostname,
                port,
                pathname,
                search,
                hash,
                origin,
                failure,
            } => {
                test_num += 1;

                if failure == Some(true) {
                    match Url::parse(&input, base.as_deref()) {
                        Ok(_) => {
                            result.failed += 1;
                            result.failures.push(WptFailure {
                                test_num,
                                input: input.clone(),
                                base: base.clone(),
                                field: "parsing".to_string(),
                                expected: "failure".to_string(),
                                actual: "success".to_string(),
                            });
                        }
                        Err(_) => result.passed += 1,
                    }
                    continue;
                }

                let url = match Url::parse(&input, base.as_deref()) {
                    Ok(u) => u,
                    Err(_) => {
                        if href.is_some() {
                            result.failed += 1;
                            result.failures.push(WptFailure {
                                test_num,
                                input: input.clone(),
                                base: base.clone(),
                                field: "parsing".to_string(),
                                expected: "success".to_string(),
                                actual: "parse error".to_string(),
                            });
                        } else {
                            result.passed += 1;
                        }
                        continue;
                    }
                };

                let mut test_passed = true;

                if let Some(expected) = &href {
                    let actual = url.href();
                    if &actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "href".to_string(),
                            expected: expected.clone(),
                            actual: actual.to_string(),
                        });
                        test_passed = false;
                    }
                }

                if let Some(expected) = &protocol {
                    let actual = url.protocol();
                    if actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "protocol".to_string(),
                            expected: expected.clone(),
                            actual: actual.to_string(),
                        });
                        test_passed = false;
                    }
                }

                if let Some(expected) = &username {
                    let actual = url.username();
                    if actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "username".to_string(),
                            expected: expected.clone(),
                            actual: actual.to_string(),
                        });
                        test_passed = false;
                    }
                }

                if let Some(expected) = &password {
                    let actual = url.password();
                    if actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "password".to_string(),
                            expected: expected.clone(),
                            actual: actual.to_string(),
                        });
                        test_passed = false;
                    }
                }

                if let Some(expected) = &host {
                    let actual = url.host();
                    if &actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "host".to_string(),
                            expected: expected.clone(),
                            actual: actual.to_string(),
                        });
                        test_passed = false;
                    }
                }

                if let Some(expected) = hostname.as_deref() {
                    let actual = url.hostname();
                    if actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "hostname".to_string(),
                            expected: expected.to_string(),
                            actual: actual.to_string(),
                        });
                        test_passed = false;
                    }
                }

                if let Some(expected) = port.as_deref() {
                    let actual = url.port();
                    if actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "port".to_string(),
                            expected: expected.to_string(),
                            actual: actual.to_string(),
                        });
                        test_passed = false;
                    }
                }

                if let Some(expected) = pathname.as_deref() {
                    let actual = url.pathname();
                    if actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "pathname".to_string(),
                            expected: expected.to_string(),
                            actual: actual.to_string(),
                        });
                        test_passed = false;
                    }
                }

                if let Some(expected) = search.as_deref() {
                    let actual = url.search();
                    if actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "search".to_string(),
                            expected: expected.to_string(),
                            actual: actual.to_string(),
                        });
                        test_passed = false;
                    }
                }

                if let Some(expected) = hash.as_deref() {
                    let actual = url.hash();
                    if actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "hash".to_string(),
                            expected: expected.to_string(),
                            actual: actual.to_string(),
                        });
                        test_passed = false;
                    }
                }

                if let Some(expected) = origin.as_deref() {
                    let actual = url.origin();
                    if actual != expected {
                        result.failures.push(WptFailure {
                            test_num,
                            input: input.clone(),
                            base: base.clone(),
                            field: "origin".to_string(),
                            expected: expected.to_string(),
                            actual,
                        });
                        test_passed = false;
                    }
                }

                if test_passed {
                    result.passed += 1;
                } else {
                    result.failed += 1;
                }
            }
        }
    }

    result
}

#[test]
fn test_full_wpt_suite() {
    let test_data = include_str!("./urltestdata.json");
    let tests: Vec<TestCase> =
        serde_json::from_str(test_data).expect("Failed to parse WPT test data");

    println!("\nRunning {} WPT tests...", tests.len());

    let result = run_wpt_tests(tests);

    println!("\n{}", result.summary());

    if !result.failures.is_empty() {
        println!("\nShowing first 20 failures:");
        for (i, failure) in result.failures.iter().take(20).enumerate() {
            println!("\n{}. Test #{}: {}", i + 1, failure.test_num, failure.field);
            println!("   Input: {}", failure.input);
            if let Some(ref base) = failure.base {
                println!("   Base: {base}");
            }
            println!("   Expected: {}", failure.expected);
            println!("   Actual: {}", failure.actual);
        }

        if result.failures.len() > 20 {
            println!("\n... and {} more failures", result.failures.len() - 20);
        }
    }

    println!("\nPass rate: {:.2}%", result.pass_rate());

    // WPT Compliance: Require 100% pass rate
    assert_eq!(
        result.failed,
        0,
        "\n\n❌ WPT Compliance Test Failed!\n\
         Passed: {}, Failed: {}, Pass Rate: {:.2}%\n\
         \n\
         ars_url must maintain 100% WPT compliance.\n\
         Run with `cargo test test_full_wpt_suite -- --nocapture` to see failure details.\n",
        result.passed,
        result.failed,
        result.pass_rate()
    );

    // Also verify total test count hasn't changed unexpectedly
    let total_tests = result.passed + result.failed + result.skipped;
    assert!(
        total_tests >= 873,
        "Expected at least 873 WPT tests, but found {total_tests}",
    );
}
