//! The Help domain — a single command, opening the Help `View` (`crate::view::help`).

use crossterm::event::KeyCode;

use super::{Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[Command {
    id: CommandId::Help,
    name: "help",
    chord: Chord(&[KeyCode::Char('?')]),
    description: crate::msg::tui_command_help_description,
    args: &[],
}];
