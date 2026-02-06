/// Check if a string could be an IPv4 address (fast preliminary check).
/// Based on ada-url's `checkers::is_ipv4`.
/// Returns true if the string has the format of a potential IPv4 address.
pub fn is_ipv4(input: &str) -> bool {
    let input = input.strip_suffix('.').unwrap_or(input);

    // Must be non-empty with valid last character (digit, hex a-f/A-F, or x/X)
    let Some(last_char) = input.chars().next_back() else {
        return false;
    };
    if !last_char.is_ascii_digit() && !matches!(last_char, 'a'..='f' | 'A'..='F' | 'x' | 'X') {
        return false;
    }

    // Extract last segment (after last dot, or entire string if no dots)
    let last_segment = input.rsplit('.').next().unwrap_or(input);

    // Check if last segment is all digits (decimal)
    if last_segment.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    // Check if last segment is hexadecimal (0x...)
    if let Some(hex_part) = last_segment
        .strip_prefix("0x")
        .or_else(|| last_segment.strip_prefix("0X"))
    {
        return hex_part.is_empty() || hex_part.chars().all(|c| c.is_ascii_hexdigit());
    }

    false
}

/// Parse a port string to u16.
/// Returns None if empty, contains non-digit characters, or is out of range.
pub fn parse_port(port: &str) -> Option<u16> {
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    port.parse::<u16>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ipv4() {
        // Decimal
        assert!(is_ipv4("192.168.1.1"));
        assert!(is_ipv4("127.0.0.1"));
        assert!(is_ipv4("255.255.255.255"));
        assert!(is_ipv4("192.168.1.1.")); // Trailing dot

        // Hexadecimal (requires 0x prefix)
        assert!(is_ipv4("0xC0A80101"));
        assert!(is_ipv4("192.0x00A80001"));
        assert!(is_ipv4("0x")); // "0x" alone is valid (matches ada-url)
        assert!(is_ipv4("0X")); // Same for uppercase

        // Not IPv4
        assert!(!is_ipv4(""));
        assert!(!is_ipv4("."));
        assert!(!is_ipv4("example.com"));
        assert!(!is_ipv4("192.168.1.g")); // Invalid hex
        assert!(!is_ipv4("192.168.1.X")); // Uppercase X at end
        assert!(!is_ipv4("ab")); // Bare hex without 0x prefix (ada-url behavior)
    }

    #[test]
    fn test_parse_port() {
        assert_eq!(parse_port("80"), Some(80));
        assert_eq!(parse_port("8080"), Some(8080));
        assert_eq!(parse_port("443"), Some(443));
        assert_eq!(parse_port("65535"), Some(65535));
        assert_eq!(parse_port("65536"), None); // Out of range
        assert_eq!(parse_port("abc"), None);
        assert_eq!(parse_port(""), None);
    }
}
