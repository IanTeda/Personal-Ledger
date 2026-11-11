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
//! Each Settings page with keys has its own section (ADR-0057); a key in any other section, such
//! as the old `[Personal-Ledger]`, is ignored.
//!
//! ```ini
//! [General]
//! data = "~/Documents/Personal-Ledger"
//! file = "~/Documents/Personal-Ledger/My-Personal-Ledger.pldb"
//!
//! [Display]
//! date_style = "iso"
//!
//! [Logs]
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

/// Between the prefix and the section in a variable's name.
const ENV_PREFIX_SEPARATOR: &str = "_";

/// Between the section and the key in a variable's name, `config`'s nesting separator, so single
/// underscores inside a key, like `LOG_FILE_PATH`, are kept.
const ENV_SEPARATOR: &str = "__";

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct LedgerConfig {
    /// Settings › General's keys, from the `[General]` section (ADR-0057).
    #[serde(default)]
    pub general: crate::GeneralConfig,

    /// Settings › Display's keys, from the `[Display]` section.
    #[serde(default)]
    pub display: crate::DisplayConfig,

    /// Settings › Logs' keys, from the `[Logs]` section, which all three binaries read.
    #[serde(default)]
    pub logs: crate::LogsConfig,

    /// Keyboard shortcut configuration, read by the TUI/Desktop Clients. Populated from the
    /// `[keybindings]` section; the Sync Server merges its defaults in but never reads them.
    #[serde(alias = "Keybindings", alias = "KeyBindings")]
    pub keybindings: crate::KeyBindingConfig,

    /// Sync-Server-only settings. Populated from the `[sync-server]` section (see
    /// [`Self::normalise_ini`] for the `-`/`_` translation); never read by Clients.
    #[serde(alias = "SyncServer", alias = "Sync-Server", alias = "sync-server")]
    pub sync_server: crate::SyncServerConfig,

    /// `[theme]` Colour Role overrides, read by the TUI/Desktop Clients. Filled from the
    /// merged configuration by hand rather than by serde, so a bad entry is dropped with a
    /// warning instead of failing the whole parse (ADR-0023).
    #[serde(skip)]
    pub theme: crate::ThemeConfig,

    /// Where Settings writes Configuration, and what outranks it: the user's file under
    /// [`Self::parse`], nothing under [`Self::parse_for_sync_server`].
    ///
    /// It carries the overrides report: each key a source above the user's file sets.
    #[serde(skip)]
    pub(crate) writer: crate::ConfigWriter,
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
        Self::parse_with_detector(config_file, None, sys_locale::get_locale)
    }

    /// [`Self::parse`] with an optional injected environment (`None` reads the process
    /// environment) and operating-system Locale detector. Tests use this because mutating
    /// the process environment is `unsafe` and the real machine's Locale is not repeatable.
    pub(crate) fn parse_with_detector(
        config_file: Option<&Path>,
        env: Option<HashMap<String, String>>,
        detect_locale: impl FnOnce() -> Option<String>,
    ) -> crate::Result<LedgerConfig> {
        let mut config_builder = Self::defaults_builder()?;
        // Each source above the user's file reports the keys it sets as it is layered in, lowest
        // first, so the one that wins the parse is the one the report names.
        let mut overrides = crate::Overrides::default();

        // A unit test reads none of the machine's files, so the developer's own config never leaks
        // into what it asserts; the writer still names the user file as its target. Each file is
        // paired with whether it sits above the user's.
        let machine_files = if cfg!(test) {
            Vec::new()
        } else {
            vec![
                (Self::get_system_config_path(), false),
                (Self::get_user_config_path(), false),
                (Self::get_executable_config_path(), true),
            ]
        };
        let cwd_config = if config_file.is_none() {
            Some(Self::get_cwd_config_path()?)
        } else {
            None
        };
        let files = machine_files
            .into_iter()
            .chain([(cwd_config, true)])
            .filter_map(|(file, above)| file.map(|file| (file, above)));
        for (file, above) in files.filter(|(file, _)| file.exists()) {
            let report = above.then_some(&mut overrides);
            config_builder = Self::add_ini_source(config_builder, &file, report)?;
        }

        config_builder =
            Self::add_explicit_and_env(config_builder, config_file, env, &mut overrides)?;

        let mut ledger_config = Self::build(config_builder)?;
        ledger_config.general.resolve_locale(detect_locale)?;
        ledger_config.writer = crate::ConfigWriter::new(Self::get_user_config_path(), overrides);
        Ok(ledger_config)
    }

    /// The configuration `path` alone gives over the built-in defaults: no other file, no
    /// environment and no operating-system Locale, the configured one checked as [`Self::parse`]
    /// checks it. What [`crate::ConfigWriter::stored`] reads its target back with.
    ///
    /// # Errors
    /// As [`Self::parse`], for the one file.
    pub(crate) fn parse_file_alone(path: &Path) -> crate::Result<LedgerConfig> {
        let config_builder = Self::add_ini_source(Self::defaults_builder()?, path, None)?;
        let mut ledger_config = Self::build(config_builder)?;
        ledger_config.general.resolve_locale(|| None)?;
        Ok(ledger_config)
    }

    /// Parse configuration for the Sync Server: a reduced defaults -> explicit path ->
    /// environment variables chain (ADR-0014) -- the Client-only system/user/executable-
    /// directory/working-directory search tiers don't correspond to anything meaningful
    /// inside a Docker container.
    ///
    /// # Errors
    /// Returns an error if the explicit config file (when given) can't be read/parsed, or if
    /// the merged result doesn't deserialize into a valid `LedgerConfig`. The Locale is not
    /// resolved: the Sync Server never reads it.
    pub fn parse_for_sync_server(config_file: Option<&Path>) -> crate::Result<LedgerConfig> {
        Self::parse_for_sync_server_with_env(config_file, None)
    }

    /// [`Self::parse_for_sync_server`] with an optional injected environment.
    fn parse_for_sync_server_with_env(
        config_file: Option<&Path>,
        env: Option<HashMap<String, String>>,
    ) -> crate::Result<LedgerConfig> {
        // The Sync Server writes no file, so nothing it reads outranks one: its report is unused.
        let config_builder = Self::defaults_builder()?;
        let config_builder = Self::add_explicit_and_env(
            config_builder,
            config_file,
            env,
            &mut crate::Overrides::default(),
        )?;
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

        for (key, value) in crate::GeneralConfig::default_config_values()
            .into_iter()
            .chain(crate::LogsConfig::default_config_values())
        {
            config_builder = config_builder.set_default(key, value)?;
        }

        Ok(config_builder)
    }

    /// Add the explicit config file (if given and it exists) and environment variable
    /// overrides -- the two highest-precedence tiers, shared by both entry points. Env vars
    /// (e.g. `PERSONAL_LEDGER_LOGS__LOG=debug`) always come last/highest: the
    /// single `_` after the prefix is followed by `SECTION__KEY`, where `__` is the nesting
    /// separator (so single underscores inside a key, like `LOG_FILE_PATH`, are preserved).
    /// `env` replaces the process environment when given. Both tiers sit above the user's file, so
    /// each reports the keys it sets into `overrides`.
    // Crate-visible so the `[theme]` tests can drive the same env wiring `parse` uses.
    pub(crate) fn add_explicit_and_env(
        config_builder: ConfigBuilder<DefaultState>,
        config_file: Option<&Path>,
        env: Option<HashMap<String, String>>,
        overrides: &mut crate::Overrides,
    ) -> crate::Result<ConfigBuilder<DefaultState>> {
        let mut config_builder = config_builder;

        if let Some(explicit_config) = config_file.filter(|p| p.exists()) {
            config_builder =
                Self::add_ini_source(config_builder, explicit_config, Some(&mut *overrides))?;
        }

        match &env {
            Some(env) => overrides.record_environment(env.clone()),
            None => overrides.record_environment(std::env::vars()),
        }
        Ok(config_builder.add_source(Self::environment(env)))
    }

    /// The environment source the parse layers last: `env`, else the process environment, read
    /// through [`ENV_PREFIX`] and its separators. [`Self::env_var`] spells a name by the same
    /// constants, and [`Self::env_key`] reads one through this source.
    fn environment(env: Option<HashMap<String, String>>) -> config::Environment {
        // `prefix_separator` must be explicit: `config` otherwise reuses `separator` for it.
        config::Environment::with_prefix(ENV_PREFIX)
            .prefix_separator(ENV_PREFIX_SEPARATOR)
            .separator(ENV_SEPARATOR)
            .source(env)
    }

    /// The dotted key path the parse's environment source reads the variable `name` set to
    /// `value` as (`general.toasts`), or `None` for a variable it doesn't read.
    pub(crate) fn env_key(name: &str, value: &str) -> Option<String> {
        use config::Source;
        let one = HashMap::from([(name.to_string(), value.to_string())]);
        Self::environment(Some(one))
            .collect()
            .ok()?
            .into_keys()
            .next()
    }

    /// The variable that sets `id`, spelt as [`Self::environment`] reads it:
    /// `PERSONAL_LEDGER_<SECTION>__<KEY>`.
    pub(crate) fn env_var((section, name): crate::keys::KeyId) -> String {
        format!(
            "{ENV_PREFIX}{ENV_PREFIX_SEPARATOR}{}{ENV_SEPARATOR}{}",
            section.normalised().to_uppercase(),
            name.to_uppercase()
        )
    }

    /// Read an INI file, normalise its section headers, and add it as a source. A file above the
    /// user's records the keys it sets into `overrides`, given.
    fn add_ini_source(
        config_builder: ConfigBuilder<DefaultState>,
        path: &Path,
        overrides: Option<&mut crate::Overrides>,
    ) -> crate::Result<ConfigBuilder<DefaultState>> {
        let normalised = Self::normalise_ini(path)?;
        if let Some(overrides) = overrides {
            for id in Self::keys_set(&normalised)? {
                overrides.record(id, crate::Shadow::File(path.to_path_buf()));
            }
        }
        Ok(config_builder.add_source(config::File::from_str(&normalised, config::FileFormat::Ini)))
    }

    /// The keys of the written sections `path` alone sets, read by the parser.
    ///
    /// # Errors
    /// When the file can't be read or doesn't parse.
    pub(crate) fn keys_in_file(path: &Path) -> crate::Result<Vec<crate::keys::KeyId>> {
        Self::keys_set(&Self::normalise_ini(path)?)
    }

    /// The keys of the written sections an already-normalised file sets, by `config`'s own INI
    /// reading of it, the reading the layered parse applies.
    fn keys_set(normalised: &str) -> crate::Result<Vec<crate::keys::KeyId>> {
        let file = Config::builder()
            .add_source(config::File::from_str(normalised, config::FileFormat::Ini))
            .build()?;
        Ok(crate::keys::reported()
            .filter(|id| file.get::<config::Value>(&crate::keys::path(*id)).is_ok())
            .collect())
    }

    /// Read an INI file and normalise its section headers: lower-cased (so `[General]`/
    /// `[general]` are equivalent) and with `-` translated to `_` (so `[sync-server]`
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
        let theme = crate::ThemeConfig::from_config(&config);
        let mut ledger_config: LedgerConfig = config.try_deserialize()?;
        ledger_config.theme = theme;
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

    /// Get the `[theme]` Colour Role overrides.
    pub fn theme_config(&self) -> &crate::ThemeConfig {
        &self.theme
    }

    /// Where Settings writes Configuration back to, and what outranks it there
    /// ([`crate::ConfigWriter::overrides`]).
    pub fn writer(&self) -> &crate::ConfigWriter {
        &self.writer
    }

    /// Settings › General's keys, from `[General]`.
    pub fn general_config(&self) -> &crate::GeneralConfig {
        &self.general
    }

    /// Settings › Display's keys, from `[Display]`.
    pub fn display_config(&self) -> &crate::DisplayConfig {
        &self.display
    }

    /// Settings › Logs' keys, from `[Logs]`.
    pub fn logs_config(&self) -> &crate::LogsConfig {
        &self.logs
    }
}

/// The configuration a Client parses from a file holding `text`, under `env` alone and with no
/// system Locale, for the section modules' tests.
#[cfg(test)]
pub(crate) fn parsed_for_test(text: &str, env: &[(&str, &str)]) -> LedgerConfig {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("personal-ledger.conf");
    std::fs::write(&path, text).unwrap();
    let env = env
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect();
    LedgerConfig::parse_with_detector(Some(&path), Some(env), || None).unwrap()
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
            assert_eq!(config.logs.log(), crate::LogsConfig::default().log());
            assert_eq!(
                config.general.data(),
                crate::GeneralConfig::default().data()
            );
            assert_eq!(config.general.file(), None);
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
            assert_eq!(config.logs_config().log(), config.logs.log());
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
        [General]
        file = "/tmp/test-ledger.pldb"
        [Logs]
        log = "debug"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let config = LedgerConfig::parse(Some(&config_file)).unwrap();
        assert_eq!(
            config.general.file(),
            Some(Path::new("/tmp/test-ledger.pldb"))
        );
        assert_eq!(config.logs.log(), lib_tracing::Levels::DEBUG);
    }

    #[test]
    fn each_settings_page_reads_its_own_section_and_personal_ledger_is_ignored() {
        let config = parsed_for_test(
            "[General]\non_start = none\n[Display]\ndate_style = iso\n[Logs]\nlog = warn\n\
             [Personal-Ledger]\ntoasts = false\nrow_density = roomy\nlog = trace\n",
            &[],
        );
        assert_eq!(
            config.general_config().on_start(),
            lib_core::OnStart::NoLedger
        );
        assert_eq!(
            config.display_config().date_style(),
            Some(lib_core::DateStyle::Iso)
        );
        assert_eq!(config.logs_config().log(), lib_tracing::Levels::WARN);

        // `[Personal-Ledger]` is neither read nor migrated (ADR-0057).
        assert!(config.general_config().toasts());
        assert_eq!(
            config.display_config().row_density(),
            crate::RowDensity::Regular
        );
        let config = parsed_for_test("[Personal-Ledger]\nlog = trace\n", &[]);
        assert_eq!(
            config.logs_config().log(),
            crate::LogsConfig::default().log()
        );
    }

    #[test]
    fn a_config_key_in_a_file_is_ignored_and_only_the_flag_picks_the_file() {
        let temp_dir = TempDir::new().unwrap();
        let elsewhere = temp_dir.path().join("elsewhere.conf");
        fs::write(&elsewhere, "[Logs]\nlog = error\n").unwrap();
        let chosen = temp_dir.path().join("chosen.conf");
        fs::write(
            &chosen,
            format!(
                "[General]\nconfig = \"{}\"\n[Logs]\nlog = warn\n",
                elsewhere.display()
            ),
        )
        .unwrap();

        let config =
            LedgerConfig::parse_with_detector(Some(&chosen), env_map(&[]), || None).unwrap();
        assert_eq!(config.logs_config().log(), lib_tracing::Levels::WARN);
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
                [logs]
                log = "warn"
                "#,
            )
            .unwrap();

            // A Client (`parse`) would pick this up; the Sync Server must not.
            let config = LedgerConfig::parse_for_sync_server(None).unwrap();
            assert_eq!(config.logs.log(), crate::LogsConfig::default().log());
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
        [LOGS]
        log = "warn"
        "#;
        fs::write(&config_file, config_content).unwrap();

        let config = LedgerConfig::parse(Some(&config_file)).unwrap();
        assert_eq!(config.logs.log(), lib_tracing::Levels::WARN);
    }

    #[test]
    fn parse_with_invalid_config_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("invalid.conf");

        // Missing closing bracket -- malformed INI.
        let config_content = r#"
        [logs
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
        [logs]
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
                [Logs]
                log = "warn"
                "#,
            )
            .unwrap();

            let explicit_file = PathBuf::from("explicit.conf");
            fs::write(
                &explicit_file,
                r#"
                [Logs]
                log = "debug"
                "#,
            )
            .unwrap();

            let config = LedgerConfig::parse(Some(&explicit_file)).unwrap();
            assert_eq!(config.logs.log(), lib_tracing::Levels::DEBUG);
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
        let env = env_map(&[("PERSONAL_LEDGER_LOGS__LOG", "trace")]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert_eq!(config.logs.log(), lib_tracing::Levels::TRACE);
    }

    #[test]
    fn env_overrides_keybinding() {
        let env = env_map(&[("PERSONAL_LEDGER_KEYBINDINGS__SUPER_KEY", "alt")]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert_eq!(config.keybindings.super_key(), "alt");
    }

    #[test]
    fn toast_keybindings_default_to_ctrl_l_and_unbound() {
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env_map(&[])).unwrap();
        assert_eq!(config.keybindings.key_for("dismiss_toasts"), Some("ctrl+l"));
        assert_eq!(config.keybindings.key_for("toast_history"), None);
    }

    #[test]
    fn toast_keybindings_remap_from_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("toasts.conf");
        fs::write(
            &config_file,
            r#"
        [keybindings]
        dismiss_toasts = "ctrl+d"
        toast_history = "ctrl+t"
        "#,
        )
        .unwrap();

        let config =
            LedgerConfig::parse_for_sync_server_with_env(Some(&config_file), env_map(&[])).unwrap();
        assert_eq!(config.keybindings.key_for("dismiss_toasts"), Some("ctrl+d"));
        assert_eq!(config.keybindings.key_for("toast_history"), Some("ctrl+t"));
    }

    #[test]
    fn toast_keybindings_remap_from_env() {
        let env = env_map(&[
            ("PERSONAL_LEDGER_KEYBINDINGS__DISMISS_TOASTS", "ctrl+x"),
            ("PERSONAL_LEDGER_KEYBINDINGS__TOAST_HISTORY", "ctrl+m"),
        ]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert_eq!(config.keybindings.key_for("dismiss_toasts"), Some("ctrl+x"));
        assert_eq!(config.keybindings.key_for("toast_history"), Some("ctrl+m"));
    }

    #[test]
    fn env_preserves_single_underscores_within_a_key() {
        let env = env_map(&[("PERSONAL_LEDGER_LOGS__LOG_FILE_PATH", "/tmp/ledger.log")]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert_eq!(
            config.logs.log_file_path(),
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
            [logs]
            log = "warn"
            "#,
        )
        .unwrap();

        let env = env_map(&[("PERSONAL_LEDGER_LOGS__LOG", "trace")]);
        let config = LedgerConfig::parse_for_sync_server_with_env(Some(&config_file), env).unwrap();
        assert_eq!(config.logs.log(), lib_tracing::Levels::TRACE);
    }

    #[test]
    fn client_parse_honours_injected_env() {
        in_empty_cwd(|| {
            let env = env_map(&[("PERSONAL_LEDGER_LOGS__LOG", "error")]);
            let config = LedgerConfig::parse_with_detector(None, env, || None).unwrap();
            assert_eq!(config.logs.log(), lib_tracing::Levels::ERROR);
        });
    }

    #[test]
    fn theme_variant_file_section_and_env_override_merge() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("theme.conf");
        fs::write(
            &config_file,
            r##"
            [Theme]
            accent = "#111111"
            muted = "#444444"
            [theme.dark]
            accent = "#222222"
            "##,
        )
        .unwrap();

        let env = env_map(&[
            ("PERSONAL_LEDGER_THEME__MUTED", "555555"),
            ("PERSONAL_LEDGER_THEME__LIGHT__POSITIVE", "#00AA00"),
            ("PERSONAL_LEDGER_THEME__DARK__NEGATIVE", "not-a-colour"),
        ]);
        let config = LedgerConfig::parse_for_sync_server_with_env(Some(&config_file), env).unwrap();
        let theme = config.theme_config();
        let hex = |v: &str| lib_core::HexColor::parse(v).unwrap();
        use lib_colour_theme::{ColourRole, ColourVariant};

        assert_eq!(
            theme.overrides.get(ColourVariant::Dark, ColourRole::Accent),
            Some(&hex("#222222"))
        );
        assert_eq!(
            theme
                .overrides
                .get(ColourVariant::Light, ColourRole::Accent),
            Some(&hex("#111111"))
        );
        assert_eq!(
            theme.overrides.get(ColourVariant::Light, ColourRole::Muted),
            Some(&hex("#555555"))
        );
        assert_eq!(
            theme
                .overrides
                .get(ColourVariant::Light, ColourRole::Positive),
            Some(&hex("#00AA00"))
        );
        assert_eq!(
            theme
                .overrides
                .get(ColourVariant::Light, ColourRole::Negative),
            None
        );
        assert_eq!(theme.invalid.len(), 1);
        assert_eq!(theme.invalid[0].key, "theme.dark.negative");
    }

    #[test]
    fn terminal_colours_defaults_off_and_reads_env() {
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env_map(&[])).unwrap();
        assert!(!config.display.terminal_colours());

        let env = env_map(&[("PERSONAL_LEDGER_DISPLAY__TERMINAL_COLOURS", "true")]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert!(config.display.terminal_colours());
    }

    #[test]
    fn status_glyphs_and_the_hint_bar_default_to_unicode_and_shown_and_read_env() {
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env_map(&[])).unwrap();
        assert_eq!(config.display.status_glyphs(), crate::StatusGlyphs::Unicode);
        assert!(config.display.hint_bar());

        let env = env_map(&[
            ("PERSONAL_LEDGER_DISPLAY__STATUS_GLYPHS", "ascii"),
            ("PERSONAL_LEDGER_DISPLAY__HINT_BAR", "false"),
        ]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert_eq!(
            config.display.status_glyphs(),
            crate::StatusGlyphs::AsciiFallback
        );
        assert!(!config.display.hint_bar());
    }

    #[test]
    fn the_client_writes_to_the_user_file_and_the_sync_server_nowhere() {
        in_empty_cwd(|| {
            let config = LedgerConfig::parse_with_detector(None, env_map(&[]), || None).unwrap();
            assert_eq!(
                config.writer().target(),
                LedgerConfig::get_user_config_path().as_deref()
            );
            let env = env_map(&[("PERSONAL_LEDGER_DISPLAY__HINT_BAR", "false")]);
            let config = LedgerConfig::parse_with_detector(None, env, || None).unwrap();
            assert_eq!(
                config
                    .writer()
                    .overrides()
                    .get(crate::ConfigKey::HintBar)
                    .cloned(),
                Some(crate::Shadow::Env(
                    "PERSONAL_LEDGER_DISPLAY__HINT_BAR".into()
                ))
            );
        });
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env_map(&[])).unwrap();
        assert_eq!(config.writer().target(), None);
    }

    const LOCALE_ENV: &str = "PERSONAL_LEDGER_GENERAL__LOCALE";

    fn write_locale_file(dir: &TempDir, locale: &str) -> PathBuf {
        let config_file = dir.path().join("locale.conf");
        fs::write(&config_file, format!("[General]\nlocale = \"{locale}\"\n")).unwrap();
        config_file
    }

    fn resolve(
        config_file: Option<&Path>,
        env: Option<HashMap<String, String>>,
        system: Option<&str>,
    ) -> crate::Result<(String, crate::LocaleSource)> {
        in_empty_cwd(|| {
            let config =
                LedgerConfig::parse_with_detector(config_file, env, || system.map(String::from))?;
            let (tag, source) = config.general.resolved_locale();
            Ok((tag.to_string(), source))
        })
    }

    #[test]
    fn locale_defaults_when_nothing_else_supplies_one() {
        let (tag, source) = resolve(None, None, None).unwrap();
        assert_eq!(
            (tag.as_str(), source),
            ("en-US", crate::LocaleSource::Default)
        );
    }

    #[test]
    fn locale_uses_the_system_locale_when_unset() {
        let (tag, source) = resolve(None, None, Some("en-AU")).unwrap();
        assert_eq!(
            (tag.as_str(), source),
            ("en-AU", crate::LocaleSource::System)
        );
    }

    #[test]
    fn the_system_locale_is_known_even_when_configuration_sets_one() {
        let env = env_map(&[(LOCALE_ENV, "en-GB")]);
        in_empty_cwd(|| {
            let config =
                LedgerConfig::parse_with_detector(None, env, || Some("en-AU".into())).unwrap();
            assert_eq!(config.general.system_locale(), Some("en-AU"));
            assert_eq!(config.general.resolved_locale().0, "en-GB");
        });
    }

    #[test]
    fn the_locale_flag_shadows_the_locale_key() {
        in_empty_cwd(|| {
            let mut config =
                LedgerConfig::parse_with_detector(None, env_map(&[]), || None).unwrap();
            let args = crate::ConfigArgs {
                locale: Some("en-GB".to_string()),
                ..crate::ConfigArgs::default()
            };
            args.apply_overrides(&mut config);
            assert_eq!(
                config
                    .writer()
                    .overrides()
                    .get(crate::ConfigKey::Locale)
                    .cloned(),
                Some(crate::Shadow::Flag("--locale en-GB".to_string()))
            );
        });
    }

    #[test]
    fn the_data_flag_shadows_the_data_key() {
        in_empty_cwd(|| {
            let mut config =
                LedgerConfig::parse_with_detector(None, env_map(&[]), || None).unwrap();
            let args = crate::ConfigArgs {
                data: Some(PathBuf::from("/tmp/ledgers")),
                ..crate::ConfigArgs::default()
            };
            args.apply_overrides(&mut config);
            assert_eq!(
                config
                    .writer()
                    .overrides()
                    .get(crate::ConfigKey::Data)
                    .cloned(),
                Some(crate::Shadow::Flag("--data /tmp/ledgers".to_string()))
            );
        });
    }

    /// The overrides report `config` carries for `key`.
    fn reported(config: &LedgerConfig, key: crate::ConfigKey) -> Option<crate::Shadow> {
        config.writer().overrides().get(key).cloned()
    }

    #[test]
    fn the_working_directory_s_file_reports_the_keys_it_sets_in_their_own_sections() {
        in_empty_cwd(|| {
            let cwd_file = LedgerConfig::get_cwd_config_path().unwrap();
            fs::create_dir_all(cwd_file.parent().unwrap()).unwrap();
            fs::write(
                &cwd_file,
                "[display]\nhint_bar = false\nstatus_glyphs = ascii\n\
                 [Personal-Ledger]\ntoasts = false\n[General]\n# locale = en-GB\n",
            )
            .unwrap();
            let config = LedgerConfig::parse_with_detector(None, env_map(&[]), || None).unwrap();
            // Recorded as the parse layered the file: gone now, it is still what this run read.
            fs::remove_file(&cwd_file).unwrap();

            let by_file = Some(crate::Shadow::File(cwd_file));
            assert_eq!(reported(&config, crate::ConfigKey::HintBar), by_file);
            assert_eq!(reported(&config, crate::ConfigKey::StatusGlyphs), by_file);
            assert_eq!(reported(&config, crate::ConfigKey::Toasts), None);
            assert_eq!(reported(&config, crate::ConfigKey::Locale), None);
        });
    }

    #[test]
    fn the_explicit_file_reports_the_keys_it_sets_and_file_among_them() {
        let temp_dir = TempDir::new().unwrap();
        let explicit = temp_dir.path().join("explicit.conf");
        fs::write(
            &explicit,
            "[General]\nlocale = en-GB\nfile = /tmp/a.pldb\n[Logs]\nlog = warn\n",
        )
        .unwrap();
        let config =
            LedgerConfig::parse_with_detector(Some(&explicit), env_map(&[]), || None).unwrap();
        let by_file = Some(crate::Shadow::File(explicit.clone()));
        assert_eq!(reported(&config, crate::ConfigKey::Locale), by_file);
        assert_eq!(reported(&config, crate::ConfigKey::Log), by_file);
        assert_eq!(
            config.writer().overrides().of(crate::keys::FILE).cloned(),
            by_file
        );
        assert_eq!(reported(&config, crate::ConfigKey::HintBar), None);
    }

    #[test]
    fn a_variable_reports_its_key_over_a_file_and_a_flag_over_both() {
        let temp_dir = TempDir::new().unwrap();
        let explicit = temp_dir.path().join("explicit.conf");
        fs::write(
            &explicit,
            "[General]\nlocale = en-GB\ndata = /tmp/from-file\ntoasts = false\n",
        )
        .unwrap();
        let env = env_map(&[
            (LOCALE_ENV, "en-AU"),
            ("PERSONAL_LEDGER_GENERAL__DATA", "/tmp/from-env"),
            ("PERSONAL_LEDGER_DISPLAY__ROW_DENSITY", "roomy"),
        ]);
        let mut config = LedgerConfig::parse_with_detector(Some(&explicit), env, || None).unwrap();
        crate::ConfigArgs {
            locale: Some("en-US".to_string()),
            ..crate::ConfigArgs::default()
        }
        .apply_overrides(&mut config);

        assert_eq!(
            reported(&config, crate::ConfigKey::Locale),
            Some(crate::Shadow::Flag("--locale en-US".into()))
        );
        assert_eq!(
            reported(&config, crate::ConfigKey::Data),
            Some(crate::Shadow::Env("PERSONAL_LEDGER_GENERAL__DATA".into()))
        );
        assert_eq!(
            reported(&config, crate::ConfigKey::RowDensity),
            Some(crate::Shadow::Env(
                "PERSONAL_LEDGER_DISPLAY__ROW_DENSITY".into()
            ))
        );
        assert_eq!(
            reported(&config, crate::ConfigKey::Toasts),
            Some(crate::Shadow::File(explicit))
        );
        assert_eq!(reported(&config, crate::ConfigKey::OnStart), None);
    }

    #[test]
    fn locale_file_beats_system() {
        let temp_dir = TempDir::new().unwrap();
        let file = write_locale_file(&temp_dir, "en-GB");
        let (tag, source) = resolve(Some(&file), None, Some("en-AU")).unwrap();
        assert_eq!(
            (tag.as_str(), source),
            ("en-GB", crate::LocaleSource::Config)
        );
    }

    #[test]
    fn locale_env_beats_file() {
        let temp_dir = TempDir::new().unwrap();
        let file = write_locale_file(&temp_dir, "en-GB");
        let env = env_map(&[(LOCALE_ENV, "en-AU")]);
        let (tag, source) = resolve(Some(&file), env, Some("en-US")).unwrap();
        assert_eq!(
            (tag.as_str(), source),
            ("en-AU", crate::LocaleSource::Config)
        );
    }

    #[test]
    fn locale_cli_beats_env() {
        let env = env_map(&[(LOCALE_ENV, "en-AU")]);
        in_empty_cwd(|| {
            let mut config = LedgerConfig::parse_with_detector(None, env, || None).unwrap();
            let args = crate::ConfigArgs {
                locale: Some("en-GB".to_string()),
                ..crate::ConfigArgs::default()
            };
            args.apply_overrides(&mut config);
            assert_eq!(
                config.general.resolved_locale(),
                ("en-GB", crate::LocaleSource::Config)
            );
        });
    }

    #[test]
    fn locale_is_stored_in_canonical_casing() {
        let env = env_map(&[(LOCALE_ENV, "en-gb")]);
        let (tag, _) = resolve(None, env, None).unwrap();
        assert_eq!(tag, "en-GB");
    }

    #[test]
    fn malformed_locale_is_a_startup_error() {
        let env = env_map(&[(LOCALE_ENV, "not a locale")]);
        let error = resolve(None, env, None).unwrap_err();
        assert!(
            matches!(error, crate::Error::InvalidLocale { ref value } if value == "not a locale")
        );
    }

    #[test]
    fn unsupported_but_well_formed_locale_passes_through() {
        let env = env_map(&[(LOCALE_ENV, "fr-FR")]);
        let (tag, source) = resolve(None, env, None).unwrap();
        assert_eq!(
            (tag.as_str(), source),
            ("fr-FR", crate::LocaleSource::Config)
        );
    }

    #[test]
    fn unparseable_system_locale_is_ignored() {
        for value in ["C", "POSIX", "posix", "C.UTF-8", "", "not a locale"] {
            let (tag, source) = resolve(None, None, Some(value)).unwrap();
            assert_eq!(
                (tag.as_str(), source),
                ("en-US", crate::LocaleSource::Default),
                "system locale {value:?}"
            );
        }
    }

    #[test]
    fn sync_server_ignores_the_locale_field() {
        let env = env_map(&[(LOCALE_ENV, "not a locale")]);
        let config = LedgerConfig::parse_for_sync_server_with_env(None, env).unwrap();
        assert_eq!(
            config.general.resolved_locale(),
            ("not a locale", crate::LocaleSource::Config)
        );
    }

    /// The example configuration shipped at the repository root. It is not merely
    /// documentation: `get_cwd_config_path` puts `config/personal-ledger.conf` in the
    /// working-directory precedence tier, so this exact file is loaded whenever a binary is
    /// run from the repository root. The two tests below keep it honest against the schema.
    fn shipped_conf_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../config/personal-ledger.conf")
    }

    /// Guards against a section going stale: the file once shipped `[Database]` and
    /// `[Tracing]` sections, long after `LedgerConfig` stopped reading either. Nothing
    /// complained, because `serde` ignores unknown fields here -- every key under them was
    /// silently discarded in favour of the built-in defaults. Parsing alone therefore proves
    /// nothing; the section headers have to be checked against the ones that are read.
    #[test]
    fn shipped_conf_has_no_section_lib_config_ignores() {
        let raw = fs::read_to_string(shipped_conf_path()).unwrap();

        // The same normalisation `normalise_ini` applies before the file reaches `serde`, so
        // `[Sync-Server]`, `[sync-server]` and `[sync_server]` all compare equal here.
        let sections: Vec<String> = raw
            .lines()
            .map(str::trim)
            .filter_map(|line| line.strip_prefix('[')?.strip_suffix(']'))
            .map(|name| name.to_lowercase().replace('-', "_"))
            .collect();

        assert!(
            !sections.is_empty(),
            "no sections found -- the file or this parser is wrong"
        );

        for section in &sections {
            assert!(
                matches!(
                    section.as_str(),
                    "general"
                        | "display"
                        | "logs"
                        | "keybindings"
                        | "sync_server"
                        | "theme"
                        | "theme.light"
                        | "theme.dark"
                ),
                "[{section}] is not a section LedgerConfig reads, so every key under it is \
                 silently ignored -- remove it or wire it up"
            );
        }
    }

    /// The other half of the guard: that the values in the file actually land on the parsed
    /// config, so renaming a section (or breaking `normalise_ini`) fails here rather than
    /// quietly falling back to defaults. Both assertions deliberately use values the file
    /// sets *differently* from the built-in defaults -- `bind_address` and `database_uri`
    /// are not asserted, because the file repeats their defaults verbatim and asserting them
    /// would pass even if the whole section were dropped.
    #[test]
    fn shipped_conf_values_reach_the_parsed_config() {
        in_empty_cwd(|| {
            let config = LedgerConfig::parse(Some(&shipped_conf_path())).unwrap();

            // The file sets `log = "debug"`; the built-in default is `INFO`.
            assert_eq!(
                config.logs.log(),
                lib_tracing::Levels::DEBUG,
                "[Logs] log did not reach the config"
            );

            // The file's `data` key is a literal tilde path -- `config` does no shell
            // expansion -- whereas the default is an expanded absolute path.
            assert_eq!(
                config.general.data(),
                Path::new("~/Documents/Personal-Ledger"),
                "[General] data did not reach the config"
            );

            // `file` overrides On start, so the shipped file leaves it commented out.
            assert_eq!(config.general.file(), None);

            // `super_key` and the ten command bindings match the built-in defaults, so they
            // cannot discriminate -- but the file must at least not define an invalid
            // modifier or bind two commands to one key.
            config.keybindings.validate().unwrap();
        });
    }

    /// The `[section]` a line opens, normalised, whether live or commented as `# [section]`.
    fn section_of(line: &str) -> Option<String> {
        let line = line.trim();
        let line = line.strip_prefix('#').map_or(line, str::trim);
        let name = line.strip_prefix('[')?.strip_suffix(']')?;
        Some(name.to_lowercase().replace('-', "_"))
    }

    /// The key a line sets, whether live or commented as `# key = ...`.
    fn key_of(line: &str) -> Option<&str> {
        let line = line.trim();
        let line = line.strip_prefix('#').map_or(line, str::trim);
        let (key, _) = line.split_once(" = ")?;
        (!key.is_empty() && key.chars().all(|c| c.is_ascii_lowercase() || c == '_')).then_some(key)
    }

    /// Each section of the shipped file with the table's keys it holds, in file order.
    /// `[Keybindings]` keeps only `super_key`: its bindings are an open-ended map.
    fn shipped_keys(raw: &str) -> Vec<(String, Vec<String>)> {
        let mut found: Vec<(String, Vec<String>)> = Vec::new();
        for line in raw.lines() {
            if let Some(section) = section_of(line) {
                found.push((section, Vec::new()));
            } else if let (Some(key), Some((section, keys))) = (key_of(line), found.last_mut())
                && (section != "keybindings" || key == "super_key")
            {
                keys.push(key.to_string());
            }
        }
        found
    }

    fn sorted(keys: impl IntoIterator<Item = impl Into<String>>) -> Vec<String> {
        let mut keys: Vec<String> = keys.into_iter().map(Into::into).collect();
        keys.sort();
        keys
    }

    #[test]
    fn key_table_matches_the_structs() {
        let value = serde_json::to_value(LedgerConfig::default()).unwrap();
        for (section, keys) in crate::keys::KEYS {
            let field = match *section {
                "theme" | "theme.light" | "theme.dark" => continue,
                other => other,
            };
            let object = value[field].as_object().unwrap_or_else(|| {
                panic!("[{section}] is in the key table but LedgerConfig has no such section")
            });
            let mut parsed: Vec<&str> = object.keys().map(String::as_str).collect();
            if *section == "keybindings" {
                parsed.retain(|key| *key == "super_key");
            }
            assert_eq!(
                sorted(parsed),
                sorted(keys.iter().copied()),
                "[{section}]: the key table and its struct disagree"
            );
        }
        // Every serialised section has a table row.
        for field in value.as_object().unwrap().keys() {
            assert!(
                crate::keys::KEYS
                    .iter()
                    .any(|(section, _)| section == field),
                "LedgerConfig section `{field}` has no row in the key table"
            );
        }
    }

    #[test]
    fn key_table_theme_roles_are_the_colour_roles() {
        let roles = lib_colour_theme::ColourRole::ALL.map(lib_colour_theme::ColourRole::key);
        for (section, keys) in crate::keys::KEYS {
            if section.starts_with("theme") {
                assert_eq!(sorted(keys.iter().copied()), sorted(roles));
            }
        }
    }

    #[test]
    fn shipped_conf_holds_exactly_the_tables_keys() {
        let raw = fs::read_to_string(shipped_conf_path()).unwrap();
        let shipped = shipped_keys(&raw);
        for (section, keys) in crate::keys::KEYS {
            let held: Vec<String> = shipped
                .iter()
                .filter(|(name, _)| name == section)
                .flat_map(|(_, keys)| keys.clone())
                .collect();
            assert_eq!(
                sorted(held),
                sorted(keys.iter().copied()),
                "[{section}]: the shipped file and the key table disagree"
            );
        }
        for (name, _) in &shipped {
            assert!(
                crate::keys::KEYS.iter().any(|(section, _)| section == name),
                "[{name}] is in the shipped file but not the key table"
            );
        }
    }

    #[test]
    fn shipped_conf_examples_all_parse() {
        let raw = fs::read_to_string(shipped_conf_path()).unwrap();
        let uncommented: Vec<String> = raw
            .lines()
            .map(|line| {
                if section_of(line).is_some() || key_of(line).is_some() {
                    line.trim().trim_start_matches('#').trim().to_string()
                } else {
                    line.to_string()
                }
            })
            .collect();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("personal-ledger.conf");
        fs::write(&path, uncommented.join("\n")).unwrap();

        in_empty_cwd(|| {
            let config = LedgerConfig::parse(Some(&path)).unwrap();
            assert!(
                config.theme.invalid.is_empty(),
                "a Theme example is invalid: {:?}",
                config.theme.invalid
            );
            config.keybindings.validate().unwrap();
            assert_eq!(config.general.resolved_locale().0, "en-AU");
        });
    }
}
