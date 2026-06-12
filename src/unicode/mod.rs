pub mod idna;
pub mod percent_encode;

pub(crate) const fn is_forbidden_domain_code_point(byte: u8) -> bool {
    matches!(
        byte,
        0x00..=0x20 | 0x7f..=0xff | b'#' | b'/' | b':' | b'<' | b'>' | b'?' | b'@' | b'[' | b'\\' | b']' | b'^' | b'|' | b'%'
    )
}
