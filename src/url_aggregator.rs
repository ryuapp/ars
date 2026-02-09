use crate::checkers::parse_port;
use crate::compat::{String, ToString, format};
use crate::error::Result;
use crate::parser::Parseable;
use crate::scheme::get_scheme_type;
use crate::types::SchemeType;
use crate::unicode::idna::domain_to_ascii;
use crate::unicode::percent_encode::percent_encode_userinfo;
use crate::url_base::UrlBase;
use crate::url_components::UrlComponents;

/// Normalize a hostname: ASCII-lowercase, or IDNA process if non-ASCII.
/// IPv6 addresses (starting with '[') are returned as-is.
fn normalize_hostname(hostname: &str) -> Option<String> {
    if hostname.starts_with('[') {
        return Some(hostname.to_string());
    }
    if hostname.is_ascii() {
        Some(hostname.to_ascii_lowercase())
    } else {
        domain_to_ascii(hostname).ok()
    }
}

/// Parse host string into hostname and optional port parts.
fn parse_host_port_parts(host: &str) -> (&str, Option<&str>) {
    if host.starts_with('[') {
        // IPv6 address
        if let Some(bracket_end) = host.find(']') {
            let ipv6_part = &host[0..=bracket_end];
            let port_part = &host[bracket_end + 1..];
            let port = port_part.strip_prefix(':');
            return (ipv6_part, port);
        }
        return (host, None);
    }

    // Regular host or IPv4
    match host.rfind(':') {
        Some(colon_pos) => (&host[0..colon_pos], Some(&host[colon_pos + 1..])),
        None => (host, None),
    }
}

/// URL structure that stores all components in a single buffer
/// This is more memory-efficient and provides zero-copy getters
///
/// Example buffer: "<https://user:pass@example.com:8080/path?query#hash>"
/// Components track offsets into this buffer for zero-copy access
#[derive(Debug, Clone)]
pub struct UrlAggregator {
    pub(crate) buffer: String,
    pub(crate) components: UrlComponents,
    pub(crate) scheme_type: SchemeType,
}

impl UrlAggregator {
    /// Create a URL aggregator with pre-allocated capacity
    /// This reduces allocations during parsing (internal use only)
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
            components: UrlComponents::new(),
            scheme_type: SchemeType::NotSpecial,
        }
    }

    /// Check if a URL string can be parsed without actually parsing it
    ///
    /// This is optimized for validation - it only checks URL structure
    /// without allocating buffers or encoding strings (ada-url optimization).
    ///
    /// # Examples
    ///
    /// ```
    /// use ars::Url;
    ///
    /// assert!(Url::can_parse("http://example.com", None));
    /// assert!(Url::can_parse("/path", Some("http://example.com")));
    /// assert!(!Url::can_parse("not a url", None));
    /// ```
    pub fn can_parse(input: &str, base: Option<&str>) -> bool {
        crate::parser::validate_url(input, base).is_ok()
    }

    /// Create from buffer and components (internal use)
    pub(crate) fn from_buffer(buffer: String, components: UrlComponents) -> Self {
        let protocol_end = components.protocol_end as usize;
        let scheme = if protocol_end > 0 && buffer.len() >= protocol_end {
            buffer[..protocol_end].trim_end_matches(':')
        } else {
            ""
        };
        let scheme_type = get_scheme_type(scheme);

        Self {
            buffer,
            components,
            scheme_type,
        }
    }

    /// Get a component by range (zero-copy)
    fn get_component(&self, start: u32, end: u32) -> &str {
        let start = start as usize;
        let end = end as usize;
        if start <= end && end <= self.buffer.len() {
            &self.buffer[start..end]
        } else {
            ""
        }
    }

    /// Get the scheme type
    pub fn scheme_type(&self) -> SchemeType {
        self.scheme_type
    }

    /// Get direct access to the buffer (for internal use)
    pub(crate) fn buffer_mut(&mut self) -> &mut String {
        &mut self.buffer
    }

    /// Get direct access to components (for internal use)
    pub(crate) fn components(&self) -> &UrlComponents {
        &self.components
    }

    /// Get direct access to components (for internal use)
    pub(crate) fn components_mut(&mut self) -> &mut UrlComponents {
        &mut self.components
    }

    /// Get the end position of the pathname (before search or hash, or end of buffer).
    fn pathname_end(&self) -> u32 {
        if self.components.search_start > 0 {
            self.components.search_start
        } else if self.components.hash_start > 0 {
            self.components.hash_start
        } else {
            self.buffer.len() as u32
        }
    }

    /// Set the scheme type (for internal use)
    pub(crate) fn set_scheme_type(&mut self, scheme_type: SchemeType) {
        self.scheme_type = scheme_type;
    }

    /// Copy authority (username, password, host, port) from base URL
    pub(crate) fn copy_authority_from(&mut self, base: &UrlAggregator) {
        let base_buf = &base.buffer;
        let base_comp = &base.components;
        let protocol_end_pos = base_comp.protocol_end as usize;

        // Check if base has authority (// after protocol)
        let base_has_authority = base_buf.get(protocol_end_pos..protocol_end_pos + 2) == Some("//");

        if !base_has_authority {
            // No authority - set all offsets to protocol_end
            let end = self.components.protocol_end;
            self.components.username_end = end;
            self.components.password_end = end;
            self.components.host_start = end;
            self.components.host_end = end;
            self.components.pathname_start = end;
            return;
        }

        // Base has authority - add "//" marker
        self.buffer.push_str("//");

        // Copy username if exists
        let username_start = base_comp.protocol_end as usize + 2; // Skip "//"
        if base_comp.username_end > base_comp.protocol_end + 2 {
            let username = &base_buf[username_start..base_comp.username_end as usize];
            self.buffer.push_str(username);
            self.components.username_end = self.buffer.len() as u32;
        } else {
            self.components.username_end = self.components.protocol_end + 2;
        }

        // Copy password if exists
        if base_comp.password_end > base_comp.username_end {
            let password_with_colon =
                &base_buf[base_comp.username_end as usize..base_comp.password_end as usize];
            self.buffer.push_str(password_with_colon);
            self.components.password_end = self.buffer.len() as u32;
        } else {
            self.components.password_end = self.components.username_end;
        }

        // Copy @ if credentials exist
        // Check if we actually have username or password (not just empty authority)
        if base_comp.username_end > base_comp.protocol_end + 2
            || base_comp.password_end > base_comp.username_end
        {
            self.buffer.push('@');
        }

        // Copy host
        self.components.host_start = self.buffer.len() as u32;
        let host = &base_buf[base_comp.host_start as usize..base_comp.host_end as usize];
        self.buffer.push_str(host);
        self.components.host_end = self.buffer.len() as u32;

        // Copy port and/or dash-dot marker if they exist
        self.components.port = base_comp.port;
        let port_or_marker_start = base_comp.host_end as usize;
        let port_or_marker_end = base_comp.pathname_start as usize;
        if port_or_marker_start < port_or_marker_end {
            // Copy everything between host_end and pathname_start
            // This includes port (if present) and/or dash-dot marker (if present)
            let content = &base_buf[port_or_marker_start..port_or_marker_end];
            self.buffer.push_str(content);
        }

        self.components.pathname_start = self.buffer.len() as u32;
    }
}

impl UrlBase for UrlAggregator {
    /// Get the full URL (zero-copy)
    fn href(&self) -> &str {
        &self.buffer
    }

    /// Get the protocol (zero-copy)
    fn protocol(&self) -> &str {
        self.get_component(0, self.components.protocol_end)
    }

    /// Get the username (zero-copy)
    fn username(&self) -> &str {
        self.get_component(
            self.components.username_start(),
            self.components.username_end,
        )
    }

    /// Get the password (zero-copy)
    fn password(&self) -> &str {
        self.get_component(
            self.components.password_start(),
            self.components.password_end,
        )
    }

    /// Get the host including port if present (zero-copy)
    /// Returns "example.com:8080" or "example.com"
    fn host(&self) -> &str {
        // Opaque path URLs don't have a host
        if self.has_opaque_path() {
            return "";
        }

        let start = self.components.host_start as usize;
        let host_end = self.components.host_end as usize;
        let pathname_start = self.components.pathname_start as usize;

        // Check if we have a "/." marker between host_end and pathname_start
        let has_dash_dot = pathname_start == host_end + 2
            && pathname_start <= self.buffer.len()
            && self.buffer.get(host_end..host_end + 2) == Some("/.");

        // Skip the "/." marker when returning host
        let end = if has_dash_dot {
            host_end
        } else {
            pathname_start
        };

        if start <= end && end <= self.buffer.len() {
            &self.buffer[start..end]
        } else {
            ""
        }
    }

    /// Get the hostname without port (zero-copy)
    fn hostname(&self) -> &str {
        // Opaque path URLs don't have a hostname
        if self.has_opaque_path() {
            return "";
        }
        self.get_component(self.components.host_start, self.components.host_end)
    }

    /// Get the port as string (zero-copy)
    /// Returns "8080" or "" if no non-default port
    fn port(&self) -> &str {
        if !self.has_port() {
            return "";
        }

        // Port is stored in buffer after hostname
        // Buffer: "example.com:8080"
        //                     ^host_end  ^pathname_start
        let start = self.components.host_end as usize + 1; // Skip ':'
        let end = self.components.pathname_start as usize;

        if start < end && end <= self.buffer.len() {
            &self.buffer[start..end]
        } else {
            ""
        }
    }

    /// Get the pathname (zero-copy)
    fn pathname(&self) -> &str {
        self.get_component(self.components.pathname_start, self.pathname_end())
    }

    /// Get the search/query string (zero-copy).
    fn search(&self) -> &str {
        if self.components.search_start == 0 {
            return "";
        }
        let end = if self.components.hash_start > 0 {
            self.components.hash_start
        } else {
            self.buffer.len() as u32
        };
        let result = self.get_component(self.components.search_start, end);
        // Return empty for lone "?"
        if result == "?" { "" } else { result }
    }

    /// Get the hash/fragment (zero-copy).
    fn hash(&self) -> &str {
        if self.components.hash_start == 0 {
            return "";
        }
        let result = self.get_component(self.components.hash_start, self.buffer.len() as u32);
        // Return empty for lone "#"
        if result == "#" { "" } else { result }
    }

    /// Get the origin (requires allocation).
    fn origin(&self) -> String {
        let scheme = self.protocol().trim_end_matches(':');

        // Handle blob: URLs - parse the path as a URL and return its origin
        if scheme == "blob" {
            let inner_url = crate::parser::parse::<UrlAggregator>(self.pathname(), None);
            if let Ok(inner_url) = inner_url {
                let inner_scheme = inner_url.protocol().trim_end_matches(':');
                if inner_scheme == "http" || inner_scheme == "https" {
                    return inner_url.origin();
                }
            }
            return "null".to_string();
        }

        if self.scheme_type.is_special() {
            format!("{}//{}", self.protocol(), self.host())
        } else {
            "null".to_string()
        }
    }

    fn has_credentials(&self) -> bool {
        !self.username().is_empty() || !self.password().is_empty()
    }

    fn has_hostname(&self) -> bool {
        self.components.host_start < self.components.host_end
    }

    fn has_port(&self) -> bool {
        self.components
            .port
            .is_some_and(|port| self.scheme_type.default_port() != Some(port))
    }

    fn has_search(&self) -> bool {
        self.components.search_start > 0
    }

    fn has_hash(&self) -> bool {
        self.components.hash_start > 0
    }

    fn has_opaque_path(&self) -> bool {
        // Per WHATWG: A URL has an opaque path if its scheme is not special
        // AND it doesn't have an authority section AND path doesn't start with /
        // Examples:
        //   - "sc:sd" -> opaque path (no authority, pathname: "sd")
        //   - "sc:sd/sd" -> opaque path (no authority, pathname: "sd/sd")
        //   - "sc:/pa/pa" -> NOT opaque (pathname: "/pa/pa" starts with /)
        //   - "h://." -> NOT opaque (has authority)
        if self.scheme_type.is_special() {
            return false;
        }

        // If there's an authority section, then it's not an opaque path
        let has_authority =
            self.components.host_start as usize > self.components.protocol_end as usize;
        if has_authority {
            return false;
        }

        // No authority - check if pathname starts with /
        !self.pathname().starts_with('/')
    }

    fn has_empty_hostname(&self) -> bool {
        self.components.host_start == self.components.host_end
    }

    // Setters
    fn set_href(&mut self, href: &str) -> Result<()> {
        use crate::parser::parse_url_aggregator;

        // Parse new URL and replace self
        let new_url = parse_url_aggregator(href, None)?;
        *self = new_url;
        Ok(())
    }

    fn set_protocol(&mut self, protocol: &str) -> bool {
        let protocol = protocol.trim_end_matches(':');
        let new_scheme_type = get_scheme_type(protocol);

        // Can't change between special and non-special schemes
        if self.scheme_type.is_special() != new_scheme_type.is_special() {
            return false;
        }

        // Can't change file: to anything else or vice versa
        if self.protocol() == "file:" || protocol == "file" {
            return false;
        }

        let new_protocol = format!("{}:", protocol.to_ascii_lowercase());
        self.replace_range(0, self.components.protocol_end, &new_protocol);
        self.scheme_type = new_scheme_type;

        true
    }

    fn set_username(&mut self, username: &str) -> bool {
        // Can't set username on non-special schemes without authority
        if !self.scheme_type.is_special() && self.components.host_start == 0 {
            return false;
        }
        let encoded = percent_encode_userinfo(username);
        let protocol_end = self.components.protocol_end;

        if self.components.username_end > 0 {
            // Replace existing username
            let start = protocol_end + 2; // Skip "//"
            self.replace_range(start, self.components.username_end, &encoded);
        } else {
            // Insert new username (need to add // and @ if not present)
            let has_slashes = self.buffer[protocol_end as usize..].starts_with("//");

            let insertion = if has_slashes {
                format!("{encoded}@")
            } else {
                format!("//{encoded}@")
            };

            let actual_insert_pos = if has_slashes {
                protocol_end + 2
            } else {
                protocol_end
            };

            self.buffer
                .insert_str(actual_insert_pos as usize, &insertion);
            self.components.username_end = actual_insert_pos + encoded.len() as u32;

            // Adjust subsequent offsets
            let delta = insertion.len() as u32;
            if self.components.password_end >= actual_insert_pos {
                self.components.password_end += delta;
            }
            if self.components.host_start >= actual_insert_pos {
                self.components.host_start += delta;
            }
            if self.components.host_end >= actual_insert_pos {
                self.components.host_end += delta;
            }
            if self.components.pathname_start >= actual_insert_pos {
                self.components.pathname_start += delta;
            }
            if self.components.search_start >= actual_insert_pos {
                self.components.search_start += delta;
            }
            if self.components.hash_start >= actual_insert_pos {
                self.components.hash_start += delta;
            }
        }

        true
    }

    fn set_password(&mut self, password: &str) -> bool {
        // Can't set password without username
        if self.components.username_end == 0 {
            return false;
        }

        let encoded = percent_encode_userinfo(password);

        if self.components.password_end > self.components.username_end {
            // Replace existing password (includes the :)
            let start = self.components.username_end;
            let end = self.components.password_end;

            if encoded.is_empty() {
                // Remove password and colon
                self.replace_range(start, end, "");
                self.components.password_end = self.components.username_end;
            } else {
                self.replace_range(start, end, &format!(":{encoded}"));
                self.components.password_end = start + 1 + encoded.len() as u32;
            }
        } else if !encoded.is_empty() {
            // Add new password
            let insert_pos = self.components.username_end;
            let insertion = format!(":{encoded}");
            self.buffer.insert_str(insert_pos as usize, &insertion);
            self.components.password_end = insert_pos + insertion.len() as u32;

            // Adjust subsequent offsets
            let delta = insertion.len() as u32;
            if self.components.host_start >= insert_pos {
                self.components.host_start += delta;
            }
            if self.components.host_end >= insert_pos {
                self.components.host_end += delta;
            }
            if self.components.pathname_start >= insert_pos {
                self.components.pathname_start += delta;
            }
            if self.components.search_start >= insert_pos {
                self.components.search_start += delta;
            }
            if self.components.hash_start >= insert_pos {
                self.components.hash_start += delta;
            }
        }

        true
    }

    fn set_host(&mut self, host: &str) -> bool {
        // Can't set host on non-special schemes
        if !self.scheme_type.is_special() {
            return false;
        }

        // Parse host:port
        let (hostname, port) = parse_host_port_parts(host);

        // Validate and normalize hostname
        let Some(normalized_hostname) = normalize_hostname(hostname) else {
            return false;
        };

        // Build new host string
        let hostname_len = normalized_hostname.len() as u32;
        let new_host = match port {
            Some(port_str) => format!("{normalized_hostname}:{port_str}"),
            None => normalized_hostname,
        };

        // Replace in buffer
        let start = self.components.host_start;
        let end = self.components.pathname_start;
        self.replace_range(start, end, &new_host);

        // Update host_end and port
        self.components.host_end = start + hostname_len;
        self.components.port = port.and_then(parse_port);

        true
    }

    fn set_hostname(&mut self, hostname: &str) -> bool {
        // Can't set hostname on non-special schemes
        if !self.scheme_type.is_special() {
            return false;
        }

        let Some(normalized_hostname) = normalize_hostname(hostname) else {
            return false;
        };

        // Replace just the hostname part (not the port)
        let start = self.components.host_start;
        let hostname_len = normalized_hostname.len() as u32;
        self.replace_range(start, self.components.host_end, &normalized_hostname);
        self.components.host_end = start + hostname_len;

        true
    }

    fn set_port(&mut self, port: &str) -> bool {
        // Can't set port on non-special schemes
        if !self.scheme_type.is_special() {
            return false;
        }

        if port.is_empty() {
            // Remove port if it exists
            if self.components.port.is_some() {
                self.replace_range(self.components.host_end, self.components.pathname_start, "");
                self.components.port = None;
            }
            return true;
        }

        let Some(port_num) = parse_port(port) else {
            return false;
        };

        let is_default = self.scheme_type.default_port() == Some(port_num);

        if is_default {
            // Setting default port - remove it from buffer but store in components
            if self.components.port.is_some() {
                self.replace_range(self.components.host_end, self.components.pathname_start, "");
            }
            self.components.port = Some(port_num);
            return true;
        }

        // Non-default port - write to buffer
        let new_port_str = format!(":{port}");

        if self.components.port.is_some() {
            // Replace existing port
            self.replace_range(
                self.components.host_end,
                self.components.pathname_start,
                &new_port_str,
            );
        } else {
            // Insert new port between hostname and pathname
            let insert_pos = self.components.host_end as usize;
            self.buffer.insert_str(insert_pos, &new_port_str);

            // Adjust offsets after host_end
            let delta = new_port_str.len() as u32;
            if self.components.pathname_start >= self.components.host_end {
                self.components.pathname_start += delta;
            }
            if self.components.search_start >= self.components.host_end {
                self.components.search_start += delta;
            }
            if self.components.hash_start >= self.components.host_end {
                self.components.hash_start += delta;
            }
        }

        self.components.port = Some(port_num);
        true
    }

    fn set_pathname(&mut self, pathname: &str) -> bool {
        // Special schemes require leading /
        if !pathname.starts_with('/') && self.scheme_type.is_special() {
            return false;
        }

        // For non-special URLs without authority, if pathname starts with "//",
        // insert "/." to prevent ambiguity (pathname would be interpreted as authority)
        let has_authority =
            self.components.host_start as usize > self.components.protocol_end as usize;

        if !self.scheme_type.is_special() && !has_authority && pathname.starts_with("//") {
            let host_end = self.components.host_end as usize;
            let pathname_start = self.components.pathname_start as usize;

            // Only insert if "/." doesn't already exist
            if pathname_start != host_end + 2
                || self.buffer.get(host_end..pathname_start) != Some("/.")
            {
                self.buffer.insert_str(host_end, "/.");
                self.components.pathname_start += 2;
                if self.components.search_start > 0 {
                    self.components.search_start += 2;
                }
                if self.components.hash_start > 0 {
                    self.components.hash_start += 2;
                }
            }
        }

        let start = self.components.pathname_start;
        let end = self.pathname_end();
        self.replace_range(start, end, pathname);
        true
    }

    fn set_search(&mut self, search: &str) {
        // Ensure pathname exists for special URLs
        if self.scheme_type.is_special() && self.pathname().is_empty() {
            self.buffer
                .insert(self.components.pathname_start as usize, '/');
            if self.components.search_start > 0 {
                self.components.search_start += 1;
            }
            if self.components.hash_start > 0 {
                self.components.hash_start += 1;
            }
        }

        let end = if self.components.hash_start > 0 {
            self.components.hash_start
        } else {
            self.buffer.len() as u32
        };

        if self.components.search_start > 0 {
            if search.is_empty() {
                self.replace_range(self.components.search_start, end, "");
                self.components.search_start = 0;
            } else {
                let new_search = if search.starts_with('?') {
                    search.to_string()
                } else {
                    format!("?{search}")
                };
                self.replace_range(self.components.search_start, end, &new_search);
            }
        } else if !search.is_empty() {
            let insert_pos = end;
            let new_search = if search.starts_with('?') {
                search.to_string()
            } else {
                format!("?{search}")
            };

            self.buffer.insert_str(insert_pos as usize, &new_search);
            self.components.search_start = insert_pos;

            if self.components.hash_start > 0 {
                self.components.hash_start += new_search.len() as u32;
            }
        }
    }

    fn set_hash(&mut self, hash: &str) {
        // Ensure pathname exists for special URLs
        if self.scheme_type.is_special() && self.pathname().is_empty() {
            let insert_pos = self.components.pathname_start as usize;
            self.buffer.insert(insert_pos, '/');
            if self.components.search_start > 0 {
                self.components.search_start += 1;
            }
            if self.components.hash_start > 0 {
                self.components.hash_start += 1;
            }
        }

        if self.components.hash_start > 0 {
            self.buffer.truncate(self.components.hash_start as usize);
            if hash.is_empty() {
                self.components.hash_start = 0;
                return;
            }
        } else if hash.is_empty() {
            return;
        } else {
            self.components.hash_start = self.buffer.len() as u32;
        }

        if !hash.starts_with('#') {
            self.buffer.push('#');
        }
        self.buffer.push_str(hash);
    }
}

impl UrlAggregator {
    /// Replace a range in the buffer and adjust all offsets
    /// Returns the delta (`new_len` - `old_len`)
    fn replace_range(&mut self, start: u32, end: u32, replacement: &str) -> i32 {
        let start_idx = start as usize;
        let end_idx = end as usize;
        let old_len = end_idx - start_idx;
        let new_len = replacement.len();
        let delta = new_len as i32 - old_len as i32;

        self.buffer.replace_range(start_idx..end_idx, replacement);

        // For insertions (start == end), adjust offsets > start
        // For replacements (start < end), adjust offsets >= end
        let is_insertion = start == end;
        let threshold = if is_insertion { start } else { end };

        let adjust = |offset: &mut u32| {
            let should_adjust = if is_insertion {
                *offset > threshold
            } else {
                *offset >= threshold
            };
            if should_adjust {
                *offset = (*offset as i32 + delta) as u32;
            }
        };

        adjust(&mut self.components.protocol_end);
        adjust(&mut self.components.username_end);
        adjust(&mut self.components.password_end);
        adjust(&mut self.components.host_start);
        adjust(&mut self.components.host_end);
        adjust(&mut self.components.pathname_start);
        adjust(&mut self.components.search_start);
        adjust(&mut self.components.hash_start);

        delta
    }
}

impl UrlAggregator {
    /// Parse a URL string with an optional base URL (ada-url compatible API)
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is invalid according to the WHATWG URL Standard.
    pub fn parse(input: &str, base: Option<&str>) -> Result<Self> {
        crate::parser::parse_url_aggregator(input, base)
    }

    // Public API methods that delegate to UrlBase trait implementation
    // This allows callers to use these methods without importing UrlBase

    /// Get the full URL string (zero-copy)
    pub fn href(&self) -> &str {
        <Self as UrlBase>::href(self)
    }

    /// Get the protocol (e.g., "http:", "https:")
    pub fn protocol(&self) -> &str {
        <Self as UrlBase>::protocol(self)
    }

    /// Get the username
    pub fn username(&self) -> &str {
        <Self as UrlBase>::username(self)
    }

    /// Get the password
    pub fn password(&self) -> &str {
        <Self as UrlBase>::password(self)
    }

    /// Get the host including port (e.g., "example.com:8080", zero-copy)
    pub fn host(&self) -> &str {
        <Self as UrlBase>::host(self)
    }

    /// Get the hostname without port (e.g., "example.com")
    pub fn hostname(&self) -> &str {
        <Self as UrlBase>::hostname(self)
    }

    /// Get the port as string (e.g., "8080"), or empty string if default
    pub fn port(&self) -> &str {
        <Self as UrlBase>::port(self)
    }

    /// Get the pathname (e.g., "/path/to/page")
    pub fn pathname(&self) -> &str {
        <Self as UrlBase>::pathname(self)
    }

    /// Get the search/query string (e.g., "?key=value")
    pub fn search(&self) -> &str {
        <Self as UrlBase>::search(self)
    }

    /// Get the hash/fragment (e.g., "#section")
    pub fn hash(&self) -> &str {
        <Self as UrlBase>::hash(self)
    }

    /// Get the origin (e.g., "<http://example.com:8080>")
    pub fn origin(&self) -> String {
        <Self as UrlBase>::origin(self)
    }

    // Setter methods that delegate to UrlBase trait

    /// Set the full href (re-parses the URL)
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is invalid according to the WHATWG URL Standard.
    pub fn set_href(&mut self, href: &str) -> Result<()> {
        <Self as UrlBase>::set_href(self, href)
    }

    /// Set the protocol/scheme
    pub fn set_protocol(&mut self, protocol: &str) -> bool {
        <Self as UrlBase>::set_protocol(self, protocol)
    }

    /// Set the username
    pub fn set_username(&mut self, username: &str) -> bool {
        <Self as UrlBase>::set_username(self, username)
    }

    /// Set the password
    pub fn set_password(&mut self, password: &str) -> bool {
        <Self as UrlBase>::set_password(self, password)
    }

    /// Set the host (hostname + port)
    pub fn set_host(&mut self, host: &str) -> bool {
        <Self as UrlBase>::set_host(self, host)
    }

    /// Set the hostname
    pub fn set_hostname(&mut self, hostname: &str) -> bool {
        <Self as UrlBase>::set_hostname(self, hostname)
    }

    /// Set the port
    pub fn set_port(&mut self, port: &str) -> bool {
        <Self as UrlBase>::set_port(self, port)
    }

    /// Set the pathname
    pub fn set_pathname(&mut self, pathname: &str) -> bool {
        <Self as UrlBase>::set_pathname(self, pathname)
    }

    /// Set the search string
    pub fn set_search(&mut self, search: &str) {
        <Self as UrlBase>::set_search(self, search);
    }

    /// Set the hash
    pub fn set_hash(&mut self, hash: &str) {
        <Self as UrlBase>::set_hash(self, hash);
    }

    // Has methods that delegate to UrlBase trait

    /// Check if URL has credentials
    pub fn has_credentials(&self) -> bool {
        <Self as UrlBase>::has_credentials(self)
    }

    /// Check if URL has a hostname
    pub fn has_hostname(&self) -> bool {
        <Self as UrlBase>::has_hostname(self)
    }

    /// Check if URL has a non-default port
    pub fn has_port(&self) -> bool {
        <Self as UrlBase>::has_port(self)
    }

    /// Check if URL has a search string
    pub fn has_search(&self) -> bool {
        <Self as UrlBase>::has_search(self)
    }

    /// Check if URL has a hash
    pub fn has_hash(&self) -> bool {
        <Self as UrlBase>::has_hash(self)
    }

    /// Check if URL has an opaque path
    pub(crate) fn has_opaque_path(&self) -> bool {
        <Self as UrlBase>::has_opaque_path(self)
    }

    /// Check if URL has empty hostname
    pub fn has_empty_hostname(&self) -> bool {
        <Self as UrlBase>::has_empty_hostname(self)
    }
}

impl Parseable for UrlAggregator {
    fn parse(input: &str, base: Option<&str>) -> Result<Self> {
        Self::parse(input, base)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_url_aggregator_from_buffer() {
        let buffer = "http://example.com/path?query#hash".to_string();
        let mut components = UrlComponents::new();
        components.protocol_end = 5; // "http:"
        components.host_start = 7;
        components.host_end = 18;
        components.pathname_start = 18;
        components.search_start = 23;
        components.hash_start = 29;

        let url = UrlAggregator::from_buffer(buffer, components);

        assert_eq!(url.protocol(), "http:");
        assert!(url.scheme_type.is_special());
    }

    #[test]
    fn test_url_aggregator_parse() {
        let url = UrlAggregator::parse("http://example.com:8080/path?query#hash", None).unwrap();

        assert_eq!(url.protocol(), "http:");
        assert_eq!(url.hostname(), "example.com");
        assert_eq!(url.port(), "8080");
        assert_eq!(url.host(), "example.com:8080");
        assert_eq!(url.pathname(), "/path");
        assert_eq!(url.search(), "?query");
        assert_eq!(url.hash(), "#hash");
        assert_eq!(url.href(), "http://example.com:8080/path?query#hash");
    }

    #[test]
    fn test_can_parse_valid_absolute_urls() {
        assert!(UrlAggregator::can_parse("http://example.com", None));
        assert!(UrlAggregator::can_parse("https://example.com/path", None));
        assert!(UrlAggregator::can_parse(
            "https://user:pass@example.com:8080/path?query#hash",
            None
        ));
        assert!(UrlAggregator::can_parse(
            "ftp://ftp.example.com/file.txt",
            None
        ));
        assert!(UrlAggregator::can_parse("ws://localhost:3000", None));
    }

    #[test]
    fn test_can_parse_relative_urls_with_base() {
        let base = "http://example.com/base/";
        assert!(UrlAggregator::can_parse("/path", Some(base)));
        assert!(UrlAggregator::can_parse("../other", Some(base)));
        assert!(UrlAggregator::can_parse("?query", Some(base)));
        assert!(UrlAggregator::can_parse("#hash", Some(base)));
        assert!(UrlAggregator::can_parse("relative/path", Some(base)));
    }

    #[test]
    fn test_can_parse_invalid_urls() {
        assert!(!UrlAggregator::can_parse("not a url", None));
        assert!(!UrlAggregator::can_parse("", None));
        assert!(!UrlAggregator::can_parse("   ", None));
        assert!(!UrlAggregator::can_parse("/relative/path", None)); // No base
        assert!(!UrlAggregator::can_parse("//example.com", None)); // Protocol-relative without base
    }

    #[test]
    fn test_can_parse_edge_cases() {
        // Invalid characters in host
        assert!(!UrlAggregator::can_parse("http://exa mple.com", None));
        assert!(!UrlAggregator::can_parse("http://example<>.com", None));

        // Valid IPv4 and IPv6
        assert!(UrlAggregator::can_parse("http://192.168.1.1/", None));
        assert!(UrlAggregator::can_parse("http://[2001:db8::1]/", None));

        // Unicode domains
        assert!(UrlAggregator::can_parse("https://総務省.jp", None));
    }

    // Regression tests for pathname "/." insertion bug
    // See: https://github.com/ada-url/ada/pull/1077
    #[test]
    fn test_set_pathname_with_query() {
        let mut url = UrlAggregator::parse("foo:/?q", None).unwrap();
        assert!(url.set_pathname("//bar"));
        assert_eq!(url.pathname(), "//bar");
        assert_eq!(url.search(), "?q");
        assert!(url.href().contains("/.//bar"));
    }

    #[test]
    fn test_set_pathname_with_hash() {
        let mut url = UrlAggregator::parse("foo:/#h", None).unwrap();
        assert!(url.set_pathname("//bar"));
        assert_eq!(url.pathname(), "//bar");
        assert_eq!(url.hash(), "#h");
        assert!(url.href().contains("/.//bar"));
    }

    #[test]
    fn test_set_pathname_with_query_and_hash() {
        let mut url = UrlAggregator::parse("foo:/?q#h", None).unwrap();
        assert!(url.set_pathname("//bar"));
        assert_eq!(url.pathname(), "//bar");
        assert_eq!(url.search(), "?q");
        assert_eq!(url.hash(), "#h");
        assert!(url.href().contains("/.//bar"));
    }

    #[test]
    fn test_set_pathname_blob_scheme() {
        let mut url = UrlAggregator::parse("blob:/?q", None).unwrap();
        assert!(url.set_pathname("//p"));
        assert_eq!(url.pathname(), "//p");
        assert_eq!(url.search(), "?q");
        assert!(url.href().contains("/.//p"));
    }

    #[test]
    fn test_set_pathname_sequential_setters() {
        let mut url = UrlAggregator::parse("foo:xyz", None).unwrap();
        assert!(url.set_pathname("//x"));
        url.set_search("abc");
        assert_eq!(url.pathname(), "//x");
        assert_eq!(url.search(), "?abc");
        assert!(url.href().contains("/.//x"));
    }
}
