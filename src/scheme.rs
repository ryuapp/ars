use crate::types::SchemeType;

/// Get the scheme type from a scheme string.
/// Uses perfect hash based on length + first byte to minimize comparisons.
pub fn get_scheme_type(scheme: &str) -> SchemeType {
    let bytes = scheme.as_bytes();

    // Perfect hash: filter by length first, then first byte, then full comparison
    match (bytes.len(), bytes.first()) {
        (2, Some(b'w')) if bytes == b"ws" => SchemeType::Ws,
        (3, Some(b'w')) if bytes == b"wss" => SchemeType::Wss,
        (3, Some(b'f')) if bytes == b"ftp" => SchemeType::Ftp,
        (4, Some(b'h')) if bytes == b"http" => SchemeType::Http,
        (4, Some(b'f')) if bytes == b"file" => SchemeType::File,
        (5, Some(b'h')) if bytes == b"https" => SchemeType::Https,
        _ => SchemeType::NotSpecial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheme_type() {
        assert_eq!(get_scheme_type("http"), SchemeType::Http);
        assert_eq!(get_scheme_type("https"), SchemeType::Https);
        assert_eq!(get_scheme_type("ftp"), SchemeType::Ftp);
        assert_eq!(get_scheme_type("custom"), SchemeType::NotSpecial);
    }
}
