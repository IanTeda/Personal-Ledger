//! # On start
//!
//! Which Ledger file, if any, a Client opens as it starts (#872, ADR-0051): the On start
//! Preference, and the one resolver both Clients ask at start.

use std::path::{Path, PathBuf};

/// The On start Preference: Client-scoped, so each device starts its own way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnStart {
    /// Reopen the Ledger open when the Client last quit, or none if none was.
    #[default]
    ReopenLastLedger,
    /// Start with no Ledger open, whatever was open at the last quit.
    NoLedger,
}

/// The Ledger file a Client opens as it starts, or `None` to start with no Ledger open.
///
/// An explicit `file` (Configuration: `--file`, the environment or a config file) is the
/// person asking for that file this run, so it outranks the Preference.
pub fn ledger_to_open(
    on_start: OnStart,
    file: Option<&Path>,
    remembered: Option<&Path>,
) -> Option<PathBuf> {
    match (file, on_start) {
        (Some(file), _) => Some(file.to_path_buf()),
        (None, OnStart::ReopenLastLedger) => remembered.map(Path::to_path_buf),
        (None, OnStart::NoLedger) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_file_wins_over_the_preference_and_the_remembered_ledger() {
        let file = Path::new("/ledgers/explicit.pldb");
        let remembered = Path::new("/ledgers/last.pldb");
        for on_start in [OnStart::ReopenLastLedger, OnStart::NoLedger] {
            assert_eq!(
                ledger_to_open(on_start, Some(file), Some(remembered)),
                Some(PathBuf::from("/ledgers/explicit.pldb"))
            );
        }
    }

    #[test]
    fn reopen_last_ledger_opens_the_remembered_ledger() {
        assert_eq!(
            ledger_to_open(
                OnStart::ReopenLastLedger,
                None,
                Some(Path::new("/ledgers/last.pldb"))
            ),
            Some(PathBuf::from("/ledgers/last.pldb"))
        );
    }

    #[test]
    fn reopen_last_ledger_with_none_remembered_starts_with_no_ledger_open() {
        assert_eq!(ledger_to_open(OnStart::ReopenLastLedger, None, None), None);
    }

    #[test]
    fn start_with_no_ledger_open_ignores_the_remembered_ledger() {
        assert_eq!(
            ledger_to_open(
                OnStart::NoLedger,
                None,
                Some(Path::new("/ledgers/last.pldb"))
            ),
            None
        );
    }

    #[test]
    fn reopen_last_ledger_is_the_default() {
        assert_eq!(OnStart::default(), OnStart::ReopenLastLedger);
    }
}
