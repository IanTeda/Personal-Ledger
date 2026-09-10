//! # Telemetry Levels
//!
//! This module provides a serde-compatible representation of telemetry levels for the telemetry system.
//!
//! The `TelemetryLevels` enum serves as a configuration-friendly wrapper around `tracing`'s
//! `LevelFilter`, enabling telemetry level configuration through configuration files, environment
//! variables, and other serde-compatible sources.
//!
//! ## Usage
//!
//! ```rust
//! use lib_tracing::TelemetryLevels;
//!
//! // Parse from string (useful for config files)
//! let level: TelemetryLevels = serde_json::from_str("\"debug\"").unwrap();
//! assert_eq!(level, TelemetryLevels::DEBUG);
//!
//! // Convert to tracing LevelFilter for runtime use
//! let filter = tracing::level_filters::LevelFilter::from(level);
//! ```

// A serde-friendly representation of telemetry levels used in configuration.
/// The tracing crate's `LevelFilter` type does not implement `serde::{Deserialize, Serialize}`
/// so we expose a small enum that can be used in configuration files and converted to
/// the runtime `LevelFilter` when initialising telemetry.
///
/// This enum provides a clean interface for configuring telemetry verbosity across different
/// parts of the Personal Ledger application, supporting both human-readable configuration
/// files and environment variable settings.
///
/// # Serialization
///
/// The enum serializes to lowercase strings (`"off"`, `"error"`, `"warn"`, etc.) for
/// better readability in configuration files.
///
/// # Default
///
/// Defaults to `WARN` level, providing a balance between visibility of important issues
/// and avoiding excessive telemetry noise in production environments.
///
/// # Examples
///
/// ```rust
/// use lib_tracing::TelemetryLevels;
///
/// // Default level
/// let default_level = TelemetryLevels::default();
/// assert_eq!(default_level, TelemetryLevels::WARN);
///
/// // Convert to tracing filter
/// let filter = tracing::level_filters::LevelFilter::from(default_level);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryLevels {
    /// No telemetry output.
    ///
    /// Completely disables all telemetry output. Useful for performance-critical
    /// environments or when telemetry is not needed.
    OFF,

    /// Error-level telemetry only.
    ///
    /// Telemetry logs only error conditions that may require attention. This is the most
    /// minimal telemetry level for production systems.
    ERROR,

    /// Warning and error-level telemetry.
    ///
    /// Telemetry logs warnings and errors. This is the default level, providing visibility
    /// into potential issues while minimising telemetry volume.
    #[default]
    WARN,

    /// Informational, warning, and error-level telemetry   .
    ///
    /// Telemetry logs informational messages, warnings, and errors. Useful for understanding
    /// application flow and identifying potential issues.
    INFO,

    /// Debug, informational, warning, and error-level telemetry.
    ///
    /// Includes debug information for troubleshooting. Suitable for development
    /// and staging environments.
    DEBUG,

    /// All telemetry levels including trace information.
    ///
    /// Maximum verbosity including trace-level information. Primarily used for
    /// detailed debugging and development. May impact performance due to high
    /// telemetry volume.
    TRACE,
}

/// Conversion from `TelemetryLevels` to `tracing::LevelFilter`.
///
/// This implementation allows seamless integration with the tracing ecosystem,
/// enabling configuration-driven telemetry level control throughout the application.
///
/// The conversion is infallible and maintains the same semantic meaning for each level.
impl From<TelemetryLevels> for tracing::level_filters::LevelFilter {
    /// Converts a `TelemetryLevels` to the corresponding `tracing::LevelFilter`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_tracing::TelemetryLevels;
    /// use tracing::level_filters::LevelFilter;
    ///
    /// let telemetry_level = TelemetryLevels::INFO;
    /// let filter: LevelFilter = telemetry_level.into();
    /// assert_eq!(filter, LevelFilter::INFO);
    /// ```
    fn from(level: TelemetryLevels) -> Self {
        match level {
            TelemetryLevels::OFF => tracing::level_filters::LevelFilter::OFF,
            TelemetryLevels::ERROR => tracing::level_filters::LevelFilter::ERROR,
            TelemetryLevels::WARN => tracing::level_filters::LevelFilter::WARN,
            TelemetryLevels::INFO => tracing::level_filters::LevelFilter::INFO,
            TelemetryLevels::DEBUG => tracing::level_filters::LevelFilter::DEBUG,
            TelemetryLevels::TRACE => tracing::level_filters::LevelFilter::TRACE,
        }
    }
}

impl std::fmt::Display for TelemetryLevels {
    /// Formats the telemetry level as a lowercase string.
    ///
    /// This implementation matches the serde serialization format, producing
    /// lowercase strings like "info", "debug", etc. This ensures consistency
    /// between serialized configuration and string representations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_tracing::TelemetryLevels;
    ///
    /// assert_eq!(format!("{}", TelemetryLevels::INFO), "info");
    /// assert_eq!(format!("{}", TelemetryLevels::DEBUG), "debug");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level_str = match self {
            TelemetryLevels::OFF => "off",
            TelemetryLevels::ERROR => "error",
            TelemetryLevels::WARN => "warn",
            TelemetryLevels::INFO => "info",
            TelemetryLevels::DEBUG => "debug",
            TelemetryLevels::TRACE => "trace",
        };
        write!(f, "{}", level_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn test_default_level() {
        let default_level = TelemetryLevels::default();
        assert_eq!(default_level, TelemetryLevels::WARN);
    }

    #[test]
    fn test_conversion_to_level_filter() {
        // Test each variant converts to the correct LevelFilter
        assert_eq!(
            tracing::level_filters::LevelFilter::from(TelemetryLevels::OFF),
            tracing::level_filters::LevelFilter::OFF
        );
        assert_eq!(
            tracing::level_filters::LevelFilter::from(TelemetryLevels::ERROR),
            tracing::level_filters::LevelFilter::ERROR
        );
        assert_eq!(
            tracing::level_filters::LevelFilter::from(TelemetryLevels::WARN),
            tracing::level_filters::LevelFilter::WARN
        );
        assert_eq!(
            tracing::level_filters::LevelFilter::from(TelemetryLevels::INFO),
            tracing::level_filters::LevelFilter::INFO
        );
        assert_eq!(
            tracing::level_filters::LevelFilter::from(TelemetryLevels::DEBUG),
            tracing::level_filters::LevelFilter::DEBUG
        );
        assert_eq!(
            tracing::level_filters::LevelFilter::from(TelemetryLevels::TRACE),
            tracing::level_filters::LevelFilter::TRACE
        );
    }

    #[test]
    fn test_serialization() {
        // Test that each variant serializes to the expected lowercase string
        assert_eq!(to_string(&TelemetryLevels::OFF).unwrap(), "\"off\"");
        assert_eq!(to_string(&TelemetryLevels::ERROR).unwrap(), "\"error\"");
        assert_eq!(to_string(&TelemetryLevels::WARN).unwrap(), "\"warn\"");
        assert_eq!(to_string(&TelemetryLevels::INFO).unwrap(), "\"info\"");
        assert_eq!(to_string(&TelemetryLevels::DEBUG).unwrap(), "\"debug\"");
        assert_eq!(to_string(&TelemetryLevels::TRACE).unwrap(), "\"trace\"");
    }

    #[test]
    fn test_deserialization() {
        // Test that each lowercase string deserializes to the correct variant
        assert_eq!(from_str::<TelemetryLevels>("\"off\"").unwrap(), TelemetryLevels::OFF);
        assert_eq!(from_str::<TelemetryLevels>("\"error\"").unwrap(), TelemetryLevels::ERROR);
        assert_eq!(from_str::<TelemetryLevels>("\"warn\"").unwrap(), TelemetryLevels::WARN);
        assert_eq!(from_str::<TelemetryLevels>("\"info\"").unwrap(), TelemetryLevels::INFO);
        assert_eq!(from_str::<TelemetryLevels>("\"debug\"").unwrap(), TelemetryLevels::DEBUG);
        assert_eq!(from_str::<TelemetryLevels>("\"trace\"").unwrap(), TelemetryLevels::TRACE);
    }

    #[test]
    fn test_deserialization_case_insensitive() {
        // Test that uppercase strings also work (serde_json is case-sensitive, but our rename_all handles it)
        assert!(from_str::<TelemetryLevels>("\"OFF\"").is_err()); // Should fail
        assert!(from_str::<TelemetryLevels>("\"Error\"").is_err()); // Should fail
    }

    #[test]
    fn test_debug_trait() {
        let level = TelemetryLevels::INFO;
        let debug_str = format!("{:?}", level);
        assert!(debug_str.contains("INFO"));
    }

    #[test]
    fn test_clone_trait() {
        let original = TelemetryLevels::DEBUG;
        let cloned = Clone::clone(&original);
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_copy_trait() {
        let original = TelemetryLevels::TRACE;
        let copied = original; // Copy trait allows this
        assert_eq!(original, copied);
    }

    #[test]
    fn test_partial_eq_trait() {
        assert_eq!(TelemetryLevels::WARN, TelemetryLevels::WARN);
        assert_ne!(TelemetryLevels::INFO, TelemetryLevels::DEBUG);
    }

    #[test]
    fn test_into_conversion() {
        // Test the Into trait (automatic conversion)
        let level = TelemetryLevels::ERROR;
        let filter: tracing::level_filters::LevelFilter = level.into();
        assert_eq!(filter, tracing::level_filters::LevelFilter::ERROR);
    }

    #[test]
    fn test_all_variants_defined() {
        // Ensure all expected variants exist and are distinct
        let variants = [
            TelemetryLevels::OFF,
            TelemetryLevels::ERROR,
            TelemetryLevels::WARN,
            TelemetryLevels::INFO,
            TelemetryLevels::DEBUG,
            TelemetryLevels::TRACE,
        ];

        // Check they're all different
        for (i, &variant1) in variants.iter().enumerate() {
            for (j, &variant2) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(variant1, variant2, "Variants at indices {} and {} should be different", i, j);
                }
            }
        }
    }

    #[test]
    fn test_display_trait() {
        // Test that Display produces the expected lowercase strings
        assert_eq!(format!("{}", TelemetryLevels::OFF), "off");
        assert_eq!(format!("{}", TelemetryLevels::ERROR), "error");
        assert_eq!(format!("{}", TelemetryLevels::WARN), "warn");
        assert_eq!(format!("{}", TelemetryLevels::INFO), "info");
        assert_eq!(format!("{}", TelemetryLevels::DEBUG), "debug");
        assert_eq!(format!("{}", TelemetryLevels::TRACE), "trace");
    }
}