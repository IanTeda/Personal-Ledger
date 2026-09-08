//! The Categories domain — commands grounded in what `crate::screen::categories_list`/
//! `category_detail` actually support (list/new/edit/delete); bindings are
//! `docs/ux/tui/README.md`'s aspirational scheme (the list's `g c` jump, `n`/`e`/`d` for the
//! verbs), not the old screens' own real keys — that scheme belongs to the navigation model
//! (ADR-0013) already being replaced.

use crossterm::event::KeyCode;

use super::{Chord, Command};

pub const COMMANDS: &[Command] = &[
    Command {
        name: "category list",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('c')]),
        description: "categories and 30-day totals",
    },
    Command {
        name: "category new <name> <type>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "add a category",
    },
    Command {
        name: "category edit <name>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted category",
    },
    Command {
        name: "category delete <name>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "delete — categories in use can't be removed",
    },
];
