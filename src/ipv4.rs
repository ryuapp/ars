/// IPv4 address parser supporting decimal, octal, and hexadecimal notation
/// Based on WHATWG URL specification
use crate::compat::{String, Vec, format};
use crate::error::{ParseError, Result};

/// Parse an IPv4 address string into a u32.
/// Supports:
/// - Decimal: 192.168.1.1
/// - Hex: 0xC0A80101
/// - Octal: 0300.0250.01.01
/// - Mixed: 192.0x00A80001
pub fn parse_ipv4(input: &str) -> Result<u32> {
    if input.is_empty() {
        return Err(ParseError::InvalidIpv4);
    }

    // Remove trailing dot if present (WHATWG: trailing dot is allowed and ignored)
    let input = input.strip_suffix('.').unwrap_or(input);

    // Split by dots and parse each part
    let parts: Vec<&str> = input.split('.').collect();
    let part_count = parts.len();

    if part_count == 0 || part_count > 4 {
        return Err(ParseError::InvalidIpv4);
    }

    let numbers: Vec<u64> = parts
        .iter()
        .map(|part| {
            if part.is_empty() {
                Err(ParseError::InvalidIpv4)
            } else {
                parse_ipv4_number(part)
            }
        })
        .collect::<Result<Vec<_>>>()?;

    // Validate: last number must be < 256^(5-n)
    let last = numbers[part_count - 1];
    let max = 256u64.pow((5 - part_count) as u32);
    if last >= max {
        return Err(ParseError::InvalidIpv4);
    }

    // Check that all but the last number are < 256
    if numbers.iter().take(part_count - 1).any(|&num| num >= 256) {
        return Err(ParseError::InvalidIpv4);
    }

    // Combine into IPv4 address according to WHATWG spec:
    // Each of the first (n-1) numbers represents a single byte
    // The last number fills the remaining bytes
    let mut ipv4: u32 = 0;

    // Place each of the first (n-1) parts as individual bytes
    for (i, &number) in numbers.iter().enumerate().take(part_count - 1) {
        let byte_pos = 3 - i; // Position from right (byte 3, 2, 1, 0)
        ipv4 |= (number as u32) << (byte_pos * 8);
    }

    // Add the last part (fills remaining bytes)
    ipv4 |= numbers[part_count - 1] as u32;

    Ok(ipv4)
}

/// Parse a single IPv4 number component (supports decimal, hex, octal).
fn parse_ipv4_number(input: &str) -> Result<u64> {
    if input.is_empty() {
        return Err(ParseError::InvalidIpv4);
    }

    // Check for hex prefix (0x or 0X)
    if let Some(hex_part) = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
    {
        // Bare "0x" or "0X" is treated as 0 (ada-url compatible)
        return if hex_part.is_empty() {
            Ok(0)
        } else {
            u64::from_str_radix(hex_part, 16).map_err(|_| ParseError::InvalidIpv4)
        };
    }

    // Octal (starts with 0 but not just "0")
    if input.len() >= 2 && input.starts_with('0') {
        return u64::from_str_radix(input, 8).map_err(|_| ParseError::InvalidIpv4);
    }

    // Decimal
    input.parse::<u64>().map_err(|_| ParseError::InvalidIpv4)
}

/// Serialize an IPv4 address (u32) to dotted decimal notation
pub fn serialize_ipv4(ipv4: u32) -> String {
    format!(
        "{}.{}.{}.{}",
        (ipv4 >> 24) & 0xFF,
        (ipv4 >> 16) & 0xFF,
        (ipv4 >> 8) & 0xFF,
        ipv4 & 0xFF
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::unreadable_literal)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ipv4_decimal() {
        assert_eq!(parse_ipv4("192.168.1.1").unwrap(), 0xC0A80101);
        assert_eq!(parse_ipv4("127.0.0.1").unwrap(), 0x7F000001);
    }

    #[test]
    fn test_parse_ipv4_hex() {
        assert_eq!(parse_ipv4("0xC0A80101").unwrap(), 0xC0A80101);
        assert_eq!(parse_ipv4("192.0x00A80001").unwrap(), 0xC0A80001);
    }

    #[test]
    fn test_parse_ipv4_octal() {
        assert_eq!(parse_ipv4("0300.0250.01.01").unwrap(), 0xC0A80101);
    }

    #[test]
    fn test_serialize_ipv4() {
        assert_eq!(serialize_ipv4(0xC0A80101), "192.168.1.1");
        assert_eq!(serialize_ipv4(0x7F000001), "127.0.0.1");
    }
}
