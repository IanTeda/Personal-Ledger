//! The Toasts domain — dismissing the Toast overlay (`crate::toast`). `dismiss all` is also the
//! `dismiss_toasts` binding (`Ctrl+L`), which `Chord` cannot show, so both read `—`.

use super::{Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::Dismiss,
        name: "dismiss",
        chord: Chord::NONE,
        description: crate::msg::tui_command_dismiss_description,
        args: &[],
    },
    Command {
        id: CommandId::DismissAll,
        name: "dismiss all",
        chord: Chord::NONE,
        description: crate::msg::tui_command_dismiss_all_description,
        args: &[],
    },
];
