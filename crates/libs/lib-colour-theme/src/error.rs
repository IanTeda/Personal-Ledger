//! The crate's error type. No function returns `Result<T>` yet: a Colour Theme file's parse
//! failures stay beside `parse_theme` in [`crate::ParseError`] (`build.rs` includes `parse.rs` on
//! its own), which returns it directly, so `Error` has no variant wrapping it.

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {}

pub type Result<T> = std::result::Result<T, Error>;
