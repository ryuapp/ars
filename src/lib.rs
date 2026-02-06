#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

// Compatibility layer for std/no_std
mod compat;

// Internal modules (not public API)
mod character_sets;
mod checkers;
mod error;
mod helpers;
mod ipv4;
mod ipv6;
mod parser;
mod scheme;
mod types;
mod unicode;
mod url_aggregator;
mod url_base;
#[doc(hidden)]
pub use url_base::UrlBase;
mod url_components;
mod url_search_params;

// Public API
pub use error::ParseError;
pub use url_aggregator::UrlAggregator as Url;
pub use url_search_params::UrlSearchParams;

pub type Result<T> = core::result::Result<T, ParseError>;
