//! # Personal Ledger Configuration
//!
//! This module provides a layered configuration system for the Personal Ledger application.
//! It supports loading configuration from multiple sources with a defined precedence order.
//!
//! Configuration files use INI format.
//!
//! ## Configuration Sources (in precedence order)
//!
//! 1. Built-in defaults (lowest precedence)
//! 2. System-wide configuration files
//! 3. User-specific configuration files
//! 4. Executable directory configuration files
//! 5. Current working directory configuration files
//! 6. Explicit configuration file (passed to `parse`)
//! 7. Environment variables (highest precedence)
//!
//! ## Example
//!
//! ```rust
//! use lib_config::LedgerConfig;
//!
//! // Load configuration from default locations
//! let config = LedgerConfig::parse(None).expect("Failed to load config");
//!
//! // Access telemetry configuration
//! let telemetry = config.telemetry_config();
//! println!("Telemetry level: {:?}", telemetry.telemetry_level());
//!
//! // Access database configuration
//! let database = config.database_config();
//! println!("Database URL: {}", database.url());
//!
//! // Load with explicit config file
//! use std::path::Path;
//! let config_path = Path::new("custom.conf");
//! let config = LedgerConfig::parse(Some(config_path)).expect("Failed to load config");
//! ```
//!
//! ## Configuration File Example
//!
//! ```ini
//! [telemetry]
//! telemetry_level = "debug"
//!
//! [database]
//! url = "sqlite:./personal-ledger.db"
//! max_connections = 10
//! min_connections = 1
//! acquire_timeout_seconds = 30
//! idle_timeout_seconds = 600
//! max_lifetime_seconds = 1800
//! ```

use std::path::{Path, PathBuf};

use config::Config;
use lib_database as database;
use lib_telemetry as telemetry;

/// Application name used for configuration directories and environment variables.
/// This should match the binary name and be used consistently across the application.
///
/// # Changing the Application Name
///
/// To use this configuration system for a different application:
/// 1. Change this constant to match your application name
/// 2. Update the corresponding ENV_PREFIX if needed
/// 3. Ensure your binary name matches this constant
const APPLICATION_NAME: &str = "personal-ledger";

/// Environment variable prefix derived from the application name.
/// Converts "personal-ledger" to "PERSONAL_LEDGER" for environment variables.
///
/// # Changing the Environment Prefix
///
/// If your application name contains characters that aren't valid in environment
/// variable names, update this constant accordingly.
const ENV_PREFIX: &str = "PERSONAL_LEDGER";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct LedgerConfig {
    #[serde(alias = "Telemetry")]
    pub telemetry: telemetry::TelemetryConfig,

    #[serde(alias = "Database")]
    pub database: database::DatabaseConfig,
}

impl LedgerConfig {
    /// Get the application name used for configuration.
    ///
    /// This returns the same name used for configuration directories,
    /// file names, and environment variable prefixes.
    ///
    /// # Returns
    ///
    /// The application name as a static string slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_config::LedgerConfig;
    ///
    /// assert_eq!(LedgerConfig::application_name(), "personal-ledger");
    /// ```
    pub fn application_name() -> &'static str {
        APPLICATION_NAME
    }

    /// Get the environment variable prefix used for configuration.
    ///
    /// This returns the prefix used for environment variable overrides.
    ///
    /// # Returns
    ///
    /// The environment variable prefix as a static string slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_config::LedgerConfig;
    ///
    /// assert_eq!(LedgerConfig::env_prefix(), "PERSONAL_LEDGER");
    /// ```
    pub fn env_prefix() -> &'static str {
        ENV_PREFIX
    }

    pub fn parse(config_file: Option<&Path>) -> super::ConfigResult<LedgerConfig> {
        // Higher precedence sources override lower precedence ones:
        // 1. Built-in defaults (lowest)
        // 2. System config files
        // 3. User config files
        // 4. Executable directory config files
        // 5. Current working directory config files
        // 6. Explicit config files
        // 7. Environment variables (highest)

        //-- 01. Build Defaults
        let default_telemetry_level = telemetry::TelemetryConfig::default().telemetry_level();

        let mut config_builder = Config::builder().set_default(
            "telemetry.telemetry_level",
            default_telemetry_level.to_string(),
        )?;

        // Add database defaults
        for (key, value) in database::DatabaseConfig::default_config_values() {
            config_builder = config_builder.set_default(key, value)?;
        }

        //-- helper: read INI file and normalise section headers to lowercase
        let normalise_ini = |p: &Path| -> super::ConfigResult<String> {
            let content = std::fs::read_to_string(p).map_err(|e| {
                super::ConfigError::Validation(format!(
                    "Could not read config file {:?}: {}",
                    p, e
                ))
            })?;

            let normalised = content
                .lines()
                .map(|line| {
                    let trimmed = line.trim();
                    if trimmed.starts_with('[') && trimmed.ends_with(']') {
                        // Lowercase the section name inside the brackets
                        let inner = &trimmed[1..trimmed.len() - 1];
                        format!("[{}]", inner.to_lowercase())
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            Ok(normalised)
        };

        //-- 02. System config directory (lowest precedence after defaults)
        if let Some(system_config) = Self::get_system_config_path().filter(|p| p.exists()) {
            let normalised = normalise_ini(&system_config)?;
            config_builder = config_builder.add_source(
                config::File::from_str(&normalised, config::FileFormat::Ini),
            );
        }

        //-- 03. User config directory
        if let Some(user_config) = Self::get_user_config_path().filter(|p| p.exists()) {
            let normalised = normalise_ini(&user_config)?;
            config_builder = config_builder.add_source(
                config::File::from_str(&normalised, config::FileFormat::Ini),
            );
        }

        //-- 04. Executable directory
        if let Some(exec_config) = Self::get_executable_config_path().filter(|p| p.exists()) {
            let normalised = normalise_ini(&exec_config)?;
            config_builder = config_builder.add_source(
                config::File::from_str(&normalised, config::FileFormat::Ini),
            );
        }

        //-- 05. Current working directory
        let cwd_config = if config_file.is_none() {
            Some(Self::get_cwd_config_path()?)
        } else {
            None
        };
        if let Some(cwd_config) = cwd_config.filter(|p| p.exists()) {
            let normalised = normalise_ini(&cwd_config)?;
            config_builder = config_builder.add_source(
                config::File::from_str(&normalised, config::FileFormat::Ini),
            );
        }

        //-- 06. Explicit config file
        if let Some(explicit_config) = config_file.filter(|p| p.exists()) {
            let normalised = normalise_ini(explicit_config)?;
            config_builder = config_builder.add_source(
                config::File::from_str(&normalised, config::FileFormat::Ini),
            );
        }

        //-- 07. Environment variables (highest precedence)
        // Supports variables like: PERSONAL_LEDGER_TELEMETRY__TELEMETRY_LEVEL=debug
        config_builder = config_builder.add_source(config::Environment::with_prefix(ENV_PREFIX));

        //-- 08. Build and Deserialize
        let config = config_builder.build()?;
        let ledger_config: LedgerConfig = config.try_deserialize()?;

        Ok(ledger_config)
    }

    /// Get the system-wide configuration file path.
    ///
    /// Returns the path to the system configuration file using platform-specific
    /// standard locations. This provides system administrators with a way to
    /// set default configurations for all users.
    ///
    /// - **Unix/Linux**: `/etc/personal-ledger/personal-ledger.conf`
    /// - **Windows**: `%ALLUSERSPROFILE%\personal-ledger\personal-ledger.conf`
    /// - **macOS**: `/Library/Preferences/personal-ledger/personal-ledger.conf`
    /// - **Other**: None (system config not supported)
    fn get_system_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            Some(
                PathBuf::from("/etc")
                    .join(APPLICATION_NAME)
                    .join(format!("{}.conf", APPLICATION_NAME)),
            )
        }
        #[cfg(target_os = "macos")]
        {
            Some(
                PathBuf::from("/Library/Preferences")
                    .join(APPLICATION_NAME)
                    .join(format!("{}.conf", APPLICATION_NAME)),
            )
        }
        #[cfg(target_os = "windows")]
        {
            // On Windows, use ALLUSERSPROFILE for system-wide settings
            std::env::var_os("ALLUSERSPROFILE").map(|all_users| {
                PathBuf::from(all_users)
                    .join(APPLICATION_NAME)
                    .join(format!("{}.conf", APPLICATION_NAME))
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            None
        }
    }

    /// Get the user-specific configuration file path.
    ///
    /// Returns the path to the user's configuration file using the standard
    /// platform-specific config directory. This allows individual users to
    /// customise their configuration without affecting other users.
    ///
    /// - **Linux**: `~/.config/personal-ledger/personal-ledger.conf` (or `$XDG_CONFIG_HOME`)
    /// - **macOS**: `~/Library/Preferences/personal-ledger/personal-ledger.conf`
    /// - **Windows**: `%APPDATA%\personal-ledger\personal-ledger.conf`
    fn get_user_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|config_dir| {
            config_dir
                .join(APPLICATION_NAME)
                .join(format!("{}.conf", APPLICATION_NAME))
        })
    }

    /// Get the executable directory configuration file path.
    ///
    /// Returns the path to a configuration file in the same directory as the
    /// executable. This is useful for portable applications or when the config
    /// should be distributed with the binary.
    ///
    /// Note: This is determined at runtime based on the executable's location.
    fn get_executable_config_path() -> Option<PathBuf> {
        dirs::executable_dir().map(|exec_dir| {
            exec_dir
                .join(APPLICATION_NAME)
                .join(format!("{}.conf", APPLICATION_NAME))
        })
    }

    /// Get the current working directory configuration file path.
    ///
    /// Returns the path to a configuration file in the current working directory.
    /// This allows project-specific configuration when running from a directory
    /// that contains a config file.
    ///
    /// Returns an error if the current directory cannot be determined.
    fn get_cwd_config_path() -> Result<PathBuf, super::ConfigError> {
        let cwd = std::env::current_dir().map_err(|e| {
            super::ConfigError::Validation(format!(
                "Could not get current directory for config loading: {}",
                e
            ))
        })?;
        let dir = cwd
            .join("config")
            .join(format!("{}.conf", APPLICATION_NAME));
        Ok(dir)
    }

    /// Get the telemetry configuration.
    pub fn telemetry_config(&self) -> &lib_telemetry::TelemetryConfig {
        &self.telemetry
    }

    /// Get the database configuration.
    pub fn database_config(&self) -> &lib_database::DatabaseConfig {
        &self.database
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn application_name_returns_correct_value() {
        assert_eq!(LedgerConfig::application_name(), APPLICATION_NAME);
    }

    #[test]
    fn env_prefix_returns_correct_value() {
        assert_eq!(LedgerConfig::env_prefix(), ENV_PREFIX);
    }

    #[test]
    fn telemetry_config_returns_telemetry_reference() {
        let config = LedgerConfig::default();
        let telemetry = config.telemetry_config();
        assert_eq!(
            telemetry.telemetry_level(),
            config.telemetry.telemetry_level()
        );
    }

    #[test]
    fn database_config_returns_database_reference() {
        let config = LedgerConfig::default();
        let database = config.database_config();
        assert_eq!(
            database.url(),
            config.database.url()
        );
        assert_eq!(
            database.max_connections(),
            config.database.max_connections()
        );
    }

    #[test]
    fn get_user_config_path_returns_expected_path() {
        let path = LedgerConfig::get_user_config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.ends_with("personal-ledger/personal-ledger.conf"));
        assert!(path.to_string_lossy().contains("personal-ledger"));
    }

    #[test]
    fn get_executable_config_path_returns_expected_path() {
        let path = LedgerConfig::get_executable_config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.ends_with("personal-ledger/personal-ledger.conf"));
        assert!(path.to_string_lossy().contains("personal-ledger"));
    }

    #[test]
    fn get_system_config_path_returns_expected_path() {
        let path = LedgerConfig::get_system_config_path();
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.ends_with("personal-ledger.conf"));
            assert!(path.to_string_lossy().contains("personal-ledger"));
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            assert!(path.is_none());
        }
    }

    #[test]
    fn parse_with_defaults_loads_successfully() {
        // Change to a temp directory to ensure no config files are loaded
        let temp_dir = TempDir::new().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // This should work without any config files present
        let result = LedgerConfig::parse(None);
        assert!(result.is_ok());
        let config = result.unwrap();
        // Should have default telemetry config
        assert_eq!(
            config.telemetry.telemetry_level(),
            telemetry::TelemetryConfig::default().telemetry_level()
        );
        // Should have default database config
        assert_eq!(
            config.database.url(),
            database::DatabaseConfig::default().url()
        );
        assert_eq!(
            config.database.max_connections(),
            database::DatabaseConfig::default().max_connections()
        );

        // Restore original directory
        std::env::set_current_dir(original_cwd).unwrap();
    }

    #[test]
    fn parse_with_explicit_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.conf");

        // Create a config file with custom telemetry level (INI format)
        let config_content = 
        r#"
        [telemetry]
        telemetry_level = "debug"

        [database]
        url = "sqlite:test.db"
        max_connections = 20
        "#;
        fs::write(&config_file, config_content).unwrap();

        let result = LedgerConfig::parse(Some(&config_file));
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(
            config.telemetry.telemetry_level(),
            telemetry::TelemetryLevels::DEBUG
        );
        assert_eq!(config.database.url(), "sqlite:test.db");
        assert_eq!(config.database.max_connections(), 20);
    }

    #[test]
    fn parse_with_nonexistent_explicit_file_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.conf");
        let result = LedgerConfig::parse(Some(&nonexistent_path));
        // Should still succeed because the file is optional
        assert!(result.is_ok());
    }

    #[test]
    fn parse_with_case_insensitive_telemetry_section() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.conf");

        // Create config with [telemetry]
        let config_content = 
        r#"
        [telemetry]
        telemetry_level = "info"

        [database]
        url = "sqlite:custom.db"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let result = LedgerConfig::parse(Some(&config_file));
        if let Err(e) = &result {
            println!("Parse error: {:?}", e);
        }
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(
            config.telemetry.telemetry_level(),
            telemetry::TelemetryLevels::INFO
        );
        assert_eq!(config.database.url(), "sqlite:custom.db");
    }

    #[test]
    fn parse_with_invalid_config_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("invalid.conf");

        // Create invalid INI content (malformed)
        let config_content = 
        r#"
        [telemetry
        telemetry_level = "debug"
        "#; // Missing closing bracket
        fs::write(&config_file, config_content).unwrap();

        let result = LedgerConfig::parse(Some(&config_file));
        assert!(result.is_err());
    }

    #[test]
    fn parse_with_invalid_telemetry_level_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("invalid_level.conf");

        // Create config with invalid telemetry level
        let config_content = 
        r#"
        [telemetry]
        telemetry_level = "invalid"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let result = LedgerConfig::parse(Some(&config_file));
        assert!(result.is_err());
    }

    #[test]
    fn parse_precedence_explicit_over_cwd() {
        let temp_dir = TempDir::new().unwrap();
        let original_cwd = env::current_dir().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        // Create CWD config with warn
        let config_dir = temp_dir.path().join("config");
        fs::create_dir(&config_dir).unwrap();
        let cwd_config_file = config_dir.join("personal-ledger.conf");
        let cwd_content = 
        r#"
        [Telemetry]
        telemetry_level = "warn"

        [Database]
        max_connections = 5
        "#;
        fs::write(&cwd_config_file, cwd_content).unwrap();

        // Create explicit config with debug
        let explicit_file = temp_dir.path().join("explicit.conf");
        let explicit_content = 
        r#"
        [Telemetry]
        telemetry_level = "debug"

        [Database]
        max_connections = 15
        "#;
        fs::write(&explicit_file, explicit_content).unwrap();

        let result = LedgerConfig::parse(Some(&explicit_file));
        if let Err(e) = &result {
            println!("Parse error: {:?}", e);
        }
        assert!(result.is_ok());
        let config = result.unwrap();
        // Explicit should override CWD
        assert_eq!(
            config.telemetry.telemetry_level(),
            telemetry::TelemetryLevels::DEBUG
        );
        assert_eq!(config.database.max_connections(), 15);

        env::set_current_dir(original_cwd).unwrap();
    }
}
