//! The crate's error type. Moving a selection through a list is total (every function clamps, and
//! an empty list never panics), so there is nothing to refuse yet and `Error` has no variants.

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;
