//! The Payees domain — commands grounded in what `crate::screen::payees_list`/`payee_detail`
//! actually support (list/new/edit/delete). Payee is a first-class entity (ADR-0012), but
//! predates `docs/ux/tui/README.md`'s own navigation scheme, so it has no documented jump —
//! `g p` follows the scheme's own pattern (first letter of the entity).

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "payee list",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('p')]),
        description: "payees and their transaction totals",
    },
    Command {
        name: "payee new <name>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "add a payee",
    },
    Command {
        name: "payee edit <name>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted payee",
    },
    Command {
        name: "payee delete <name>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "delete — payees in use can't be removed",
    },
];
