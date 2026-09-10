//TODO: Add log file export

mod error;
mod init;
mod levels;

// Re-export main types for easier access
pub use error::Error;

// Re-export log level types
pub use levels::Levels;

// Reexport init module
pub use init::init;

/// Result type alias for telemetry operations.
/// This type simplifies function signatures by standardizing the return type for
/// operations that result in a `TelemetryError`.
pub type TelemetryResult<T> = std::result::Result<T, Error>;
