//! What the integration tests read back of the command palette, kept apart from `shell.rs` so
//! the snapshot shape and its accessor sit together.

use lib_shell::PaletteRow;

use crate::shell::router::Root;

/// What a test can see of an open command palette.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteSnapshot {
    pub input: String,
    pub matches: Vec<&'static str>,
    /// Each match's key column, as drawn (`—` while unbound).
    pub keys: Vec<String>,
    pub selected: Option<&'static str>,
}

impl Root {
    /// The command palette's state for a test: `None` while it is closed.
    #[doc(hidden)]
    pub fn palette_snapshot(&self) -> Option<PaletteSnapshot> {
        let model = self.presenter.palette()?;
        let entries: Vec<_> = model
            .rows
            .into_iter()
            .filter_map(|row| match row {
                PaletteRow::Entry(entry) => Some(entry),
                PaletteRow::Header(_) => None,
            })
            .collect();
        Some(PaletteSnapshot {
            input: model.input,
            matches: entries.iter().map(|entry| entry.name).collect(),
            keys: entries.iter().map(|entry| entry.key.clone()).collect(),
            selected: entries
                .iter()
                .find(|entry| entry.selected)
                .map(|entry| entry.name),
        })
    }
}
