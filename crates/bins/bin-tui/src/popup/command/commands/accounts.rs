//! The Accounts domain — the `:account` grammar `docs/ux/tui/accounts/README.md` "Command
//! grammar" specifies (that doc's own text writes it as `:acct`; the command name is
//! `:account`, matching every other domain's own full-word convention — `unit`, `category`,
//! `budget`, `payee`, none abbreviated), grounded in what `view::accounts::AccountsView` and
//! its three popups (`crate::popup::account`) actually support now that all of "Accounts
//! screen, views and popup" (issue #115) is built. Bindings are the list's own real keys (`g
//! a`, `n`, `e`, `d`, `a`), not the old flat-Accounts screen's aspirational ones this domain
//! used to describe (`screen::accounts_list`/`account_detail` are retired — see
//! "Retire old Screen-architecture Accounts code").
//!
//! **Dispatch is narrower than the grammar's own argument lists suggest**, for exactly the
//! reason `commands::categories`'s own module doc gives: the command popup has no real
//! argument-typing, so every entry here that reaches real behaviour does it against whatever's
//! currently selected on the Accounts list (`View::account_selection`), never against a typed
//! `<name>`/`<acct>`/`<type>`/`<unit>`. `account new` opens the new popup blank (there's no
//! "parent" to prefill from a selection the way `category new` has); `account edit`/`delete`
//! open their popup for the selection; `account off`/`on` apply immediately, also to the
//! selection. `account check` (Balance Check/reconcile, `b`) is out of this map's destination
//! entirely (README §*Not yet designed*) and has no special-cased arm in `Shell`, falling
//! through to the existing "not yet built" message — same as `category rename`/`merge`/`tree`.

use crossterm::event::KeyCode;

use super::{
    Arg, Chord, Command, CommandId, list_selection_inactive_preview, list_selection_preview,
    off_description,
};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::Account,
        name: "account",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('a')]),
        description: crate::msg::tui_command_account_description,
        args: &[],
    },
    Command {
        id: CommandId::AccountNew,
        name: "account new <name> <type> <unit>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: crate::msg::tui_command_account_new_description,
        args: &[Arg {
            placeholder: "<name>",
            preview: crate::msg::tui_command_preview_account_new,
        }],
    },
    Command {
        id: CommandId::AccountEdit,
        name: "account edit <acct>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: crate::msg::tui_command_account_edit_description,
        args: &[Arg {
            placeholder: "<acct>",
            preview: crate::msg::tui_command_preview_account_edit,
        }],
    },
    Command {
        id: CommandId::AccountDelete,
        name: "account delete <acct> [into <acct>]",
        chord: Chord(&[KeyCode::Char('d')]),
        description: crate::msg::tui_command_account_delete_description,
        args: &[Arg {
            placeholder: "<acct>",
            preview: delete_preview,
        }],
    },
    Command {
        id: CommandId::AccountOff,
        name: "account off <acct>",
        chord: Chord(&[KeyCode::Char('a')]),
        description: off_description,
        args: &[Arg {
            placeholder: "<acct>",
            preview: list_selection_preview,
        }],
    },
    Command {
        id: CommandId::AccountOn,
        name: "account on <acct>",
        chord: Chord::NONE,
        description: on_description,
        args: &[Arg {
            placeholder: "<acct>",
            preview: list_selection_inactive_preview,
        }],
    },
    Command {
        id: CommandId::AccountCheck,
        name: "account check <acct> <amount> [date]",
        chord: Chord(&[KeyCode::Char('b')]),
        description: crate::msg::tui_command_account_check_description,
        args: &[Arg {
            placeholder: "<amount>",
            preview: check_preview,
        }],
    },
];

/// `:account on`'s description names the command it reverses, which stays a stable English id.
fn on_description() -> String {
    crate::msg::tui_command_on_description("account off")
}

/// `:account delete`'s preview names the optional transfer-target argument its own usage line
/// spells, so the two always read the same.
fn delete_preview() -> String {
    crate::msg::tui_command_preview_account_delete("[into <acct>]")
}

/// `:account check` has no design yet, so its preview points at the doc that will carry one.
fn check_preview() -> String {
    crate::msg::tui_command_preview_account_check("docs/ux/tui/accounts/README.md")
}
