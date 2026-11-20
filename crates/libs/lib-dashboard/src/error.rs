//! The crate's error type. No function here is fallible yet (the Dashboard's Presenter and view model read ports that do not fail through this crate), so there is nothing to refuse
//! and `Error` has no variants.

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {}

pub type Result<T> = std::result::Result<T, Error>;
