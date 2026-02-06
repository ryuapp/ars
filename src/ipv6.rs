/// IPv6 address parsing and validation
/// Implements WHATWG URL specification for IPv6 addresses
use crate::compat::{String, Vec};
use crate::error::{ParseError, Result};
use core::fmt::Write;

/// Parse an IPv6 address from bracket notation (e.g., "[`::1`]" or "[`2001:db8::1`]").
/// Returns the 8 u16 segments if valid, or an error if malformed.
pub fn parse_ipv6(input: &str) -> Result<[u16; 8]> {
    // Remove brackets if present
    let input = input
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(input);

    // Reject zone IDs (%) - not allowed in URLs (WPT test #326)
    if input.contains('%') {
        return Err(ParseError::InvalidIpv6);
    }

    // Check for embedded IPv4 (e.g., "::127.0.0.1")
    let has_embedded_ipv4 = input
        .rfind(':')
        .is_some_and(|pos| input[pos + 1..].contains('.'));

    if has_embedded_ipv4 {
        parse_ipv6_with_ipv4(input)
    } else {
        parse_ipv6_pure(input)
    }
}

/// Parse pure IPv6 address (no embedded IPv4).
fn parse_ipv6_pure(input: &str) -> Result<[u16; 8]> {
    let mut segments = [0u16; 8];

    let Some(double_colon_pos) = input.find("::") else {
        // No :: compression - must have exactly 8 segments
        let parsed = parse_segments(input)?;
        if parsed.len() != 8 {
            return Err(ParseError::InvalidIpv6);
        }
        segments.copy_from_slice(&parsed);
        return Ok(segments);
    };

    // Split around :: and parse both parts
    let before = &input[..double_colon_pos];
    let after = &input[double_colon_pos + 2..];
    let before_segments = parse_segments(before)?;
    let after_segments = parse_segments(after)?;

    // Check total segments
    let total = before_segments.len() + after_segments.len();
    if total > 7 {
        return Err(ParseError::InvalidIpv6);
    }

    // Fill segments array
    for (i, &seg) in before_segments.iter().enumerate() {
        segments[i] = seg;
    }

    let after_start = before_segments.len() + (8 - total);
    for (i, &seg) in after_segments.iter().enumerate() {
        segments[after_start + i] = seg;
    }

    Ok(segments)
}

/// Parse IPv6 with embedded IPv4 (e.g., "`::127.0.0.1`" or "`::ffff:192.168.1.1`").
fn parse_ipv6_with_ipv4(input: &str) -> Result<[u16; 8]> {
    // Find the last : before the IPv4 part
    let last_colon = input.rfind(':').ok_or(ParseError::InvalidIpv6)?;
    let ipv6_part = &input[..last_colon];
    let ipv4_part = &input[last_colon + 1..];

    // Parse IPv4 address and convert to two u16 segments
    let ipv4_addr = parse_ipv4(ipv4_part)?;
    let ipv4_high = ((ipv4_addr >> 16) & 0xFFFF) as u16;
    let ipv4_low = (ipv4_addr & 0xFFFF) as u16;

    let mut segments = [0u16; 8];

    if ipv6_part.is_empty() || ipv6_part == ":" {
        segments[6] = ipv4_high;
        segments[7] = ipv4_low;
        return Ok(segments);
    }

    if let Some(double_colon_pos) = ipv6_part.find("::") {
        let before = &ipv6_part[..double_colon_pos];
        let after = &ipv6_part[double_colon_pos + 2..];
        let before_segments = parse_segments(before)?;
        let after_segments = parse_segments(after)?;

        let total = before_segments.len() + after_segments.len();
        if total > 6 {
            return Err(ParseError::InvalidIpv6);
        }

        for (i, &seg) in before_segments.iter().enumerate() {
            segments[i] = seg;
        }

        let after_start = before_segments.len() + (6 - total);
        for (i, &seg) in after_segments.iter().enumerate() {
            segments[after_start + i] = seg;
        }
    } else {
        // No :: compression - must have exactly 6 segments
        let parsed = parse_segments(ipv6_part)?;
        if parsed.len() != 6 {
            return Err(ParseError::InvalidIpv6);
        }
        segments[..6].copy_from_slice(&parsed);
    }

    segments[6] = ipv4_high;
    segments[7] = ipv4_low;

    Ok(segments)
}

/// Parse a single hex segment (0-ffff).
fn parse_hex_segment(s: &str) -> Result<u16> {
    if s.is_empty() || s.len() > 4 {
        return Err(ParseError::InvalidIpv6);
    }
    u16::from_str_radix(s, 16).map_err(|_| ParseError::InvalidIpv6)
}

/// Parse colon-separated hex segments from a string.
fn parse_segments(s: &str) -> Result<Vec<u16>> {
    if s.is_empty() {
        return Ok(Vec::new());
    }
    s.split(':').map(parse_hex_segment).collect()
}

/// Parse an IPv4 address to u32.
fn parse_ipv4(s: &str) -> Result<u32> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return Err(ParseError::InvalidIpv4);
    }

    parts.iter().try_fold(0u32, |acc, part| {
        let byte: u8 = part.parse().map_err(|_| ParseError::InvalidIpv4)?;
        Ok((acc << 8) | u32::from(byte))
    })
}

/// Serialize IPv6 segments to string with compression.
pub fn serialize_ipv6(segments: &[u16; 8]) -> String {
    // Find longest sequence of zeros for compression
    let (compress_start, compress_len) = find_longest_zero_sequence(segments);

    let mut result = String::with_capacity(39);
    result.push('[');

    // Only compress sequences of 2+ zeros
    let compress_range = compress_start
        .filter(|_| compress_len > 1)
        .map(|start| start..start + compress_len);

    let mut i = 0;
    while i < 8 {
        // Check if we should insert :: for zero compression
        if let Some(ref range) = compress_range
            && range.start == i
        {
            result.push_str("::");
            i = range.end;
            continue;
        }

        if i > 0 && !result.ends_with("::") {
            result.push(':');
        }

        let _ = write!(&mut result, "{:x}", segments[i]);
        i += 1;
    }

    result.push(']');
    result
}

/// Find the longest sequence of consecutive zeros in IPv6 segments.
fn find_longest_zero_sequence(segments: &[u16; 8]) -> (Option<usize>, usize) {
    let mut best_start: Option<usize> = None;
    let mut best_len = 0;
    let mut current_start: Option<usize> = None;
    let mut current_len = 0;

    for (i, &segment) in segments.iter().enumerate() {
        if segment == 0 {
            if current_start.is_none() {
                current_start = Some(i);
                current_len = 1;
            } else {
                current_len += 1;
            }
            if current_len > best_len {
                best_start = current_start;
                best_len = current_len;
            }
        } else {
            current_start = None;
            current_len = 0;
        }
    }

    (best_start, best_len)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ipv6_loopback() {
        let result = parse_ipv6("[::1]").unwrap();
        assert_eq!(result, [0, 0, 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn test_parse_ipv6_full() {
        let result = parse_ipv6("[2001:db8:0:0:1:0:0:1]").unwrap();
        assert_eq!(result, [0x2001, 0xdb8, 0, 0, 1, 0, 0, 1]);
    }

    #[test]
    fn test_parse_ipv6_compressed() {
        let result = parse_ipv6("[2001:db8::1]").unwrap();
        assert_eq!(result, [0x2001, 0xdb8, 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn test_parse_ipv6_with_ipv4() {
        let result = parse_ipv6("[::127.0.0.1]").unwrap();
        // 127.0.0.1 = 0x7F000001 = high:0x7F00, low:0x0001
        assert_eq!(result, [0, 0, 0, 0, 0, 0, 0x7f00, 0x0001]);
    }

    #[test]
    fn test_parse_ipv6_with_ipv4_2() {
        let result = parse_ipv6("[::ffff:192.168.1.1]").unwrap();
        // 192.168.1.1 = 0xC0A80101 = high:0xC0A8, low:0x0101
        assert_eq!(result, [0, 0, 0, 0, 0, 0xffff, 0xc0a8, 0x0101]);
    }

    #[test]
    fn test_serialize_ipv6() {
        assert_eq!(serialize_ipv6(&[0, 0, 0, 0, 0, 0, 0, 1]), "[::1]");
        assert_eq!(
            serialize_ipv6(&[0x2001, 0xdb8, 0, 0, 0, 0, 0, 1]),
            "[2001:db8::1]"
        );
        assert_eq!(
            serialize_ipv6(&[0, 0, 0, 0, 0, 0, 0x7f00, 0x0001]),
            "[::7f00:1]"
        );
    }
}
