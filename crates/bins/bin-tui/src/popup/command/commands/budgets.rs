//! The Budgets domain — commands grounded in what `crate::screen::budgets_list`/
//! `budget_detail` actually support (list/new/edit/delete); bindings and wording follow
//! `docs/ux/tui/README.md`'s own `:help` window example for this domain almost verbatim
//! (`g b`, `n`, `e`, `x` for deactivate — realised here as delete, since the real screen has
//! no separate deactivate verb).

use crossterm::event::KeyCode;

use super::{Arg, Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "budget list [period]",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('b')]),
        description: "budgets vs actual for the period",
        args: &[Arg {
            placeholder: "[period]",
            preview: "current: SEP 2026 · optional, defaults to this period",
        }],
    },
    Command {
        name: "budget new <category> <limit>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "start tracking a category",
        args: &[Arg {
            placeholder: "<category>",
            preview: "e.g. dining, groceries — one budget per category",
        }],
    },
    Command {
        name: "budget edit <category> [limit]",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "change the limit or period",
        args: &[Arg {
            placeholder: "<category>",
            preview: "dining · limit 300.00 · actual 412.00 · over by 112.00",
        }],
    },
    Command {
        name: "budget delete <category>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "stop tracking a category",
        args: &[Arg {
            placeholder: "<category>",
            preview: "dining · limit 300.00 — stops tracking, keeps past transactions",
        }],
    },
];
