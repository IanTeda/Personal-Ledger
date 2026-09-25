//! The Payees domain — the `:payee` grammar `docs/ux/tui/payees/README.md` "Command grammar"
//! specifies, grounded in what `view::payees::PayeesView` and its four popups
//! (`crate::popup::payee`) actually support now that all of "Payees screen, views and popup"
//! (issue #134) is built. Bindings are the list's own real keys (`g p`, `n`, `e`, `m`, `a`,
//! `d`), not the old flat-Payees screen's aspirational ones this domain used to describe
//! (`screen::payees_list`/`payee_detail` are retired — see "Retire old
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

use super::{
    Arg, Chord, Command, CommandId, list_selection_inactive_preview, list_selection_preview,
    off_description,
};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::Payee,
        name: "payee",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('p')]),
        description: crate::msg::tui_command_payee_description,
        args: &[],
    },
    Command {
        id: CommandId::PayeeNew,
        name: "payee new <name>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: crate::msg::tui_command_payee_new_description,
        args: &[Arg {
            placeholder: "<name>",
            preview: crate::msg::tui_command_preview_payee_new,
        }],
    },
    Command {
        id: CommandId::PayeeEdit,
        name: "payee edit <payee>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: crate::msg::tui_command_payee_edit_description,
        args: &[Arg {
            placeholder: "<payee>",
            preview: crate::msg::tui_command_preview_payee_edit,
        }],
    },
    Command {
        id: CommandId::PayeeRename,
        name: "payee rename <payee> <new>",
        chord: Chord::NONE,
        description: crate::msg::tui_command_payee_rename_description,
        args: &[Arg {
            placeholder: "<new>",
            preview: crate::msg::tui_command_preview_payee_rename,
        }],
    },
    Command {
        id: CommandId::PayeeMatch,
        name: "payee match <payee>",
        chord: Chord(&[KeyCode::Char('m')]),
        description: crate::msg::tui_command_payee_match_description,
        args: &[Arg {
            placeholder: "<payee>",
            preview: crate::msg::tui_command_preview_payee_match,
        }],
    },
    Command {
        id: CommandId::PayeeMatchAdd,
        name: "payee match add <payee> <text>",
        chord: Chord::NONE,
        description: crate::msg::tui_command_payee_match_add_description,
        args: &[Arg {
            placeholder: "<text>",
            preview: crate::msg::tui_command_preview_payee_match_add,
        }],
    },
    Command {
        id: CommandId::PayeeDefault,
        name: "payee default <payee> <category>",
        chord: Chord::NONE,
        description: crate::msg::tui_command_payee_default_description,
        args: &[Arg {
            placeholder: "<category>",
            preview: crate::msg::tui_command_preview_payee_default,
        }],
    },
    Command {
        id: CommandId::PayeeOff,
        name: "payee off <payee>",
        chord: Chord(&[KeyCode::Char('a')]),
        description: off_description,
        args: &[Arg {
            placeholder: "<payee>",
            preview: list_selection_preview,
        }],
    },
    Command {
        id: CommandId::PayeeOn,
        name: "payee on <payee>",
        chord: Chord::NONE,
        description: on_description,
        args: &[Arg {
            placeholder: "<payee>",
            preview: list_selection_inactive_preview,
        }],
    },
    Command {
        id: CommandId::PayeeDelete,
        name: "payee delete <payee>",
        chord: Chord(&[KeyCode::Char('d')]),
        description: crate::msg::tui_command_payee_delete_description,
        args: &[Arg {
            placeholder: "<payee>",
            preview: crate::msg::tui_command_preview_payee_delete,
        }],
    },
];

/// `:payee on`'s description names the command it reverses, which stays a stable English id.
fn on_description() -> String {
    crate::msg::tui_command_on_description("payee off")
}
