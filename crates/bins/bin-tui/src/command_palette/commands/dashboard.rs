//! The Dashboard domain — a single command, the app's home view (`crate::view::dashboard`).

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[Command {
    name: "dashboard",
    chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('d')]),
    description: "financial position — the default view",
}];
