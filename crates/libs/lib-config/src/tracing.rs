//! # Tracing Configuration
//!
//! This module provides configuration structures for the tracing system in the Personal Ledger application.
//!
//! The tracing configuration allows users to customize logging verbosity and behavior through
//! configuration files, environment variables, or programmatic config. It serves as the
//! bridge between user preferences and the tracing initialization system.
//!
//! ## Configuration Structure
//!
//! The `TracingConfig` struct encapsulates all tracing-related config:
//! - **Log Level**: Controls the verbosity of tracing output (OFF, ERROR, WARN, INFO, DEBUG, TRACE)
//! - **Log File Path**: Optional file that tracing output is also written to, alongside the console
//! - **Default Behavior**: Provides sensible defaults for production use

/// Default enabled state if none is provided.
const DEFAULT_ENABLED: bool = true;

/// Default tracing level for production use.
///
/// This constant defines the baseline logging verbosity when no specific configuration
/// is provided. INFO level provides a good balance between visibility and performance
/// for production deployments.
const DEFAULT_TRACING_LEVEL: lib_tracing::Levels = lib_tracing::Levels::INFO;

/// Default show config startup state if none is provided.
const SHOW_CONFIG_AT_STARTUP: bool = false;

/// Configuration structure for tracing config.
///
/// This struct encapsulates all configurable aspects of the tracing system,
/// providing a clean interface for applications to customize logging behavior.
/// The configuration is designed to be serializable, allowing it to be loaded
/// from various sources like configuration files, environment variables, or
/// programmatically constructed.
///
/// # Serialization
///
/// The struct supports both JSON and TOML serialization formats, making it
/// compatible with common configuration file formats.
///
/// # Default Values
///
/// When created with `Default::default()`, the configuration uses production-ready
/// defaults that balance observability with performance.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct TracingConfig {
    /// Whether to enable tracing logging.
    pub enabled: bool,

    /// The tracing logging level for the application.
    ///
    /// Available levels (from least to most verbose):
    /// - `OFF`: No tracing output
    /// - `ERROR`: Only error conditions
    /// - `WARN`: Errors and warnings
    /// - `INFO`: General information (default)
    /// - `DEBUG`: Detailed debugging information
    /// - `TRACE`: Very detailed execution tracing
    pub level: lib_tracing::Levels,

    /// Whether to show configuration at startup.
    pub show_config_at_startup: bool,

    /// Optional path to a file that tracing output should also be written to, in addition
    /// to the console. Defaults to `None` (console output only) when the key is absent
    /// from every configuration source.
    #[serde(default)]
    pub log_file_path: Option<std::path::PathBuf>,
}

impl Default for TracingConfig {
    /// Creates a default tracing configuration.
    ///
    /// The default configuration uses `INFO` level logging, which provides
    /// a good balance between observability and performance for production use.
    /// This level shows general application flow, important events, and
    /// non-critical warnings while avoiding excessive detail. File logging is
    /// disabled by default.
    fn default() -> Self {
        Self {
            enabled: DEFAULT_ENABLED,
            level: DEFAULT_TRACING_LEVEL,
            show_config_at_startup: SHOW_CONFIG_AT_STARTUP,
            log_file_path: None,
        }
    }
}

impl TracingConfig {
    /// Get the configured telemetry log level.
    ///
    /// Returns the telemetry level that should be used for initializing
    /// the telemetry system. This level determines which log messages
    /// will be processed and displayed.
    ///
    /// # Returns
    ///
    /// The configured `TelemetryLevels` value that can be passed directly
    /// to `lib_tracing::init()`.
    pub fn telemetry_level(&self) -> lib_tracing::Levels {
        self.level
    }

    /// Get the configured log file path, if any.
    ///
    /// Returns `None` when file logging is disabled, in which case
    /// [`lib_tracing::init`] only writes telemetry to the console.
    pub fn log_file_path(&self) -> Option<&std::path::Path> {
        self.log_file_path.as_deref()
    }

    /// Get the default configuration values as key-value pairs, for seeding a layered
    /// configuration builder's defaults -- mirrors `DatabaseConfig::default_config_values()`.
    pub fn default_config_values() -> Vec<(&'static str, String)> {
        let default_config = Self::default();
        vec![
            ("telemetry.enabled", default_config.enabled.to_string()),
            (
                "telemetry.telemetry_level",
                default_config.telemetry_level().to_string(),
            ),
            (
                "telemetry.show_config_at_startup",
                default_config.show_config_at_startup.to_string(),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_tracing::Levels;

    #[test]
    fn default_enables_tracing_at_info_level() {
        let config = TracingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.level, Levels::INFO);
        assert!(!config.show_config_at_startup);
        assert_eq!(config.log_file_path(), None);
    }

    #[test]
    fn log_file_path_returns_configured_path() {
        let config = TracingConfig {
            log_file_path: Some(std::path::PathBuf::from("/var/log/personal-ledger.log")),
            ..Default::default()
        };
        assert_eq!(
            config.log_file_path(),
            Some(std::path::Path::new("/var/log/personal-ledger.log"))
        );
    }

    #[test]
    fn deserialize_without_log_file_path_defaults_to_none() {
        let json =
            r#"{"enabled": true, "telemetry_level": "info", "show_config_at_startup": false}"#;
        let config: TracingConfig = serde_json::from_str(json).expect("deserialize TracingConfig");
        assert_eq!(config.log_file_path(), None);
    }

    #[test]
    fn deserialize_with_explicit_log_file_path() {
        let json = r#"{"enabled": true, "telemetry_level": "info", "show_config_at_startup": false, "log_file_path": "personal-ledger.log"}"#;
        let config: TracingConfig = serde_json::from_str(json).expect("deserialize TracingConfig");
        assert_eq!(
            config.log_file_path(),
            Some(std::path::Path::new("personal-ledger.log"))
        );
    }

    #[test]
    fn telemetry_level_returns_configured_level() {
        let config = TracingConfig {
            enabled: true,
            level: Levels::DEBUG,
            show_config_at_startup: false,
            log_file_path: None,
        };
        assert_eq!(config.telemetry_level(), Levels::DEBUG);
    }

    #[test]
    fn telemetry_level_returns_correct_value_for_all_variants() {
        for level in [
            Levels::OFF,
            Levels::ERROR,
            Levels::WARN,
            Levels::INFO,
            Levels::DEBUG,
            Levels::TRACE,
        ] {
            let config = TracingConfig {
                enabled: true,
                level,
                show_config_at_startup: false,
                log_file_path: None,
            };
            assert_eq!(config.telemetry_level(), level);
        }
    }

    #[test]
    fn clone_produces_equal_value() {
        let config = TracingConfig {
            enabled: false,
            level: Levels::TRACE,
            show_config_at_startup: true,
            log_file_path: None,
        };
        assert_eq!(config.clone(), config);
    }

    #[test]
    fn equal_config_compare_equal() {
        let a = TracingConfig::default();
        let b = TracingConfig::default();
        assert_eq!(a, b);
    }

    #[test]
    fn different_levels_compare_unequal() {
        let a = TracingConfig {
            level: Levels::DEBUG,
            ..Default::default()
        };
        let b = TracingConfig {
            level: Levels::ERROR,
            ..Default::default()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn different_enabled_flags_compare_unequal() {
        let a = TracingConfig {
            enabled: true,
            ..Default::default()
        };
        let b = TracingConfig {
            enabled: false,
            ..Default::default()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn different_show_config_flags_compare_unequal() {
        let a = TracingConfig {
            show_config_at_startup: false,
            ..Default::default()
        };
        let b = TracingConfig {
            show_config_at_startup: true,
            ..Default::default()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn debug_format_includes_struct_name_and_level() {
        let config = TracingConfig::default();
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("TracingConfig"));
        assert!(debug_str.contains("INFO"));
    }

    #[test]
    fn serialize_to_json_roundtrip() {
        let config = TracingConfig {
            enabled: true,
            level: Levels::WARN,
            show_config_at_startup: true,
            log_file_path: None,
        };
        let json = serde_json::to_string(&config).expect("serialize TracingConfig");
        let deserialized: TracingConfig =
            serde_json::from_str(&json).expect("deserialize TracingConfig");
        assert_eq!(config, deserialized);
    }

    #[test]
    fn serialize_level_as_lowercase_string() {
        let config = TracingConfig {
            level: Levels::DEBUG,
            ..Default::default()
        };
        let json = serde_json::to_string(&config).expect("serialize TracingConfig");
        assert!(
            json.contains("\"debug\""),
            "level should serialize as lowercase: {json}"
        );
    }

    #[test]
    fn deserialize_from_explicit_json_values() {
        let json =
            r#"{"enabled": false, "telemetry_level": "trace", "show_config_at_startup": true}"#;
        let config: TracingConfig = serde_json::from_str(json).expect("deserialize TracingConfig");
        assert!(!config.enabled);
        assert_eq!(config.level, Levels::TRACE);
        assert!(config.show_config_at_startup);
    }

    #[test]
    fn deserialize_missing_field_fails() {
        let result: Result<TracingConfig, _> = serde_json::from_str("{}");
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_invalid_level_string() {
        let json =
            r#"{"enabled": true, "telemetry_level": "verbose", "show_config_at_startup": false}"#;
        let result: Result<TracingConfig, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_uppercase_level_string() {
        let json =
            r#"{"enabled": true, "telemetry_level": "INFO", "show_config_at_startup": false}"#;
        let result: Result<TracingConfig, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn serialize_produces_expected_json_field_names() {
        let config = TracingConfig::default();
        let json = serde_json::to_string(&config).expect("serialize TracingConfig");
        assert!(
            json.contains("\"enabled\""),
            "missing 'enabled' field: {json}"
        );
        assert!(
            json.contains("\"telemetry_level\""),
            "missing 'level' field: {json}"
        );
        assert!(
            json.contains("\"show_config_at_startup\""),
            "missing 'show_config_at_startup' field: {json}"
        );
    }

    #[test]
    fn deserialize_all_level_variants() {
        for (json_str, expected) in [
            ("\"off\"", Levels::OFF),
            ("\"error\"", Levels::ERROR),
            ("\"warn\"", Levels::WARN),
            ("\"info\"", Levels::INFO),
            ("\"debug\"", Levels::DEBUG),
            ("\"trace\"", Levels::TRACE),
        ] {
            let json = format!(
                r#"{{"enabled": true, "telemetry_level": {json_str}, "show_config_at_startup": false}}"#
            );
            let config: TracingConfig = serde_json::from_str(&json)
                .unwrap_or_else(|_| panic!("failed to deserialize level {json_str}"));
            assert_eq!(config.level, expected);
        }
    }
}
