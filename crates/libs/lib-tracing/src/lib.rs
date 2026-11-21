mod capture;
mod error;
mod init;
mod levels;

// Re-export main types for easier access
pub use error::{Error, Result};

// Re-export log level types
pub use levels::{Levels, level_filter};

// The opt-in in-memory capture of recent events, for an in-app log view
pub use capture::{LOG_CAPACITY, LogBuffer, LogEntry};

// Reexport init module
pub use init::init;
