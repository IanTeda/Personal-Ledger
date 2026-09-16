//! # Shared CLI Arguments
//!
//! A reusable `clap::Args` group for locating an explicit configuration file and overriding
//! the `[Personal-Ledger]` section, meant to be `#[command(flatten)]`-ed into each binary's
//! own `clap::Parser` struct (rather than each of `bin-tui`/`bin-desktop`/`bin-sync-server`
//! redefining the same flags). Kept as `Args`, not `Parser`, so each binary's own top-level
//! struct still picks up its own name/version from its own crate at compile time.

/// Shared configuration-related CLI arguments: where's the config file, plus overrides for
/// the `[Personal-Ledger]` section's `data`/`file`/`log` settings (see
/// `docs/configuration.md`).
#[derive(Debug, Clone, Default, clap::Args)]
pub struct ConfigArgs {
    /// Path to an explicit configuration file. Takes precedence over the file-location
    /// search, but is still overridden by environment variables -- see
    /// LedgerConfig::parse's documented precedence order.
    #[arg(short = 'c', long = "config", value_name = "PATH")]
    pub path: Option<std::path::PathBuf>,

    /// Override the `[Personal-Ledger]` section's data directory. Highest precedence --
    /// applied via [`Self::apply_overrides`] after the normal layered parse.
    #[arg(short = 'd', long = "data", value_name = "DIR")]
    pub data: Option<std::path::PathBuf>,

    /// Override the `[Personal-Ledger]` section's database file. Highest precedence --
    /// applied via [`Self::apply_overrides`] after the normal layered parse.
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: Option<std::path::PathBuf>,

    /// Override the `[Personal-Ledger]` section's logging level. Highest precedence --
    /// applied via [`Self::apply_overrides`] after the normal layered parse.
    #[arg(short = 'l', long = "log", value_name = "LEVEL")]
    pub log: Option<lib_tracing::Levels>,
}

impl ConfigArgs {
    /// Apply this invocation's `--data`/`--file`/`--log` overrides directly onto an
    /// already-parsed [`crate::Config`]'s `[Personal-Ledger]` section.
    ///
    /// These sit above even environment variables in `docs/configuration.md`'s precedence
    /// hierarchy, so they're applied as a final step after `LedgerConfig::parse`/
    /// `parse_for_sync_server` rather than through the layered `config`-crate builder --
    /// unset (`None`) fields leave the already-parsed value untouched.
    pub fn apply_overrides(&self, config: &mut crate::Config) {
        if let Some(data) = &self.data {
            config.personal_ledger.data = data.clone();
        }
        if let Some(file) = &self.file {
            config.personal_ledger.file = file.clone();
        }
        if let Some(log) = self.log {
            config.personal_ledger.log = log;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_overrides_leaves_unset_fields_untouched() {
        let mut config = crate::Config::default();
        let default_data = config.personal_ledger.data.clone();
        let default_file = config.personal_ledger.file.clone();
        let default_log = config.personal_ledger.log;

        ConfigArgs::default().apply_overrides(&mut config);

        assert_eq!(config.personal_ledger.data, default_data);
        assert_eq!(config.personal_ledger.file, default_file);
        assert_eq!(config.personal_ledger.log, default_log);
    }

    #[test]
    fn apply_overrides_sets_every_provided_field() {
        let mut config = crate::Config::default();
        let args = ConfigArgs {
            path: None,
            data: Some(std::path::PathBuf::from("/tmp/data")),
            file: Some(std::path::PathBuf::from("/tmp/data/ledger.pldb")),
            log: Some(lib_tracing::Levels::TRACE),
        };

        args.apply_overrides(&mut config);

        assert_eq!(
            config.personal_ledger.data,
            std::path::PathBuf::from("/tmp/data")
        );
        assert_eq!(
            config.personal_ledger.file,
            std::path::PathBuf::from("/tmp/data/ledger.pldb")
        );
        assert_eq!(config.personal_ledger.log, lib_tracing::Levels::TRACE);
    }
}
