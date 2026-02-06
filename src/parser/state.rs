/// URL parser state machine states
/// Based on WHATWG URL Standard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Scheme start state
    SchemeStart,
    /// No scheme state
    NoScheme,
    /// Special relative or authority state
    SpecialRelativeOrAuthority,
    /// Path or authority state
    PathOrAuthority,
    /// Relative state
    Relative,
    /// Relative slash state
    RelativeSlash,
    /// Special authority slashes state
    SpecialAuthoritySlashes,
    /// Authority state
    Authority,
    /// File state
    File,
    /// File slash state
    FileSlash,
    /// File host state
    FileHost,
    /// Path state
    Path,
    /// Opaque path state (for non-special schemes)
    OpaquePath,
    /// Query state
    Query,
    /// Fragment state
    Fragment,
}
