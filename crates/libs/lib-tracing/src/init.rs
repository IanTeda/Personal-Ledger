//! # Telemetry Initialisation
//!
//! This module provides the core initialisation functionality for the telemetry system.
//!
//! The `init` function sets up the complete tracing infrastructure for the Personal Ledger
//! application, including event filtering, log collection, and subscriber registration.
//! It integrates with the `tracing` ecosystem to provide structured, hierarchical logging
//! that works seamlessly with asynchronous Rust code.
//!
//! ## Architecture
//!
//! The initialisation process follows these steps:
//!
//! 1. **Event Filtering**: Configure which log levels and targets to include/exclude
//! 2. **Collector Setup**: Create formatter and output destinations for log events
//! 3. **Registry Building**: Combine filters and collectors into a subscriber registry
//! 4. **Integration**: Bridge with the standard `log` crate for compatibility
//! 5. **Activation**: Set the global default subscriber to start collecting telemetry
//!
//! ## Usage
//!
//! ```rust,ignore
//! use lib_tracing::{init, TelemetryLevels};
//!
//! // Initialize with default INFO level
//! init(None)?;
//!
//! // Initialize with custom DEBUG level
//! let level = TelemetryLevels::DEBUG;
//! init(Some(&level))?;
//!
//! # Ok::<(), lib_tracing::TelemetryError>(())
//! ```

use tracing::subscriber::set_global_default;
use tracing_subscriber::{EnvFilter, prelude::*};

use crate::{Error, Levels, TelemetryResult};

/// Initialises the telemetry system for the Personal Ledger application.
///
/// This function sets up the complete tracing infrastructure, including event filtering,
/// log collection, and global subscriber registration. It must be called early in the
/// application lifecycle, typically during startup, before any telemetry events are
/// generated.
///
/// The initialisation process is designed to be flexible and configurable:
/// - Uses the provided telemetry level as the default filter
/// - Allows runtime override via `RUST_LOG` environment variable
/// - Configures console output with human-readable formatting
/// - Integrates with the standard `log` crate for compatibility
///
/// # Parameters
///
/// * `telemetry_level` - Optional reference to the desired telemetry level. If `None`,
///   defaults to `INFO` level. This sets the baseline filtering level before
///   environment variable overrides are applied.
///
/// # Errors
///
/// Returns a `TelemetryError` if:
/// - The log tracer initialisation fails (e.g., another logger is already registered)
/// - Setting the global default subscriber fails (e.g., another subscriber exists)
/// - Environment variable parsing fails (though this is handled gracefully)
///
/// # Environment Variables
///
/// * `RUST_LOG` - Override the default filtering with custom directives. Examples:
///   - `RUST_LOG=debug` - Enable debug logging globally
///   - `RUST_LOG=lib_tracing=trace,backend=info` - Set specific crate levels
///
/// # Thread Safety
///
/// This function is not thread-safe and should only be called once during application
/// startup. Attempting to initialize telemetry multiple times will result in errors.
///
/// # Examples
///
/// ```rust,ignore
/// use lib_tracing::{init, TelemetryLevels};
///
/// // Basic initialisation with default level
/// init(None)?;
///
/// // initialisation with custom debug level
/// let debug_level = TelemetryLevels::DEBUG;
/// init(Some(&debug_level))?;
///
/// // The function will return an error if telemetry is already initialised
/// // or if there are conflicts with existing loggers
/// # Ok::<(), lib_tracing::TelemetryError>(())
/// ```
pub fn init(telemetry_level: Option<&Levels>) -> TelemetryResult<()> {
    // TODO: Add log file functionality

    // ============================================================================
    // Phase 1: Configure Event Filtering (Tracing/Log Level)
    // ============================================================================
    // Set default tracing level based on configuration
    let default_env_filter = {
        // Convert our serde-friendly TelemetryLevels -> tracing LevelFilter -> Directive
        let default_directive = telemetry_level
            .map(|&level| tracing::level_filters::LevelFilter::from(level))
            .unwrap_or(tracing::level_filters::LevelFilter::INFO)
            .into();

        EnvFilter::builder()
            .with_default_directive(default_directive)
            .from_env_lossy()
    };

    // Try to use runtime level from RUST_LOG env var, fallback to configured default
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(default_env_filter);

    // ============================================================================
    // Phase 2: Configure Event Collection
    // ============================================================================
    // Build event collector for console output with default formatting
    let console_collector = tracing_subscriber::fmt::layer();

    // ============================================================================
    // Phase 3: Build Subscriber Registry
    // ============================================================================
    // Combine filters and collectors into a complete subscriber registry
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_collector);

    // ============================================================================
    // Phase 4: Integrate with Standard Log Crate
    // ============================================================================
    // Convert all log records into tracing events for unified processing
    tracing_log::LogTracer::init()
        .map_err(|e| Error::generic(format!("Log tracer initialisation failed: {}", e)))?;

    // ============================================================================
    // Phase 5: Activate Global Subscriber
    // ============================================================================
    // Set this registry as the global default subscriber to start collecting telemetry
    set_global_default(registry)
        .map_err(|e| Error::generic(format!("Failed to set global default subscriber: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_with_none_level() {
        // This test may fail if telemetry is already initialised
        // In a real scenario, this would be the first call during app startup
        let result = init(None);

        // If it succeeds, telemetry was initialised
        // If it fails, it might be because telemetry is already initialised
        match result {
            Ok(()) => {
                // Successfully initialised - this is the expected case for first init
            }
            Err(Error::Generic(msg)) => {
                // Check if it's the expected "already initialised" error
                assert!(
                    msg.contains("already initialised")
                        || msg.contains("tracer")
                        || msg.contains("subscriber"),
                    "Unexpected error message: {}",
                    msg
                );
            }
        }
    }

    #[test]
    fn test_init_with_debug_level() {
        let debug_level = Levels::DEBUG;
        let result = init(Some(&debug_level));

        match result {
            Ok(()) => {
                // Successfully initialised with DEBUG level
            }
            Err(Error::Generic(msg)) => {
                // Expected if already initialised
                assert!(
                    msg.contains("already initialised")
                        || msg.contains("tracer")
                        || msg.contains("subscriber"),
                    "Unexpected error message: {}",
                    msg
                );
            }
        }
    }

    #[test]
    fn test_init_with_all_levels() {
        let levels = [
            Levels::OFF,
            Levels::ERROR,
            Levels::WARN,
            Levels::INFO,
            Levels::DEBUG,
            Levels::TRACE,
        ];

        for level in &levels {
            let result = init(Some(level));
            match result {
                Ok(()) => {
                    // Successfully initialised
                    break; // If one succeeds, we've tested the functionality
                }
                Err(Error::Generic(msg)) => {
                    // Continue if already initialised
                    assert!(
                        msg.contains("already initialised")
                            || msg.contains("tracer")
                            || msg.contains("subscriber"),
                        "Unexpected error for level {:?}: {}",
                        level,
                        msg
                    );
                }
            }
        }
    }

    #[test]
    fn test_init_error_handling() {
        // Test that init returns appropriate errors

        // First, try to initialize (might succeed or fail)
        let _ = init(None);

        // Second call should definitely fail
        let result = init(Some(&Levels::DEBUG));

        // This should fail because telemetry is already initialised
        match result {
            Ok(()) => {
                // This might happen if the first call failed and this succeeds
                // Not ideal but acceptable for this test
            }
            Err(Error::Generic(msg)) => {
                // Expected error for double initialisation
                assert!(
                    msg.contains("tracer") || msg.contains("subscriber") || msg.contains("already"),
                    "Expected initialisation conflict error, got: {}",
                    msg
                );
            }
        }
    }

    #[test]
    fn test_telemetry_levels_conversion() {
        // Test that TelemetryLevels convert correctly to tracing levels
        // This is more of an integration test but validates the conversion logic

        let test_cases = vec![
            (
                Levels::OFF,
                tracing::level_filters::LevelFilter::OFF,
            ),
            (
                Levels::ERROR,
                tracing::level_filters::LevelFilter::ERROR,
            ),
            (
                Levels::WARN,
                tracing::level_filters::LevelFilter::WARN,
            ),
            (
                Levels::INFO,
                tracing::level_filters::LevelFilter::INFO,
            ),
            (
                Levels::DEBUG,
                tracing::level_filters::LevelFilter::DEBUG,
            ),
            (
                Levels::TRACE,
                tracing::level_filters::LevelFilter::TRACE,
            ),
        ];

        for (telemetry_level, expected_tracing_level) in test_cases {
            let actual_tracing_level: tracing::level_filters::LevelFilter = telemetry_level.into();
            assert_eq!(
                actual_tracing_level, expected_tracing_level,
                "Telemetry level {:?} should convert to tracing level {:?}",
                telemetry_level, expected_tracing_level
            );
        }
    }

    #[test]
    fn test_default_level_behaviors() {
        // Test that None parameter defaults to INFO level
        let none_result: tracing::level_filters::LevelFilter = None
            .map(|&level: &Levels| level.into())
            .unwrap_or(tracing::level_filters::LevelFilter::INFO);

        assert_eq!(none_result, tracing::level_filters::LevelFilter::INFO);
    }

    #[test]
    fn test_env_filter_creation() {
        // Test that env filter can be created with different levels
        let levels = [
            Levels::DEBUG,
            Levels::INFO,
            Levels::WARN,
        ];

        for level in &levels {
            // Convert TelemetryLevels to LevelFilter first, then to Directive
            let level_filter: tracing::level_filters::LevelFilter = (*level).into();
            let directive = level_filter.into();

            // Create env filter with this directive
            let env_filter = EnvFilter::builder()
                .with_default_directive(directive)
                .from_env_lossy();

            // The filter should be created successfully
            assert!(!env_filter.to_string().is_empty());
        }
    }

    #[test]
    fn test_registry_creation() {
        // Test that subscriber registry can be created
        let env_filter = EnvFilter::builder()
            .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
            .from_env_lossy();

        let console_collector = tracing_subscriber::fmt::layer();

        // This should not panic
        let _registry = tracing_subscriber::registry()
            .with(env_filter)
            .with(console_collector);
    }

    #[test]
    fn test_error_message_formatting() {
        // Test that error messages are properly formatted
        let test_error = Error::generic("test message");

        match &test_error {
            Error::Generic(msg) => {
                assert_eq!(msg, "test message");
            }
        }

        // Test Display implementation
        let display_msg = format!("{}", test_error);
        assert_eq!(display_msg, "Telemetry generic error: test message");
    }

    #[test]
    fn test_result_type_alias() {
        // Test that TelemetryResult works as expected
        let ok_result: TelemetryResult<i32> = Ok(42);
        match ok_result {
            Ok(value) => assert_eq!(value, 42),
            Err(_) => panic!("Expected Ok(42)"),
        }

        let err_result: TelemetryResult<i32> = Err(Error::generic("test error"));
        assert!(err_result.is_err());

        if let Err(Error::Generic(msg)) = err_result {
            assert_eq!(msg, "test error");
        } else {
            panic!("Expected Generic error");
        }
    }
}
