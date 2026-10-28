//! The Settings domain (`docs/ux/tui-mockups/05-settings/README.md` § Command grammar):
//! `settings [page]`, opening the Settings index, or one page by its command name
//! (`crate::view::settings::SettingsPage::command_name`); `set <field> [value]`, opening a form
//! page's field in place or saving a value to it, through the one `Action::SetSetting` the
//! field's own keys send; and `sync now|edit` and `backup now|export|git`, which run the Sync
//! Server and Backup pages' keys on their page (`crate::view::settings::PageCommand`). `Tab`
//! completes `[page]` and `<field>`, and the subcommands as the command names they are.
//!
//! `undo` and `redo` stay out: History's undo needs its own design first (#897).

use crossterm::event::KeyCode;

use super::{Arg, Chord, Command, CommandId};
use crate::view::settings::SettingsPage;
use crate::view::settings::field::SettingField;

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::Settings,
        name: "settings [page]",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('s')]),
        description: crate::msg::tui_command_settings_description,
        args: &[Arg {
            placeholder: "[page]",
            preview: crate::msg::tui_command_preview_settings_page,
        }],
    },
    Command {
        id: CommandId::Set,
        name: "set <field> [value]",
        chord: Chord(&[KeyCode::Enter]),
        description: crate::msg::tui_command_set_description,
        args: &[
            Arg {
                placeholder: "<field>",
                preview: crate::msg::tui_command_preview_set_field,
            },
            Arg {
                placeholder: "[value]",
                preview: crate::msg::tui_command_preview_set_value,
            },
        ],
    },
    Command {
        id: CommandId::SyncNow,
        name: "sync now",
        chord: Chord(&[KeyCode::Char('s')]),
        description: crate::msg::tui_command_sync_now_description,
        args: &[],
    },
    Command {
        id: CommandId::SyncEdit,
        name: "sync edit",
        chord: Chord(&[KeyCode::Char('e')]),
        description: crate::msg::tui_command_sync_edit_description,
        args: &[],
    },
    Command {
        id: CommandId::BackupNow,
        name: "backup now",
        chord: Chord(&[KeyCode::Char('b')]),
        description: crate::msg::tui_command_backup_now_description,
        args: &[],
    },
    Command {
        id: CommandId::BackupExport,
        name: "backup export",
        chord: Chord(&[KeyCode::Char('X')]),
        description: crate::msg::tui_command_backup_export_description,
        args: &[],
    },
    Command {
        id: CommandId::BackupGit,
        name: "backup git",
        chord: Chord(&[KeyCode::Char('e')]),
        description: crate::msg::tui_command_backup_git_description,
        args: &[],
    },
];

/// The values `[page]` completes on, in the index's order.
pub fn pages() -> Vec<&'static str> {
    SettingsPage::ALL
        .iter()
        .map(|page| page.command_name())
        .collect()
}

/// The values `<field>` completes on, in the form pages' row order.
pub fn fields() -> Vec<&'static str> {
    SettingField::ALL
        .iter()
        .map(|field| field.command_name())
        .collect()
}
