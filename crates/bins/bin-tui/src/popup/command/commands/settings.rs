//! The Settings domain: `settings [page]`, opening the Settings index, or one page by its
//! command name (`crate::view::settings::SettingsPage::command_name`), which `Tab` completes.
//! The rest of the README's grammar (`set`, `sync`, `backup`) is Stage 5 of its work plan.

use crossterm::event::KeyCode;

use super::{Arg, Chord, Command, CommandId};
use crate::view::settings::SettingsPage;

pub const COMMANDS: &[Command] = &[Command {
    id: CommandId::Settings,
    name: "settings [page]",
    chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('s')]),
    description: crate::msg::tui_command_settings_description,
    args: &[Arg {
        placeholder: "[page]",
        preview: crate::msg::tui_command_preview_settings_page,
    }],
}];

/// The values `[page]` completes on, in the index's order.
pub fn pages() -> Vec<&'static str> {
    SettingsPage::ALL
        .iter()
        .map(|page| page.command_name())
        .collect()
}
