//! # Shared CLI Arguments
//!
//! A reusable `clap::Args` group for locating an explicit configuration file and overriding
//! the keys of `[General]` and `[Logs]`, meant to be `#[command(flatten)]`-ed into each binary's
//! own `clap::Parser` struct (rather than each of `bin-tui`/`bin-desktop`/`bin-sync-server`
//! redefining the same flags). Kept as `Args`, not `Parser`, so each binary's own top-level
//! struct still picks up its own name/version from its own crate at compile time.

/// Shared configuration-related CLI arguments: where's the config file, plus overrides for
/// `[General]`'s `data`/`file`/`locale` and `[Logs]`' `log` (see
/// `docs/settings.md`) applied via [`Self::apply_overrides`].
#[derive(Debug, Clone, Default, clap::Args)]
pub struct ConfigArgs {
    /// Path to an explicit configuration file. Takes precedence over the file-location
    /// search, but is still overridden by environment variables -- see
    /// LedgerConfig::parse's documented precedence order.
    #[arg(short = 'c', long = "config", value_name = "PATH")]
    pub path: Option<std::path::PathBuf>,

    /// Override `[General]`'s data directory.
    #[arg(short = 'd', long = "data", value_name = "DIR")]
    pub data: Option<std::path::PathBuf>,

    /// Override `[General]`'s Ledger file.
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: Option<std::path::PathBuf>,

    /// Override `[Logs]`' logging level.
    #[arg(short = 'l', long = "log", value_name = "LEVEL")]
    pub log: Option<lib_tracing::Levels>,

    /// Override `[General]`'s Locale (a BCP-47 tag such as `en-GB`).
    /// Long form only: `-l` is `--log`, and `-L` would differ from it only by case.
    #[arg(long = "locale", value_name = "TAG", value_parser = crate::general::parse_locale_tag)]
    pub locale: Option<String>,
}

impl ConfigArgs {
    /// Apply this invocation's `--data`/`--file`/`--log`/`--locale` overrides onto an already-parsed
    /// [`crate::Config`]. These sit above even environment variables in
    /// `docs/settings.md`'s precedence hierarchy, so they're applied as a final step
    /// after `LedgerConfig::parse`/`parse_for_sync_server` rather than through the layered
    /// `config`-crate builder -- unset (`None`) fields leave the parsed value untouched. Each flag
    /// applied is recorded in the writer's overrides report as it is applied, outranking the rest.
    pub fn apply_overrides(&self, config: &mut crate::Config) {
        for flag in self.given() {
            let (key, typed) = flag.apply(config);
            config
                .writer
                .overrides
                .record(key, crate::Shadow::Flag(typed));
        }
    }

    /// The flags this invocation gave that override a key, in the order they apply.
    fn given(&self) -> impl Iterator<Item = Given<'_>> {
        [
            self.data.as_deref().map(Given::Data),
            self.file.as_deref().map(Given::File),
            self.log.map(Given::Log),
            self.locale.as_deref().map(Given::Locale),
        ]
        .into_iter()
        .flatten()
    }
}

/// A flag given, with the value it overrides its key with.
enum Given<'a> {
    Data(&'a std::path::Path),
    File(&'a std::path::Path),
    Log(lib_tracing::Levels),
    Locale(&'a str),
}

impl Given<'_> {
    /// Sets the flag's key on `config`, answering the key it set and the flag as it was typed, for
    /// the overrides report: an arm can't set a key without naming it.
    fn apply(self, config: &mut crate::Config) -> (crate::keys::KeyId, String) {
        use crate::ConfigKey;
        match self {
            Self::Data(dir) => {
                config.general.data = dir.to_path_buf();
                (
                    crate::keys::id(ConfigKey::Data),
                    format!("--data {}", dir.display()),
                )
            }
            Self::File(file) => {
                config.general.file = Some(file.to_path_buf());
                (crate::keys::FILE, format!("--file {}", file.display()))
            }
            Self::Log(level) => {
                config.logs.log = level;
                (crate::keys::id(ConfigKey::Log), format!("--log {level}"))
            }
            Self::Locale(tag) => {
                config.general.locale = Some(tag.to_string());
                (
                    crate::keys::id(ConfigKey::Locale),
                    format!("--locale {tag}"),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_overrides_leaves_unset_fields_untouched() {
        let mut config = crate::Config::default();
        let default_data = config.general.data.clone();
        let default_file = config.general.file.clone();
        let default_log = config.logs.log;

        ConfigArgs::default().apply_overrides(&mut config);

        assert_eq!(config.general.data, default_data);
        assert_eq!(config.general.file, default_file);
        assert_eq!(config.logs.log, default_log);
    }

    #[test]
    fn apply_overrides_sets_every_provided_field() {
        let mut config = crate::Config::default();
        let args = ConfigArgs {
            path: None,
            data: Some(std::path::PathBuf::from("/tmp/data")),
            file: Some(std::path::PathBuf::from("/tmp/data/ledger.pldb")),
            log: Some(lib_tracing::Levels::TRACE),
            locale: Some("en-GB".to_string()),
        };

        args.apply_overrides(&mut config);

        assert_eq!(config.general.data, std::path::PathBuf::from("/tmp/data"));
        assert_eq!(
            config.general.file(),
            Some(std::path::Path::new("/tmp/data/ledger.pldb"))
        );
        assert_eq!(config.logs.log, lib_tracing::Levels::TRACE);
        assert_eq!(config.general.locale(), Some("en-GB"));
    }

    #[derive(clap::Parser)]
    struct TestCli {
        #[command(flatten)]
        config: ConfigArgs,
    }

    #[test]
    fn locale_flag_is_canonicalised_at_parse_time() {
        use clap::Parser;
        let cli = TestCli::try_parse_from(["test", "--locale", "en-gb"]).unwrap();
        assert_eq!(cli.config.locale.as_deref(), Some("en-GB"));
    }

    #[test]
    fn malformed_locale_flag_is_rejected_at_parse_time() {
        use clap::Parser;
        assert!(TestCli::try_parse_from(["test", "--locale", "not a tag"]).is_err());
    }

    #[test]
    fn locale_flag_has_no_short_form() {
        use clap::Parser;
        assert!(TestCli::try_parse_from(["test", "-L", "en-GB"]).is_err());
    }

    /// A value for each flag that differs from its key's default, by long name: a flag added
    /// without one fails [`every_flag_that_overrides_a_key_reports_it`] until it has one.
    const SAMPLES: [(&str, &str); 4] = [
        ("data", "/tmp/flagged-data"),
        ("file", "/tmp/flagged.pldb"),
        ("log", "trace"),
        ("locale", "en-GB"),
    ];

    /// Each key a flag can set, with its value in `config`: every key Settings writes, and `file`.
    fn keyed_values(config: &crate::Config) -> Vec<(crate::keys::KeyId, String)> {
        crate::ConfigKey::ALL
            .into_iter()
            .map(|key| {
                let value = crate::keys::read_value(key, config);
                (crate::keys::id(key), format!("{value:?}"))
            })
            .chain([(
                crate::keys::FILE,
                format!("{:?}", config.general_config().file()),
            )])
            .collect()
    }

    /// Every flag `ConfigArgs` declares, found through clap rather than listed, so a new one is
    /// checked without being named here: given alone, each key it changes is reported as set by it,
    /// and it reports at least one.
    #[test]
    fn every_flag_that_overrides_a_key_reports_it() {
        use clap::{Args, Parser};
        let command = ConfigArgs::augment_args(clap::Command::new("test"));
        for arg in command.get_arguments() {
            let id = arg.get_id().as_str();
            if id == "path" {
                // `--config` names a file to parse, not a key to override.
                continue;
            }
            let long = arg
                .get_long()
                .unwrap_or_else(|| panic!("`{id}` has no long form to report itself by"));
            let (_, sample) = SAMPLES
                .iter()
                .find(|(name, _)| *name == long)
                .unwrap_or_else(|| panic!("--{long} needs a value in SAMPLES"));
            let cli = TestCli::try_parse_from(["test", &format!("--{long}"), sample])
                .unwrap_or_else(|error| panic!("--{long} {sample}: {error}"));

            let mut config = crate::Config::default();
            let before = keyed_values(&config);
            cli.config.apply_overrides(&mut config);
            let after = keyed_values(&config);

            let by_flag = |id: crate::keys::KeyId| {
                matches!(
                    config.writer().overrides().of(id),
                    Some(crate::Shadow::Flag(typed)) if typed.starts_with(&format!("--{long} "))
                )
            };
            let changed: Vec<_> = before
                .iter()
                .zip(&after)
                .filter(|(was, is)| was != is)
                .map(|((id, _), _)| *id)
                .collect();
            assert!(!changed.is_empty(), "--{long} {sample} changed no key");
            for id in &changed {
                assert!(by_flag(*id), "--{long} changes {id:?} without reporting it");
            }
            assert!(
                crate::keys::reported().any(by_flag),
                "--{long} reports nothing"
            );
        }
    }

    #[test]
    fn each_flag_is_reported_as_it_was_typed() {
        let mut config = crate::Config::default();
        ConfigArgs {
            path: None,
            data: Some("/tmp/ledgers".into()),
            file: Some("/tmp/ledgers/a.pldb".into()),
            log: Some(lib_tracing::Levels::DEBUG),
            locale: Some("en-GB".into()),
        }
        .apply_overrides(&mut config);
        let overrides = config.writer().overrides();
        let flag = |text: &str| Some(crate::Shadow::Flag(text.to_string()));
        assert_eq!(
            overrides.get(crate::ConfigKey::Data).cloned(),
            flag("--data /tmp/ledgers")
        );
        assert_eq!(
            overrides.of(crate::keys::FILE).cloned(),
            flag("--file /tmp/ledgers/a.pldb")
        );
        assert_eq!(
            overrides.get(crate::ConfigKey::Log).cloned(),
            flag("--log debug")
        );
        assert_eq!(
            overrides.get(crate::ConfigKey::Locale).cloned(),
            flag("--locale en-GB")
        );
        assert_eq!(overrides.get(crate::ConfigKey::Toasts), None);
    }
}
