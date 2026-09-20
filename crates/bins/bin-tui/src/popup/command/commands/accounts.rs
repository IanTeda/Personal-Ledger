//! The Accounts domain — the `:account` grammar `docs/ux/tui/accounts/README.md` "Command
//! grammar" specifies (that doc's own text writes it as `:acct`; the command name is
//! `:account`, matching every other domain's own full-word convention — `unit`, `category`,
//! `budget`, `payee`, none abbreviated), grounded in what `view::accounts::AccountsView` and
//! its three popups (`crate::popup::account`) actually support now that all of "Accounts
//! screen, views and popup" (issue #115) is built. Bindings are the list's own real keys (`g
//! a`, `n`, `e`, `d`, `a`), not the old flat-Accounts screen's aspirational ones this domain
//! used to describe (`crate::screen::accounts_list`/`account_detail` are retired — see
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

use super::{Arg, Chord, Command, CommandId};

pub const COMMANDS: &[Command] = &[
    Command {
        id: CommandId::Account,
        name: "account",
        chord: Chord(&[KeyCode::Char('g'), KeyCode::Char('a')]),
        description: "accounts grouped by type, per-unit subtotals, ledger",
        args: &[],
    },
    Command {
        id: CommandId::AccountNew,
        name: "account new <name> <type> <unit>",
        chord: Chord(&[KeyCode::Char('n')]),
        description: "add an account — opens the new popup, blank",
        args: &[Arg {
            placeholder: "<name>",
            preview: "e.g. Everyday Spending, Mortgage Offset",
        }],
    },
    Command {
        id: CommandId::AccountEdit,
        name: "account edit <acct>",
        chord: Chord(&[KeyCode::Char('e')]),
        description: "edit the highlighted account",
        args: &[Arg {
            placeholder: "<acct>",
            preview: "Everyday Spending · bank · AUD 4 210.65",
        }],
    },
    Command {
        id: CommandId::AccountDelete,
        name: "account delete <acct> [into <acct>]",
        chord: Chord(&[KeyCode::Char('d')]),
        description: "delete — a non-empty account needs a same-unit transfer target",
        args: &[Arg {
            placeholder: "<acct>",
            preview: "Everyday Spending · 1 284 txns — needs [into <acct>]",
        }],
    },
    Command {
        id: CommandId::AccountOff,
        name: "account off <acct>",
        chord: Chord(&[KeyCode::Char('a')]),
        description: "is_active = 0 — hides it from the list unless za",
        args: &[Arg {
            placeholder: "<acct>",
            preview: "the list selection",
        }],
    },
    Command {
        id: CommandId::AccountOn,
        name: "account on <acct>",
        chord: Chord::NONE,
        description: "is_active = 1 — reverses account off",
        args: &[Arg {
            placeholder: "<acct>",
            preview: "the list selection, with za held to see it",
        }],
    },
    Command {
        id: CommandId::AccountCheck,
        name: "account check <acct> <amount> [date]",
        chord: Chord(&[KeyCode::Char('b')]),
        description: "records a Balance Check, prints the variance",
        args: &[Arg {
            placeholder: "<amount>",
            preview: "not yet designed — docs/ux/tui/accounts/README.md",
        }],
    },
];
