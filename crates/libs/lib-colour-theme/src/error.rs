//! The crate's error type. A Colour Theme file's parse failures stay beside `parse_theme` in
//! [`ParseError`] (`build.rs` includes `parse.rs` on its own) and are wrapped here, so a caller
//! with more than one fallible call in this crate has one type to name.

use crate::ParseError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A Colour Theme file failed to parse.
    #[error("colour theme file: {0}")]
    Parse(#[from] ParseError),
}

pub type Result<T> = std::result::Result<T, Error>;
