//! The Dashboard domain — a single command, the app's home view (`crate::view::dashboard`).

use crossterm::event::KeyCode;

use super::{Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[Command {
    id: CommandId::Dashboard,
    name: "dashboard",
    chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('d')]),
    description: crate::msg::tui_command_dashboard_description,
    args: &[],
}];
