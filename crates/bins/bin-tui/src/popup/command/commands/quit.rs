//! The Quit domain — a single command, exiting the app (`docs/ux/tui/README.md`'s global `Q`).

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[Command {
    name: "quit",
    chord: Chord(&[KeyCode::Char('Q')]),
    description: "quit the app",
    args: &[],
}];
