/// WPT (Web Platform Tests) module
///
/// This module contains all WHATWG URL specification compliance tests
/// from the Web Platform Tests suite.
#[path = "wpt/wpt_loader.rs"]
mod wpt_loader;

#[path = "wpt/wpt_runner.rs"]
mod wpt_runner;

#[path = "wpt/wpt_full_tests.rs"]
mod wpt_full_tests;

#[path = "wpt/wpt_canparse_tests.rs"]
mod wpt_canparse_tests;
