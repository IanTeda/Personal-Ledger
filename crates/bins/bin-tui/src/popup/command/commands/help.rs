//! The Help domain — a single command, opening `crate::screen::help`.

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[Command {
    name: "help",
    chord: Chord(&[KeyCode::Char('?')]),
    description: "browse every command",
    args: &[],
}];
