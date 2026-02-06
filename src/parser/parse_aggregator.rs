use super::State;
use crate::checkers::{is_ipv4, parse_port};
use crate::compat::{Cow, String, ToString, Vec};
/// High-performance parser with single-buffer allocation (ada-url architecture)
/// Writes directly to buffer with offset tracking - eliminates multiple String allocations
use crate::error::{ParseError, Result};
use crate::ipv6::{parse_ipv6, serialize_ipv6};
use crate::scheme::get_scheme_type;
use crate::types::SchemeType;
use crate::unicode::idna::domain_to_ascii;
use crate::unicode::percent_encode::{
    percent_decode, percent_encode_fragment_into, percent_encode_path_into,
    percent_encode_userinfo_into,
};
use crate::url_aggregator::UrlAggregator;
use crate::url_components::UrlComponents;

/// Get the end position of pathname in a `UrlAggregator`'s buffer.
fn get_pathname_end(comp: &UrlComponents, buf_len: usize) -> usize {
    if comp.search_start > 0 {
        comp.search_start as usize
    } else if comp.hash_start > 0 {
        comp.hash_start as usize
    } else {
        buf_len
    }
}

/// Get the end position of search/query in a `UrlAggregator`'s buffer.
fn get_search_end(comp: &UrlComponents, buf_len: usize) -> usize {
    if comp.hash_start > 0 {
        comp.hash_start as usize
    } else {
        buf_len
    }
}

/// Check if a port number is the default port for a scheme type.
fn is_default_port(scheme_type: SchemeType, port: u16) -> bool {
    scheme_type.default_port() == Some(port)
}

/// Copy authority section from base URL to buffer and update components.
/// Returns true if base had authority section.
fn copy_authority_from_base(
    buffer: &mut String,
    components: &mut UrlComponents,
    base: &UrlAggregator,
) -> bool {
    let base_buf = &base.buffer;
    let base_comp = &base.components;
    let protocol_end_pos = base_comp.protocol_end as usize;

    // Check if base has authority (// after protocol)
    let has_authority = base_buf.get(protocol_end_pos..protocol_end_pos + 2) == Some("//");

    if !has_authority {
        // Set all authority-related offsets to protocol_end (no authority)
        let end = components.protocol_end;
        components.username_end = end;
        components.password_end = end;
        components.host_start = end;
        components.host_end = end;
        components.pathname_start = end;
        return false;
    }

    // Copy authority section and compute offset
    let auth_end = base_comp.pathname_start as usize;
    buffer.push_str(&base_buf[protocol_end_pos..auth_end]);
    let offset = buffer.len() as u32 - base_comp.pathname_start;

    // Apply offset to all authority components
    components.username_end = base_comp.username_end + offset;
    components.password_end = base_comp.password_end + offset;
    components.host_start = base_comp.host_start + offset;
    components.host_end = base_comp.host_end + offset;
    components.pathname_start = buffer.len() as u32;

    true
}

/// Check if bytes starting at position form a valid Windows drive letter.
/// A valid drive letter is: [a-zA-Z][:|] followed by [/\?#] or end of string.
fn is_windows_drive_letter(bytes: &[u8], pos: usize) -> bool {
    // Need at least 2 bytes for drive letter
    if pos + 1 >= bytes.len() {
        return false;
    }

    let first = bytes[pos];
    let second = bytes[pos + 1];

    // First must be ASCII letter, second must be : or |
    if !first.is_ascii_alphabetic() || !matches!(second, b':' | b'|') {
        return false;
    }

    // End of string is valid, otherwise third must be a delimiter
    pos + 2 >= bytes.len() || matches!(bytes[pos + 2], b'/' | b'\\' | b'?' | b'#')
}

/// Validate a URL without storing values (ada-url optimization)
///
/// This is optimized for `can_parse()` - validates URL structure
/// with early rejection for obviously invalid inputs.
///
/// # Errors
///
/// Returns an error if the URL is invalid according to the WHATWG URL Standard.
pub fn validate_url(input: &str, base_url: Option<&str>) -> Result<()> {
    // Early rejection optimizations for invalid URLs
    let trimmed = input.trim_matches(|c: char| c as u32 <= 0x20);

    // Empty string is invalid without base
    if trimmed.is_empty() && base_url.is_none() {
        return Err(ParseError::InvalidScheme);
    }

    // Quick check for obvious non-URLs (no scheme, no slash, contains spaces)
    if base_url.is_none() {
        let bytes = trimmed.as_bytes();

        // Must contain ':' for absolute URL
        if !trimmed.contains(':') {
            return Err(ParseError::InvalidScheme);
        }

        // First char must be ASCII alphabetic for scheme
        if let Some(&first) = bytes.first() {
            if !first.is_ascii_alphabetic() {
                return Err(ParseError::InvalidScheme);
            }
        }

        // Quick scan for invalid characters in first 32 bytes (likely scheme/host)
        let scan_len = bytes.len().min(32);
        for &b in &bytes[..scan_len] {
            // Reject obvious invalid chars (space, tab, <, >, etc.)
            if matches!(b, b' ' | b'\t' | b'<' | b'>' | b'{' | b'}') {
                return Err(ParseError::InvalidHost);
            }
        }
    }

    // Delegate to full parser
    parse_url_aggregator(input, base_url).map(|_| ())
}

/// Parse directly to `UrlAggregator` (single buffer allocation)
///
/// # Errors
///
/// Returns an error if the URL is invalid according to the WHATWG URL Standard.
pub fn parse_url_aggregator(input: &str, base_url: Option<&str>) -> Result<UrlAggregator> {
    // WHATWG spec step 1-2: Remove tabs/newlines and trim from FULL input (before fragment pruning)
    // Optimization: Most URLs don't have tabs/newlines, so check first (ada-url pattern)
    let input: Cow<str> = if crate::helpers::has_tabs_or_newline(input) {
        // Rare case: has tabs/newlines, need full cleaning
        Cow::Owned(crate::helpers::clean_tabs_and_newlines(input).into_owned())
    } else {
        // Common case: no tabs/newlines, just trim C0 controls + space (zero-copy)
        Cow::Borrowed(input.trim_matches(|c: char| c as u32 <= 0x20))
    };
    let input = input.as_ref();

    // Optimization: Prune fragment AFTER cleaning (ada-url pattern)
    // Most URLs don't have fragments, so this saves processing time
    // Fragment is returned WITHOUT the leading '#' (matches ada-url)
    let (input, fragment) = crate::helpers::prune_fragment(input);

    // Parse base URL if provided
    let base = base_url
        .map(|s| parse_url_aggregator(s, None))
        .transpose()?;

    // Try fast path for simple HTTP/HTTPS URLs (no base)
    if base.is_none()
        && input.len() >= 7
        && let Some(mut fast_result) = try_http_fast_path(input)
    {
        // Add fragment if present
        if let Some(frag) = fragment {
            fast_result.components.hash_start = fast_result.buffer.len() as u32;
            fast_result.buffer.push('#');
            percent_encode_fragment_into(&mut fast_result.buffer, frag);
        }
        return Ok(fast_result);
    }

    // Reserve capacity (ada-url: next power of 2)
    let capacity = input.len().max(1).next_power_of_two().max(16);

    let mut buffer = String::with_capacity(capacity);
    let mut components = UrlComponents::new();
    let mut scheme_type = SchemeType::NotSpecial;
    let mut state = State::SchemeStart;

    let bytes = input.as_bytes();
    let mut pointer = 0;

    // Main state machine
    while pointer <= bytes.len() {
        let c = if pointer < bytes.len() {
            let b = bytes[pointer];
            if b < 128 {
                Some(b as char)
            } else {
                // Multi-byte UTF-8 character - get it safely
                if input.is_char_boundary(pointer) {
                    input[pointer..].chars().next()
                } else {
                    // Skip to next char boundary
                    pointer += 1;
                    continue;
                }
            }
        } else {
            None
        };

        match state {
            State::SchemeStart => {
                // Batch processing: scan for entire scheme
                if pointer >= bytes.len() {
                    if base.is_some() {
                        state = State::NoScheme;
                        continue;
                    }
                    return Err(ParseError::InvalidScheme);
                }

                let first_byte = bytes[pointer];
                if !first_byte.is_ascii_alphabetic() {
                    state = State::NoScheme;
                    continue;
                }

                // Scan ahead for scheme end
                let scheme_start = pointer;
                let mut scheme_end = pointer;
                let mut valid = true;

                while scheme_end < bytes.len() {
                    let b = bytes[scheme_end];
                    if b == b':' {
                        break;
                    }
                    if !b.is_ascii_alphanumeric() && b != b'+' && b != b'-' && b != b'.' {
                        // Invalid scheme char - treat as no scheme
                        valid = false;
                        break;
                    }
                    scheme_end += 1;
                }

                if !valid || scheme_end >= bytes.len() || bytes[scheme_end] != b':' {
                    if base.is_some() {
                        state = State::NoScheme;
                        pointer = 0;
                        continue;
                    }
                    return Err(ParseError::InvalidScheme);
                }

                // Lowercase scheme directly into buffer (avoid allocation)
                let scheme_buffer_start = buffer.len();
                for b in input[scheme_start..scheme_end].bytes() {
                    buffer.push(b.to_ascii_lowercase() as char);
                }
                buffer.push(':');
                components.protocol_end = buffer.len() as u32;

                // Get scheme type from what we just wrote
                let scheme = &buffer[scheme_buffer_start..buffer.len() - 1];
                scheme_type = get_scheme_type(scheme);
                let is_file = scheme == "file";

                pointer = scheme_end + 1; // Skip ':'

                // Check if scheme matches base (for relative URLs like "http:path")
                if let Some(ref base) = base
                    && scheme_type.is_special()
                    && base.scheme_type() == scheme_type
                {
                    state = State::SpecialRelativeOrAuthority;
                    continue;
                }

                if is_file {
                    state = State::File;
                } else if scheme_type.is_special() {
                    state = State::SpecialAuthoritySlashes;
                } else {
                    state = State::PathOrAuthority;
                }
                continue;
            }

            State::NoScheme => {
                let Some(base_ref) = base.as_ref() else {
                    return Err(ParseError::RelativeUrlWithoutBase);
                };

                // Per WHATWG spec: "If url includes credentials or url's base URL is null,
                // or url's base URL has an opaque path and url is not a fragment-only URL, validation error, return failure."
                // Opaque path = non-special scheme with path not starting with "/"
                // Examples: "sc:sd", "sc:sd/sd" (opaque), vs "sc:/pa/pa" (not opaque - has authority)
                if base_ref.has_opaque_path() {
                    // Fragment-only = empty input + fragment present
                    let is_fragment_only = pointer >= bytes.len() && fragment.is_some();
                    if !is_fragment_only {
                        // Not fragment-only - opaque path base URL cannot resolve relative URLs
                        return Err(ParseError::RelativeUrlWithoutBase);
                    }
                }

                // Empty input (possibly with fragment) - copy base and update fragment
                if pointer >= bytes.len() {
                    // Clone the base
                    let mut temp_url = base_ref.clone();
                    // Remove old hash
                    if temp_url.components().hash_start > 0 {
                        let hash_start = temp_url.components().hash_start as usize;
                        temp_url.buffer_mut().truncate(hash_start);
                        temp_url.components_mut().hash_start = 0;
                    }
                    // Add new fragment if present
                    if let Some(frag) = fragment {
                        temp_url.components_mut().hash_start = temp_url.buffer_mut().len() as u32;
                        temp_url.buffer_mut().push('#');
                        percent_encode_fragment_into(temp_url.buffer_mut(), frag);
                    }
                    return Ok(temp_url);
                }

                // Otherwise, go to relative state
                state = State::Relative;
                continue;
            }

            State::SpecialRelativeOrAuthority => {
                // Check if next is "//"
                if pointer + 1 < bytes.len() && bytes[pointer] == b'/' && bytes[pointer + 1] == b'/'
                {
                    buffer.push_str("//");
                    pointer += 2;
                    // For file: URLs, use FileHost state to detect Windows drive letters
                    if scheme_type == SchemeType::File {
                        state = State::FileHost;
                    } else {
                        state = State::Authority;
                    }
                } else {
                    // Go to relative state
                    state = State::Relative;
                }
                continue;
            }

            State::Relative => {
                // Copy scheme from base (only if we don't already have one)
                let Some(base_ref) = base.as_ref() else {
                    return Err(ParseError::RelativeUrlWithoutBase);
                };
                if components.protocol_end == 0 {
                    let base_scheme = base_ref.protocol();
                    buffer.push_str(base_scheme);
                    components.protocol_end = buffer.len() as u32;
                    scheme_type = base_ref.scheme_type();
                } else {
                    // Scheme already set (came from SpecialRelativeOrAuthority)
                    // scheme_type is already set
                }

                // Check next character
                if pointer >= bytes.len() {
                    // End of input - copy everything from base
                    copy_authority_from_base(&mut buffer, &mut components, base_ref);

                    // Copy pathname from base
                    let base_comp = &base_ref.components;
                    let base_buf = &base_ref.buffer;
                    let path_start = base_comp.pathname_start as usize;
                    let path_end = get_pathname_end(base_comp, base_buf.len());
                    buffer.push_str(&base_buf[path_start..path_end]);

                    // Copy query from base
                    if base_comp.search_start > 0 {
                        let search_end = get_search_end(base_comp, base_buf.len());
                        components.search_start = buffer.len() as u32;
                        buffer.push_str(&base_buf[base_comp.search_start as usize..search_end]);
                    }

                    // Add new fragment if present
                    if let Some(frag) = fragment {
                        components.hash_start = buffer.len() as u32;
                        buffer.push('#');
                        percent_encode_fragment_into(&mut buffer, frag);
                    }

                    return Ok(UrlAggregator {
                        buffer,
                        components,
                        scheme_type,
                    });
                }

                let next_char = bytes[pointer] as char;

                // For file: URLs, check if we have a Windows drive letter
                if scheme_type == SchemeType::File && is_windows_drive_letter(bytes, pointer) {
                    let first = bytes[pointer];

                    // Windows drive letter - copy host from base, replace path
                    copy_authority_from_base(&mut buffer, &mut components, base_ref);

                    // Write the drive letter path
                    components.pathname_start = buffer.len() as u32;
                    buffer.push('/');
                    buffer.push(first as char); // Preserve original case
                    buffer.push(':'); // Normalize | to :
                    pointer += 2;

                    // Check if there's more to parse
                    if pointer < bytes.len() {
                        let next_byte = bytes[pointer];
                        if next_byte == b'/' || next_byte == b'\\' {
                            state = State::Path;
                            continue;
                        } else if next_byte == b'?' {
                            components.search_start = buffer.len() as u32;
                            buffer.push('?');
                            pointer += 1;
                            state = State::Query;
                            continue;
                        } else if next_byte == b'#' {
                            components.hash_start = buffer.len() as u32;
                            buffer.push('#');
                            pointer += 1;
                            state = State::Fragment;
                            continue;
                        }
                    }
                    break;
                }

                if next_char == '/' || (scheme_type.is_special() && next_char == '\\') {
                    state = State::RelativeSlash;
                    pointer += 1;
                } else if next_char == '?' {
                    // Copy authority and path from base, then parse query
                    copy_authority_from_base(&mut buffer, &mut components, base_ref);

                    // Copy pathname from base
                    let base_comp = &base_ref.components;
                    let base_buf = &base_ref.buffer;
                    let path_start = base_comp.pathname_start as usize;
                    let path_end = get_pathname_end(base_comp, base_buf.len());
                    buffer.push_str(&base_buf[path_start..path_end]);

                    components.search_start = buffer.len() as u32;
                    buffer.push('?');
                    pointer += 1;
                    state = State::Query;
                } else if next_char == '#' {
                    // Fragment-only URL - copy everything from base (including query), then parse fragment
                    copy_authority_from_base(&mut buffer, &mut components, base_ref);

                    // Copy pathname from base
                    let base_comp = &base_ref.components;
                    let base_buf = &base_ref.buffer;
                    let path_start = base_comp.pathname_start as usize;
                    let path_end = get_pathname_end(base_comp, base_buf.len());
                    buffer.push_str(&base_buf[path_start..path_end]);

                    // Copy query from base
                    if base_comp.search_start > 0 {
                        let search_end = get_search_end(base_comp, base_buf.len());
                        components.search_start = buffer.len() as u32;
                        buffer.push_str(&base_buf[base_comp.search_start as usize..search_end]);
                    }

                    components.hash_start = buffer.len() as u32;
                    buffer.push('#');
                    pointer += 1;
                    state = State::Fragment;
                } else {
                    // Copy authority and path from base, then shorten path and continue with Path state
                    // Only copy authority if base doesn't have opaque path
                    if !base_ref.has_opaque_path() {
                        copy_authority_from_base(&mut buffer, &mut components, base_ref);
                    }

                    // Copy pathname from base
                    let base_comp = &base_ref.components;
                    let base_buf = &base_ref.buffer;
                    let path_start = base_comp.pathname_start as usize;
                    let path_end = get_pathname_end(base_comp, base_buf.len());
                    buffer.push_str(&base_buf[path_start..path_end]);

                    // Shorten path - find last '/' and truncate there
                    let pathname_start = components.pathname_start as usize;
                    if let Some(last_slash) =
                        memchr::memrchr(b'/', &buffer.as_bytes()[pathname_start..buffer.len()])
                    {
                        buffer.truncate(pathname_start + last_slash);
                    }

                    state = State::Path;
                    continue;
                }
                continue;
            }

            State::RelativeSlash => {
                // Check if next char is '/' or '\' (for authority - special URLs normalize backslash)
                if pointer < bytes.len() {
                    let next = bytes[pointer];
                    if next == b'/' || (scheme_type.is_special() && next == b'\\') {
                        buffer.push_str("//");
                        pointer += 1;
                        // For file: URLs, use FileHost state to detect Windows drive letters
                        state = if scheme_type == SchemeType::File {
                            State::FileHost
                        } else {
                            State::Authority
                        };
                        continue;
                    }
                }

                // Just a single slash - copy authority from base and parse path
                let Some(base_ref) = base.as_ref() else {
                    return Err(ParseError::RelativeUrlWithoutBase);
                };
                let mut temp_url = UrlAggregator::with_capacity(buffer.capacity());
                temp_url.buffer_mut().clone_from(&buffer);
                temp_url.components_mut().protocol_end = components.protocol_end;
                temp_url.set_scheme_type(scheme_type);
                temp_url.copy_authority_from(base_ref);

                // For file: URLs, preserve Windows drive letter from base path if present
                // BUT only if the input doesn't already contain a drive letter
                if scheme_type == SchemeType::File {
                    // Check if remaining input starts with a Windows drive letter
                    let input_has_drive_letter = is_windows_drive_letter(bytes, pointer);

                    if !input_has_drive_letter {
                        let base_pathname = base_ref.pathname();
                        // Check if base has a Windows drive letter (e.g., "/C:")
                        if base_pathname.len() >= 3 && base_pathname.starts_with('/') {
                            let chars: Vec<char> = base_pathname.chars().collect();
                            if chars.len() >= 3 && chars[1].is_ascii_alphabetic() && chars[2] == ':'
                            {
                                // Base has drive letter - copy it to the new path
                                let drive_letter = &base_pathname[0..3]; // "/C:"
                                temp_url.buffer_mut().push_str(drive_letter);
                            }
                        }
                    }
                }

                buffer.clone_from(temp_url.buffer_mut());
                components = temp_url.components().clone();
                state = State::Path;
                continue;
            }

            State::SpecialAuthoritySlashes => {
                // Special URLs normalize backslash to forward slash
                if let Some(ch) = c {
                    if ch == '/' || ch == '\\' {
                        // Check if next char is also '/' or '\'
                        if pointer + 1 < bytes.len() {
                            let next = bytes[pointer + 1] as char;
                            if next == '/' || next == '\\' {
                                buffer.push_str("//");
                                pointer += 1;
                                state = State::Authority;
                            } else {
                                // Single slash/backslash - add missing slash and consume it
                                buffer.push_str("//");
                                pointer += 1;
                                state = State::Authority;
                                continue;
                            }
                        } else {
                            // At end, single slash/backslash
                            buffer.push_str("//");
                            pointer += 1;
                            state = State::Authority;
                            continue;
                        }
                    } else {
                        // No slash/backslash - still need authority marker
                        buffer.push_str("//");
                        state = State::Authority;
                        continue;
                    }
                } else {
                    // EOF - still need authority marker
                    buffer.push_str("//");
                    state = State::Authority;
                    continue;
                }
            }

            State::Authority => {
                // Parse authority: use memchr for faster scanning
                let mut auth_start = pointer;

                // For special schemes, skip leading slashes/backslashes in authority (Tests #856-859)
                // "///test" at pointer → skip leading "/" → start at "test"
                if scheme_type.is_special() {
                    while auth_start < bytes.len()
                        && (bytes[auth_start] == b'/' || bytes[auth_start] == b'\\')
                    {
                        auth_start += 1;
                    }
                }

                let remaining = &bytes[auth_start..];

                // Find end of authority (/, ?, #, or \ for special URLs)
                let auth_len = if scheme_type.is_special() {
                    // For special URLs, also treat backslash as path delimiter
                    let mut end = remaining.len();
                    for (i, &b) in remaining.iter().enumerate() {
                        if b == b'/' || b == b'?' || b == b'#' || b == b'\\' {
                            end = i;
                            break;
                        }
                    }
                    end
                } else {
                    memchr::memchr3(b'/', b'?', b'#', remaining).unwrap_or(remaining.len())
                };
                let auth_end = auth_start + auth_len;

                // Find LAST @ if present (for credentials) - WHATWG spec says use last @
                let at_pos =
                    memchr::memrchr(b'@', &bytes[auth_start..auth_end]).map(|pos| auth_start + pos);

                let authority = &input[auth_start..auth_end];

                if let Some(at_idx) = at_pos {
                    let credentials = &authority[0..(at_idx - auth_start)];

                    // Check if we have credentials to write
                    let has_credentials = !credentials.is_empty() && credentials != ":";

                    if has_credentials {
                        // Find FIRST colon to split username/password
                        if let Some(colon) = memchr::memchr(b':', credentials.as_bytes()) {
                            let username = &credentials[0..colon];
                            let password = &credentials[colon + 1..]; // Everything after first colon

                            // Write username (can be empty)
                            if !username.is_empty() {
                                percent_encode_userinfo_into(&mut buffer, username);
                            }
                            components.username_end = buffer.len() as u32;

                            // Write password only if not empty
                            if password.is_empty() {
                                // Empty password - don't write ":"
                                components.password_end = components.username_end;
                            } else {
                                buffer.push(':');
                                percent_encode_userinfo_into(&mut buffer, password);
                                components.password_end = buffer.len() as u32;
                            }
                        } else {
                            // Only username, no colon
                            percent_encode_userinfo_into(&mut buffer, credentials);
                            components.username_end = buffer.len() as u32;
                            components.password_end = components.username_end;
                        }

                        // Only add @ if we actually wrote credentials
                        if components.username_end > components.protocol_end + 2
                            || components.password_end > components.username_end
                        {
                            buffer.push('@');
                        }
                    } else {
                        // Empty credentials - set positions but don't write anything
                        components.username_end = components.protocol_end + 2;
                        components.password_end = components.username_end;
                    }

                    let host_part = &authority[(at_idx - auth_start + 1)..];
                    // Empty host with credentials is invalid (both special and non-special, except file:)
                    // Per WHATWG: if url includes credentials and url's host is null, validation error, return failure
                    if scheme_type != SchemeType::File && host_part.is_empty() {
                        return Err(ParseError::InvalidHost);
                    }
                    components.host_start = buffer.len() as u32;
                    parse_host_and_port(host_part, &mut buffer, &mut components, scheme_type)?;
                } else {
                    // Check if authority contains only port separator (":") with no host
                    // This handles cases like "sc://:" and "sc://:8080/"
                    if authority.starts_with(':') {
                        return Err(ParseError::InvalidHost);
                    }

                    // Empty authority (no @ present) with empty host
                    // Allow if followed by a path starting with '/' (e.g., "///test" -> empty auth + "/test" path)
                    let followed_by_path = auth_end < bytes.len() && bytes[auth_end] == b'/';
                    if scheme_type.is_special()
                        && scheme_type != SchemeType::File
                        && authority.is_empty()
                        && !followed_by_path
                    {
                        return Err(ParseError::InvalidHost);
                    }
                    components.host_start = buffer.len() as u32;
                    parse_host_and_port(authority, &mut buffer, &mut components, scheme_type)?;
                }

                if components.pathname_start == 0 {
                    components.pathname_start = buffer.len() as u32;
                }
                pointer = auth_end;
                // For non-special schemes, use OpaquePath to preserve slashes
                if scheme_type.is_special() {
                    state = State::Path;
                } else {
                    state = State::OpaquePath;
                }
                continue;
            }

            State::PathOrAuthority => {
                // Check if we have "//" for authority
                if pointer + 1 < bytes.len() && bytes[pointer] == b'/' && bytes[pointer + 1] == b'/'
                {
                    // Authority
                    buffer.push_str("//");
                    pointer += 2;
                    state = State::Authority;
                    continue; // Don't fall through to pointer += 1
                }
                // No "//" - this is an opaque path for non-special schemes
                // No authority, so set host markers to current position (empty host)
                components.host_start = buffer.len() as u32;
                components.host_end = buffer.len() as u32;
                components.pathname_start = buffer.len() as u32;
                state = State::OpaquePath;
                continue;
            }

            State::File => {
                // Set empty host for file: URLs
                buffer.push_str("//");
                components.host_start = buffer.len() as u32;
                components.host_end = buffer.len() as u32;

                // Check for Windows drive letter using strict validation
                if is_windows_drive_letter(bytes, pointer) {
                    let first = bytes[pointer];

                    // Windows drive letter - start path with /
                    components.pathname_start = buffer.len() as u32;
                    buffer.push('/');
                    // Preserve original case of drive letter
                    buffer.push(first as char);
                    // Normalize | to :
                    buffer.push(':');
                    pointer += 2;
                    state = State::Path;
                    continue;
                }

                // Check next character for slashes
                if pointer < bytes.len() {
                    let next = bytes[pointer] as char;
                    if next == '/' || next == '\\' {
                        state = State::FileSlash;
                        pointer += 1;
                        continue;
                    }
                }

                // No slash - go directly to path
                components.pathname_start = buffer.len() as u32;
                state = State::Path;
                continue;
            }

            State::FileSlash => {
                // After file:/ we might have another / for file:///
                if pointer < bytes.len() {
                    let next = bytes[pointer] as char;
                    if next == '/' || next == '\\' {
                        // file:// - go to FileHost
                        pointer += 1;
                        state = State::FileHost;
                        continue;
                    }
                }

                // Just file:/ - start path
                // Decrement pointer so Path state can re-process the current character
                // (This matches ada-url's "decrease pointer by 1" behavior)
                components.pathname_start = buffer.len() as u32;
                pointer -= 1;
                state = State::Path;
                continue;
            }

            State::FileHost => {
                // Parse host for file:// URLs
                let host_start = pointer;
                let mut host_end = pointer;

                // Find end of host (/, \, ?, or #)
                while host_end < bytes.len() {
                    let b = bytes[host_end];
                    if b == b'/' || b == b'\\' || b == b'?' || b == b'#' {
                        break;
                    }
                    host_end += 1;
                }

                let host_str = &input[host_start..host_end];

                // Check if percent-encoded content would decode to a Windows drive letter
                // Per WHATWG spec, percent-encoded drive letters must be rejected in file: URLs
                if host_str.contains('%') {
                    use crate::unicode::percent_encode::percent_decode;
                    if let Ok(decoded) = percent_decode(host_str) {
                        let decoded_bytes = decoded.as_bytes();
                        if decoded_bytes.len() >= 2 {
                            let first = decoded_bytes[0];
                            let second = decoded_bytes[1];
                            // Check if decoded content forms a drive letter (e.g., "%43%3A" -> "C:")
                            if first.is_ascii_alphabetic() && (second == b':' || second == b'|') {
                                // Reject: percent-encoded drive letters are invalid
                                return Err(ParseError::InvalidHost);
                            }
                        }
                    }
                }

                // Check if it's a Windows drive letter using strict validation
                if is_windows_drive_letter(bytes, host_start) {
                    // This is a Windows drive letter, not a host
                    // Set empty host (host_start == host_end)
                    components.host_start = buffer.len() as u32;
                    components.host_end = buffer.len() as u32;
                    // Write the drive letter to buffer
                    components.pathname_start = buffer.len() as u32;
                    buffer.push('/');
                    // Preserve case of drive letter
                    buffer.push(bytes[host_start] as char);
                    // Normalize | to :
                    buffer.push(':');
                    pointer = host_start + 2;

                    // Check if there's more to parse (path continuation)
                    if pointer < bytes.len() {
                        let next_byte = bytes[pointer];
                        if next_byte == b'/'
                            || next_byte == b'\\'
                            || next_byte == b'?'
                            || next_byte == b'#'
                        {
                            // Continue with Path/Query/Hash state
                            state = State::Path;
                            continue;
                        }
                    }
                    // No path continuation - done (drive letter only, no trailing slash)
                    break;
                }

                // Parse as host
                // Set host_start before processing
                components.host_start = buffer.len() as u32;

                if !host_str.is_empty() {
                    // Step 1: Percent-decode the hostname for file: URLs
                    use crate::unicode::percent_encode::percent_decode;
                    let decoded_host = if host_str.contains('%') {
                        // Validate percent encoding first (% must be followed by 2 hex digits)
                        let bytes = host_str.as_bytes();
                        let mut i = 0;
                        while i < bytes.len() {
                            if bytes[i] == b'%' {
                                if i + 2 >= bytes.len()
                                    || !bytes[i + 1].is_ascii_hexdigit()
                                    || !bytes[i + 2].is_ascii_hexdigit()
                                {
                                    return Err(ParseError::InvalidHost);
                                }
                                i += 3;
                            } else {
                                i += 1;
                            }
                        }
                        // Decode
                        percent_decode(host_str)?
                    } else {
                        host_str.to_string()
                    };

                    // Step 2: Remove soft hyphens (U+00AD)
                    let without_soft_hyphens: String = decoded_host
                        .chars()
                        .filter(|&ch| ch != '\u{00AD}')
                        .collect();

                    // Step 3: Check if hostname becomes empty after removing soft hyphens
                    // If the original had soft hyphens and becomes empty, it's invalid (Test #721, #722)
                    if decoded_host.contains('\u{00AD}') && without_soft_hyphens.is_empty() {
                        return Err(ParseError::InvalidHost);
                    }

                    let normalized_host = without_soft_hyphens;

                    // Step 4: Check for invalid punycode (Test #723)
                    // "xn--" followed by nothing or just "/" is invalid punycode
                    let lower = normalized_host.to_lowercase();
                    if lower == "xn--" || lower.starts_with("xn--") && lower.len() == 4 {
                        return Err(ParseError::InvalidHost);
                    }

                    // Step 5: Process hostname through IDNA if it contains non-ASCII
                    // This handles Test #720 (mathematical alphanumeric symbols → ASCII)
                    let processed_host = if normalized_host.is_ascii() {
                        normalized_host
                    } else {
                        use crate::unicode::idna::domain_to_ascii;
                        domain_to_ascii(&normalized_host)?
                    };

                    // Step 6: Check if it's "localhost" (Test #720)
                    // After IDNA processing, check case-insensitively
                    let is_localhost = processed_host.eq_ignore_ascii_case("localhost");

                    if !is_localhost {
                        // file: URLs cannot have ports (colons indicate port, except in IPv6)
                        if processed_host.contains(':')
                            && !(processed_host.starts_with('[') && processed_host.ends_with(']'))
                        {
                            return Err(ParseError::InvalidHost);
                        }

                        // Check for IPv6
                        if processed_host.starts_with('[') {
                            if !processed_host.ends_with(']') {
                                return Err(ParseError::InvalidHost);
                            }
                            // Validate IPv6 syntax
                            let _ = parse_ipv6(&processed_host)?;
                            buffer.push_str(&processed_host);
                        } else {
                            // ASCII-lowercase the hostname
                            for b in processed_host.bytes() {
                                buffer.push(b.to_ascii_lowercase() as char);
                            }
                        }
                    }
                    // If localhost, don't write anything to buffer (empty hostname)
                }
                // Set host_end after processing (same as host_start if empty/localhost)
                components.host_end = buffer.len() as u32;

                pointer = host_end;
                components.pathname_start = buffer.len() as u32;
                state = State::Path;
                continue;
            }

            State::Path => {
                // Batch processing: scan ahead for entire path
                let path_start = pointer;
                let mut path_end = pointer;

                // Find end of path
                while path_end < bytes.len() {
                    let b = bytes[path_end];
                    if b == b'?' || b == b'#' {
                        break;
                    }
                    path_end += 1;
                }

                // Process path in segments
                let path = if input.is_char_boundary(path_start) && input.is_char_boundary(path_end)
                {
                    &input[path_start..path_end]
                } else {
                    pointer = path_end;
                    if path_end < bytes.len() {
                        continue;
                    }
                    break;
                };

                // For special URLs, normalize backslashes to forward slashes
                let mut normalized_path: Cow<str> =
                    if scheme_type.is_special() && path.contains('\\') {
                        Cow::Owned(path.replace('\\', "/"))
                    } else {
                        Cow::Borrowed(path)
                    };

                // For file: URLs, normalize Windows drive letters (e.g., "/c|/" -> "/c:/" or "c|/" -> "c:/")
                if scheme_type == SchemeType::File && normalized_path.contains('|') {
                    let bytes = normalized_path.as_bytes();
                    let mut should_normalize = false;

                    // Check for pattern: /X| (starts with slash)
                    if normalized_path.len() >= 3
                        && bytes[0] == b'/'
                        && bytes[1].is_ascii_alphabetic()
                        && bytes[2] == b'|'
                    {
                        // Check what follows the |
                        if normalized_path.len() == 3 || bytes[3] == b'/' || bytes[3] == b'\\' {
                            should_normalize = true;
                        }
                    }
                    // Check for pattern: X| (starts without slash)
                    else if normalized_path.len() >= 2
                        && bytes[0].is_ascii_alphabetic()
                        && bytes[1] == b'|'
                    {
                        // Check what follows the |
                        if normalized_path.len() == 2 || bytes[2] == b'/' || bytes[2] == b'\\' {
                            should_normalize = true;
                        }
                    }

                    if should_normalize {
                        let replaced = normalized_path.replacen('|', ":", 1);
                        normalized_path = Cow::Owned(replaced);
                    }
                }

                // Process path segments using ada-url algorithm (preserves consecutive slashes)
                let pathname_start = components.pathname_start as usize;

                // Get current path from buffer (might be non-empty from base URL)
                let mut path = buffer[pathname_start..].to_string();

                // ada-url's PATH_START state skips the leading '/' before calling consume_prepared_path
                // So we strip it here to match that behavior
                let mut input = normalized_path.as_ref();
                let has_leading_slash = input.starts_with('/');
                if has_leading_slash {
                    input = &input[1..];
                }

                // Fast path: if path has no dots and buffer path is empty, skip segment processing
                // Check for patterns that need normalization: /., /.., %2e, %2E
                let needs_normalization = input.contains("/.")
                    || input.starts_with('.')
                    || input.contains("%2e")
                    || input.contains("%2E");

                if !needs_normalization && path.is_empty() {
                    // Fast path: no dot segments to resolve, just encode directly
                    buffer.push('/');
                    percent_encode_path_into(&mut buffer, input);

                    // Update pointer
                    pointer = path_end;

                    // Handle query or hash
                    if pointer < bytes.len() {
                        if bytes[pointer] == b'?' {
                            components.search_start = buffer.len() as u32;
                            buffer.push('?');
                            state = State::Query;
                            pointer += 1;
                        } else if bytes[pointer] == b'#' {
                            components.hash_start = buffer.len() as u32;
                            buffer.push('#');
                            state = State::Fragment;
                            pointer += 1;
                        }
                        continue;
                    }
                    break;
                }

                // Slow path: need to process segments for dot resolution
                loop {
                    // Find next slash
                    let location = memchr::memchr(b'/', input.as_bytes());
                    let segment = if let Some(loc) = location {
                        let seg = &input[0..loc];
                        input = &input[loc + 1..];
                        seg
                    } else {
                        input
                    };

                    // Check for dot segments
                    let is_single_dot = segment == "." || segment.eq_ignore_ascii_case("%2e");
                    let is_double_dot = segment == ".."
                        || segment.eq_ignore_ascii_case(".%2e")
                        || segment.eq_ignore_ascii_case("%2e.")
                        || segment.eq_ignore_ascii_case("%2e%2e");

                    if is_double_dot {
                        // Shorten path (remove last segment)
                        if let Some(slash_pos) = path.rfind('/') {
                            // For file: URLs, preserve Windows drive letter
                            // Check if current path is a drive letter (e.g., "/d:" or "/d:/")
                            let is_drive_letter_path = scheme_type == SchemeType::File
                                && path.len() >= 3
                                && path.starts_with('/')
                                && path.as_bytes()[1].is_ascii_alphabetic()
                                && path.as_bytes()[2] == b':'
                                && (path.len() == 3
                                    || (path.len() == 4 && path.as_bytes()[3] == b'/'));

                            if !is_drive_letter_path {
                                // Also check if truncation would remove drive letter
                                let would_truncate_to = &path.as_bytes()[0..slash_pos];
                                let should_preserve = scheme_type == SchemeType::File
                                    && would_truncate_to.len() == 2
                                    && would_truncate_to[0] == b'/'
                                    && would_truncate_to[1].is_ascii_alphabetic();

                                if !should_preserve {
                                    // Per WPT test #150: when shortening a path that consists only of slashes,
                                    // we cannot shorten below "//" (which represents one empty segment)
                                    // Check if path is all slashes and would become just "/"
                                    let is_all_slashes = path.chars().all(|c| c == '/');
                                    if is_all_slashes && slash_pos == 1 {
                                        // Keep as "//" instead of shortening to "/"
                                        // Don't truncate
                                    } else {
                                        path.truncate(slash_pos);
                                    }
                                }
                            }
                        }
                        // If this is the last segment, add trailing slash (if not already present)
                        if location.is_none() && !path.ends_with('/') {
                            path.push('/');
                        }
                    } else if is_single_dot && location.is_none() {
                        // Single dot at end - just add trailing slash
                        path.push('/');
                    } else if !is_single_dot {
                        // Normal segment (ada-url: always add '/' then content)
                        path.push('/');
                        if !segment.is_empty() {
                            // Encode into a temp string, then append to path
                            let mut encoded = String::new();
                            percent_encode_path_into(&mut encoded, segment);
                            path.push_str(&encoded);
                        }
                    }
                    // else: single dot in middle - skip it

                    // If no more slashes, we're done
                    if location.is_none() {
                        break;
                    }
                }

                // Replace path in buffer
                buffer.truncate(pathname_start);
                buffer.push_str(&path);

                // Update pointer
                pointer = path_end;

                // Handle query or hash
                if pointer < bytes.len() {
                    if bytes[pointer] == b'?' {
                        components.search_start = buffer.len() as u32;
                        buffer.push('?');
                        state = State::Query;
                        pointer += 1;
                    } else if bytes[pointer] == b'#' {
                        components.hash_start = buffer.len() as u32;
                        buffer.push('#');
                        state = State::Fragment;
                        pointer += 1;
                    }
                    continue;
                }
                // Per ada-url: insert "/." before pathname if:
                // 1. pathname starts with "//"
                // 2. no authority (no "//" after protocol in buffer)
                // 3. not a special scheme
                let pathname_start_pos = components.pathname_start as usize;
                let protocol_end_pos = components.protocol_end as usize;
                let has_authority = protocol_end_pos + 2 <= components.host_start as usize
                    && buffer[protocol_end_pos..protocol_end_pos + 2] == *"//";
                if pathname_start_pos + 2 <= buffer.len()
                    && buffer[pathname_start_pos..].starts_with("//")
                    && !has_authority
                    && !scheme_type.is_special()
                {
                    // Insert "/." at pathname_start
                    buffer.insert_str(pathname_start_pos, "/.");
                    // Adjust offsets
                    components.pathname_start += 2;
                    if components.search_start > 0 {
                        components.search_start += 2;
                    }
                    if components.hash_start > 0 {
                        components.hash_start += 2;
                    }
                }
                break;
            }

            State::Query => {
                // Batch processing: scan for end of query
                let query_start = pointer;
                let mut query_end = pointer;

                while query_end < bytes.len() && bytes[query_end] != b'#' {
                    query_end += 1;
                }

                // Append query string with percent encoding
                // Use special query encoding for special URLs (matches ada-url)
                if query_start < query_end
                    && input.is_char_boundary(query_start)
                    && input.is_char_boundary(query_end)
                {
                    use crate::unicode::percent_encode::{
                        QUERY_SET, SPECIAL_QUERY_SET, percent_encode_into,
                    };
                    let encode_set = if scheme_type.is_special() {
                        SPECIAL_QUERY_SET
                    } else {
                        QUERY_SET
                    };
                    percent_encode_into(&mut buffer, &input[query_start..query_end], encode_set);
                }

                pointer = query_end;
                if pointer < bytes.len() && bytes[pointer] == b'#' {
                    components.hash_start = buffer.len() as u32;
                    buffer.push('#');
                    state = State::Fragment;
                    pointer += 1;
                    continue;
                }
                break;
            }

            State::OpaquePath => {
                // For opaque paths, process dot segments but preserve structure
                let path_start = pointer;
                let mut path_end = pointer;

                // Find end of path (? or #)
                while path_end < bytes.len() {
                    let b = bytes[path_end];
                    if b == b'?' || b == b'#' {
                        break;
                    }
                    path_end += 1;
                }

                // Process path with dot segment normalization
                if path_start < path_end
                    && input.is_char_boundary(path_start)
                    && input.is_char_boundary(path_end)
                {
                    let path_str = &input[path_start..path_end];

                    // Track if path starts with / and ends with / (for proper reconstruction)
                    let starts_with_slash = path_str.starts_with('/');
                    let ends_with_slash = path_str.ends_with('/') && path_str.len() > 1;

                    // Process segments separated by /
                    let mut segments: Vec<String> = Vec::new();
                    let parts: Vec<&str> = path_str.split('/').collect();

                    for (i, &segment) in parts.iter().enumerate() {
                        // Handle empty segments carefully to preserve consecutive slashes
                        if segment.is_empty() {
                            if i == 0 && starts_with_slash {
                                // Leading slash - skip this empty segment
                                continue;
                            } else if i == parts.len() - 1 && ends_with_slash {
                                // Trailing slash - will be handled separately
                                continue;
                            }
                            // Middle empty segment (consecutive slashes) - ADD it as empty to preserve structure
                            segments.push(String::new());
                            continue;
                        }

                        // Check for dot segments
                        let is_single_dot = segment == "." || segment.eq_ignore_ascii_case("%2e");
                        let is_double_dot = segment == ".."
                            || segment.eq_ignore_ascii_case(".%2e")
                            || segment.eq_ignore_ascii_case("%2e.")
                            || segment.eq_ignore_ascii_case("%2e%2e");

                        if is_double_dot {
                            // Remove last segment
                            segments.pop();
                        } else if !is_single_dot {
                            // Encode and add non-dot segment
                            let has_authority = components.host_start < components.host_end;

                            let encoded = if has_authority {
                                // With authority: use OPAQUE_PATH_SET (encodes space, <, >, etc.)
                                use crate::unicode::percent_encode::{
                                    OPAQUE_PATH_SET, percent_encode_with_set,
                                };
                                percent_encode_with_set(segment, OPAQUE_PATH_SET)
                            } else {
                                // Without authority: encode C0 controls, special chars, but keep internal spaces
                                // EXCEPT: only the LAST trailing space (right before end) is encoded
                                let trimmed_segment = segment.trim_end_matches(' ');
                                let trailing_spaces = segment.len() - trimmed_segment.len();

                                let mut result = String::new();

                                // Encode the main part (with internal spaces kept)
                                for ch in trimmed_segment.chars() {
                                    let code = ch as u32;
                                    if code < 0x20 || code == 0x7F || code >= 0x80 {
                                        // Encode C0 controls, DEL, and non-ASCII
                                        use core::fmt::Write;
                                        for byte in ch.to_string().bytes() {
                                            let _ = write!(result, "%{byte:02X}");
                                        }
                                    } else {
                                        result.push(ch);
                                    }
                                }

                                // Keep all trailing spaces literal EXCEPT the last one
                                if trailing_spaces > 0 {
                                    // Add all but last space as literal
                                    for _ in 0..(trailing_spaces - 1) {
                                        result.push(' ');
                                    }
                                    // Encode only the LAST space
                                    result.push_str("%20");
                                }

                                result
                            };

                            segments.push(encoded);
                        }
                        // Single dots are ignored
                    }

                    // Reconstruct path
                    if starts_with_slash {
                        buffer.push('/');
                    }

                    for (i, segment) in segments.iter().enumerate() {
                        if i > 0 {
                            buffer.push('/');
                        }
                        buffer.push_str(segment);
                    }

                    // Add trailing slash if original had one
                    if ends_with_slash && !segments.is_empty() {
                        buffer.push('/');
                    }
                }

                pointer = path_end;

                // Per WHATWG spec: encode trailing spaces in opaque paths before query/hash delimiters
                // Tests #278-284: "opaque  ?" → "opaque %20?" (last space encoded, first kept literal)
                if pointer < bytes.len() && (bytes[pointer] == b'?' || bytes[pointer] == b'#') {
                    // Check if pathname ends with space(s)
                    let pathname_start_pos = components.pathname_start as usize;
                    let current_pathname = &buffer[pathname_start_pos..];

                    // Count trailing spaces
                    let trailing_spaces = current_pathname
                        .chars()
                        .rev()
                        .take_while(|&c| c == ' ')
                        .count();

                    if trailing_spaces > 0 {
                        // Remove all trailing spaces
                        let new_len = buffer.len() - trailing_spaces;
                        buffer.truncate(new_len);

                        // Add back all but last space as literals
                        for _ in 0..(trailing_spaces - 1) {
                            buffer.push(' ');
                        }

                        // Encode the last space as %20
                        buffer.push_str("%20");
                    }
                }

                // Handle query or hash
                if pointer < bytes.len() {
                    if bytes[pointer] == b'?' {
                        components.search_start = buffer.len() as u32;
                        buffer.push('?');
                        pointer += 1;
                        state = State::Query;
                        continue;
                    } else if bytes[pointer] == b'#' {
                        components.hash_start = buffer.len() as u32;
                        buffer.push('#');
                        pointer += 1;
                        state = State::Fragment;
                        continue;
                    }
                }

                // Per ada-url: insert "/." before pathname if:
                // 1. pathname starts with "//"
                // 2. no authority (no "//" after protocol in buffer)
                // 3. not a special scheme
                let pathname_start_pos = components.pathname_start as usize;
                let protocol_end_pos = components.protocol_end as usize;
                let has_authority = protocol_end_pos + 2 <= components.host_start as usize
                    && buffer[protocol_end_pos..protocol_end_pos + 2] == *"//";
                if pathname_start_pos + 2 <= buffer.len()
                    && buffer[pathname_start_pos..].starts_with("//")
                    && !has_authority
                    && !scheme_type.is_special()
                {
                    // Insert "/." at pathname_start
                    buffer.insert_str(pathname_start_pos, "/.");
                    // Adjust offsets
                    components.pathname_start += 2;
                    if components.search_start > 0 {
                        components.search_start += 2;
                    }
                    if components.hash_start > 0 {
                        components.hash_start += 2;
                    }
                }

                break;
            }

            State::Fragment => {
                // Batch processing: append rest of URL with percent encoding
                if pointer < bytes.len() && input.is_char_boundary(pointer) {
                    percent_encode_fragment_into(&mut buffer, &input[pointer..]);
                }
                break;
            }
        }

        pointer += 1;
    }

    // Ensure special schemes have at least "/" as pathname
    let pathname_start = components.pathname_start as usize;
    if scheme_type.is_special() && buffer.len() == pathname_start {
        buffer.push('/');
    }

    // Add fragment (ada-url pattern: done at the end for optimization)
    if let Some(frag) = fragment {
        components.hash_start = buffer.len() as u32;
        buffer.push('#');
        percent_encode_fragment_into(&mut buffer, frag);
    }

    Ok(UrlAggregator::from_buffer(buffer, components))
}

/// Parse host and port, write to buffer and update components
fn parse_host_and_port(
    host_and_port: &str,
    buffer: &mut String,
    components: &mut UrlComponents,
    scheme_type: SchemeType,
) -> Result<()> {
    // Separate hostname and port
    let (hostname, port_str) = if host_and_port.starts_with('[') {
        // IPv6: [::1]:8080 or [::1]
        if let Some(bracket_end) = memchr::memchr(b']', host_and_port.as_bytes()) {
            let ipv6_part = &host_and_port[0..=bracket_end];
            let port_part = &host_and_port[bracket_end + 1..];
            if let Some(stripped) = port_part.strip_prefix(':') {
                (ipv6_part, Some(stripped))
            } else {
                (ipv6_part, None)
            }
        } else {
            (host_and_port, None)
        }
    } else {
        // Regular host or IPv4
        if let Some(colon_pos) = memchr::memrchr(b':', host_and_port.as_bytes()) {
            (
                &host_and_port[0..colon_pos],
                Some(&host_and_port[colon_pos + 1..]),
            )
        } else {
            (host_and_port, None)
        }
    };

    // For non-special schemes, use opaque host (don't decode)
    if !scheme_type.is_special() {
        // IPv6 addresses are allowed in non-special schemes (Tests #673-675)
        if hostname.starts_with('[') {
            if !hostname.ends_with(']') {
                return Err(ParseError::InvalidHost);
            }
            // Validate and parse IPv6 address
            let segments = parse_ipv6(hostname)?;

            // Serialize with compression (:: notation) - NO brackets, they come from hostname()
            let serialized = serialize_ipv6(&segments);
            buffer.push_str(&serialized);
            components.host_end = buffer.len() as u32;
        } else {
            // Reject full-width percent sign
            if hostname.contains('％') {
                return Err(ParseError::InvalidHost);
            }

            // Special case: "." is a valid hostname (WPT test #317)
            // Empty hostname check should be done after this

            // For opaque hosts (Tests #384, #385, #484):
            // Per WHATWG spec: opaque host processing
            // 1. Reject specific forbidden characters: null (0x00), space, #, /, <, >, ?, @, [, \, ], ^, |
            // 2. Percent-encode: C0 controls (except null), DEL, non-ASCII
            // 3. Keep as-is: Other printable ASCII (!, ", $, %, &, ', (, ), *, +, etc.)
            for ch in hostname.chars() {
                // Reject specific forbidden host code points
                if ch == '\0' ||  // Null byte (test #384)
                   ch == ' ' ||   // Space
                   ch == '#' || ch == '/' || ch == '<' || ch == '>' ||
                   ch == '?' || ch == '@' || ch == '[' || ch == '\\' || ch == ']' ||
                   ch == '^' || ch == '|'
                {
                    return Err(ParseError::InvalidHost);
                }
            }

            // Percent-encode C0 controls (except null, handled above), DEL, and non-ASCII
            // Keep printable ASCII as-is (test #484)
            let mut encoded = String::with_capacity(hostname.len());
            for ch in hostname.chars() {
                let code = ch as u32;
                if (code > 0 && code <= 0x1F) || code == 0x7F || !ch.is_ascii() {
                    // Percent-encode C0 controls (excluding null 0x00), DEL (0x7F), and non-ASCII
                    for byte in ch.to_string().bytes() {
                        use core::fmt::Write;
                        let _ = write!(&mut encoded, "%{byte:02X}");
                    }
                } else {
                    // Keep as-is
                    encoded.push(ch);
                }
            }

            buffer.push_str(&encoded);
            components.host_end = buffer.len() as u32;
        }

        // Write port if present
        if let Some(port) = port_str
            && !port.is_empty()
        {
            if let Some(port_num) = parse_port(port) {
                components.port = Some(port_num);
                if !is_default_port(scheme_type, port_num) {
                    buffer.push(':');
                    buffer.push_str(&port_num.to_string());
                }
            } else {
                return Err(ParseError::InvalidPort);
            }
        }

        components.pathname_start = buffer.len() as u32;
        return Ok(());
    }

    // For special schemes: validate, decode, apply IDNA
    // Validate percent encoding before decoding
    if hostname.contains('％') {
        return Err(ParseError::InvalidHost);
    }
    if hostname.contains('%') {
        let bytes = hostname.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                if i + 2 >= bytes.len()
                    || !bytes[i + 1].is_ascii_hexdigit()
                    || !bytes[i + 2].is_ascii_hexdigit()
                {
                    return Err(ParseError::InvalidHost);
                }
                i += 3;
            } else {
                i += 1;
            }
        }
    }

    // Percent-decode hostname for special schemes
    // BUT: Don't decode IPv6 addresses (brackets), as % in IPv6 indicates zone ID (forbidden)
    let decoded_hostname = if hostname.contains('%') && !hostname.starts_with('[') {
        Cow::Owned(percent_decode(hostname)?)
    } else {
        Cow::Borrowed(hostname)
    };

    // Check if decoded hostname contains ONLY soft hyphens (Tests #796-797)
    // If so, it's invalid (empty after removal)
    if decoded_hostname.contains('\u{00AD}') {
        let without_soft_hyphens: String = decoded_hostname
            .chars()
            .filter(|&c| c != '\u{00AD}')
            .collect();
        if without_soft_hyphens.is_empty() {
            return Err(ParseError::InvalidHost);
        }
    }

    // Note: Soft hyphens will be removed later, AFTER IPv4 heuristic check
    // This ensures "a\u{00AD}b" is not treated as IPv4-like "ab"
    let hostname = decoded_hostname.as_ref();

    // Reject invalid punycode (Test #798)
    // "xn--" followed by nothing or very short content is invalid
    let lower = hostname.to_lowercase();
    if lower == "xn--" || (lower.starts_with("xn--") && lower.len() <= 4) {
        return Err(ParseError::InvalidHost);
    }

    // Normalize full-width ASCII characters (U+FF01-FF5E) to ASCII
    // WHATWG URL spec requires this normalization before IDNA processing
    // Other full-width characters (U+FF00, U+FF5F-FFEF) are invalid and should be rejected
    let normalized_hostname: Cow<str> = {
        let mut needs_normalization = false;
        for ch in hostname.chars() {
            let code = ch as u32;
            // Check if we need normalization (contains full-width chars)
            if (0xFF00..=0xFFEF).contains(&code) {
                needs_normalization = true;
                break;
            }
        }

        if needs_normalization {
            let mut result = String::with_capacity(hostname.len());
            for ch in hostname.chars() {
                let code = ch as u32;
                if (0xFF01..=0xFF5E).contains(&code) {
                    // Full-width ASCII → ASCII (offset: 0xFEE0)
                    // E.g., U+FF27 (Ｇ) → U+0047 (G)
                    let ascii_code = code - 0xFEE0;
                    let ascii_char = char::from_u32(ascii_code).ok_or(ParseError::InvalidHost)?;
                    result.push(ascii_char);
                } else if (0xFF00..=0xFFEF).contains(&code) {
                    // Other full-width characters are invalid
                    return Err(ParseError::InvalidHost);
                } else {
                    result.push(ch);
                }
            }
            Cow::Owned(result)
        } else {
            Cow::Borrowed(hostname)
        }
    };

    // Keep normalized hostname (WITH soft hyphens for now)
    // Soft hyphens will be removed later, after IPv4 heuristic check
    let hostname_with_soft_hyphens = normalized_hostname.as_ref();

    // Write hostname
    if hostname_with_soft_hyphens.starts_with('[') && hostname_with_soft_hyphens.ends_with(']') {
        // IPv6 address - parse, validate, and serialize
        use crate::ipv6::{parse_ipv6, serialize_ipv6};
        let segments = parse_ipv6(hostname_with_soft_hyphens)?;
        let serialized = serialize_ipv6(&segments);
        buffer.push_str(&serialized);
    } else {
        // Check if it's an IPv4 address (ada-url's is_ipv4 heuristic)
        // Important: Check heuristic BEFORE removing soft hyphens (Test #795)
        // So "a\u{00AD}b" is not treated as IPv4-like "ab"
        // Use ada-url's is_ipv4 function (Test: is_ipv4_like only)
        use crate::checkers::is_ipv4;
        let is_ipv4_like = is_ipv4(hostname_with_soft_hyphens);

        if is_ipv4_like && !hostname_with_soft_hyphens.is_empty() {
            use crate::ipv4::{parse_ipv4, serialize_ipv4};
            // Remove soft hyphens before IPv4 parsing (Test #795)
            let hostname_for_ipv4 = if hostname_with_soft_hyphens.contains('\u{00AD}') {
                hostname_with_soft_hyphens
                    .chars()
                    .filter(|&c| c != '\u{00AD}')
                    .collect::<String>()
            } else {
                hostname_with_soft_hyphens.to_string()
            };
            // Try parsing as IPv4
            // If it looks like IPv4 but fails to parse, it's an error (WHATWG spec)
            let ipv4 = parse_ipv4(&hostname_for_ipv4)?;

            // Successfully parsed as IPv4 - serialize in dotted decimal
            let serialized = serialize_ipv4(ipv4);
            buffer.push_str(&serialized);
            components.host_end = buffer.len() as u32;
            // Write port if present
            if let Some(port) = port_str {
                if scheme_type == SchemeType::File {
                    return Err(ParseError::InvalidHost);
                }
                if !port.is_empty() {
                    if let Some(port_num) = parse_port(port) {
                        components.port = Some(port_num);
                        if !is_default_port(scheme_type, port_num) {
                            buffer.push(':');
                            buffer.push_str(&port_num.to_string());
                        }
                    } else {
                        return Err(ParseError::InvalidPort);
                    }
                }
            }
            components.pathname_start = buffer.len() as u32;
            return Ok(());
        }

        // Remove soft hyphens before domain processing (Test #795)
        let hostname = if hostname_with_soft_hyphens.contains('\u{00AD}') {
            Cow::Owned(
                hostname_with_soft_hyphens
                    .chars()
                    .filter(|&c| c != '\u{00AD}')
                    .collect(),
            )
        } else {
            Cow::Borrowed(hostname_with_soft_hyphens)
        };
        let hostname = hostname.as_ref();

        // Regular hostname - check for invalid characters
        // Colons are not allowed in hostnames (only in IPv6 with brackets)
        if hostname.contains(':') {
            return Err(ParseError::InvalidHost);
        }

        // For file: URLs, "localhost" is treated as empty host
        let is_localhost =
            scheme_type == SchemeType::File && hostname.eq_ignore_ascii_case("localhost");

        if !is_localhost {
            // For special schemes, always apply IDNA validation (catches invalid punycode too)
            // For non-special schemes, percent-encode forbidden characters (opaque host)
            if scheme_type.is_special() {
                // Validate for forbidden host code points (special schemes only)
                // WHATWG spec: forbidden host code points are:
                // 0x00-0x1F (C0 controls), 0x20 (space), "#", "%", "/", ":", "<", ">", "?", "@", "[", "\", "]", "^", "|", 0x7F
                // Also reject any Unicode whitespace (including ideographic space U+3000)
                for ch in hostname.chars() {
                    let code = ch as u32;
                    if code <= 0x20
                        || code == 0x7F
                        || ch == '#'
                        || ch == '%'
                        || ch == '/'
                        || ch == ':'
                        || ch == '<'
                        || ch == '>'
                        || ch == '?'
                        || ch == '@'
                        || ch == '['
                        || ch == '\\'
                        || ch == ']'
                        || ch == '^'
                        || ch == '|'
                        || ch.is_whitespace()
                    {
                        // Catch all Unicode whitespace
                        return Err(ParseError::InvalidHost);
                    }
                }
                // Optimization: Skip IDNA for ASCII-only hostnames without punycode (common case)
                // Check for punycode markers: "xn--" (case-insensitive)
                let has_punycode = {
                    let bytes = hostname.as_bytes();
                    let mut has = false;
                    for i in 0..bytes.len().saturating_sub(3) {
                        if (bytes[i] == b'x' || bytes[i] == b'X')
                            && (bytes[i + 1] == b'n' || bytes[i + 1] == b'N')
                            && bytes[i + 2] == b'-'
                            && bytes[i + 3] == b'-'
                        {
                            has = true;
                            break;
                        }
                    }
                    has
                };

                let ascii = if hostname.is_ascii() && !has_punycode {
                    // Fast path: ASCII-only and no punycode, just lowercase it
                    hostname.to_ascii_lowercase()
                } else {
                    // Slow path: Non-ASCII or contains punycode, need IDNA processing/validation
                    domain_to_ascii(hostname)?
                };

                // After IDNA, check if result is IPv4 (matches ada-url behavior)
                // This handles cases like full-width digits: ０Ｘｃ０ → 0xc0
                // Check if IDNA-processed result looks like IPv4 (same function as pre-IDNA)
                let is_ipv4_after_idna = is_ipv4(&ascii);

                if is_ipv4_after_idna {
                    // Parse as IPv4 and serialize
                    use crate::ipv4::{parse_ipv4, serialize_ipv4};
                    let ipv4 = parse_ipv4(&ascii)?;
                    let serialized = serialize_ipv4(ipv4);
                    buffer.push_str(&serialized);
                } else {
                    buffer.push_str(&ascii);
                }
            } else {
                // Non-special schemes (opaque host): percent-encode forbidden characters
                // WHATWG opaque host: percent-encode C0 controls, space, ", #, /, :, <, >, ?, @, [, \, ], ^, |, DEL, and non-ASCII
                for ch in hostname.chars() {
                    let code = ch as u32;
                    let needs_encoding = code <= 0x20 || code == 0x7F ||  // C0 controls, space, DEL
                        ch == '"' || ch == '#' || ch == '/' || ch == ':' || ch == '<' || ch == '>' ||
                        ch == '?' || ch == '@' || ch == '[' || ch == '\\' || ch == ']' || ch == '^' || ch == '|' ||
                        !ch.is_ascii(); // Non-ASCII

                    if needs_encoding {
                        // Percent-encode
                        use core::fmt::Write;
                        for byte in ch.to_string().bytes() {
                            let _ = write!(buffer, "%{byte:02X}");
                        }
                    } else {
                        buffer.push(ch); // Keep as-is (preserve case for opaque hosts)
                    }
                }
            }
        }
        // else: localhost in file: URL - don't write anything (empty host)
    }
    components.host_end = buffer.len() as u32;

    // Write port if present and not default
    if let Some(port) = port_str {
        // file: URLs cannot have ports
        if scheme_type == SchemeType::File {
            return Err(ParseError::InvalidHost);
        }

        if !port.is_empty() {
            // Validate and parse port
            if let Some(port_num) = parse_port(port) {
                components.port = Some(port_num);
                // Only write non-default ports to buffer
                if !is_default_port(scheme_type, port_num) {
                    buffer.push(':');
                    buffer.push_str(&port_num.to_string());
                }
            } else {
                return Err(ParseError::InvalidPort);
            }
        }
    }

    components.pathname_start = buffer.len() as u32;
    Ok(())
}

/// Fast path for simple HTTP/HTTPS URLs
/// Handles common cases: <http://example.com/path>
/// Single-pass validation and construction
fn try_http_fast_path(input: &str) -> Option<UrlAggregator> {
    let bytes = input.as_bytes();
    let len = bytes.len();

    // Check scheme (http:// or https://)
    let (_scheme, scheme_type, mut pos) = if len >= 8 && &bytes[..8] == b"https://" {
        ("https", SchemeType::Https, 8)
    } else if len >= 7 && &bytes[..7] == b"http://" {
        ("http", SchemeType::Http, 7)
    } else {
        return None;
    };

    if pos >= len {
        return None; // No host
    }

    // Pre-allocate buffer
    let mut buffer = String::with_capacity(input.len());
    let mut components = UrlComponents::new();

    // Write scheme with separator in one go
    if scheme_type == SchemeType::Https {
        buffer.push_str("https://");
        components.protocol_end = 6; // "https:".len()
    } else {
        buffer.push_str("http://");
        components.protocol_end = 5; // "http:".len()
    }

    // Check for credentials (user:pass@) in a single scan
    let mut has_credentials = false;
    let mut cred_end = pos;

    // Single pass: find @ if present, otherwise find end of authority
    while cred_end < len {
        let b = bytes[cred_end];
        if b == b'@' {
            has_credentials = true;
            break;
        }
        // Authority section ends at path/query/fragment
        if b == b'/' || b == b'?' || b == b'#' {
            break;
        }
        cred_end += 1;
    }

    // Parse credentials if present
    if has_credentials {
        let cred_start = pos;
        let cred_str = &input[cred_start..cred_end];

        // Find : separator between username and password
        if let Some(colon_pos) = cred_str.find(':') {
            let username = &cred_str[..colon_pos];
            let password = &cred_str[colon_pos + 1..];

            // Per WHATWG spec: empty username and empty password = no credentials written
            if username.is_empty() && password.is_empty() {
                // Skip credentials entirely
                pos = cred_end + 1; // Skip past @
                components.username_end = buffer.len() as u32;
                components.password_end = buffer.len() as u32;
            } else if username.is_empty() {
                // Empty username with non-empty password: not valid in fast path
                // This edge case needs special handling, fall back to slow path
                return None;
            } else {
                // Encode and write username
                use crate::unicode::percent_encode::percent_encode_userinfo_into;
                percent_encode_userinfo_into(&mut buffer, username);
                components.username_end = buffer.len() as u32;

                // Write password if non-empty
                if password.is_empty() {
                    // Empty password - don't write colon
                    components.password_end = components.username_end;
                } else {
                    buffer.push(':');
                    percent_encode_userinfo_into(&mut buffer, password);
                    components.password_end = buffer.len() as u32;
                }

                buffer.push('@');
                pos = cred_end + 1; // Skip past @
            }
        } else {
            // Username only, no colon
            use crate::unicode::percent_encode::percent_encode_userinfo_into;

            // Empty username with @ = no credentials
            if cred_str.is_empty() {
                pos = cred_end + 1; // Skip past @
                components.username_end = buffer.len() as u32;
                components.password_end = buffer.len() as u32;
            } else {
                percent_encode_userinfo_into(&mut buffer, cred_str);
                components.username_end = buffer.len() as u32;
                components.password_end = components.username_end;
                buffer.push('@');
                pos = cred_end + 1; // Skip past @
            }
        }
    }
    // If no credentials, username_end and password_end remain at 0

    // Parse host: single pass, validate + lowercase
    components.host_start = buffer.len() as u32;
    let host_start_pos = pos;

    // Find end of host (before : or / or end)
    // Optimized: batch process valid characters, handle special cases
    while pos < len {
        let start = pos;

        // Batch scan for lowercase/digit/dot/dash (most common case for hostnames)
        // These can be copied directly without conversion
        while pos < len {
            let b = bytes[pos];
            if b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-' {
                pos += 1;
            } else {
                break;
            }
        }

        // Batch copy the validated bytes
        if pos > start {
            let safe_slice = unsafe { core::str::from_utf8_unchecked(&bytes[start..pos]) };
            buffer.push_str(safe_slice);
        }

        // Handle special character if we haven't reached end
        if pos >= len {
            break;
        }

        let b = bytes[pos];

        // Delimiter check (most common after batch copy)
        if b == b':' || b == b'/' {
            break;
        }

        // Medium path: uppercase letters - convert to lowercase
        if b.is_ascii_uppercase() {
            buffer.push((b + 32) as char);
            pos += 1;
            continue;
        }

        // Rare path: use classification for other characters
        match crate::character_sets::classify_hostname_byte(b) {
            1 => buffer.push(b as char),
            3 => break,       // Delimiter
            _ => return None, // Invalid character (class 0, 2, 4)
        }
        pos += 1;
    }

    let host = &input[host_start_pos..pos];
    if host.is_empty() {
        return None;
    }

    // Check for IPv4 (needs special parsing)
    if is_ipv4(host) {
        return None;
    }

    // Check for Punycode (needs IDNA processing) - case insensitive
    // Use zero-allocation check
    if crate::unicode::idna::has_punycode(host) {
        return None;
    }

    components.host_end = buffer.len() as u32;

    // Parse port if present
    if pos < len && bytes[pos] == b':' {
        pos += 1; // Skip ':'
        let port_start = pos;

        // Collect digits
        while pos < len && bytes[pos].is_ascii_digit() {
            pos += 1;
        }

        if pos == port_start {
            return None; // Empty port
        }

        // Check what comes after port
        if pos < len && bytes[pos] != b'/' {
            return None; // Invalid char after port
        }

        let port_str = &input[port_start..pos];
        let port = crate::checkers::parse_port(port_str)?;

        // Write non-default port
        let default_port = scheme_type.default_port();
        if default_port != Some(port) {
            buffer.push(':');
            // Use original port string instead of converting back
            buffer.push_str(port_str);
        }
        components.port = Some(port);
    }

    components.pathname_start = buffer.len() as u32;

    // Parse path if present, or add default /
    if pos < len {
        if bytes[pos] != b'/' {
            return None; // Must start with /
        }

        // Check for dots that need normalization (.. and .)
        // Quick scan for /. patterns that need special handling
        let path_slice = &input[pos..];
        if path_slice.contains("/..") || path_slice.contains("/.") {
            return None; // Needs path normalization
        }

        // Check for percent-encoded dots (%2e or %2E) that need normalization
        // These must be decoded and normalized by the slow path
        if path_slice.contains("%2e") || path_slice.contains("%2E") {
            return None; // Needs decoding and normalization
        }

        // Copy path with validation (batch copy for performance)
        let start = pos;
        while pos < len {
            let b = bytes[pos];
            let class = crate::character_sets::classify_path_byte(b);

            match class {
                1 => pos += 1,    // Valid path character, keep scanning
                2 => break,       // Query/fragment delimiter
                _ => return None, // Invalid character
            }
        }

        // Batch copy all valid path characters at once
        if pos > start {
            // Safety: bytes[start..pos] are all valid ASCII path characters (class 1)
            let safe_slice = unsafe { core::str::from_utf8_unchecked(&bytes[start..pos]) };
            buffer.push_str(safe_slice);
        }

        // Handle query if present (fragment is handled after fast path returns)
        if pos < len {
            let b = bytes[pos];
            if b == b'?' {
                // Query string
                components.search_start = buffer.len() as u32;
                buffer.push('?');
                pos += 1;

                // Copy query until end (fragments are pruned before fast path)
                // Bulk copy optimization: scan ahead for safe query characters
                while pos < len && bytes[pos] != b'#' {
                    let start = pos;

                    // Scan ahead for safe query characters
                    while pos < len && bytes[pos] != b'#' {
                        let b = bytes[pos];

                        // Safe query characters that can be bulk copied
                        if b.is_ascii_lowercase()
                            || b.is_ascii_uppercase()
                            || b.is_ascii_digit()
                            || b == b'='
                            || b == b'&'
                            || b == b'%'
                            || b == b'-'
                            || b == b'_'
                            || b == b'.'
                            || b == b'~'
                        {
                            pos += 1;
                            continue;
                        }
                        break;
                    }

                    // Bulk copy the safe characters
                    if pos > start {
                        // Safety: we know bytes[start..pos] are all valid ASCII
                        let safe_slice =
                            unsafe { core::str::from_utf8_unchecked(&bytes[start..pos]) };
                        buffer.push_str(safe_slice);
                    }

                    // Handle the current byte if not at end
                    if pos < len && bytes[pos] != b'#' {
                        let b = bytes[pos];

                        // Reject control chars, non-ASCII, and chars that need encoding
                        if !(0x20..0x7F).contains(&b) {
                            return None; // Needs encoding
                        }
                        // Reject specific chars that need percent encoding in query
                        // Per WHATWG: ' " < > and others need encoding
                        if matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\'' | b'"' | b'<' | b'>') {
                            return None;
                        }

                        // Other valid ASCII characters
                        buffer.push(b as char);
                        pos += 1;
                    }
                }
            }
        }
    } else {
        // No explicit path - add default /
        buffer.push('/');
    }

    // search_start/hash_start are set above if query/fragment present
    // Leave at 0 if not present (matches slow path behavior)

    Some(UrlAggregator {
        buffer,
        components,
        scheme_type,
    })
}
