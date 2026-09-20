//! The Payees domain — the `:payee` grammar `docs/ux/tui/payees/README.md` "Command grammar"
//! specifies, grounded in what `view::payees::PayeesView` and its four popups
//! (`crate::popup::payee`) actually support now that all of "Payees screen, views and popup"
//! (issue #134) is built. Bindings are the list's own real keys (`g p`, `n`, `e`, `m`, `a`,
//! `d`), not the old flat-Payees screen's aspirational ones this domain used to describe
//! (`crate::screen::payees_list`/`payee_detail` are retired — see "Retire old
//! Screen-architecture Payees code").
//!
//! **Dispatch is narrower than the grammar's own argument lists suggest**, for exactly the
//! reason `commands::accounts`'s own module doc gives: the command popup has no real
//! argument-typing, so every entry here that reaches real behaviour does it against whatever's
//! currently selected on the Payees list (`View::payee_selection`), never against a typed
//! `<payee>`/`<name>`. `payee new` opens the new popup blank (no selection prerequisite —
//! guarded by `View::payee_store` being `Some` instead, mirroring `account new`/`tag new`);
//! `payee edit`/`match`/`delete` open their popup for the selection; `payee off`/`on` apply
//! immediately via `Action::SetPayeeActive`, the exact same `PayeeStore::set_active` path the
//! list's own bare `a` key uses. `payee rename <payee> <new>` (needs a typed new name),
//! `payee match add <payee> <text>` (needs typed alias text) and `payee default <payee>
//! <category>` (needs a typed category) all have no path to real dispatch yet and fall through
//! to the existing "not yet built" message — genuinely blocked on real argument-typing, not an
//! oversight, matching Categories' own "rename"/"merge"/"tree".

use crossterm::event::KeyCode;

use super::{Arg, Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::Payee,
        name: "payee",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('p')]),
        description: "payees ranked by spend, curation flags, record, matches",
        args: &[],
    },
    Command {
        id: CommandId::PayeeNew,
        name: "payee new <name>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "add a payee — opens the new popup, blank",
        args: &[Arg {
            placeholder: "<name>",
            preview: "e.g. Woolworths, ATO, Telstra",
        }],
    },
    Command {
        id: CommandId::PayeeEdit,
        name: "payee edit <payee>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted payee",
        args: &[Arg {
            placeholder: "<payee>",
            preview: "Woolworths · -18 402.55 · 184 txns",
        }],
    },
    Command {
        id: CommandId::PayeeRename,
        name: "payee rename <payee> <new>",
        chord: Chord::NONE,
        description: "rename — keeps the old name as a match",
        args: &[Arg {
            placeholder: "<new>",
            preview: "e.g. Woolworths Group",
        }],
    },
    Command {
        id: CommandId::PayeeMatch,
        name: "payee match <payee>",
        chord: Chord(&[KeyCode::Char('m')]),
        description: "rename matches — add/edit/remove, live resolution test",
        args: &[Arg {
            placeholder: "<payee>",
            preview: "Woolworths · 2 matches",
        }],
    },
    Command {
        id: CommandId::PayeeMatchAdd,
        name: "payee match add <payee> <text>",
        chord: Chord::NONE,
        description: "adds an exact-text match — refuses on cross-payee collision",
        args: &[Arg {
            placeholder: "<text>",
            preview: "e.g. WW Metro",
        }],
    },
    Command {
        id: CommandId::PayeeDefault,
        name: "payee default <payee> <category>",
        chord: Chord::NONE,
        description: "sets the default category — pre-fills it on transaction entry",
        args: &[Arg {
            placeholder: "<category>",
            preview: "e.g. food/groceries",
        }],
    },
    Command {
        id: CommandId::PayeeOff,
        name: "payee off <payee>",
        chord: Chord(&[KeyCode::Char('a')]),
        description: "is_active = 0 — hides it from the list unless za",
        args: &[Arg {
            placeholder: "<payee>",
            preview: "the list selection",
        }],
    },
    Command {
        id: CommandId::PayeeOn,
        name: "payee on <payee>",
        chord: Chord::NONE,
        description: "is_active = 1 — reverses payee off",
        args: &[Arg {
            placeholder: "<payee>",
            preview: "the list selection, with za held to see it",
        }],
    },
    Command {
        id: CommandId::PayeeDelete,
        name: "payee delete <payee>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "opens the delete popup — the database refuses a referenced payee",
        args: &[Arg {
            placeholder: "<payee>",
            preview: "Woolworths · 184 txns · 2 matches — refused",
        }],
    },
];
