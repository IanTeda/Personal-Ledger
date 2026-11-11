//! # The `[Logs]` section
//!
//! The keys Settings › Logs shows (ADR-0057): the tracing level and the optional log file. All
//! three binaries read it, the Sync Server included.

/// Default logging level for the `[Logs]` section.
const DEFAULT_LOG: lib_tracing::Levels = lib_tracing::Levels::INFO;

/// Configuration for the `[Logs]` section.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct LogsConfig {
    /// The tracing logging level for the application (see [`lib_tracing::Levels`]).
    pub log: lib_tracing::Levels,

    /// Optional path to a file that tracing output should also be written to, in addition
    /// to the console. Defaults to `None` (console output only) when the key is absent
    /// from every configuration source.
    pub log_file_path: Option<std::path::PathBuf>,
}

impl Default for LogsConfig {
    fn default() -> Self {
        Self {
            log: DEFAULT_LOG,
            log_file_path: None,
        }
    }
}

impl LogsConfig {
    /// Returns the configured logging level.
    pub fn log(&self) -> lib_tracing::Levels {
        self.log
    }

    /// Returns the configured log file path, if any.
    ///
    /// `None` means console output only -- [`lib_tracing::init`] doesn't also write to a
    /// file.
    pub fn log_file_path(&self) -> Option<&std::path::Path> {
        self.log_file_path.as_deref()
    }

    /// The defaults seeded into a layered configuration builder, mirroring
    /// `SyncServerConfig::default_config_values()`. `log_file_path` has no meaningful default.
    pub fn default_config_values() -> Vec<(&'static str, String)> {
        vec![("logs.log", DEFAULT_LOG.to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_log_level_and_no_file() {
        let config = LogsConfig::default();
        assert_eq!(config.log(), lib_tracing::Levels::INFO);
        assert_eq!(config.log_file_path(), None);
    }

    #[test]
    fn default_config_values_seed_the_log_level() {
        assert_eq!(
            LogsConfig::default_config_values(),
            vec![("logs.log", "info".to_string())]
        );
    }

    #[test]
    fn the_logs_section_reads_its_keys_and_an_environment_variable_outranks_it() {
        let section = crate::ledger::parsed_for_test(
            "[Logs]\nlog = warn\nlog_file_path = /tmp/ledger.log\n",
            &[],
        )
        .logs_config()
        .clone();
        assert_eq!(section.log(), lib_tracing::Levels::WARN);
        assert_eq!(
            section.log_file_path(),
            Some(std::path::Path::new("/tmp/ledger.log"))
        );

        let section = crate::ledger::parsed_for_test(
            "[Logs]\nlog = warn\n",
            &[("PERSONAL_LEDGER_LOGS__LOG", "trace")],
        )
        .logs_config()
        .clone();
        assert_eq!(section.log(), lib_tracing::Levels::TRACE);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = LogsConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: LogsConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
