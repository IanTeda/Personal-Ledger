//! What the integration tests read back of the command palette, kept apart from `shell.rs` so
//! the snapshot shape and its accessor sit together.

use crate::shell::Shell;

/// What a test can see of an open command palette.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteSnapshot {
    pub input: String,
    pub matches: Vec<&'static str>,
    pub selected: Option<&'static str>,
}

impl Shell {
    /// The command palette's state for a test: `None` while it is closed.
    #[doc(hidden)]
    pub fn palette_snapshot(&self) -> Option<PaletteSnapshot> {
        self.palette.as_ref().map(|palette| PaletteSnapshot {
            input: palette.input().to_string(),
            matches: palette.match_names(),
            selected: palette.selected_command().map(|command| command.name),
        })
    }
}
