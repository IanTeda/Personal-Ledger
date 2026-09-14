//! Restart persistence for the shell's `noun`, `primary_rail`, and window geometry --
//! everything else in `NavState` (and the window's own transient state) resets on every
//! launch, per `docs/ux/desktop/README.md`'s "Persistence" note. A small dedicated JSON file,
//! not `lib_config`'s layered INI config: that config is for static, user-editable settings,
//! this is transient UI state a user never hand-edits.

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::nav::{Noun, RailMode};

/// Directory name under the platform's state directory, mirroring `lib_config`'s own
/// `APPLICATION_NAME` convention (`crates/libs/lib-config/src/ledger.rs`).
const APPLICATION_NAME: &str = "personal-ledger";

const STATE_FILE_NAME: &str = "desktop-state.json";

/// A window's position and size, in logical pixels. Not `gpui::Bounds<Pixels>` directly --
/// that type isn't `serde`-serializable, and decoupling the persisted format from `gpui`'s
/// own types means a future `gpui` upgrade can't silently change what's on disk. `crate::main`
/// converts to/from `gpui::Bounds<Pixels>` where it actually opens the window.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The subset of shell state that survives restart. `Default` (via `Noun`/`RailMode`'s own
/// defaults and `window: None`) is "nothing persisted yet".
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PersistedState {
    pub noun: Noun,
    pub primary_rail: RailMode,
    pub window: Option<WindowGeometry>,
}

/// Where the state file lives on this platform: `dirs::state_dir()` (Linux's
/// `$XDG_STATE_HOME`) where that exists, else `dirs::data_local_dir()` (macOS/Windows, which
/// have no separate "state" concept) -- `None` only if the platform gives no usable directory
/// at all.
pub fn state_file_path() -> Option<PathBuf> {
    dirs::state_dir()
        .or_else(dirs::data_local_dir)
        .map(|dir| dir.join(APPLICATION_NAME).join(STATE_FILE_NAME))
}

/// Loads the persisted state from `path`. A missing, unreadable, or unparseable file is
/// treated the same as "nothing persisted yet" (`PersistedState::default()`) rather than a
/// startup error -- there's nothing a user could do to "fix" a corrupt transient-state file,
/// and refusing to launch over it would be worse than just resetting it.
pub fn load_from(path: &Path) -> PersistedState {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

/// Loads from the real platform state file (see [`state_file_path`]), or the default state if
/// the platform has no usable directory.
pub fn load() -> PersistedState {
    state_file_path()
        .map(|path| load_from(&path))
        .unwrap_or_default()
}

/// Saves `state` to `path`, creating its parent directory if needed.
pub fn save_to(path: &Path, state: &PersistedState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(state)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    std::fs::write(path, contents)
}

/// Saves to the real platform state file (see [`state_file_path`]). A no-op (not an error) if
/// the platform has no usable directory -- there's nowhere to put it, and losing restart
/// persistence isn't worth failing shutdown over.
pub fn save(state: &PersistedState) -> io::Result<()> {
    match state_file_path() {
        Some(path) => save_to(&path, state),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_missing_file_returns_default() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let path = dir.path().join("does-not-exist.json");

        let state = load_from(&path);

        assert_eq!(state, PersistedState::default());
    }

    #[test]
    fn load_from_corrupt_file_returns_default() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let path = dir.path().join("desktop-state.json");
        std::fs::write(&path, "not valid json").expect("write should succeed");

        let state = load_from(&path);

        assert_eq!(state, PersistedState::default());
    }

    #[test]
    fn save_then_load_round_trips_through_a_nested_new_directory() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let path = dir.path().join("nested").join(STATE_FILE_NAME);
        let state = PersistedState {
            noun: Noun::Accounts,
            primary_rail: RailMode::Collapsed,
            window: Some(WindowGeometry {
                x: 10.0,
                y: 20.0,
                width: 1280.0,
                height: 800.0,
            }),
        };

        save_to(&path, &state).expect("save should create the parent dir and write the file");
        let loaded = load_from(&path);

        assert_eq!(loaded, state);
    }

    #[test]
    fn save_then_load_round_trips_a_window_less_state() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let path = dir.path().join(STATE_FILE_NAME);
        let state = PersistedState {
            noun: Noun::Settings,
            primary_rail: RailMode::Expanded,
            window: None,
        };

        save_to(&path, &state).expect("save should succeed");
        let loaded = load_from(&path);

        assert_eq!(loaded, state);
    }
}
