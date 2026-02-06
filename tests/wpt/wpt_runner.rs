use super::wpt_loader::{TestCase, WptFailure, WptTestResult, get_inline_tests};
/// WPT test runner
///
/// Runs WHATWG URL tests against ars_url implementation
use ars::Url;

/// Run WPT tests and return results
pub fn run_wpt_tests(tests: Vec<TestCase>) -> WptTestResult {
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

                // Check if test expects failure
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
                        Err(_) => {
                            result.passed += 1;
                        }
                    }
                    continue;
                }

                // Try to parse the URL
                let url = match Url::parse(&input, base.as_deref()) {
                    Ok(u) => u,
                    Err(_) => {
                        if href.is_some() {
                            // Expected to parse but failed
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

                // Check each field
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_inline_wpt_tests() {
        let tests = get_inline_tests();
        let result = run_wpt_tests(tests);

        println!("\n{}", result.summary());

        if !result.failures.is_empty() {
            println!("\nFailures:");
            for failure in &result.failures {
                println!("  Test #{}: {}", failure.test_num, failure.field);
                println!("    Input: {}", failure.input);
                if let Some(ref base) = failure.base {
                    println!("    Base: {}", base);
                }
                println!("    Expected: {}", failure.expected);
                println!("    Actual: {}", failure.actual);
            }
        }

        // We expect at least some tests to pass
        assert!(result.passed > 0, "No tests passed!");

        // Print pass rate
        println!("\nPass rate: {:.2}%", result.pass_rate());
    }
}
