use crate::compat::String;
use crate::error::Result;

/// Base trait for URL types
/// Provides common interface for Url and UrlAggregator
#[doc(hidden)] // Internal trait, not part of public API docs
pub trait UrlBase {
    // Getters (11 methods)

    /// Get the full URL as a string (zero-copy)
    fn href(&self) -> &str;

    /// Get the protocol/scheme (e.g., "http:", "https:")
    fn protocol(&self) -> &str;

    /// Get the username component
    fn username(&self) -> &str;

    /// Get the password component
    fn password(&self) -> &str;

    /// Get the host (hostname + port if non-default, zero-copy)
    /// Example: "example.com:8080" or "example.com"
    fn host(&self) -> &str;

    /// Get the hostname (without port)
    fn hostname(&self) -> &str;

    /// Get the port as a string (empty if default port)
    fn port(&self) -> &str;

    /// Get the pathname
    fn pathname(&self) -> &str;

    /// Get the search/query string (including leading ?)
    fn search(&self) -> &str;

    /// Get the hash/fragment (including leading #)
    fn hash(&self) -> &str;

    /// Get the origin (scheme + host for special schemes)
    fn origin(&self) -> String;

    // Has checks (11 methods)

    /// Check if URL has credentials (username or password)
    fn has_credentials(&self) -> bool;

    /// Check if URL has a hostname
    fn has_hostname(&self) -> bool;

    /// Check if URL has a non-default port
    fn has_port(&self) -> bool;

    /// Check if URL has a search string
    fn has_search(&self) -> bool;

    /// Check if URL has a hash
    fn has_hash(&self) -> bool;

    /// Check if URL has an opaque path
    fn has_opaque_path(&self) -> bool;

    /// Check if URL has empty hostname
    fn has_empty_hostname(&self) -> bool;

    // Setters (10 methods)

    /// Set the full href (re-parses the URL)
    fn set_href(&mut self, href: &str) -> Result<()>;

    /// Set the protocol/scheme
    fn set_protocol(&mut self, protocol: &str) -> bool;

    /// Set the username
    fn set_username(&mut self, username: &str) -> bool;

    /// Set the password
    fn set_password(&mut self, password: &str) -> bool;

    /// Set the host (hostname + port)
    fn set_host(&mut self, host: &str) -> bool;

    /// Set the hostname
    fn set_hostname(&mut self, hostname: &str) -> bool;

    /// Set the port
    fn set_port(&mut self, port: &str) -> bool;

    /// Set the pathname
    fn set_pathname(&mut self, pathname: &str) -> bool;

    /// Set the search string
    fn set_search(&mut self, search: &str);

    /// Set the hash
    fn set_hash(&mut self, hash: &str);
}
