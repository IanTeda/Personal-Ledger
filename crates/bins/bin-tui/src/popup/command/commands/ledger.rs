//! The Ledger domain — opening, creating and closing a Ledger file
//! (`docs/ux/tui-mockups/01-shell/README.md`'s 1a). The path is the one argument the command popup
//! reads as typed (`CommandPopup::argument`), since a file can't be picked from a list.

use super::{Arg, Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::LedgerOpen,
        name: "open [path]",
        chord: Chord::NONE,
        description: crate::msg::tui_command_open_description,
        args: &[Arg {
            placeholder: "[path]",
            preview: crate::msg::tui_command_preview_open_path,
        }],
    },
    Command {
        id: CommandId::LedgerNew,
        name: "new <path>",
        chord: Chord::NONE,
        description: crate::msg::tui_command_new_description,
        args: &[Arg {
            placeholder: "<path>",
            preview: crate::msg::tui_command_preview_new_path,
        }],
    },
    Command {
        id: CommandId::LedgerClose,
        name: "close",
        chord: Chord::NONE,
        description: crate::msg::tui_command_close_description,
        args: &[],
    },
];
