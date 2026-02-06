/// URL component offsets for aggregated storage
/// Tracks where each component starts and ends in a single buffer
///
/// Buffer layout: "<https://user:pass@example.com:8080/path?query#hash>"
/// - `protocol_end`: 6 (end of "https:")
/// - `username_end`: 11 (end of "user")
/// - `password_end`: 16 (end of "pass")
/// - `host_start`: 17 (start of "example.com", after "@")
/// - `host_end`: 28 (end of "example.com", before ":")
/// - port: Some(8080) (actual port number)
/// - `pathname_start`: 33 (start of "/path", after ":8080")
/// - `search_start`: 38 (start of "?query")
/// - `hash_start`: 44 (start of "#hash")
#[derive(Debug, Clone, Default)]
pub struct UrlComponents {
    pub protocol_end: u32,
    pub username_end: u32,
    pub password_end: u32,
    pub host_start: u32,
    pub host_end: u32,
    pub port: Option<u16>,
    pub pathname_start: u32,
    pub search_start: u32,
    pub hash_start: u32,
}

impl UrlComponents {
    /// Create a new `UrlComponents` with all offsets at 0
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the username start position (after "://")
    pub fn username_start(&self) -> u32 {
        // If host_start > protocol_end, there's a "//" authority marker
        if self.host_start > self.protocol_end {
            self.protocol_end + 2
        } else {
            self.protocol_end
        }
    }

    /// Get the password start position (after username and ":")
    pub fn password_start(&self) -> u32 {
        // If password_end > username_end, there's a ":" separator
        if self.password_end > self.username_end {
            self.username_end + 1
        } else {
            self.username_end
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_components_new() {
        let components = UrlComponents::new();
        assert_eq!(components.protocol_end, 0);
        assert_eq!(components.username_end, 0);
        assert_eq!(components.password_end, 0);
        assert_eq!(components.port, None);
    }

    #[test]
    fn test_username_start() {
        let mut components = UrlComponents::new();
        components.protocol_end = 5; // "http:"
        components.host_start = 7; // After "//"
        assert_eq!(components.username_start(), 7); // Skips "//"
    }

    #[test]
    fn test_username_start_no_authority() {
        let mut components = UrlComponents::new();
        components.protocol_end = 5; // "data:"
        components.host_start = 5; // No authority
        assert_eq!(components.username_start(), 5); // No "//" to skip
    }

    #[test]
    fn test_port_storage() {
        let mut components = UrlComponents::new();
        components.port = Some(8080);
        assert_eq!(components.port, Some(8080));

        components.port = None;
        assert_eq!(components.port, None);
    }
}
