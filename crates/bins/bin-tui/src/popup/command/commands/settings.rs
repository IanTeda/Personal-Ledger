//! The Settings domain — a single command, opening `crate::view::settings` (`docs/ux/tui/
//! settings/README.md` §4a). That view is itself wireframe-stage: the database-backed
//! registry, in-place editor and base-unit guard the README also specifies (`:set <key>
//! <value>`, `:settings log`, `:settings reset <key>`) aren't built yet, so only the "at rest"
//! screen has a command behind it, matching `dashboard`/`help`'s own single-command domains.

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[Command {
    name: "settings",
    chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('s')]),
    description: "groups, overrides and the settings table",
    args: &[],
}];
