//! What the TUI keeps on this device between runs, in `tui-state.json`: its Client-scoped
//! Preferences and the Ledger open when it last quit (ADR-0051). The Desktop keeps the same in its
//! own `desktop-state.json`; each Client is its own device's choice, so they share no file.
//!
//! A small JSON file in the state directory, not `lib_config`'s INI: that is Configuration a
//! person edits by hand, and this is state the TUI writes for itself.

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

/// Directory name under the platform's state directory, as the Desktop names its own.
const APPLICATION_NAME: &str = "personal-ledger";

const STATE_FILE_NAME: &str = "tui-state.json";

/// What survives a restart. `Default` is "nothing kept yet", and every field defaults on its own,
/// so a file from an older release still loads.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TuiState {
    /// Toasts, the Colour Theme and Appearance, the date style and On start.
    pub preferences: lib_core::ClientPreferences,
    /// The Ledger file open when the TUI last quit, which On start's Reopen last Ledger opens.
    /// `None` when none was, so `:close` then quit forgets it.
    pub last_ledger: Option<PathBuf>,
}

/// Where the state file lives: `dirs::state_dir()` (Linux's `$XDG_STATE_HOME`), else
/// `dirs::data_local_dir()` on platforms with no separate state directory, as the Desktop's is.
pub fn state_file_path() -> Option<PathBuf> {
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .map(|dir| dir.join(APPLICATION_NAME).join(STATE_FILE_NAME))
}

/// Loads the state from `path`. A missing, unreadable or unparseable file loads as nothing kept:
/// there is nothing a person could do to fix it, and refusing to start would be worse.
pub fn load_from(path: &Path) -> TuiState {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

/// Loads from the platform's state file, or nothing kept when there is no state directory.
pub fn load() -> TuiState {
    state_file_path()
        .map(|path| load_from(&path))
        .unwrap_or_default()
}

/// Saves `state` to `path`, creating its directory if needed.
pub fn save_to(path: &Path, state: &TuiState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(state)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    std::fs::write(path, contents)
}

/// Saves to the platform's state file; with no state directory there is nowhere to keep it, which
/// isn't worth failing the quit over.
pub fn save(state: &TuiState) -> io::Result<()> {
    match state_file_path() {
        Some(path) => save_to(&path, state),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_or_corrupt_file_loads_as_nothing_kept() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join(STATE_FILE_NAME);
        assert_eq!(load_from(&path), TuiState::default());
        std::fs::write(&path, "not json").expect("write");
        assert_eq!(load_from(&path), TuiState::default());
    }

    #[test]
    fn saved_state_loads_back_through_a_new_directory() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("nested").join(STATE_FILE_NAME);
        let state = TuiState {
            preferences: lib_core::ClientPreferences {
                toasts_on: false,
                colour_theme: Some("gruvbox".into()),
                colour_appearance: Some("dark".into()),
                date_style: Some(lib_core::DateStyle::Long),
                on_start: lib_core::OnStart::NoLedger,
            },
            last_ledger: Some(PathBuf::from("/ledgers/household.pldb")),
        };
        save_to(&path, &state).expect("save creates the directory");
        assert_eq!(load_from(&path), state);
    }
}
