//! The overrides report (#1154): for each key of the sections Settings writes, the source that
//! outranks the user's file, recorded where precedence is applied rather than worked out again
//! after the parse. `LedgerConfig`'s parse records each file above the user's and each
//! environment variable as it layers them, and `ConfigArgs::apply_overrides` records each flag as
//! it applies it; the highest source recorded for a key wins, as it does in the parse.
//!
//! The report travels on the [`crate::ConfigWriter`], the one object the Clients hold for
//! Settings, since what it names is what outranks that writer's file.

use std::path::PathBuf;

use crate::keys::{ConfigKey, KeyId};

/// The source that outranks the file a write goes to, so the write changes nothing until it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shadow {
    /// A command-line flag, as it was given (`--locale en-GB`).
    Flag(String),
    /// An environment variable, by name as it is set.
    Env(String),
    /// A configuration file above the user's in the hierarchy.
    File(PathBuf),
}

impl Shadow {
    /// The source as Settings names it: the flag as given, the variable's name or the file's path.
    pub fn source(&self) -> String {
        match self {
            Self::Flag(flag) => flag.clone(),
            Self::Env(var) => var.clone(),
            Self::File(path) => path.display().to_string(),
        }
    }
}

/// Each overridden key with the highest source that sets it above the user's file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overrides {
    entries: Vec<(KeyId, Shadow)>,
}

impl Overrides {
    /// The source that outranks `key`'s write, or `None` when a write would apply.
    pub fn get(&self, key: ConfigKey) -> Option<&Shadow> {
        self.of(key.id())
    }

    /// As [`Self::get`], for any key of the written sections, `file` among them.
    pub(crate) fn of(&self, id: KeyId) -> Option<&Shadow> {
        self.entries
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, shadow)| shadow)
    }

    /// The report with `key` set by `shadow`, over whatever set it before: for a Client's test
    /// that starts on a key already overridden.
    pub fn with(mut self, key: ConfigKey, shadow: Shadow) -> Self {
        self.record(key.id(), shadow);
        self
    }

    /// The keys `env`'s variables set, each named as it is set, read by the parse's own
    /// environment source; a variable that sets no key of the written sections is left out.
    pub fn environment(env: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut overrides = Self::default();
        overrides.record_environment(env);
        overrides
    }

    /// Records that `shadow` sets `id`, over a lower source: the parse records its sources lowest
    /// first, so the last recorded is the one that wins.
    pub(crate) fn record(&mut self, id: KeyId, shadow: Shadow) {
        self.entries.retain(|(key, _)| *key != id);
        self.entries.push((id, shadow));
    }

    /// Records each key `env`'s variables set, as [`Self::environment`] reads them.
    pub(crate) fn record_environment(&mut self, env: impl IntoIterator<Item = (String, String)>) {
        for (name, value) in env {
            let Some(path) = crate::ledger::LedgerConfig::env_key(&name, &value) else {
                continue;
            };
            if let Some(id) = crate::keys::reported().find(|id| crate::keys::path(*id) == path) {
                self.record(id, Shadow::Env(name));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_higher_source_replaces_a_lower_one_for_its_key_only() {
        let overrides = Overrides::default()
            .with(ConfigKey::Toasts, Shadow::File("/etc/above.conf".into()))
            .with(ConfigKey::HintBar, Shadow::Env("X".into()))
            .with(ConfigKey::Toasts, Shadow::Flag("--toasts".into()));
        assert_eq!(
            overrides.get(ConfigKey::Toasts),
            Some(&Shadow::Flag("--toasts".into()))
        );
        assert_eq!(
            overrides.get(ConfigKey::HintBar),
            Some(&Shadow::Env("X".into()))
        );
        assert_eq!(overrides.get(ConfigKey::Locale), None);
    }

    #[test]
    fn a_variable_is_named_as_it_is_set_and_only_its_section_s_name_counts() {
        let overrides = Overrides::environment([
            ("PERSONAL_LEDGER_GENERAL__TOASTS".into(), "false".into()),
            (
                "PERSONAL_LEDGER_DISPLAY__ROW_DENSITY".into(),
                "compact".into(),
            ),
            ("personal_ledger_logs__log".into(), "warn".into()),
            ("PERSONAL_LEDGER_GENERAL__FILE".into(), "/tmp/a.pldb".into()),
            // The old section's name no longer sets anything.
            (
                "PERSONAL_LEDGER_PERSONAL_LEDGER__ON_START".into(),
                "none".into(),
            ),
            ("PERSONAL_LEDGER_KEYBINDINGS__BACK".into(), "esc".into()),
            ("HOME".into(), "/home/someone".into()),
        ]);
        assert_eq!(
            overrides.get(ConfigKey::Toasts),
            Some(&Shadow::Env("PERSONAL_LEDGER_GENERAL__TOASTS".into()))
        );
        assert_eq!(
            overrides.get(ConfigKey::RowDensity),
            Some(&Shadow::Env("PERSONAL_LEDGER_DISPLAY__ROW_DENSITY".into()))
        );
        assert_eq!(
            overrides.get(ConfigKey::Log),
            Some(&Shadow::Env("personal_ledger_logs__log".into()))
        );
        assert_eq!(
            overrides.of(crate::keys::FILE),
            Some(&Shadow::Env("PERSONAL_LEDGER_GENERAL__FILE".into()))
        );
        assert_eq!(overrides.get(ConfigKey::OnStart), None);
        assert_eq!(overrides.get(ConfigKey::StatusGlyphs), None);
    }
}
