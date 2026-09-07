//! The Balance Checks domain — commands grounded in what
//! `crate::screen::balance_checks_list`/`balance_check_detail` actually support
//! (list/new/edit/delete/import); `docs/ux/tui/README.md` calls this domain "reconcile" in
//! its navigation jump (`g k`) but "check" in its command grammar — the real screen's own
//! `i` import key (CSV import, FR.33) has no README precedent, so it's carried over as-is.

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "check list",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('k')]),
        description: "balance checks and clearing",
    },
    Command {
        name: "check new <account> <date> <balance>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "record a balance check",
    },
    Command {
        name: "check edit",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted balance check",
    },
    Command {
        name: "check delete",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "delete the highlighted balance check",
    },
    Command {
        name: "check import <path.csv>",
        chord: Chord(&[KeyCode::Char('i')]),
        description: "import balance checks from a CSV file",
    },
];
