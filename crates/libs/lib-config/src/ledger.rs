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
//! An explicit path is still optional -- a missing file falls back to the rest of the
//! chain rather than erroring.
//!
//! ## Configuration File Example
//!
//! ```ini
//! [personal-ledger]
//! data = "~/Documents/Personal-Ledger"
//! file = "~/Documents/Personal-Ledger/My-Personal-Ledger.pldb"
//! log = "debug"
//! log_file_path = "/var/log/personal-ledger/personal-ledger.log"
//!
//! # Only read by bin-sync-server, via `LedgerConfig::parse_for_sync_server`.
//! [sync-server]
//! bind_address = "0.0.0.0:50051"
//! database_uri = "sqlite:./sync-server.sqlite"
//!
//! # Only read by bin-tui/bin-desktop; merged into the Sync Server's config but never used.
//! [keybindings]
//! super_key = "ctrl"
//! back = "esc"
//! open_command_popup = ":"
//! ```

use std::collections::HashMap;
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
    /// Bootstrap/startup settings, not synced across computer systems. Populated from the
    /// `[Personal-Ledger]` section.
    #[serde(
        alias = "PersonalLedger",
        alias = "Personal-Ledger",
        alias = "personal-ledger"
    )]
    pub personal_ledger: crate::PersonalLedgerConfig,

    /// Keyboard shortcut configuration, read by the TUI/Desktop Clients. Populated from the
    /// `[keybindings]` section; the Sync Server merges its defaults in but never reads them.
    #[serde(alias = "Keybindings", alias = "KeyBindings")]
    pub keybindings: crate::KeyBindingConfig,

    /// Sync-Server-only settings. Populated from the `[sync-server]` section (see
    /// [`Self::normalise_ini`] for the `-`/`_` translation); never read by Clients.
    #[serde(alias = "SyncServer", alias = "Sync-Server", alias = "sync-server")]
    pub sync_server: crate::SyncServerConfig,
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
    pub fn parse(config_file: Option<&Path>) -> crate::Result<LedgerConfig> {
        Self::parse_with_env(config_file, None)
    }

    /// [`Self::parse`] with an optional injected environment (`None` reads the process
    /// environment). Tests use this because mutating the process environment is `unsafe`.
    fn parse_with_env(
        config_file: Option<&Path>,
        env: Option<HashMap<String, String>>,
    ) -> crate::Result<LedgerConfig> {
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

        config_builder = Self::add_explicit_and_env(config_builder, config_file, env)?;

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
    pub fn parse_for_sync_server(config_file: Option<&Path>) -> crate::Result<LedgerConfig> {
        Self::parse_for_sync_server_with_env(config_file, None)
    }

    /// [`Self::parse_for_sync_server`] with an optional injected environment.
    fn parse_for_sync_server_with_env(
        config_file: Option<&Path>,
        env: Option<HashMap<String, String>>,
    ) -> crate::Result<LedgerConfig> {
        let config_builder = Self::defaults_builder()?;
        let config_builder = Self::add_explicit_and_env(config_builder, config_file, env)?;
        Self::build(config_builder)
    }

    /// Seed a fresh layered builder with every section's built-in defaults (lowest
    /// precedence) -- shared by both [`Self::parse`] and [`Self::parse_for_sync_server`].
    fn defaults_builder() -> crate::Result<ConfigBuilder<DefaultState>> {
        let mut config_builder = Config::builder();

        for (key, value) in crate::SyncServerConfig::default_config_values() {
            config_builder = config_builder.set_default(key, value)?;
        }

        for (key, value) in crate::KeyBindingConfig::default_config_values() {
            config_builder = config_builder.set_default(key, value)?;
        }

        for (key, value) in crate::PersonalLedgerConfig::default_config_values() {
            config_builder = config_builder.set_default(key, value)?;
        }

        Ok(config_builder)
    }

    /// Add the explicit config file (if given and it exists) and environment variable
    /// overrides -- the two highest-precedence tiers, shared by both entry points. Env vars
    /// (e.g. `PERSONAL_LEDGER_PERSONAL_LEDGER__LOG=debug`) always come last/highest: the
    /// single `_` after the prefix is followed by `SECTION__KEY`, where `__` is the nesting
    /// separator (so single underscores inside a key, like `LOG_FILE_PATH`, are preserved).
    /// `env` replaces the process environment when given.
    fn add_explicit_and_env(
        config_builder: ConfigBuilder<DefaultState>,
        config_file: Option<&Path>,
        env: Option<HashMap<String, String>>,
    ) -> crate::Result<ConfigBuilder<DefaultState>> {
        let mut config_builder = config_builder;

        if let Some(explicit_config) = config_file.filter(|p| p.exists()) {
            config_builder = Self::add_ini_source(config_builder, explicit_config)?;
        }

        // `prefix_separator` must be explicit: `config` otherwise reuses `separator` for it.
        let environment = config::Environment::with_prefix(ENV_PREFIX)
            .prefix_separator("_")
            .separator("__")
            .source(env);
        config_builder = config_builder.add_source(environment);

        Ok(config_builder)
    }

    /// Read an INI file, normalise its section headers, and add it as a source.
    fn add_ini_source(
        config_builder: ConfigBuilder<DefaultState>,
        path: &Path,
    ) -> crate::Result<ConfigBuilder<DefaultState>> {
        let normalised = Self::normalise_ini(path)?;
        Ok(config_builder.add_source(config::File::from_str(&normalised, config::FileFormat::Ini)))
    }

    /// Read an INI file and normalise its section headers: lower-cased (so `[Personal-Ledger]`/
    /// `[personal-ledger]` are equivalent) and with `-` translated to `_` (so `[sync-server]`
    /// matches the `sync_server` field/serde alias -- INI section names commonly use
    /// hyphens, but Rust field names can't).
    fn normalise_ini(p: &Path) -> crate::Result<String> {
        let content = std::fs::read_to_string(p).map_err(|e| {
            crate::Error::Validation(format!("Could not read config file {:?}: {}", p, e))
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
    fn build(config_builder: ConfigBuilder<DefaultState>) -> crate::Result<LedgerConfig> {
        let config = config_builder.build()?;
        let ledger_config: LedgerConfig = config.try_deserialize()?;
        Ok(ledger_config)
    }

    /// System-wide config file path.
    ///
    /// - **Unix/Linux**: `/etc/personal-ledger/personal-ledger.conf`
    /// - **Windows**: `%ALLUSERSPROFILE%\personal-ledger\personal-ledger.conf`
    /// - **macOS**: `/Library/Preferences/personal-ledger/personal-ledger.conf`
    /// - **Other**: unsupported
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

    /// User-specific config file path.
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

    /// Config file path in the same directory as the running executable (portable installs).
    fn get_executable_config_path() -> Option<PathBuf> {
        dirs::executable_dir().map(|exec_dir| {
            exec_dir
                .join(APPLICATION_NAME)
                .join(format!("{}.conf", APPLICATION_NAME))
        })
    }

    /// Config file path in the current working directory (`config/personal-ledger.conf`).
    ///
    /// Errors if the current directory can't be determined.
    fn get_cwd_config_path() -> crate::Result<PathBuf> {
        let cwd = std::env::current_dir().map_err(|e| {
            crate::Error::Validation(format!(
                "Could not get current directory for config loading: {}",
                e
            ))
        })?;
        let dir = cwd
            .join("config")
            .join(format!("{}.conf", APPLICATION_NAME));
        Ok(dir)
    }

    /// Get the Sync-Server-only configuration.
    pub fn sync_server_config(&self) -> &crate::SyncServerConfig {
        &self.sync_server
    }

    /// Get the key binding configuration.
    pub fn keybindings_config(&self) -> &crate::KeyBindingConfig {
        &self.keybindings
    }

    /// Get the Personal Ledger bootstrap/startup configuration.
    pub fn personal_ledger_config(&self) -> &crate::PersonalLedgerConfig {
        &self.personal_ledger
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
                config.personal_ledger.log(),
                crate::PersonalLedgerConfig::default().log()
            );
            assert_eq!(
                config.personal_ledger.data(),
                crate::PersonalLedgerConfig::default().data()
            );
            assert_eq!(
                config.personal_ledger.file(),
                crate::PersonalLedgerConfig::default().file()
            );
            assert_eq!(
                config.sync_server.bind_address(),
                crate::SyncServerConfig::default().bind_address()
            );
        });
    }

    #[test]
    fn accessors_return_the_parsed_sections() {
        in_empty_cwd(|| {
            let config = LedgerConfig::parse(None).unwrap();
            assert_eq!(
                config.personal_ledger_config().log(),
                config.personal_ledger.log()
            );
            assert_eq!(
                config.sync_server_config().bind_address(),
                config.sync_server.bind_address()
            );
            assert_eq!(
                config.keybindings_config().super_key(),
                config.keybindings.super_key()
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
                config.sync_server.database_uri(),
                crate::SyncServerConfig::default().database_uri()
            );
        });
    }

    #[test]
    fn parse_with_explicit_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.conf");

        let config_content = r#"
        [personal-ledger]
        file = "/tmp/test-ledger.pldb"
        log = "debug"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let config = LedgerConfig::parse(Some(&config_file)).unwrap();
        assert_eq!(
            config.personal_ledger.file(),
            std::path::Path::new("/tmp/test-ledger.pldb")
        );
        assert_eq!(config.personal_ledger.log(), lib_tracing::Levels::DEBUG);
    }

    #[test]
    fn parse_for_sync_server_reads_sync_server_section_from_explicit_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("sync-server.conf");

        let config_content = r#"
        [sync-server]
        bind_address = "127.0.0.1:9000"
        database_uri = "sqlite:/tmp/sync-server-test.sqlite"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let config = LedgerConfig::parse_for_sync_server(Some(&config_file)).unwrap();
        assert_eq!(config.sync_server.bind_address(), "127.0.0.1:9000");
        assert_eq!(
            config.sync_server.database_uri(),
            "sqlite:/tmp/sync-server-test.sqlite"
        );
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
                [personal-ledger]
                log = "warn"
                "#,
            )
            .unwrap();

            // A Client (`parse`) would pick this up; the Sync Server must not.
            let config = LedgerConfig::parse_for_sync_server(None).unwrap();
            assert_eq!(
                config.personal_ledger.log(),
                crate::PersonalLedgerConfig::default().log()
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
    fn parse_with_case_insensitive_sections() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.conf");

        let config_content = r#"
        [Personal-Ledger]
        log = "warn"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let config = LedgerConfig::parse(Some(&config_file)).unwrap();
        assert_eq!(config.personal_ledger.log(), lib_tracing::Levels::WARN);
    }

    #[test]
    fn parse_with_invalid_config_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("invalid.conf");

        // Missing closing bracket -- malformed INI.
        let config_content = r#"
        [personal-ledger
        log = "debug"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let result = LedgerConfig::parse(Some(&config_file));
        assert!(result.is_err());
    }

    #[test]
    fn parse_with_invalid_log_level_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("invalid_level.conf");

        let config_content = r#"
        [personal-ledger]
        log = "invalid"
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
                [Personal-Ledger]
                log = "warn"
                "#,
            )
            .unwrap();

            let explicit_file = PathBuf::from("explicit.conf");
            fs::write(
                &explicit_file,
                r#"
                [Personal-Ledger]
                log = "debug"
                "#,
            )
            .unwrap();

            let config = LedgerConfig::parse(Some(&explicit_file)).unwrap();
            assert_eq!(config.personal_ledger.log(), lib_tracing::Levels::DEBUG);
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

    fn env_map(pairs: &[(&str, &str)]) -> Option<HashMap<String, String>> {
        Some(
            pairs
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        )
    }

    #[test]
    fn env_double_underscore_overrides_section_key() {
        let env = env_map(&[("PERSONAL_LEDGER_PERSONAL_LEDGER__LOG", "trace")]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert_eq!(config.personal_ledger.log(), lib_tracing::Levels::TRACE);
    }

    #[test]
    fn env_overrides_keybinding() {
        let env = env_map(&[("PERSONAL_LEDGER_KEYBINDINGS__SUPER_KEY", "alt")]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert_eq!(config.keybindings.super_key(), "alt");
    }

    #[test]
    fn env_preserves_single_underscores_within_a_key() {
        let env = env_map(&[(
            "PERSONAL_LEDGER_PERSONAL_LEDGER__LOG_FILE_PATH",
            "/tmp/ledger.log",
        )]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert_eq!(
            config.personal_ledger.log_file_path(),
            Some(Path::new("/tmp/ledger.log"))
        );
    }

    #[test]
    fn env_reaches_hyphenated_sync_server_section() {
        let env = env_map(&[("PERSONAL_LEDGER_SYNC_SERVER__BIND_ADDRESS", "0.0.0.0:4321")]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert_eq!(config.sync_server.bind_address(), "0.0.0.0:4321");
    }

    #[test]
    fn env_beats_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.conf");
        fs::write(
            &config_file,
            r#"
            [personal-ledger]
            log = "warn"
            "#,
        )
        .unwrap();

        let env = env_map(&[("PERSONAL_LEDGER_PERSONAL_LEDGER__LOG", "trace")]);
        let config = LedgerConfig::parse_for_sync_server_with_env(Some(&config_file), env).unwrap();
        assert_eq!(config.personal_ledger.log(), lib_tracing::Levels::TRACE);
    }

    #[test]
    fn client_parse_honours_injected_env() {
        in_empty_cwd(|| {
            let env = env_map(&[("PERSONAL_LEDGER_PERSONAL_LEDGER__LOG", "error")]);
            let config = LedgerConfig::parse_with_env(None, env).unwrap();
            assert_eq!(config.personal_ledger.log(), lib_tracing::Levels::ERROR);
        });
    }
}
