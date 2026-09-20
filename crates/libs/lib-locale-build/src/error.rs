//! Error type for the Message generator.

/// Failures while reading Catalogue files or generating accessors.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A Catalogue file or directory could not be read, or the output could not be written.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// One or more Catalogue problems (syntax, missing ids, mismatched variables, collisions).
    /// Each problem is on its own line so a build reports them all at once.
    #[error("Catalogue check failed:\n{0}")]
    Generic(String),
}

/// Result alias for the generator.
pub type Result<T> = std::result::Result<T, Error>;
