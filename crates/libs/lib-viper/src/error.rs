//! The crate's error type. No function here is fallible yet (the keymap, field, select and Dialog views are total over the values they are given), so there is nothing to refuse
//! and `Error` has no variants.

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;
