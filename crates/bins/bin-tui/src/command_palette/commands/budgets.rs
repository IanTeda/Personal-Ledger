//! The Budgets domain — commands grounded in what `crate::screen::budgets_list`/
//! `budget_detail` actually support (list/new/edit/delete); bindings and wording follow
//! `docs/ux/tui/README.md`'s own `:help` window example for this domain almost verbatim
//! (`g b`, `n`, `e`, `x` for deactivate — realised here as delete, since the real screen has
//! no separate deactivate verb).

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "budget list [period]",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('b')]),
        description: "budgets vs actual for the period",
    },
    Command {
        name: "budget new <category> <limit>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "start tracking a category",
    },
    Command {
        name: "budget edit <category> [limit]",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "change the limit or period",
    },
    Command {
        name: "budget delete <category>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "stop tracking a category",
    },
];
