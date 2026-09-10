//! # Personal Ledger Configuration
//!
//! This module provides a layered configuration system for the Personal Ledger application.
//! It supports loading configuration from multiple sources with a defined precedence order.
//! Configuration files use INI format.
//!
//! Two entry points exist, differing only in how far the file-location *search* reaches --
//! the sources layered on top of it (defaults, an explicit path, environment variables) are
//! identical (ADR-0014):
//!
//! - [`LedgerConfig::parse`] -- for `bin-tui`/`bin-desktop`: the full search --
//!   defaults -> system -> user -> executable-directory -> working-directory ->
//!   explicit path -> environment variables.
//! - [`LedgerConfig::parse_for_sync_server`] -- for `bin-sync-server`: defaults ->
//!   explicit path -> environment variables only. The system/user/executable-directory/
//!   working-directory tiers don't correspond to anything meaningful inside a Docker
//!   container.
//!
//! ## Example
//!
//! ```rust
//! use lib_config::LedgerConfig;
//!
//! let config = LedgerConfig::parse(None).expect("Failed to load config");
//!
//! let telemetry = config.telemetry_config();
//! println!("Telemetry level: {:?}", telemetry.telemetry_level());
//!
//! let database = config.database_config();
//! println!("Database URL: {}", database.url());
//!
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
//!
//! # Only read by bin-sync-server, via `LedgerConfig::parse_for_sync_server`.
//! [sync-server]
//! bind_address = "0.0.0.0:50051"
//! ```

use std::path::{Path, PathBuf};

use config::{Config, ConfigBuilder, builder::DefaultState};

/// Application name used for configuration directories, file names, and environment
/// variable prefixes.
const APPLICATION_NAME: &str = "personal-ledger";

/// Environment variable prefix derived from [`APPLICATION_NAME`] (`personal-ledger` ->
/// `PERSONAL_LEDGER`).
const ENV_PREFIX: &str = "PERSONAL_LEDGER";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct LedgerConfig {
    #[serde(alias = "Database")]
    pub database: crate::DatabaseConfig,

    /// Sync-Server-only settings. Populated from the `[sync-server]` section (see
    /// [`Self::normalise_ini`] for the `-`/`_` translation); never read by Clients.
    #[serde(alias = "SyncServer", alias = "Sync-Server")]
    pub sync_server: crate::SyncServerConfig,

    /// Telemetry configuration.
    #[serde(alias = "Telemetry")]
    pub telemetry: crate::TracingConfig,
}

impl LedgerConfig {
    /// The application name used for configuration.
    pub fn application_name() -> &'static str {
        APPLICATION_NAME
    }

    /// The environment variable prefix used for configuration overrides.
    pub fn env_prefix() -> &'static str {
        ENV_PREFIX
    }

    /// Parse configuration for a Client (`bin-tui`/`bin-desktop`): the full
    /// defaults -> system -> user -> executable-directory -> working-directory ->
    /// explicit path -> environment variables precedence chain (highest last).
    ///
    /// # Errors
    /// Returns an error if any present config file can't be read/parsed, or if the merged
    /// result doesn't deserialize into a valid `LedgerConfig`.
    pub fn parse(config_file: Option<&Path>) -> super::Result<LedgerConfig> {
        let mut config_builder = Self::defaults_builder()?;

        if let Some(system_config) = Self::get_system_config_path().filter(|p| p.exists()) {
            config_builder = Self::add_ini_source(config_builder, &system_config)?;
        }
        if let Some(user_config) = Self::get_user_config_path().filter(|p| p.exists()) {
            config_builder = Self::add_ini_source(config_builder, &user_config)?;
        }
        if let Some(exec_config) = Self::get_executable_config_path().filter(|p| p.exists()) {
            config_builder = Self::add_ini_source(config_builder, &exec_config)?;
        }

        let cwd_config = if config_file.is_none() {
            Some(Self::get_cwd_config_path()?)
        } else {
            None
        };
        if let Some(cwd_config) = cwd_config.filter(|p| p.exists()) {
            config_builder = Self::add_ini_source(config_builder, &cwd_config)?;
        }

        config_builder = Self::add_explicit_and_env(config_builder, config_file)?;

        Self::build(config_builder)
    }

    /// Parse configuration for the Sync Server: a reduced defaults -> explicit path ->
    /// environment variables chain (ADR-0014) -- the Client-only system/user/executable-
    /// directory/working-directory search tiers don't correspond to anything meaningful
    /// inside a Docker container.
    ///
    /// # Errors
    /// Returns an error if the explicit config file (when given) can't be read/parsed, or if
    /// the merged result doesn't deserialize into a valid `LedgerConfig`.
    pub fn parse_for_sync_server(config_file: Option<&Path>) -> super::Result<LedgerConfig> {
        let config_builder = Self::defaults_builder()?;
        let config_builder = Self::add_explicit_and_env(config_builder, config_file)?;
        Self::build(config_builder)
    }

    /// Seed a fresh layered builder with every section's built-in defaults (lowest
    /// precedence) -- shared by both [`Self::parse`] and [`Self::parse_for_sync_server`].
    fn defaults_builder() -> super::Result<ConfigBuilder<DefaultState>> {
        let mut config_builder = Config::builder();

        for (key, value) in crate::TracingConfig::default_config_values() {
            config_builder = config_builder.set_default(key, value)?;
        }

        for (key, value) in crate::DatabaseConfig::default_config_values() {
            config_builder = config_builder.set_default(key, value)?;
        }

        for (key, value) in crate::SyncServerConfig::default_config_values() {
            config_builder = config_builder.set_default(key, value)?;
        }

        Ok(config_builder)
    }

    /// Add the explicit config file (if given and it exists) and environment variable
    /// overrides -- the two highest-precedence tiers, shared by both entry points. Env vars
    /// (e.g. `PERSONAL_LEDGER_TELEMETRY__TELEMETRY_LEVEL=debug`) always come last/highest.
    fn add_explicit_and_env(
        config_builder: ConfigBuilder<DefaultState>,
        config_file: Option<&Path>,
    ) -> super::Result<ConfigBuilder<DefaultState>> {
        let mut config_builder = config_builder;

        if let Some(explicit_config) = config_file.filter(|p| p.exists()) {
            config_builder = Self::add_ini_source(config_builder, explicit_config)?;
        }

        config_builder = config_builder.add_source(config::Environment::with_prefix(ENV_PREFIX));

        Ok(config_builder)
    }

    /// Read an INI file, normalise its section headers, and add it as a source.
    fn add_ini_source(
        config_builder: ConfigBuilder<DefaultState>,
        path: &Path,
    ) -> super::Result<ConfigBuilder<DefaultState>> {
        let normalised = Self::normalise_ini(path)?;
        Ok(config_builder.add_source(config::File::from_str(&normalised, config::FileFormat::Ini)))
    }

    /// Read an INI file and normalise its section headers: lower-cased (so `[Telemetry]`/
    /// `[telemetry]` are equivalent) and with `-` translated to `_` (so `[sync-server]`
    /// matches the `sync_server` field/serde alias -- INI section names commonly use
    /// hyphens, but Rust field names can't).
    fn normalise_ini(p: &Path) -> super::Result<String> {
        let content = std::fs::read_to_string(p).map_err(|e| {
            super::Error::Validation(format!("Could not read config file {:?}: {}", p, e))
        })?;

        let normalised = content
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    let inner = &trimmed[1..trimmed.len() - 1];
                    format!("[{}]", inner.to_lowercase().replace('-', "_"))
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(normalised)
    }

    /// Build and deserialize the layered `config::Config` into a `LedgerConfig`.
    fn build(config_builder: ConfigBuilder<DefaultState>) -> super::Result<LedgerConfig> {
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
    fn get_cwd_config_path() -> Result<PathBuf, super::Error> {
        let cwd = std::env::current_dir().map_err(|e| {
            super::Error::Validation(format!(
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
    pub fn telemetry_config(&self) -> &crate::TracingConfig {
        &self.telemetry
    }

    /// Get the database configuration.
    pub fn database_config(&self) -> &crate::DatabaseConfig {
        &self.database
    }

    /// Get the Sync-Server-only configuration.
    pub fn sync_server_config(&self) -> &crate::SyncServerConfig {
        &self.sync_server
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

    /// The process (and thus its current directory) is shared by every test thread `cargo
    /// test` runs concurrently -- serialise the tests that change cwd via this mutex so they
    /// don't stomp on each other's temp directories.
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Run `body` inside a fresh empty temp directory (so no ambient config files leak in),
    /// restoring the original cwd afterwards even if `body` panics.
    fn in_empty_cwd<T>(body: impl FnOnce() -> T) -> T {
        let _guard = CWD_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let original_cwd = env::current_dir().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));

        env::set_current_dir(original_cwd).unwrap();

        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    #[test]
    fn parse_with_defaults_loads_successfully() {
        in_empty_cwd(|| {
            let config = LedgerConfig::parse(None).unwrap();
            assert_eq!(
                config.telemetry.telemetry_level(),
                crate::TracingConfig::default().telemetry_level()
            );
            assert_eq!(
                config.database.url(),
                crate::DatabaseConfig::default().url()
            );
            assert_eq!(
                config.database.max_connections(),
                crate::DatabaseConfig::default().max_connections()
            );
            assert_eq!(
                config.sync_server.bind_address(),
                crate::SyncServerConfig::default().bind_address()
            );
        });
    }

    #[test]
    fn parse_for_sync_server_with_defaults_loads_successfully() {
        in_empty_cwd(|| {
            let config = LedgerConfig::parse_for_sync_server(None).unwrap();
            assert_eq!(
                config.sync_server.bind_address(),
                crate::SyncServerConfig::default().bind_address()
            );
            assert_eq!(
                config.database.url(),
                crate::DatabaseConfig::default().url()
            );
        });
    }

    #[test]
    fn parse_with_explicit_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.conf");

        let config_content = r#"
        [telemetry]
        telemetry_level = "debug"

        [database]
        url = "sqlite:test.db"
        max_connections = 20
        "#;
        fs::write(&config_file, config_content).unwrap();

        let config = LedgerConfig::parse(Some(&config_file)).unwrap();
        assert_eq!(
            config.telemetry.telemetry_level(),
            lib_tracing::Levels::DEBUG
        );
        assert_eq!(config.database.url(), "sqlite:test.db");
        assert_eq!(config.database.max_connections(), 20);
    }

    #[test]
    fn parse_for_sync_server_reads_sync_server_section_from_explicit_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("sync-server.conf");

        let config_content = r#"
        [sync-server]
        bind_address = "127.0.0.1:9000"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let config = LedgerConfig::parse_for_sync_server(Some(&config_file)).unwrap();
        assert_eq!(config.sync_server.bind_address(), "127.0.0.1:9000");
    }

    #[test]
    fn parse_for_sync_server_ignores_cwd_config() {
        in_empty_cwd(|| {
            let config_dir = PathBuf::from("config");
            fs::create_dir(&config_dir).unwrap();
            let cwd_config_file = config_dir.join("personal-ledger.conf");
            fs::write(
                &cwd_config_file,
                r#"
                [telemetry]
                telemetry_level = "warn"
                "#,
            )
            .unwrap();

            // A Client (`parse`) would pick this up; the Sync Server must not.
            let config = LedgerConfig::parse_for_sync_server(None).unwrap();
            assert_eq!(
                config.telemetry.telemetry_level(),
                crate::TracingConfig::default().telemetry_level()
            );
        });
    }

    #[test]
    fn parse_with_nonexistent_explicit_file_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.conf");
        // Should still succeed because the file is optional.
        let result = LedgerConfig::parse(Some(&nonexistent_path));
        assert!(result.is_ok());
    }

    #[test]
    fn parse_with_case_insensitive_telemetry_section() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.conf");

        let config_content = r#"
        [telemetry]
        telemetry_level = "info"

        [database]
        url = "sqlite:custom.db"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let config = LedgerConfig::parse(Some(&config_file)).unwrap();
        assert_eq!(
            config.telemetry.telemetry_level(),
            lib_tracing::Levels::INFO
        );
        assert_eq!(config.database.url(), "sqlite:custom.db");
    }

    #[test]
    fn parse_with_invalid_config_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("invalid.conf");

        // Missing closing bracket -- malformed INI.
        let config_content = r#"
        [telemetry
        telemetry_level = "debug"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let result = LedgerConfig::parse(Some(&config_file));
        assert!(result.is_err());
    }

    #[test]
    fn parse_with_invalid_telemetry_level_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("invalid_level.conf");

        let config_content = r#"
        [telemetry]
        telemetry_level = "invalid"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let result = LedgerConfig::parse(Some(&config_file));
        assert!(result.is_err());
    }

    #[test]
    fn parse_precedence_explicit_over_cwd() {
        in_empty_cwd(|| {
            let config_dir = PathBuf::from("config");
            fs::create_dir(&config_dir).unwrap();
            let cwd_config_file = config_dir.join("personal-ledger.conf");
            fs::write(
                &cwd_config_file,
                r#"
                [Telemetry]
                telemetry_level = "warn"

                [Database]
                max_connections = 5
                "#,
            )
            .unwrap();

            let explicit_file = PathBuf::from("explicit.conf");
            fs::write(
                &explicit_file,
                r#"
                [Telemetry]
                telemetry_level = "debug"

                [Database]
                max_connections = 15
                "#,
            )
            .unwrap();

            let config = LedgerConfig::parse(Some(&explicit_file)).unwrap();
            assert_eq!(
                config.telemetry.telemetry_level(),
                lib_tracing::Levels::DEBUG
            );
            assert_eq!(config.database.max_connections(), 15);
        });
    }

    #[test]
    fn parse_reads_hyphenated_sync_server_section() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.conf");

        fs::write(
            &config_file,
            r#"
            [Sync-Server]
            bind_address = "0.0.0.0:1234"
            "#,
        )
        .unwrap();

        let config = LedgerConfig::parse(Some(&config_file)).unwrap();
        assert_eq!(config.sync_server.bind_address(), "0.0.0.0:1234");
    }
}
