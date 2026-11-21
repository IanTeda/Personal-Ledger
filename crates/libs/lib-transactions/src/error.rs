//! The crate's error type. The filter popover's date parse refuses with `lib-locale`'s
//! `DateInputError` directly and nothing else here can fail, so `Error` has no variants.

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;
