//! The Quit domain — a single command, exiting the app (`docs/ux/tui/README.md`'s global `Q`).

use crossterm::event::KeyCode;

use super::{Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[Command {
    id: CommandId::Quit,
    name: "quit",
    chord: Chord(&[KeyCode::Char('Q')]),
    description: "quit the app",
    args: &[],
}];
