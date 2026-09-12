//! The Settings domain — a single command, opening `crate::view::settings` (`docs/ux/tui/
//! settings/README.md` §4a). That view, and the `e`/`enter` popups it opens
//! (`crate::popup::settings`, §4b/§4c), are wireframe-stage: the database-backed registry the
//! README's own command grammar assumes (`:set <key> <value>`, `:settings log`, `:settings
//! reset <key>`) isn't built yet, so only the "at rest" screen has a command behind it here,
//! matching `dashboard`/`help`'s own single-command domains.

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[Command {
    name: "settings",
    chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('s')]),
    description: "groups, overrides and the settings table",
    args: &[],
}];
