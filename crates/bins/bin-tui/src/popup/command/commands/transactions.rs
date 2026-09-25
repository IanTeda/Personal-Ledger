//! The Transactions domain — the one domain `docs/ux/tui/README.md` documents in full
//! (its `:help` window TRANSACTIONS group), so its wording and bindings come straight from
//! there, including `a` (not `n`) for "new" — a deliberate exception, since adding a
//! transaction is meant to work from any screen, not just this list.

use crossterm::event::KeyCode;

use super::{Arg, Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::TxnRecent,
        name: "txn recent",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('t')]),
        description: recent_description,
        args: &[],
    },
    Command {
        id: CommandId::TxnNew,
        name: "txn new [account]",
        chord: Chord(&[KeyCode::Char('a')]),
        description: crate::msg::tui_command_txn_new_description,
        args: &[Arg {
            placeholder: "[account]",
            preview: crate::msg::tui_command_preview_txn_new,
        }],
    },
    Command {
        id: CommandId::TxnEdit,
        name: "txn edit",
        chord: Chord(&[KeyCode::Char('e')]),
        description: crate::msg::tui_command_txn_edit_description,
        args: &[],
    },
    Command {
        id: CommandId::TxnDelete,
        name: "txn delete",
        chord: Chord(&[KeyCode::Char('d')]),
        description: crate::msg::tui_command_txn_delete_description,
        args: &[],
    },
];

/// How many transactions `:txn recent` shows, named in its own description.
const RECENT_LIMIT: i64 = 50;

/// `:txn recent`'s description, whose noun agrees with the count it names.
fn recent_description() -> String {
    crate::msg::tui_command_txn_recent_description(RECENT_LIMIT)
}
