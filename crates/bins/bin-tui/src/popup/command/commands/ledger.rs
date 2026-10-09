//! The Ledger domain — loading and closing a ledger (`docs/ux/tui-mockups/01-chrome/README.md`'s
//! 1a resting state). All three only flip `Shell`'s `ledger_open` for now, mirroring
//! `bin-desktop`'s `NavState::open_ledger`/`close_ledger`: real file I/O is later work.

use super::{Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::LedgerOpen,
        name: "open",
        chord: Chord::NONE,
        description: crate::msg::tui_command_open_description,
        args: &[],
    },
    Command {
        id: CommandId::LedgerNew,
        name: "new",
        chord: Chord::NONE,
        description: crate::msg::tui_command_new_description,
        args: &[],
    },
    Command {
        id: CommandId::LedgerClose,
        name: "close",
        chord: Chord::NONE,
        description: crate::msg::tui_command_close_description,
        args: &[],
    },
];
