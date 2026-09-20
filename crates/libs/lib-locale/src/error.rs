//! Error type for the localisation crate.

/// Failures while building the Catalogue. Lookups never fail: a miss falls back through the
/// chain and finally returns the id, so this only surfaces from loading.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A one-off loading failure.
    #[error("Locale error: {0}")]
    Generic(String),
}

/// Result alias for the localisation crate.
pub type Result<T> = std::result::Result<T, Error>;
