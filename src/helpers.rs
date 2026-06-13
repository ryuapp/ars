use crate::character_sets::is_ascii_tab_or_newline;
use crate::compat::Cow;

#[inline]
pub fn find_byte(byte: u8, input: &[u8]) -> Option<usize> {
    #[cfg(feature = "std")]
    {
        memchr::memchr(byte, input)
    }

    #[cfg(not(feature = "std"))]
    {
        input.iter().position(|&candidate| candidate == byte)
    }
}

#[inline]
pub fn rfind_byte(byte: u8, input: &[u8]) -> Option<usize> {
    #[cfg(feature = "std")]
    {
        memchr::memrchr(byte, input)
    }

    #[cfg(not(feature = "std"))]
    {
        input.iter().rposition(|&candidate| candidate == byte)
    }
}

#[inline]
pub fn find_byte3(a: u8, b: u8, c: u8, input: &[u8]) -> Option<usize> {
    #[cfg(feature = "std")]
    {
        memchr::memchr3(a, b, c, input)
    }

    #[cfg(not(feature = "std"))]
    {
        input
            .iter()
            .position(|&candidate| candidate == a || candidate == b || candidate == c)
    }
}

#[inline]
pub fn contains_byte3(a: u8, b: u8, c: u8, input: &[u8]) -> bool {
    find_byte3(a, b, c, input).is_some()
}

/// Fast check if string contains tabs or newlines
pub fn has_tabs_or_newline(input: &str) -> bool {
    contains_byte3(b'\t', b'\n', b'\r', input.as_bytes())
}

/// Prune fragment (#hash) from URL string
/// Returns (`url_without_fragment`, `fragment_without_hash`)
/// Fragment is returned WITHOUT the leading '#' (matches ada-url)
pub fn prune_fragment(input: &str) -> (&str, Option<&str>) {
    find_byte(b'#', input.as_bytes()).map_or((input, None), |pos| {
        (&input[..pos], Some(&input[pos + 1..]))
    })
}

/// Combined trim and remove tabs/newlines in single pass.
/// Returns a Cow to avoid allocation when possible.
/// Removes leading/trailing C0 controls+space and internal tabs/newlines per WHATWG URL spec.
pub fn clean_tabs_and_newlines(input: &str) -> Cow<'_, str> {
    let bytes = input.as_bytes();

    // Fast path: check if any C0/space exists
    let has_control_chars = bytes.iter().any(|&b| b <= 0x20);
    if !has_control_chars {
        return Cow::Borrowed(input);
    }

    // Find first and last non-C0/space positions (for trimming)
    let start = bytes.iter().position(|&b| b > 0x20).unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|&b| b > 0x20)
        .map_or(0, |pos| pos + 1);

    if start >= end {
        return Cow::Borrowed("");
    }

    // Remove internal tabs/newlines/CR from trimmed range
    let trimmed = &input[start..end];
    if !has_tabs_or_newline(trimmed) {
        return Cow::Borrowed(trimmed);
    }

    Cow::Owned(
        trimmed
            .chars()
            .filter(|&c| !is_ascii_tab_or_newline(c))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_tabs_and_newlines() {
        // Test trim and remove combined
        assert_eq!(clean_tabs_and_newlines("\t\nhello\r\n"), "hello");
        assert_eq!(clean_tabs_and_newlines("hello"), "hello");
        assert_eq!(clean_tabs_and_newlines("\t\n\r"), "");
        assert_eq!(clean_tabs_and_newlines("hel\tlo\nworld"), "helloworld");

        // Test with spaces (should be trimmed from edges but kept internally)
        assert_eq!(clean_tabs_and_newlines("  hello  "), "hello");
        assert_eq!(clean_tabs_and_newlines("  hello world  "), "hello world");
        assert_eq!(clean_tabs_and_newlines("  foo.com  "), "foo.com");
    }
}
