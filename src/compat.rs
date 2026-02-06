/// Compatibility layer for `std`/`no_std`
#[cfg(feature = "std")]
pub use std::{
    borrow::Cow,
    format,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(not(feature = "std"))]
pub use alloc::{
    borrow::Cow,
    format,
    string::{String, ToString},
    vec::Vec,
};
