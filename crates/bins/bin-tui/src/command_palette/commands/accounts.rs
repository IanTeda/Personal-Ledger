//! The Accounts domain — commands grounded in what `crate::screen::accounts_list`/
//! `account_detail` actually support (list/new/edit/delete); bindings are
//! `docs/ux/tui/README.md`'s aspirational scheme (the list's `g a` jump, `n`/`e`/`d` for the
//! verbs), not the old screens' own real keys — that scheme belongs to the navigation model
//! (ADR-0013) already being replaced.

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "account list",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('a')]),
        description: "accounts and their balances",
    },
    Command {
        name: "account new <name> <type>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "add an account",
    },
    Command {
        name: "account edit <name>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted account",
    },
    Command {
        name: "account delete <name>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "delete — accounts in use can't be removed",
    },
];
