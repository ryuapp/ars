/// Errors that can occur during URL parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Invalid scheme format
    InvalidScheme,
    /// Invalid host format
    InvalidHost,
    /// Invalid port number
    InvalidPort,
    /// Invalid IPv4 address
    InvalidIpv4,
    /// Invalid IPv6 address
    InvalidIpv6,
    /// Invalid character in domain
    InvalidDomainCharacter,
    /// Invalid percent encoding
    InvalidPercentEncoding,
    /// IDNA processing error
    IdnaError,
    /// Invalid URL structure
    InvalidUrl,
    /// Relative URL without base
    RelativeUrlWithoutBase,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let msg = match self {
            Self::InvalidScheme => "Invalid scheme",
            Self::InvalidHost => "Invalid host",
            Self::InvalidPort => "Invalid port",
            Self::InvalidIpv4 => "Invalid IPv4 address",
            Self::InvalidIpv6 => "Invalid IPv6 address",
            Self::InvalidDomainCharacter => "Invalid domain character",
            Self::InvalidPercentEncoding => "Invalid percent encoding",
            Self::IdnaError => "IDNA processing error",
            Self::InvalidUrl => "Invalid URL",
            Self::RelativeUrlWithoutBase => "Relative URL without base",
        };
        f.write_str(msg)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}

/// Result type for URL parsing operations
pub type Result<T> = core::result::Result<T, ParseError>;
