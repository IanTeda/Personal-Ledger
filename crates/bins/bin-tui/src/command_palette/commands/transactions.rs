//! The Transactions domain — the one domain `docs/ux/tui/README.md` documents in full
//! (its `:help` window TRANSACTIONS group), so its wording and bindings come straight from
//! there, including `a` (not `n`) for "new" — a deliberate exception, since adding a
//! transaction is meant to work from any screen, not just this list.

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "txn recent",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('t')]),
        description: "last 50 transactions, all accounts",
    },
    Command {
        name: "txn new [account]",
        chord: Chord(&[KeyCode::Char('a')]),
        description: "add a transaction from anywhere",
    },
    Command {
        name: "txn edit",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted transaction",
    },
    Command {
        name: "txn delete",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "delete — confirms by payee and amount",
    },
];
