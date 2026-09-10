//! The Reports domain — `crate::screen::reports` is one screen with an internal picker
//! across five real report kinds (Account Balance, Category Total, Payee Total, Budget vs
//! Actual, Balance Check Variance) rather than `docs/ux/tui/README.md`'s own aspirational
//! `category`/`payee`/`networth`/`variance` sub-verbs, which don't match what's built. Only
//! opening the screen has a global jump (`g r`); a specific report is reached via the
//! screen's own Tab/Left/Right picker, so each has no binding yet.

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "report list",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('r')]),
        description: "spending report and charts",
        args: &[],
    },
    Command {
        name: "report account-balance",
        chord: Chord::NONE,
        description: "every account's current balance",
        args: &[],
    },
    Command {
        name: "report category-total",
        chord: Chord::NONE,
        description: "spending by category over a range",
        args: &[],
    },
    Command {
        name: "report payee-total",
        chord: Chord::NONE,
        description: "spending by payee over a range",
        args: &[],
    },
    Command {
        name: "report budget-variance",
        chord: Chord::NONE,
        description: "budgets vs actual for the current period",
        args: &[],
    },
    Command {
        name: "report balance-check-variance",
        chord: Chord::NONE,
        description: "asserted vs computed balance, by check",
        args: &[],
    },
];
