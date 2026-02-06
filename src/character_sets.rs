/// Check if a character is an ASCII tab or newline
pub fn is_ascii_tab_or_newline(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r')
}

/// Hostname character classification for fast path
/// Returns: 0=invalid, 1=valid passthrough, 2=uppercase, 3=delimiter, 4=reject
const HOSTNAME_CHAR_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];

    // Valid passthrough chars: a-z, 0-9, ., -
    let mut i = b'a';
    while i <= b'z' {
        table[i as usize] = 1;
        i += 1;
    }
    let mut i = b'0';
    while i <= b'9' {
        table[i as usize] = 1;
        i += 1;
    }
    table[b'.' as usize] = 1;
    table[b'-' as usize] = 1;

    // Uppercase (needs conversion)
    let mut i = b'A';
    while i <= b'Z' {
        table[i as usize] = 2;
        i += 1;
    }

    // Delimiters (end of hostname)
    table[b':' as usize] = 3;
    table[b'/' as usize] = 3;

    // Reject (needs special handling)
    table[b'@' as usize] = 4;
    table[b'?' as usize] = 4;
    table[b'#' as usize] = 4;
    table[b'%' as usize] = 4;
    table[b'\\' as usize] = 4;
    table[b'\t' as usize] = 4;
    table[b'\n' as usize] = 4;
    table[b'\r' as usize] = 4;

    table
};

/// Classify a byte for hostname parsing (branchless via lookup table)
pub fn classify_hostname_byte(b: u8) -> u8 {
    // Fast rejection for non-ASCII or control chars
    if !(0x20..0x7F).contains(&b) {
        return 0; // Invalid
    }
    HOSTNAME_CHAR_TABLE[b as usize]
}

/// Path character classification for fast path
/// Returns: 0=invalid/needs encoding, 1=valid passthrough, 2=query/fragment delimiter
const PATH_CHAR_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];

    // Safe path chars that can pass through: / - _ . ~ a-z A-Z 0-9 %
    table[b'/' as usize] = 1;
    table[b'-' as usize] = 1;
    table[b'_' as usize] = 1;
    table[b'.' as usize] = 1;
    table[b'~' as usize] = 1;
    table[b'%' as usize] = 1; // Percent-encoding marker

    let mut i = b'a';
    while i <= b'z' {
        table[i as usize] = 1;
        i += 1;
    }
    let mut i = b'A';
    while i <= b'Z' {
        table[i as usize] = 1;
        i += 1;
    }
    let mut i = b'0';
    while i <= b'9' {
        table[i as usize] = 1;
        i += 1;
    }

    // Delimiters that end the path (query/fragment)
    table[b'?' as usize] = 2;
    table[b'#' as usize] = 2;

    table
};

/// Classify a byte for path parsing (optimized with full 256-entry table)
/// Now we can do direct lookup without range check
pub fn classify_path_byte(b: u8) -> u8 {
    PATH_CHAR_TABLE[b as usize]
}
