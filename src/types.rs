/// URL scheme types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SchemeType {
    #[default]
    Http,
    Https,
    Ws,
    Wss,
    Ftp,
    File,
    NotSpecial,
}

impl SchemeType {
    /// Check if this is a special scheme
    pub fn is_special(self) -> bool {
        self != Self::NotSpecial
    }

    /// Get the default port for this scheme
    pub fn default_port(self) -> Option<u16> {
        match self {
            Self::Http | Self::Ws => Some(80),
            Self::Https | Self::Wss => Some(443),
            Self::Ftp => Some(21),
            Self::File | Self::NotSpecial => None,
        }
    }
}
