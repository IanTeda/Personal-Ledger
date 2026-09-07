//! The Units domain — commands grounded in what `crate::screen::units_list`/`unit_detail`
//! actually support (list/new/edit/delete); bindings are `docs/ux/tui/README.md`'s own
//! documented scheme for this domain (`g u`, and `n` for `:unit new` verbatim from its
//! `:help` window example).

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "unit list",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('u')]),
        description: "units, details and weekly prices",
    },
    Command {
        name: "unit new <code> <type>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "add a unit — code is permanent",
    },
    Command {
        name: "unit edit <code>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted unit",
    },
    Command {
        name: "unit delete <code>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "delete — units in use can't be removed",
    },
];
